use super::*;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::atomic::AtomicUsize;

/// Memory block metadata
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
    NextFit
}

/// Core allocator structure
pub struct Allocator<A: Architecture> {
    blocks: BTreeMap<PhysicalAddress, Block>,
    strategy: AllocationStrategy,
    total_memory: usize,
    used_memory: AtomicUsize,
    last_address: PhysicalAddress,
    _phantom: PhantomData<A>
}

impl<A: Architecture> Allocator<A> {
    pub fn new(total_memory: usize) -> Self {
        // To be implemented
        todo!()
    }

    pub fn allocate(&mut self, size: usize) -> Result<MemoryHandle, MemoryError> {
        // To be implemented
        todo!()
    }

    pub fn deallocate(&mut self, handle: MemoryHandle) -> Result<(), MemoryError> {
        // To be implemented
        todo!()
    }

    pub fn get_stats(&self) -> AllocatorStats {
        // To be implemented
        todo!()
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
            assert!(matches!(result.get_stats(),
                AllocatorStats { total_memory: 0, .. }));
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
            let handles: Vec<_> = (0..5).map(|i| {
                allocator.allocate(1024).expect(&format!("Allocation {} failed", i))
            }).collect();

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

            let handle = allocator.allocate(Arch64::WORD_SIZE * 3)
                .expect("Aligned allocation failed");

            // Address should be aligned to word size
            assert_eq!(handle.0 % Arch64::WORD_SIZE, 0);
        }

        #[test]
        fn test_out_of_memory() {
            let mut allocator = create_allocator::<Arch64>(AllocationStrategy::FirstFit);

            let result = allocator.allocate(TEST_MEMORY_SIZE + 1);
            assert!(matches!(result, Err(MemoryError::OutOfMemory {
                requested: _,
                available: _
            })));
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
                allocator.deallocate(handles[i]).expect("Deallocation failed");
            }

            // Attempt to allocate a large block
            let large_allocation = allocator.allocate(TEST_MEMORY_SIZE / 2);
            assert!(matches!(large_allocation, Err(MemoryError::FragmentationError(_))));
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
            allocator.deallocate(handle).expect("First deallocation failed");
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
                allocator.deallocate(handles[i]).expect("Deallocation failed");
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