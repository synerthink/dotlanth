use crate::memory;

use super::*;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::atomic::AtomicUsize;

/// Memory block metadata
#[derive(Clone)]
struct Block {
    size: usize,
    address: PhysicalAddress,
    is_free: bool,
}

/// Memory allocation algorithm types
#[derive(Debug, Clone, Copy)]
pub enum AllocationStrategy {
    FirstFit,
    BestFit,
    NextFit,
}

/// Core allocator structure
pub struct Allocator<A: Architecture> {
    blocks: BTreeMap<PhysicalAddress, Block>,
    strategy: AllocationStrategy,
    total_memory: usize,
    used_memory: AtomicUsize,
    last_address: PhysicalAddress,
    _phantom: PhantomData<A>,
}

impl<A: Architecture> Allocator<A> {
    /// Constructor to create a new Allocator instance
    pub fn new(total_memory: usize) -> Self {
        if total_memory == 0 {
            // Special case for zero-sized memory
            return Self {
                blocks: BTreeMap::new(),
                strategy: AllocationStrategy::FirstFit,
                total_memory: 0,
                used_memory: AtomicUsize::new(0),
                last_address: memory::PhysicalAddress(0),
                _phantom: PhantomData,
            };
        }

        // Create a map to store memory blocks.
        let mut blocks = BTreeMap::new();

        // The initial block represents the entire memory as one large free block
        let initial_block = Block {
            size: total_memory,
            address: PhysicalAddress(0), // Başlangıç adresini 0 yap
            is_free: true,
        };
        blocks.insert(memory::PhysicalAddress(0), initial_block);

        // Return an instance of the Allocator with the initial state
        Self {
            blocks,
            strategy: AllocationStrategy::FirstFit, // Default allocation strategy
            total_memory,
            used_memory: AtomicUsize::new(0), // No memory is used yet
            last_address: memory::PhysicalAddress(0),
            _phantom: PhantomData,
        }
    }

    pub fn get_allocation_size(&self, handle: MemoryHandle) -> Result<usize, MemoryError> {
        let addr = PhysicalAddress::new(handle.0);
        self.blocks
            .get(&addr)
            .map(|block| block.size)
            .ok_or(MemoryError::InvalidHandle)
    }

    fn align_up(size: usize, align: usize) -> usize {
        (size + align - 1) & !(align - 1)
    }

    /// Function to allocate a block of memory
    pub fn allocate(&mut self, size: usize) -> Result<MemoryHandle, MemoryError> {
        if size == 0 {
            return Err(MemoryError::AllocationFailed("Size cannot be zero".into()));
        }

        if size % A::ALIGNMENT != 0 {
            return Err(MemoryError::InvalidAlignment(size % A::ALIGNMENT));
        }

        // Get the maximum size from the Architecture
        if size > A::MAX_MEMORY {
            return Err(MemoryError::AllocationTooLarge {
                requested: size,
                maximum: A::MAX_MEMORY,
            });
        }

        // Check total memory first
        let aligned_size = Self::align_up(size, A::ALIGNMENT);
        let available_memory = self.total_memory - self.used_memory.load(Ordering::SeqCst);

        if aligned_size > available_memory {
            return Err(self.create_out_of_memory_error(aligned_size));
        }

        // Then perform fragmentation check
        let max_contiguous = self.get_max_contiguous_free_block();
        if max_contiguous < aligned_size {
            // aligned_size kullan
            return Err(MemoryError::FragmentationError(format!(
                "Maximum contiguous block size: {}",
                max_contiguous
            )));
        }

        match self.strategy {
            AllocationStrategy::FirstFit => self.first_fit_allocate(size),
            AllocationStrategy::BestFit => self.best_fit_allocate(size),
            AllocationStrategy::NextFit => self.next_fit_allocate(size),
        }
    }

    // New helper method
    fn get_max_contiguous_free_block(&self) -> usize {
        self.blocks
            .values()
            .filter(|b| b.is_free)
            .map(|b| b.size)
            .max()
            .unwrap_or(0)
    }

    fn first_fit_allocate(&mut self, size: usize) -> Result<MemoryHandle, MemoryError> {
        let mut allocation_info = None;

        // Find the first suitable block
        for (&address, block) in self.blocks.iter() {
            if block.is_free && block.size >= size {
                allocation_info = Some((address, block.size));
                break;
            }
        }

        if let Some((address, block_size)) = allocation_info {
            self.allocate_block(address, block_size, size)
        } else {
            Err(self.create_out_of_memory_error(size))
        }
    }

    fn best_fit_allocate(&mut self, size: usize) -> Result<MemoryHandle, MemoryError> {
        let mut best_fit = None;
        let mut smallest_difference = usize::MAX;

        // Find the best fit
        for (&address, block) in self.blocks.iter() {
            if block.is_free && block.size >= size {
                let difference = block.size - size;
                if difference < smallest_difference {
                    smallest_difference = difference;
                    best_fit = Some((address, block.size));
                }
            }
        }

        if let Some((address, block_size)) = best_fit {
            self.allocate_block(address, block_size, size)
        } else {
            Err(self.create_out_of_memory_error(size))
        }
    }

    fn next_fit_allocate(&mut self, size: usize) -> Result<MemoryHandle, MemoryError> {
        let mut allocation_info = None;

        // Check blocks after the last address
        for (&address, block) in self.blocks.range(self.last_address..) {
            if block.is_free && block.size >= size {
                allocation_info = Some((address, block.size));
                break;
            }
        }

        // If not found, start from the beginning
        if allocation_info.is_none() {
            for (&address, block) in self.blocks.range(..self.last_address) {
                if block.is_free && block.size >= size {
                    allocation_info = Some((address, block.size));
                    break;
                }
            }
        }

        if let Some((address, block_size)) = allocation_info {
            self.last_address = address;
            self.allocate_block(address, block_size, size)
        } else {
            Err(self.create_out_of_memory_error(size))
        }
    }

    fn allocate_block(
        &mut self,
        address: PhysicalAddress,
        block_size: usize,
        requested_size: usize,
    ) -> Result<MemoryHandle, MemoryError> {
        // Split and allocate the block
        if block_size > requested_size {
            let remaining_size = block_size - requested_size;
            let new_block_address = memory::PhysicalAddress(address.as_usize() + requested_size);

            let new_block = Block {
                size: remaining_size,
                address: new_block_address,
                is_free: true,
            };

            if let Some(block) = self.blocks.get_mut(&address) {
                block.size = requested_size;
                block.is_free = false;
            }

            self.blocks.insert(new_block_address, new_block);
        } else if let Some(block) = self.blocks.get_mut(&address) {
            block.is_free = false;
        }

        self.used_memory
            .fetch_add(requested_size, std::sync::atomic::Ordering::SeqCst);
        Ok(MemoryHandle(address.as_usize()))
    }

    fn create_out_of_memory_error(&self, requested: usize) -> MemoryError {
        MemoryError::OutOfMemory {
            requested,
            available: self.total_memory
                - self.used_memory.load(std::sync::atomic::Ordering::SeqCst),
        }
    }

    /// Function to get allocator statistics
    pub fn get_stats(&self) -> AllocatorStats {
        let used_memory = self.used_memory.load(std::sync::atomic::Ordering::SeqCst);
        let free_memory = self.total_memory - used_memory;

        // Allocated blocks count
        let allocation_count = self.blocks.values().filter(|b| !b.is_free).count();

        // Calculate fragmentation ratio
        // If no memory is used (new allocator), fragmentation ratio should be 0.0
        let fragmentation_ratio = if used_memory == 0 {
            0.0
        } else {
            let free_blocks: Vec<_> = self.blocks.values().filter(|b| b.is_free).collect();
            if free_memory > 0 && !free_blocks.is_empty() {
                // Find the largest free block size
                let largest_free_block = free_blocks.iter().map(|b| b.size).max().unwrap_or(0);
                // Fragmentation ratio = 1 - (largest free block / total free space)
                1.0 - (largest_free_block as f64 / free_memory as f64)
            } else {
                0.0
            }
        };

        AllocatorStats {
            total_memory: self.total_memory,
            used_memory,
            free_memory,
            allocation_count,
            fragmentation_ratio,
        }
    }

    /// Function to deallocate a previously allocated block of memory
    pub fn deallocate(&mut self, handle: MemoryHandle) -> Result<(), MemoryError> {
        let address = memory::PhysicalAddress(handle.0);

        // Step 1: Retrieve a mutable reference to the block and check if it exists
        if let Some(mut block) = self.blocks.get_mut(&address) {
            if block.is_free {
                return Err(MemoryError::AlreadyDeallocated); // Memory block is already free
            }

            // Mark the block as free
            block.is_free = true;

            // Decrease the used memory counter
            self.used_memory
                .fetch_sub(block.size, std::sync::atomic::Ordering::SeqCst);

            // Save current block address and size into temporary variables
            let current_block_size = block.size;
            let current_block_address = block.address;

            // End the mutable borrow so we can take new mutable references later
            let _ = block;

            // Coalescing: Merge with the next (right) block if it is free
            let next_address =
                memory::PhysicalAddress::new(current_block_address.as_usize() + current_block_size);
            if let Some(next_block) = self.blocks.get(&next_address).cloned() {
                if next_block.is_free {
                    // Re-borrow the block to update its size
                    if let Some(mut block) = self.blocks.get_mut(&address) {
                        block.size += next_block.size; // Add the size of the next block
                    }
                    self.blocks.remove(&next_address); // Remove the next block from the map
                }
            }

            // Coalescing: Merge with the previous (left) block if it is free
            if let Some((&previous_address, mut previous_block)) =
                self.blocks.range_mut(..current_block_address).rev().next()
            {
                if previous_block.is_free {
                    // Merge the current block with the previous block
                    previous_block.size += current_block_size;
                    self.blocks.remove(&current_block_address); // Remove the current block
                }
            }

            Ok(())
        } else {
            // If the block is not found, return an InvalidHandle error
            Err(MemoryError::InvalidHandle)
        }
    }
}

/// Statistics about memory usage
pub struct AllocatorStats {
    pub total_memory: usize,
    pub used_memory: usize,
    pub free_memory: usize,
    pub allocation_count: usize,
    pub fragmentation_ratio: f64,
}

#[cfg(test)]
mod allocator_tests {
    use super::*;
    use std::sync::atomic::Ordering;

    const TEST_MEMORY_SIZE: usize = 1024 * 1024; // 1MB for testing

    fn create_allocator<A: Architecture>(strategy: AllocationStrategy) -> Allocator<A> {
        Allocator::new(TEST_MEMORY_SIZE)
    }

    mod initialization_tests {
        use super::*;

        #[test]
        fn test_new_allocator() {
            let allocator = create_allocator::<Arch64>(AllocationStrategy::FirstFit);
            let stats = allocator.get_stats();

            assert_eq!(stats.total_memory, TEST_MEMORY_SIZE);
            assert_eq!(stats.used_memory, 0);
            assert_eq!(stats.free_memory, TEST_MEMORY_SIZE);
            assert_eq!(stats.allocation_count, 0);
            assert_eq!(stats.fragmentation_ratio, 0.0);
        }

        #[test]
        fn test_invalid_memory_size() {
            let result = Allocator::<Arch64>::new(0);
            assert!(matches!(
                result.get_stats(),
                AllocatorStats {
                    total_memory: 0,
                    ..
                }
            ));
        }
    }

    mod allocation_strategy_tests {
        use super::*;

        #[test]
        fn test_first_fit_strategy() {
            let mut allocator = create_allocator::<Arch64>(AllocationStrategy::FirstFit);

            // Allocate blocks of different sizes
            let handle1 = allocator.allocate(1024).expect("First allocation failed");
            let handle2 = allocator.allocate(2048).expect("Second allocation failed");

            // Free first block and try to allocate a smaller block
            allocator.deallocate(handle1).expect("Deallocation failed");
            let handle3 = allocator.allocate(512).expect("Third allocation failed");

            // Should use the first free block even though it's larger than needed
            let stats = allocator.get_stats();
            assert!(stats.fragmentation_ratio > 0.0);
        }

        #[test]
        fn test_best_fit_strategy() {
            let mut allocator = create_allocator::<Arch64>(AllocationStrategy::BestFit);

            // Create gaps of different sizes
            let handle1 = allocator.allocate(1024).expect("First allocation failed");
            let handle2 = allocator.allocate(2048).expect("Second allocation failed");
            let handle3 = allocator.allocate(512).expect("Third allocation failed");

            // Free middle block to create a gap
            allocator.deallocate(handle2).expect("Deallocation failed");

            // Allocate a block that fits better in the third block's space, note that
            // The allocation size must be a multiple of 8 (the alignment size)
            let handle4 = allocator.allocate(504).expect("Fourth allocation failed");

            // Best fit should minimize fragmentation
            let stats = allocator.get_stats();
            assert!(stats.fragmentation_ratio < 0.2);
        }

        #[test]
        fn test_next_fit_strategy() {
            let mut allocator = create_allocator::<Arch64>(AllocationStrategy::NextFit);

            // Create several blocks
            let handles: Vec<_> = (0..5)
                .map(|i| {
                    allocator
                        .allocate(1024)
                        .expect(&format!("Allocation {} failed", i))
                })
                .collect();

            // Free alternate blocks
            for (i, handle) in handles.iter().enumerate() {
                if i % 2 == 0 {
                    allocator.deallocate(*handle).expect("Deallocation failed");
                }
            }

            // Next allocations should start from last position
            let new_handle = allocator.allocate(1024).expect("New allocation failed");
            let stats = allocator.get_stats();
            assert!(stats.allocation_count > 0);
        }
    }

    mod memory_management_tests {
        use super::*;

        #[test]
        fn test_basic_allocation() {
            let mut allocator = create_allocator::<Arch64>(AllocationStrategy::FirstFit);

            let handle = allocator.allocate(1024).expect("Allocation failed");
            let stats = allocator.get_stats();

            assert_eq!(stats.used_memory, 1024);
            assert_eq!(stats.allocation_count, 1);
        }

        #[test]
        fn test_aligned_allocation() {
            let mut allocator = create_allocator::<Arch64>(AllocationStrategy::FirstFit);

            let handle = allocator
                .allocate(Arch64::WORD_SIZE * 3)
                .expect("Aligned allocation failed");

            // Address should be aligned to word size
            assert_eq!(handle.0 % Arch64::WORD_SIZE, 0);
        }

        #[test]
        fn test_out_of_memory() {
            let mut allocator = create_allocator::<Arch64>(AllocationStrategy::FirstFit);
            let result = allocator.allocate(TEST_MEMORY_SIZE + 8); // 8 byte alignment is required
            assert!(matches!(
                result,
                Err(MemoryError::OutOfMemory {
                    requested: _,
                    available: _
                })
            ));
        }

        #[test]
        fn test_fragmentation_handling() {
            let mut allocator = create_allocator::<Arch64>(AllocationStrategy::FirstFit);
            let mut handles = Vec::new();

            // Fill all memory with small blocks
            let block_size = 64;
            let total_blocks = TEST_MEMORY_SIZE / block_size;
            for _ in 0..total_blocks {
                handles.push(
                    allocator
                        .allocate(block_size)
                        .expect("Small allocation failed"),
                );
            }

            // Free every other block
            for i in (0..handles.len()).step_by(2) {
                allocator
                    .deallocate(handles[i])
                    .expect("Deallocation failed");
            }

            // Attempt to allocate a large block
            let large_allocation = allocator.allocate(TEST_MEMORY_SIZE / 2);
            assert!(matches!(
                large_allocation,
                Err(MemoryError::FragmentationError(_))
            ));
        }
    }

    mod deallocation_tests {
        use super::*;

        #[test]
        fn test_basic_deallocation() {
            let mut allocator = create_allocator::<Arch64>(AllocationStrategy::FirstFit);

            let handle = allocator.allocate(1024).expect("Allocation failed");
            allocator.deallocate(handle).expect("Deallocation failed");

            let stats = allocator.get_stats();
            assert_eq!(stats.used_memory, 0);
            assert_eq!(stats.free_memory, TEST_MEMORY_SIZE);
        }

        #[test]
        fn test_double_deallocation() {
            let mut allocator = create_allocator::<Arch64>(AllocationStrategy::FirstFit);

            let handle = allocator.allocate(1024).expect("Allocation failed");
            allocator
                .deallocate(handle)
                .expect("First deallocation failed");
            let result = allocator.deallocate(handle);

            assert!(matches!(result, Err(MemoryError::AlreadyDeallocated)));
        }

        #[test]
        fn test_invalid_handle_deallocation() {
            let mut allocator = create_allocator::<Arch64>(AllocationStrategy::FirstFit);

            let result = allocator.deallocate(MemoryHandle(0xDEADBEEF));
            assert!(matches!(result, Err(MemoryError::InvalidHandle)));
        }
    }

    mod stats_tests {
        use super::*;

        #[test]
        fn test_allocation_stats() {
            let mut allocator = create_allocator::<Arch64>(AllocationStrategy::FirstFit);

            allocator.allocate(1024).expect("First allocation failed");
            allocator.allocate(2048).expect("Second allocation failed");

            let stats = allocator.get_stats();
            assert_eq!(stats.used_memory, 3072);
            assert_eq!(stats.free_memory, TEST_MEMORY_SIZE - 3072);
            assert_eq!(stats.allocation_count, 2);
        }

        #[test]
        fn test_fragmentation_ratio() {
            let mut allocator = create_allocator::<Arch64>(AllocationStrategy::FirstFit);
            let mut handles = Vec::new();

            // Create a fragmented state
            for _ in 0..10 {
                handles.push(allocator.allocate(64).expect("Allocation failed"));
            }

            for i in (0..handles.len()).step_by(2) {
                allocator
                    .deallocate(handles[i])
                    .expect("Deallocation failed");
            }

            let stats = allocator.get_stats();
            assert!(stats.fragmentation_ratio > 0.0);
            assert!(stats.fragmentation_ratio <= 1.0);
        }

        #[test]
        fn test_memory_tracking() {
            let mut allocator = create_allocator::<Arch64>(AllocationStrategy::FirstFit);

            // Track memory changes through multiple operations
            let handle1 = allocator.allocate(1024).expect("First allocation failed");
            let stats1 = allocator.get_stats();

            let handle2 = allocator.allocate(2048).expect("Second allocation failed");
            let stats2 = allocator.get_stats();

            allocator.deallocate(handle1).expect("Deallocation failed");
            let stats3 = allocator.get_stats();

            assert_eq!(stats1.used_memory, 1024);
            assert_eq!(stats2.used_memory, 3072);
            assert_eq!(stats3.used_memory, 2048);
        }
    }

    mod concurrent_tests {
        use super::*;
        use std::sync::Arc;
        use std::thread;

        #[test]
        fn test_atomic_memory_tracking() {
            let allocator = Arc::new(create_allocator::<Arch64>(AllocationStrategy::FirstFit));
            let allocator_clone = Arc::clone(&allocator);

            let handle = thread::spawn(move || {
                let used = allocator_clone.used_memory.load(Ordering::SeqCst);
                assert_eq!(used, 0);
            });

            let initial_used = allocator.used_memory.load(Ordering::SeqCst);
            assert_eq!(initial_used, 0);

            handle.join().expect("Thread panicked");
        }
    }
}
