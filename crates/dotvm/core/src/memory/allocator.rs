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
        assert!(total_memory > 0, "Memory size must be greater than 0");

        // Create a map to store memory blocks.
        let mut blocks = BTreeMap::new();

        // The initial block represents the entire memory as one large free block
        let initial_block = Block {
            size: total_memory,
            address: memory::PhysicalAddress(0),
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

    /// Function to allocate a block of memory
    pub fn allocate(&mut self, size: usize) -> Result<MemoryHandle, MemoryError> {
        if size == 0 {
            return Err(MemoryError::InvalidSize { available: size });
        }

        let mut split_info = None; // Temporary holder for split block info
        for (&address, block) in self.blocks.iter_mut() {
            // Check if the block is free and can accommodate the requested size
            if block.is_free && block.size >= size {
                if block.size > size {
                    // If the block is larger than needed, calculate the remainder and prepare to split
                    let remaining_size = block.size - size;
                    let new_block_address = address.as_usize() + size;
                    split_info = Some((
                        memory::PhysicalAddress(new_block_address),
                        Block {
                            size: remaining_size,
                            address: memory::PhysicalAddress(new_block_address),
                            is_free: true,
                        },
                    ));
                    block.size = size; // Resize the current block
                }
                block.is_free = false; // Mark the block as not free
                self.used_memory
                    .fetch_add(size, std::sync::atomic::Ordering::SeqCst);

                if let Some((new_block_address, new_block)) = split_info {
                    // Insert the new split block into the map
                    self.blocks.insert(new_block_address, new_block);
                }
                return Ok(MemoryHandle(address.as_usize())); // Return the memory handle
            }
        }

        // If no suitable block is found, return an OutOfMemory error
        Err(MemoryError::OutOfMemory {
            requested: size,
            available: self.total_memory
                - self.used_memory.load(std::sync::atomic::Ordering::SeqCst),
        })
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

    /// Function to retrieve allocator statistics
    pub fn get_stats(&self) -> AllocatorStats {
        let used_memory = self.used_memory.load(std::sync::atomic::Ordering::SeqCst);
        let free_memory = self.total_memory - used_memory;

        // Number of allocated blocks
        let allocation_count = self.blocks.values().filter(|b| !b.is_free).count();

        // Compute fragmentation ratio: sum of sizes of free blocks divided by total free memory
        let fragmentation_ratio = self
            .blocks
            .values()
            .filter(|b| b.is_free)
            .map(|b| b.size as f64 / free_memory as f64)
            .sum::<f64>();

        // Return statistics in an AllocatorStats struct
        AllocatorStats {
            total_memory: self.total_memory,
            used_memory,
            free_memory,
            allocation_count,
            fragmentation_ratio,
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

            // Allocate a block that fits better in the third block's space
            let handle4 = allocator.allocate(500).expect("Fourth allocation failed");

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

            let result = allocator.allocate(TEST_MEMORY_SIZE + 1);
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

            // Allocate many small blocks
            for _ in 0..100 {
                handles.push(allocator.allocate(64).expect("Small allocation failed"));
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
