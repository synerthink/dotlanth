// Dotlanth
// Copyright (C) 2025 Synerthink

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::{
    alloc::{GlobalAlloc, Layout},
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    ptr::{self, NonNull},
    sync::atomic,
    sync::{Arc, Barrier, Mutex},
    time::Instant,
};

use crate::memory::lib::{MemoryStats, align_to};
use memmap2::Mmap;
use std::fs::File;
extern crate libc;

/// Interface to NUMA (Non-Uniform Memory Access) functions
/// Used for optimizing memory allocation on NUMA architectures
#[cfg(target_os = "linux")]
unsafe extern "C" {
    /// Checks if NUMA is available on the system
    fn numa_available() -> libc::c_int;
    /// Gets the number of NUMA nodes on the system
    fn numa_num_configured_nodes() -> libc::c_int;
    /// Allocates memory on a specific NUMA node
    fn numa_alloc_onnode(size: libc::size_t, node: libc::c_int) -> *mut libc::c_void;
    /// Frees memory allocated by NUMA functions
    fn numa_free(ptr: *mut libc::c_void, size: libc::size_t);
}

/// Strategy for allocating memory blocks
/// Controls how free blocks are selected for allocation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AllocationStrategy {
    FirstFit, // Use the first block that's large enough
    BestFit,  // Use the smallest block that fits the request
}

/// Errors that can occur during memory allocation operations
#[derive(Debug)]
pub enum AllocatorError {
    OutOfMemory,      // Not enough memory available
    InvalidAlignment, // Alignment is not valid (not power of 2)
    InvalidSize,      // Size is not valid (too large or zero)
    NullPointer,      // Returned pointer was null
}

/// Internal representation of a memory block in the allocator
/// Forms a doubly-linked list of memory blocks
#[derive(Debug)]
struct Block {
    size: usize,                  // Size of this block
    is_free: bool,                // Whether this block is available
    next: Option<NonNull<Block>>, // Pointer to next block in chain
    prev: Option<NonNull<Block>>, // Pointer to previous block in chain
}

/// Information about a memory allocation
/// Used for memory profiling and leak detection
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    pub size: usize, // Size of the allocation
    pub timestamp: Instant, // When the allocation was made
                     // (Optional) stack trace or other information could be added in the future
}

/// Wrapper for memory-mapped regions
pub struct MemoryMapping {
    mmap: Mmap, // Memory-mapped region
    len: usize, // Length of the mapping
}

impl MemoryMapping {
    /// Gets a slice representing the mapped memory
    pub fn as_slice(&self) -> &[u8] {
        &self.mmap[..self.len]
    }
}

/// Custom memory allocator optimized for database operations
/// Provides fine-grained control over memory allocation strategies
pub struct CustomAllocator {
    strategy: AllocationStrategy,                                  // Allocation strategy to use
    free_blocks: Arc<Mutex<BTreeMap<usize, Vec<NonNull<Block>>>>>, // Free memory blocks by size
    allocated_blocks: Arc<Mutex<BTreeMap<usize, NonNull<Block>>>>, // Currently allocated blocks
    stats: Arc<Mutex<MemoryStats>>,                                // Memory usage statistics
    chunk_size: usize,                                             // Size of allocation chunks
    alignment: usize,                                              // Memory alignment
    allocation_map: Arc<Mutex<HashMap<usize, AllocationInfo>>>,    // Tracks allocations for profiling
    total_allocations: Arc<Mutex<u64>>,                            // Total number of allocations
    total_deallocations: Arc<Mutex<u64>>,                          // Total number of deallocations
    peak_active_allocations: Arc<Mutex<u64>>,                      // Peak number of active allocations
    max_bytes: Option<usize>,                                      // Maximum bytes the allocator can use
    used_bytes: atomic::AtomicUsize,                               // Current bytes in use
}

/// Trait for allocators that support thread-local fast allocation
pub trait ThreadLocalAlloc {
    /// Allocate memory of given size and alignment
    fn allocate(&mut self, size: usize, alignment: usize) -> Option<NonNull<u8>>;
    /// Deallocate memory
    fn deallocate(&mut self, ptr: NonNull<u8>, size: usize);
}

/// Simple thread-local arena for small allocations
/// Provides very fast allocation by pre-allocating a buffer and using bump allocation
pub struct ThreadLocalArena {
    buffer: Vec<u8>, // Pre-allocated memory buffer
    offset: usize,   // Current position in the buffer
    capacity: usize, // Total size of the buffer
}

impl ThreadLocalArena {
    /// Creates a new thread-local arena with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0u8; capacity],
            offset: 0,
            capacity,
        }
    }

    /// Allocate memory from the arena (mutable, lock-free)
    /// Uses bump allocation for extremely fast allocation
    ///
    /// # Arguments
    /// * `size` - Size of memory to allocate
    /// * `alignment` - Required alignment of the memory
    ///
    /// # Returns
    /// * `Option<NonNull<u8>>` - Pointer to allocated memory or None if out of space
    pub fn allocate_mut(&mut self, size: usize, alignment: usize) -> Option<NonNull<u8>> {
        // Calculate alignment padding
        let align_offset = (self.buffer.as_ptr() as usize + self.offset) % alignment;
        let padding = if align_offset == 0 { 0 } else { alignment - align_offset };
        let total_size = size + padding;

        // Check if we have enough space
        if self.offset + total_size > self.capacity {
            return None;
        }

        // Get pointer to the allocated memory
        let ptr = unsafe { self.buffer.as_mut_ptr().add(self.offset + padding) };
        self.offset += total_size;
        NonNull::new(ptr)
    }

    /// Reset the arena (for testing or reuse)
    /// This effectively frees all allocations at once
    pub fn reset(&mut self) {
        self.offset = 0;
    }
}

impl ThreadLocalAlloc for ThreadLocalArena {
    fn allocate(&mut self, size: usize, alignment: usize) -> Option<NonNull<u8>> {
        self.allocate_mut(size, alignment)
    }

    fn deallocate(&mut self, _ptr: NonNull<u8>, _size: usize) {
        // No-op for arena (reset on drop or manually)
        // Individual deallocations are not tracked in arena allocators
    }
}

// Thread-local instance for each thread (now with RefCell for mutability)
// This provides very fast per-thread allocations
thread_local! {
    static TLS_ARENA: RefCell<ThreadLocalArena> = RefCell::new(ThreadLocalArena::new(64 * 1024));
}

/// Slab allocator for fixed-size blocks
/// Optimized for allocating many objects of the same size
pub struct SlabAllocator {
    block_size: usize,           // Size of each block
    free_list: Vec<NonNull<u8>>, // List of free blocks
    buffer: Vec<u8>,             // Memory buffer holding all blocks
    offset: usize,               // Current offset in the buffer
    capacity: usize,             // Total capacity of the buffer
}

impl SlabAllocator {
    /// Creates a new slab allocator for fixed-size blocks
    ///
    /// # Arguments
    /// * `block_size` - Size of each block
    /// * `num_blocks` - Number of blocks to pre-allocate
    pub fn new(block_size: usize, num_blocks: usize) -> Self {
        let capacity = block_size * num_blocks;
        Self {
            block_size,
            free_list: Vec::with_capacity(num_blocks),
            buffer: vec![0u8; capacity],
            offset: 0,
            capacity,
        }
    }
}

impl ThreadLocalAlloc for SlabAllocator {
    /// Allocate a block from the slab
    /// First tries to reuse a previously freed block, then allocates a new one
    fn allocate(&mut self, _size: usize, _alignment: usize) -> Option<NonNull<u8>> {
        // Try to reuse a freed block first
        if let Some(ptr) = self.free_list.pop() {
            return Some(ptr);
        }

        // Allocate a new block if we have space
        if self.offset + self.block_size > self.capacity {
            return None;
        }

        let ptr = unsafe { self.buffer.as_mut_ptr().add(self.offset) };
        self.offset += self.block_size;
        NonNull::new(ptr)
    }

    /// Return a block to the free list for reuse
    fn deallocate(&mut self, ptr: NonNull<u8>, _size: usize) {
        self.free_list.push(ptr);
    }
}

// Thread-local slab allocator for small, fixed-size allocations (e.g., 128 bytes)
thread_local! {
    static TLS_SLAB: RefCell<SlabAllocator> = RefCell::new(SlabAllocator::new(128, 1024));
}

impl CustomAllocator {
    /// Creates a new custom allocator with the specified strategy
    pub fn new(strategy: AllocationStrategy) -> Self {
        Self {
            strategy,
            free_blocks: Arc::new(Mutex::new(BTreeMap::new())),
            allocated_blocks: Arc::new(Mutex::new(BTreeMap::new())),
            stats: Arc::new(Mutex::new(MemoryStats::default())),
            chunk_size: 64 * 1024, // 64KB chunks by default
            alignment: std::mem::align_of::<usize>(),
            allocation_map: Arc::new(Mutex::new(HashMap::new())),
            total_allocations: Arc::new(Mutex::new(0)),
            total_deallocations: Arc::new(Mutex::new(0)),
            peak_active_allocations: Arc::new(Mutex::new(0)),
            max_bytes: None,
            used_bytes: atomic::AtomicUsize::new(0),
        }
    }

    /// Sets the chunk size for this allocator
    ///
    /// # Arguments
    /// * `chunk_size` - Size of memory chunks to allocate
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        self
    }

    /// Sets the memory alignment for this allocator
    ///
    /// # Arguments
    /// * `alignment` - Memory alignment in bytes (must be power of 2)
    ///
    /// # Returns
    /// * `Self` - The allocator with the alignment set
    pub fn with_alignment(mut self, alignment: usize) -> Self {
        self.alignment = alignment;
        self
    }

    /// Sets a maximum memory limit for this allocator
    ///
    /// # Arguments
    /// * `limit` - Maximum memory in bytes that the allocator can use
    ///
    /// # Returns
    /// * `Self` - The allocator with the memory limit set
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.max_bytes = Some(limit);
        self
    }

    /// Allocates memory of the specified size and alignment
    ///
    /// # Arguments
    /// * `size` - Size of memory to allocate in bytes
    /// * `alignment` - Required alignment of the memory (must be power of 2)
    ///
    /// # Returns
    /// * `Result<NonNull<u8>, AllocatorError>` - Pointer to the allocated memory or an error
    ///
    /// # Errors
    /// * `AllocatorError::InvalidSize` - If size is zero
    /// * `AllocatorError::InvalidAlignment` - If alignment is not a power of 2
    /// * `AllocatorError::OutOfMemory` - If memory quota is exceeded or allocation fails
    pub fn allocate(&self, size: usize, alignment: usize) -> Result<NonNull<u8>, AllocatorError> {
        if size == 0 {
            return Err(AllocatorError::InvalidSize);
        }
        if !alignment.is_power_of_two() {
            return Err(AllocatorError::InvalidAlignment);
        }
        let aligned_size = align_to(size, alignment);
        // Memory quota check
        if let Some(limit) = self.max_bytes {
            let prev = self.used_bytes.load(atomic::Ordering::SeqCst);
            if prev + aligned_size > limit {
                return Err(AllocatorError::OutOfMemory);
            }
            self.used_bytes.fetch_add(aligned_size, atomic::Ordering::SeqCst);
        }
        let result = match self.strategy {
            AllocationStrategy::FirstFit => self.first_fit_allocate(aligned_size, alignment),
            AllocationStrategy::BestFit => self.best_fit_allocate(aligned_size, alignment),
        };
        if result.is_err() {
            if self.max_bytes.is_some() {
                self.used_bytes.fetch_sub(aligned_size, atomic::Ordering::SeqCst);
            }
        }
        if let Ok(ptr) = &result {
            let addr = ptr.as_ptr() as usize;
            let mut allocation_map = self.allocation_map.lock().unwrap();
            allocation_map.insert(addr, AllocationInfo { size, timestamp: Instant::now() });
            let mut total_alloc = self.total_allocations.lock().unwrap();
            *total_alloc += 1;
            let active = allocation_map.len() as u64;
            let mut peak = self.peak_active_allocations.lock().unwrap();
            if active > *peak {
                *peak = active;
            }
        }
        result
    }

    /// Deallocates previously allocated memory
    ///
    /// # Arguments
    /// * `ptr` - Pointer to the memory to deallocate
    /// * `size` - Size of the allocation (must match the original allocation size)
    ///
    /// # Returns
    /// * `Result<(), AllocatorError>` - Success or an error
    ///
    /// # Errors
    /// * `AllocatorError::InvalidSize` - If size is zero
    /// * `AllocatorError::NullPointer` - If the pointer is not found in allocated blocks
    pub fn deallocate(&self, ptr: NonNull<u8>, size: usize) -> Result<(), AllocatorError> {
        if size == 0 {
            return Err(AllocatorError::InvalidSize);
        }
        let aligned_size = align_to(size, self.alignment);
        let mut allocated_blocks = self.allocated_blocks.lock().unwrap();
        let ptr_addr = ptr.as_ptr() as usize;

        if let Some(block) = allocated_blocks.remove(&ptr_addr) {
            unsafe {
                let block_ref = block.as_ref();
                self.mark_block_free(block, block_ref.size);
            }

            let mut stats = self.stats.lock().unwrap();
            stats.record_deallocation(size);

            // Leak/profiling: Remove from allocation_map
            let mut allocation_map = self.allocation_map.lock().unwrap();
            allocation_map.remove(&ptr_addr);
            let mut total_dealloc = self.total_deallocations.lock().unwrap();
            *total_dealloc += 1;

            if self.max_bytes.is_some() {
                self.used_bytes.fetch_sub(aligned_size, atomic::Ordering::SeqCst);
            }

            Ok(())
        } else {
            Err(AllocatorError::NullPointer)
        }
    }

    /// Allocates memory using the first-fit strategy
    /// Finds the first block that's large enough for the requested size
    ///
    /// # Arguments
    /// * `size` - Size of memory to allocate in bytes
    /// * `alignment` - Required alignment of the memory
    ///
    /// # Returns
    /// * `Result<NonNull<u8>, AllocatorError>` - Pointer to the allocated memory or an error
    fn first_fit_allocate(&self, size: usize, alignment: usize) -> Result<NonNull<u8>, AllocatorError> {
        let mut free_blocks = self.free_blocks.lock().unwrap();

        // Find first suitable block
        for (&block_size, blocks) in free_blocks.iter_mut() {
            if block_size >= size {
                if let Some(block) = blocks.pop() {
                    return self.use_block(block, size, alignment, &mut free_blocks);
                }
            }
        }

        // No suitable block found, allocate new chunk
        drop(free_blocks);
        self.allocate_new_chunk(size, alignment)
    }

    /// Allocates memory using the best-fit strategy
    /// Finds the smallest block that's large enough for the requested size
    ///
    /// # Arguments
    /// * `size` - Size of memory to allocate in bytes
    /// * `alignment` - Required alignment of the memory
    ///
    /// # Returns
    /// * `Result<NonNull<u8>, AllocatorError>` - Pointer to the allocated memory or an error
    fn best_fit_allocate(&self, size: usize, alignment: usize) -> Result<NonNull<u8>, AllocatorError> {
        let mut free_blocks = self.free_blocks.lock().unwrap();
        let mut best_block: Option<(usize, NonNull<Block>)> = None;
        let mut best_size = usize::MAX;

        // Find best fitting block
        for (&block_size, blocks) in free_blocks.iter_mut() {
            if block_size >= size && block_size < best_size {
                if let Some(block) = blocks.last().copied() {
                    best_block = Some((block_size, block));
                    best_size = block_size;
                }
            }
        }

        if let Some((block_size, block)) = best_block {
            let blocks = free_blocks.get_mut(&block_size).unwrap();
            blocks.pop();
            drop(free_blocks);
            // Re-lock to pass to use_block
            let mut free_blocks = self.free_blocks.lock().unwrap();
            return self.use_block(block, size, alignment, &mut free_blocks);
        }

        // No suitable block found, allocate new chunk
        drop(free_blocks);
        self.allocate_new_chunk(size, alignment)
    }

    /// Splits a memory block when only part of it is needed
    /// Creates a new free block with the remaining memory
    ///
    /// # Arguments
    /// * `block` - The block to split
    /// * `used_size` - The size being used from the block
    /// * `free_blocks` - Map of free blocks to update with the newly created block
    fn split_block(&self, block: NonNull<Block>, used_size: usize, free_blocks: &mut BTreeMap<usize, Vec<NonNull<Block>>>) {
        unsafe {
            let block_ref = block.as_ref();
            let remaining_size = block_ref.size - used_size - std::mem::size_of::<Block>();

            if remaining_size > std::mem::size_of::<Block>() {
                let new_block_ptr = (block.as_ptr() as *mut u8).add(std::mem::size_of::<Block>()).add(used_size) as *mut Block;

                let new_block = NonNull::new_unchecked(new_block_ptr);
                ptr::write(
                    new_block_ptr,
                    Block {
                        size: remaining_size,
                        is_free: true,
                        next: block_ref.next,
                        prev: Some(block),
                    },
                );

                // Update original block
                let block_mut = block.as_ptr();
                (*block_mut).size = used_size;
                (*block_mut).next = Some(new_block);
                (*block_mut).is_free = false;

                // Use the provided free_blocks reference
                free_blocks.entry(remaining_size).or_insert_with(Vec::new).push(new_block);
            } else {
                let block_mut = block.as_ptr();
                (*block_mut).is_free = false;
            }
        }
    }

    /// Prepares a block for use by the allocator
    /// Handles block splitting if needed and updates allocator metadata
    ///
    /// # Arguments
    /// * `block` - The block to use
    /// * `size` - The size requested by the user
    /// * `_alignment` - Required alignment (not used in this function)
    /// * `free_blocks` - Map of free blocks to update
    ///
    /// # Returns
    /// * `Result<NonNull<u8>, AllocatorError>` - Pointer to the allocated memory or an error
    fn use_block(&self, block: NonNull<Block>, size: usize, _alignment: usize, free_blocks: &mut BTreeMap<usize, Vec<NonNull<Block>>>) -> Result<NonNull<u8>, AllocatorError> {
        unsafe {
            let block_ref = block.as_ref();
            let data_ptr = (block.as_ptr() as *mut u8).add(std::mem::size_of::<Block>());

            // Split block if it's significantly larger
            if block_ref.size > size + std::mem::size_of::<Block>() + 64 {
                self.split_block(block, size, free_blocks);
            }

            let mut allocated_blocks = self.allocated_blocks.lock().unwrap();
            allocated_blocks.insert(data_ptr as usize, block);

            let mut stats = self.stats.lock().unwrap();
            stats.record_allocation(size);

            Ok(NonNull::new_unchecked(data_ptr))
        }
    }

    /// Allocates a new chunk of memory when existing blocks cannot satisfy a request
    /// Creates a new memory chunk from the system allocator
    ///
    /// # Arguments
    /// * `size` - Size of memory to allocate in bytes
    /// * `alignment` - Required alignment of the memory
    ///
    /// # Returns
    /// * `Result<NonNull<u8>, AllocatorError>` - Pointer to the allocated memory or an error
    ///
    /// # Errors
    /// * `AllocatorError::InvalidAlignment` - If alignment is invalid
    /// * `AllocatorError::OutOfMemory` - If the system allocation fails
    fn allocate_new_chunk(&self, size: usize, alignment: usize) -> Result<NonNull<u8>, AllocatorError> {
        let chunk_size = std::cmp::max(self.chunk_size, size + std::mem::size_of::<Block>());
        let layout = Layout::from_size_align(chunk_size, alignment).map_err(|_| AllocatorError::InvalidAlignment)?;

        unsafe {
            let chunk_ptr = std::alloc::alloc(layout);
            if chunk_ptr.is_null() {
                return Err(AllocatorError::OutOfMemory);
            }

            let block_ptr = chunk_ptr as *mut Block;
            let block = NonNull::new_unchecked(block_ptr);

            ptr::write(
                block_ptr,
                Block {
                    size: chunk_size - std::mem::size_of::<Block>(),
                    is_free: false,
                    next: None,
                    prev: None,
                },
            );

            let data_ptr = chunk_ptr.add(std::mem::size_of::<Block>());

            // If chunk is larger than needed, split it
            if chunk_size > size + std::mem::size_of::<Block>() * 2 {
                let mut free_blocks = self.free_blocks.lock().unwrap();
                self.split_block(block, size, &mut free_blocks);
            }

            let mut allocated_blocks = self.allocated_blocks.lock().unwrap();
            allocated_blocks.insert(data_ptr as usize, block);

            let mut stats = self.stats.lock().unwrap();
            stats.record_allocation(size);

            Ok(NonNull::new_unchecked(data_ptr))
        }
    }

    /// Gets the current memory usage statistics for this allocator
    ///
    /// # Returns
    /// * `MemoryStats` - Current statistics about memory usage
    pub fn get_stats(&self) -> MemoryStats {
        self.stats.lock().unwrap().clone()
    }

    /// Coalesces adjacent free blocks in the free list
    /// Combines contiguous free blocks to reduce fragmentation
    ///
    /// # Returns
    /// * `usize` - Number of blocks that were coalesced
    pub fn defragment(&self) -> usize {
        let mut free_blocks = self.free_blocks.lock().unwrap();
        self.defragment_internal(&mut free_blocks)
    }

    /// Internal implementation of defragmentation that takes a mutex as parameter
    /// Combines contiguous free blocks to reduce fragmentation
    ///
    /// # Arguments
    /// * `free_blocks` - Map of free blocks to defragment
    ///
    /// # Returns
    /// * `usize` - Number of blocks that were coalesced
    fn defragment_internal(&self, free_blocks: &mut BTreeMap<usize, Vec<NonNull<Block>>>) -> usize {
        let mut coalesced = 0;

        // Collect all blocks and sort by address
        let mut all_blocks: Vec<_> = free_blocks.iter().flat_map(|(_, blocks)| blocks.iter().copied()).collect();
        all_blocks.sort_by_key(|block| block.as_ptr() as usize);

        // Clear free_blocks for rebuild
        free_blocks.clear();

        let mut i = 0;
        while i < all_blocks.len() {
            let mut current = all_blocks[i];
            let mut current_size = unsafe { current.as_ref().size };
            let mut j = i + 1;
            while j < all_blocks.len() {
                let next = all_blocks[j];
                let expected_addr = (current.as_ptr() as usize) + std::mem::size_of::<Block>() + current_size;
                if next.as_ptr() as usize == expected_addr {
                    // Merge next into current
                    let next_size = unsafe { next.as_ref().size };
                    current_size += std::mem::size_of::<Block>() + next_size;
                    unsafe {
                        current.as_ptr().as_mut().unwrap().size = current_size;
                    }
                    coalesced += 1;
                    j += 1;
                } else {
                    break;
                }
            }
            // Add the (possibly merged) block back to free_blocks
            free_blocks.entry(current_size).or_insert_with(Vec::new).push(current);
            i = j;
        }
        coalesced
    }

    /// Marks a block as free and adds it to the free list
    /// Also attempts to coalesce with adjacent free blocks
    ///
    /// # Arguments
    /// * `block` - The block to mark as free
    /// * `size` - The size of the block
    fn mark_block_free(&self, block: NonNull<Block>, size: usize) {
        let mut free_blocks = self.free_blocks.lock().unwrap();
        // Add the block to the free list first
        unsafe {
            let block_mut = block.as_ptr();
            (*block_mut).is_free = true;
        }
        free_blocks.entry(size).or_insert_with(Vec::new).push(block);
        // Perform eager coalescing using the locked mutex
        let _ = self.defragment_internal(&mut free_blocks);
    }

    /// Allocates memory using thread-local storage for small allocations
    /// Uses a hierarchical strategy: thread-local slab → thread-local arena → global allocator
    ///
    /// # Arguments
    /// * `size` - Size of memory to allocate in bytes
    /// * `alignment` - Required alignment of the memory
    ///
    /// # Returns
    /// * `Result<NonNull<u8>, AllocatorError>` - Pointer to the allocated memory or an error
    pub fn allocate_thread_local(&self, size: usize, alignment: usize) -> Result<NonNull<u8>, AllocatorError> {
        // Use thread-local slab for small, fixed-size allocations
        if size <= 128 && alignment <= 16 {
            if let Some(ptr) = TLS_SLAB.with(|slab| slab.borrow_mut().allocate(size, alignment)) {
                return Ok(ptr);
            }
        }
        // Use thread-local arena for other small allocations
        if size <= 1024 {
            if let Some(ptr) = TLS_ARENA.with(|arena| arena.borrow_mut().allocate(size, alignment)) {
                return Ok(ptr);
            }
        }
        // Fallback to global allocator
        self.allocate(size, alignment)
    }

    /// Deallocates memory allocated with thread-local storage
    /// Handles deallocation in either thread-local slabs or the global allocator
    ///
    /// # Arguments
    /// * `ptr` - Pointer to the memory to deallocate
    /// * `size` - Size of the allocation
    ///
    /// # Returns
    /// * `Result<(), AllocatorError>` - Success or an error
    pub fn deallocate_thread_local(&self, ptr: NonNull<u8>, size: usize) -> Result<(), AllocatorError> {
        // Try to deallocate to thread-local slab if possible
        if size <= 128 {
            TLS_SLAB.with(|slab| slab.borrow_mut().deallocate(ptr, size));
            return Ok(());
        }
        // Arena deallocation is no-op, so always fallback to global for others
        self.deallocate(ptr, size)
    }

    /// Allocates memory on a specific NUMA node for improved memory locality
    /// Uses the libnuma library to allocate memory on specific nodes
    ///
    /// # Arguments
    /// * `size` - Size of memory to allocate in bytes
    /// * `alignment` - Required alignment of the memory
    /// * `node` - The NUMA node ID to allocate on
    ///
    /// # Returns
    /// * `Result<NonNull<u8>, AllocatorError>` - Pointer to the allocated memory or an error
    ///
    /// # Errors
    /// * `AllocatorError::OutOfMemory` - If NUMA allocation fails or is not available
    /// * `AllocatorError::InvalidAlignment` - If the allocation doesn't satisfy the alignment
    #[cfg(target_os = "linux")]
    pub fn allocate_on_numa_node(&self, size: usize, alignment: usize, node: usize) -> Result<NonNull<u8>, AllocatorError> {
        unsafe {
            if numa_available() == -1 {
                return Err(AllocatorError::OutOfMemory);
            }
            let ptr = numa_alloc_onnode(size, node as libc::c_int);
            if ptr.is_null() {
                return Err(AllocatorError::OutOfMemory);
            }
            // Alignment check (optional)
            if (ptr as usize) % alignment != 0 {
                numa_free(ptr, size);
                return Err(AllocatorError::InvalidAlignment);
            }
            Ok(NonNull::new_unchecked(ptr as *mut u8))
        }
    }

    /// Deallocates memory allocated with allocate_on_numa_node
    /// Properly frees memory allocated by the NUMA subsystem
    ///
    /// # Arguments
    /// * `ptr` - Pointer to the memory to deallocate
    /// * `size` - Size of the allocation
    #[cfg(target_os = "linux")]
    pub fn deallocate_numa(&self, ptr: NonNull<u8>, size: usize) {
        unsafe {
            numa_free(ptr.as_ptr() as *mut libc::c_void, size);
        }
    }

    /// Reports memory leaks and allocation statistics
    /// Prints detailed information about unfreed allocations for debugging
    /// Also provides statistics about allocation patterns during execution
    pub fn report_leaks(&self) {
        let allocation_map = self.allocation_map.lock().unwrap();
        let total_alloc = *self.total_allocations.lock().unwrap();
        let total_dealloc = *self.total_deallocations.lock().unwrap();
        let peak = *self.peak_active_allocations.lock().unwrap();
        println!("\n[Allocator Profiling]");
        println!("Total allocations: {}", total_alloc);
        println!("Total deallocations: {}", total_dealloc);
        println!("Peak active allocations: {}", peak);
        println!("Active allocations (possible leaks): {}", allocation_map.len());
        if !allocation_map.is_empty() {
            println!("Leaked allocations:");
            for (addr, info) in allocation_map.iter() {
                println!("  ptr=0x{:x}, size={}, allocated_at={:?}", addr, info.size, info.timestamp);
            }
        }
    }

    /// Memory-maps a file for zero-copy access
    /// Creates a memory mapping that allows direct access to file contents without copying
    ///
    /// # Arguments
    /// * `path` - Path to the file to map
    ///
    /// # Returns
    /// * `Result<MemoryMapping, AllocatorError>` - The memory mapping or an error
    ///
    /// # Errors
    /// * `AllocatorError::NullPointer` - If file opening, metadata retrieval, or mapping fails
    pub fn map_file(&self, path: &str) -> Result<MemoryMapping, AllocatorError> {
        let file = File::open(path).map_err(|_| AllocatorError::NullPointer)?;
        let len = file.metadata().map_err(|_| AllocatorError::NullPointer)?.len() as usize;
        let mmap = unsafe { Mmap::map(&file).map_err(|_| AllocatorError::NullPointer)? };
        Ok(MemoryMapping { mmap, len })
    }
}

unsafe impl GlobalAlloc for CustomAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self.allocate(layout.size(), layout.align()) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Some(non_null) = NonNull::new(ptr) {
            let _ = self.deallocate(non_null, layout.size());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn test_allocator_creation() {
        let allocator = CustomAllocator::new(AllocationStrategy::FirstFit);
        let stats = allocator.get_stats();
        assert_eq!(stats.allocated, 0);
        assert_eq!(stats.current_usage, 0);
    }

    #[test]
    fn test_first_fit_allocation() {
        let allocator = CustomAllocator::new(AllocationStrategy::FirstFit);

        let ptr1 = allocator.allocate(100, 8).unwrap();
        let ptr2 = allocator.allocate(200, 8).unwrap();

        assert_ne!(ptr1.as_ptr(), ptr2.as_ptr());

        let stats = allocator.get_stats();
        assert_eq!(stats.allocation_count, 2);
    }

    #[test]
    fn test_best_fit_allocation() {
        let allocator = CustomAllocator::new(AllocationStrategy::BestFit);

        let ptr1 = allocator.allocate(100, 8).unwrap();
        let ptr2 = allocator.allocate(50, 8).unwrap();

        assert_ne!(ptr1.as_ptr(), ptr2.as_ptr());

        let stats = allocator.get_stats();
        assert_eq!(stats.allocation_count, 2);
    }

    #[test]
    fn test_deallocation() {
        let allocator = CustomAllocator::new(AllocationStrategy::FirstFit);

        let ptr = allocator.allocate(100, 8).unwrap();
        assert!(allocator.deallocate(ptr, 100).is_ok());

        let stats = allocator.get_stats();
        assert_eq!(stats.deallocation_count, 1);
    }

    #[test]
    fn test_invalid_allocation() {
        let allocator = CustomAllocator::new(AllocationStrategy::FirstFit);

        // Zero size allocation
        assert!(allocator.allocate(0, 8).is_err());

        // Invalid alignment
        assert!(allocator.allocate(100, 3).is_err());
    }

    #[test]
    fn test_allocation_strategies() {
        let first_fit = CustomAllocator::new(AllocationStrategy::FirstFit);
        let best_fit = CustomAllocator::new(AllocationStrategy::BestFit);

        let _ptr1 = first_fit.allocate(100, 8).unwrap();
        let _ptr2 = best_fit.allocate(100, 8).unwrap();

        // Both should successfully allocate
        assert_eq!(first_fit.get_stats().allocation_count, 1);
        assert_eq!(best_fit.get_stats().allocation_count, 1);
    }

    #[test]
    fn test_custom_chunk_size() {
        let allocator = CustomAllocator::new(AllocationStrategy::FirstFit).with_chunk_size(128 * 1024);

        let ptr = allocator.allocate(1000, 8).unwrap();
        assert!(!ptr.as_ptr().is_null());
    }

    #[test]
    fn test_stats_tracking() {
        let allocator = CustomAllocator::new(AllocationStrategy::FirstFit);

        let ptr1 = allocator.allocate(100, 8).unwrap();
        let ptr2 = allocator.allocate(200, 8).unwrap();

        let stats = allocator.get_stats();
        assert_eq!(stats.allocation_count, 2);
        assert!(stats.current_usage > 0);

        let _ = allocator.deallocate(ptr1, 100);
        let _ = allocator.deallocate(ptr2, 200);

        let final_stats = allocator.get_stats();
        assert_eq!(final_stats.deallocation_count, 2);
    }

    #[test]
    fn test_defragmentation() {
        let allocator = CustomAllocator::new(AllocationStrategy::FirstFit);

        let ptr1 = allocator.allocate(100, 8).unwrap();
        let ptr2 = allocator.allocate(200, 8).unwrap();

        let _ = allocator.deallocate(ptr1, 100);
        let _ = allocator.deallocate(ptr2, 200);

        // Eager defragmentation should have coalesced the blocks
        let free_blocks = allocator.free_blocks.lock().unwrap();
        let total_blocks: usize = free_blocks.values().map(|v| v.len()).sum();
        assert_eq!(total_blocks, 1, "All free blocks should be coalesced into one");
    }

    #[test]
    fn test_thread_local_arena_thread_safety_safe() {
        use std::thread;
        let mut handles = vec![];
        for _ in 0..8 {
            handles.push(thread::spawn(|| {
                // Each thread uses its own TLS_ARENA
                let mut ptrs = vec![];
                for _ in 0..1000 {
                    let ptr = TLS_ARENA.with(|arena| arena.borrow_mut().allocate(64, 8)).unwrap();
                    ptrs.push(ptr);
                }
                // Deallocation no-op, arena reset will clean it
                TLS_ARENA.with(|arena| arena.borrow_mut().reset());
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_slab_allocator_basic() {
        let mut slab = SlabAllocator::new(32, 10);
        let mut ptrs = vec![];
        for _ in 0..10 {
            let ptr = slab.allocate(32, 8).unwrap();
            ptrs.push(ptr);
        }
        // Slab should be full now
        assert!(slab.allocate(32, 8).is_none());
        // Free one and allocate again
        slab.deallocate(ptrs.pop().unwrap(), 32);
        assert!(slab.allocate(32, 8).is_some());
    }

    #[test]
    fn test_defragmentation_coalescing() {
        let allocator = CustomAllocator::new(AllocationStrategy::FirstFit);
        let ptr1 = allocator.allocate(128, 8).unwrap();
        let ptr2 = allocator.allocate(128, 8).unwrap();
        let _ = allocator.deallocate(ptr1, 128);
        let _ = allocator.deallocate(ptr2, 128);
        // Eager defragmentation should have coalesced the blocks
        let free_blocks = allocator.free_blocks.lock().unwrap();
        let total_blocks: usize = free_blocks.values().map(|v| v.len()).sum();
        assert_eq!(total_blocks, 1, "All free blocks should be coalesced into one");
    }

    #[test]
    fn test_leak_detection_and_profiling() {
        let allocator = CustomAllocator::new(AllocationStrategy::FirstFit);
        let ptr1 = allocator.allocate(256, 8).unwrap();
        let ptr2 = allocator.allocate(256, 8).unwrap();
        let _ = allocator.deallocate(ptr1, 256);
        // ptr2 intentionally not deallocated (leak)
        allocator.report_leaks();
        let allocation_map = allocator.allocation_map.lock().unwrap();
        assert!(allocation_map.contains_key(&(ptr2.as_ptr() as usize)));
    }

    #[test]
    fn test_defragmentation_performance() {
        // Test parameters
        const NUM_ALLOCATIONS: usize = 1000;
        const BLOCK_SIZES: [usize; 4] = [64, 128, 256, 512];
        const NUM_ITERATIONS: usize = 5;

        // Test both eager and lazy defragmentation
        let mut eager_times = Vec::new();
        let mut lazy_times = Vec::new();
        let mut eager_fragmentation = Vec::new();
        let mut lazy_fragmentation = Vec::new();

        for _ in 0..NUM_ITERATIONS {
            // Test eager defragmentation (every deallocation)
            let allocator = CustomAllocator::new(AllocationStrategy::FirstFit);
            let mut ptrs = Vec::new();
            let start = Instant::now();

            // Allocate blocks
            for _ in 0..NUM_ALLOCATIONS {
                let size = BLOCK_SIZES[rand::random::<usize>() % BLOCK_SIZES.len()];
                let ptr = allocator.allocate(size, 8).unwrap();
                ptrs.push((ptr, size));
            }

            // Deallocate every other block
            for i in (0..ptrs.len()).step_by(2) {
                let (ptr, size) = ptrs[i];
                allocator.deallocate(ptr, size).unwrap();
            }

            // Force eager defragmentation by modifying mark_block_free
            let mut free_blocks = allocator.free_blocks.lock().unwrap();
            let mut coalesced = 0;
            let mut all_blocks: Vec<_> = free_blocks.iter().flat_map(|(_, blocks)| blocks.iter().copied()).collect();
            all_blocks.sort_by_key(|block| block.as_ptr() as usize);
            free_blocks.clear();

            let mut i = 0;
            while i < all_blocks.len() {
                let mut current = all_blocks[i];
                let mut current_size = unsafe { current.as_ref().size };
                let mut j = i + 1;
                while j < all_blocks.len() {
                    let next = all_blocks[j];
                    let expected_addr = (current.as_ptr() as usize) + std::mem::size_of::<Block>() + current_size;
                    if next.as_ptr() as usize == expected_addr {
                        let next_size = unsafe { next.as_ref().size };
                        current_size += std::mem::size_of::<Block>() + next_size;
                        unsafe {
                            current.as_ptr().as_mut().unwrap().size = current_size;
                        }
                        coalesced += 1;
                        j += 1;
                    } else {
                        break;
                    }
                }
                free_blocks.entry(current_size).or_insert_with(Vec::new).push(current);
                i = j;
            }

            let eager_time = start.elapsed();
            eager_times.push(eager_time);
            eager_fragmentation.push(coalesced);

            // Test lazy defragmentation (manual)
            let allocator = CustomAllocator::new(AllocationStrategy::FirstFit);
            let mut ptrs = Vec::new();
            let start = Instant::now();

            // Allocate blocks
            for _ in 0..NUM_ALLOCATIONS {
                let size = BLOCK_SIZES[rand::random::<usize>() % BLOCK_SIZES.len()];
                let ptr = allocator.allocate(size, 8).unwrap();
                ptrs.push((ptr, size));
            }

            // Deallocate every other block
            for i in (0..ptrs.len()).step_by(2) {
                let (ptr, size) = ptrs[i];
                allocator.deallocate(ptr, size).unwrap();
            }

            // Manual defragmentation
            let coalesced = allocator.defragment();
            let lazy_time = start.elapsed();
            lazy_times.push(lazy_time);
            lazy_fragmentation.push(coalesced);
        }

        // Calculate averages
        let avg_eager_time: Duration = eager_times.iter().sum::<Duration>() / NUM_ITERATIONS as u32;
        let avg_lazy_time: Duration = lazy_times.iter().sum::<Duration>() / NUM_ITERATIONS as u32;
        let avg_eager_fragmentation: usize = eager_fragmentation.iter().sum::<usize>() / NUM_ITERATIONS;
        let avg_lazy_fragmentation: usize = lazy_fragmentation.iter().sum::<usize>() / NUM_ITERATIONS;

        println!("\nDefragmentation Performance Test Results:");
        println!("Average Eager Defragmentation Time: {:?}", avg_eager_time);
        println!("Average Lazy Defragmentation Time: {:?}", avg_lazy_time);
        println!("Average Eager Coalesced Blocks: {}", avg_eager_fragmentation);
        println!("Average Lazy Coalesced Blocks: {}", avg_lazy_fragmentation);

        // Verify that both methods produce similar results
        assert!(
            (avg_eager_fragmentation as f64 - avg_lazy_fragmentation as f64).abs() < 5.0,
            "Eager and lazy defragmentation should produce similar results"
        );
    }

    #[test]
    fn test_memory_quota_limit() {
        let allocator = CustomAllocator::new(AllocationStrategy::FirstFit).with_memory_limit(256);
        // 128 + 128 = 256, tam sınırda
        let ptr1 = allocator.allocate(128, 8).unwrap();
        let ptr2 = allocator.allocate(128, 8).unwrap();
        // Should fail when quota exceeded
        assert!(allocator.allocate(1, 8).is_err(), "Should fail when quota exceeded");
        // Should be released and re-allocated
        let _ = allocator.deallocate(ptr1, 128);
        let ptr3 = allocator.allocate(64, 8).unwrap();
        assert!(!ptr3.as_ptr().is_null());
    }

    #[test]
    fn test_zero_copy_memory_mapping() {
        use std::io::Write;
        use tempfile::NamedTempFile;
        // Create a temporary file
        let mut file = NamedTempFile::new().unwrap();
        let data = b"hello zero-copy mmap!";
        file.write_all(data).unwrap();
        file.flush().unwrap();
        // Map the file
        let allocator = CustomAllocator::new(AllocationStrategy::FirstFit);
        let mapping = allocator.map_file(file.path().to_str().unwrap()).unwrap();
        let slice = mapping.as_slice();
        assert_eq!(&slice[..data.len()], data);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_numa_allocation() {
        let allocator = super::CustomAllocator::new(super::AllocationStrategy::FirstFit);
        let size = 4096;
        let alignment = 8;
        let num_nodes = unsafe { numa_num_configured_nodes() };
        if num_nodes < 1 {
            println!("NUMA not available or no nodes found");
            return;
        }
        // Test based on the number of nodes
        let nodes_to_test = if num_nodes == 1 { vec![0] } else { vec![0, 1] };
        for &node in &nodes_to_test {
            let result = allocator.allocate_on_numa_node(size, alignment, node as usize);
            match result {
                Ok(ptr) => {
                    assert!(!ptr.as_ptr().is_null());
                    allocator.deallocate_numa(ptr, size);
                }
                Err(e) => {
                    println!("NUMA allocation failed for node {}: {:?}", node, e);
                }
            }
        }
    }
}
