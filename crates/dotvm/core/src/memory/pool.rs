use super::*;
use std::collections::VecDeque;

/// Memory block in a pool
#[derive(Debug)]
pub struct PoolBlock {
    address: PhysicalAddress,
    pub size: usize,
}

/// Memory pool for fixed-size allocations
pub struct MemoryPool {
    block_size: usize,
    total_blocks: usize,
    free_blocks: VecDeque<PoolBlock>,
    used_blocks: usize,
}

impl MemoryPool {
    pub fn new(block_size: usize, total_size: usize) -> Result<Self, MemoryError> {
        // To be implemented
        todo!()
    }

    pub fn allocate(&mut self) -> Result<PoolBlock, MemoryError> {
        // To be implemented
        todo!()
    }

    pub fn deallocate(&mut self, block: PoolBlock) -> Result<(), MemoryError> {
        // To be implemented
        todo!()
    }

    pub fn get_stats(&self) -> PoolStats {
        // To be implemented
        todo!()
    }
}

/// Pool usage statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub block_size: usize,
    pub total_blocks: usize,
    pub used_blocks: usize,
    pub free_blocks: usize,
    pub utilization: f64,
}

/// Memory pool manager handling multiple pools
pub struct PoolManager {
    pools: Vec<MemoryPool>,
    size_classes: Vec<usize>,
}

impl PoolManager {
    pub fn new() -> Self {
        // To be implemented
        todo!()
    }

    pub fn get_pool(&mut self, size: usize) -> Option<&mut MemoryPool> {
        // To be implemented
        todo!()
    }

    pub fn create_pool(&mut self, block_size: usize, total_size: usize) -> Result<(), MemoryError> {
        // To be implemented
        todo!()
    }
}

#[cfg(test)]
mod pool_tests {
    use super::*;

    mod single_pool_tests {
        use super::*;

        #[test]
        fn test_pool_creation() {
            let pool = MemoryPool::new(64, 1024).expect("Failed to create pool");
            let stats = pool.get_stats();

            assert_eq!(stats.block_size, 64);
            assert_eq!(stats.total_blocks, 16); // 1024/64
            assert_eq!(stats.used_blocks, 0);
            assert_eq!(stats.free_blocks, 16);
            assert_eq!(stats.utilization, 0.0);
        }

        #[test]
        fn test_invalid_pool_creation() {
            // Test zero block size
            assert!(matches!(
                MemoryPool::new(0, 1024),
                Err(MemoryError::AllocationFailed(_))
            ));

            // Test zero total size
            assert!(matches!(
                MemoryPool::new(64, 0),
                Err(MemoryError::AllocationFailed(_))
            ));

            // Test total size not multiple of block size
            assert!(matches!(
                MemoryPool::new(64, 100),
                Err(MemoryError::InvalidAlignment(_))
            ));
        }

        #[test]
        fn test_basic_allocation() {
            let mut pool = MemoryPool::new(64, 1024).expect("Failed to create pool");

            let block = pool.allocate().expect("Failed to allocate block");
            let stats = pool.get_stats();

            assert_eq!(block.size, 64);
            assert_eq!(stats.used_blocks, 1);
            assert_eq!(stats.free_blocks, 15);
            assert!(stats.utilization > 0.0);
        }

        #[test]
        fn test_multiple_allocations() {
            let mut pool = MemoryPool::new(64, 256).expect("Failed to create pool");
            let mut blocks = Vec::new();

            // Allocate all blocks
            for i in 0..4 {
                let block = pool.allocate().expect("Failed to allocate block");
                assert_eq!(block.size, 64);
                blocks.push(block);
            }

            let stats = pool.get_stats();
            assert_eq!(stats.used_blocks, 4);
            assert_eq!(stats.free_blocks, 0);
            assert_eq!(stats.utilization, 1.0);
        }

        #[test]
        fn test_pool_exhaustion() {
            let mut pool = MemoryPool::new(64, 256).expect("Failed to create pool");

            // Allocate all blocks
            for _ in 0..4 {
                pool.allocate().expect("Failed to allocate block");
            }

            // Try to allocate one more block
            assert!(matches!(
                pool.allocate(),
                Err(MemoryError::OutOfMemory {
                    requested: 64,
                    available: 0
                })
            ));
        }

        #[test]
        fn test_deallocation() {
            let mut pool = MemoryPool::new(64, 256).expect("Failed to create pool");

            let block = pool.allocate().expect("Failed to allocate block");
            assert!(pool.deallocate(block).is_ok());

            let stats = pool.get_stats();
            assert_eq!(stats.used_blocks, 0);
            assert_eq!(stats.free_blocks, 4);
            assert_eq!(stats.utilization, 0.0);
        }

        #[test]
        fn test_invalid_deallocation() {
            let mut pool = MemoryPool::new(64, 256).expect("Failed to create pool");

            // Try to deallocate a block that wasn't allocated from this pool
            let invalid_block = PoolBlock {
                address: PhysicalAddress(0xDEADBEEF),
                size: 64,
            };

            assert!(matches!(
                pool.deallocate(invalid_block),
                Err(MemoryError::InvalidAddress(_))
            ));
        }

        #[test]
        fn test_block_reuse() {
            let mut pool = MemoryPool::new(64, 256).expect("Failed to create pool");

            // Allocate and deallocate a block
            let block1 = pool.allocate().expect("Failed to allocate first block");
            let addr1 = block1.address;
            pool.deallocate(block1).expect("Failed to deallocate block");

            // Allocate again - should get the same address
            let block2 = pool.allocate().expect("Failed to allocate second block");
            assert_eq!(block2.address, addr1, "Pool should reuse freed blocks");
        }
    }

    mod pool_manager_tests {
        use super::*;

        #[test]
        fn test_manager_creation() {
            let manager = PoolManager::new();
            assert!(manager.pools.is_empty());
            assert!(manager.size_classes.is_empty());
        }

        #[test]
        fn test_pool_creation() {
            let mut manager = PoolManager::new();
            assert!(manager.create_pool(64, 1024).is_ok());

            // Try to create a pool with duplicate size class
            assert!(matches!(
                manager.create_pool(64, 2048),
                Err(MemoryError::PoolError(_))
            ));
        }

        #[test]
        fn test_pool_selection() {
            let mut manager = PoolManager::new();

            // Create pools with different size classes
            manager
                .create_pool(32, 1024)
                .expect("Failed to create 32-byte pool");
            manager
                .create_pool(64, 1024)
                .expect("Failed to create 64-byte pool");
            manager
                .create_pool(128, 1024)
                .expect("Failed to create 128-byte pool");

            // Test each pool size separately to avoid multiple mutable borrows
            {
                let pool = manager.get_pool(30);
                assert!(pool.is_some());
                assert_eq!(pool.unwrap().get_stats().block_size, 32);
            }

            {
                let pool = manager.get_pool(64);
                assert!(pool.is_some());
                assert_eq!(pool.unwrap().get_stats().block_size, 64);
            }

            {
                let pool = manager.get_pool(100);
                assert!(pool.is_some());
                assert_eq!(pool.unwrap().get_stats().block_size, 128);
            }
        }

        #[test]
        fn test_pool_size_classes() {
            let mut manager = PoolManager::new();

            // Create pools with power-of-two size classes
            for i in 0..4 {
                let size = 32 << i; // 32, 64, 128, 256
                assert!(manager.create_pool(size, 1024).is_ok());
            }

            // Verify size class selection - test each size separately
            for &request_size in &[25, 48, 100, 200] {
                let pool = manager.get_pool(request_size).expect("Failed to get pool");
                let stats = pool.get_stats();
                assert!(stats.block_size >= request_size);
                assert!(stats.block_size <= request_size * 2);
            }
        }
    }

    mod integration_tests {
        use super::*;

        #[test]
        fn test_allocation_deallocation_cycle() {
            let mut manager = PoolManager::new();
            manager
                .create_pool(64, 1024)
                .expect("Failed to create pool");

            let mut blocks = Vec::new();

            // Scope the pool borrow
            {
                let pool = manager.get_pool(60).expect("Failed to get pool");

                // Allocate some blocks
                for _ in 0..8 {
                    let block = pool.allocate().expect("Failed to allocate block");
                    blocks.push(block);
                }
            }

            // Get pool again for deallocation
            {
                let pool = manager.get_pool(60).expect("Failed to get pool");

                // Deallocate half the blocks
                for block in blocks.drain(0..4) {
                    assert!(pool.deallocate(block).is_ok());
                }

                // Verify pool state
                let stats = pool.get_stats();
                assert_eq!(stats.used_blocks, 4);
                assert_eq!(stats.free_blocks, 12);
                assert_eq!(stats.utilization, 0.25);
            }
        }

        #[test]
        fn test_mixed_size_allocations() {
            let mut manager = PoolManager::new();

            // Create pools for different sizes
            manager
                .create_pool(32, 1024)
                .expect("Failed to create 32-byte pool");
            manager
                .create_pool(64, 1024)
                .expect("Failed to create 64-byte pool");

            // Use separate scopes for different pool operations
            let block1 = {
                let pool = manager.get_pool(30).expect("Failed to get 32-byte pool");
                pool.allocate().expect("Failed to allocate 32-byte block")
            };

            let block2 = {
                let pool = manager.get_pool(60).expect("Failed to get 64-byte pool");
                pool.allocate().expect("Failed to allocate 64-byte block")
            };

            assert_eq!(block1.size, 32);
            assert_eq!(block2.size, 64);

            // Cleanup in separate scopes
            {
                let pool = manager.get_pool(30).expect("Failed to get 32-byte pool");
                assert!(pool.deallocate(block1).is_ok());
            }
            {
                let pool = manager.get_pool(60).expect("Failed to get 64-byte pool");
                assert!(pool.deallocate(block2).is_ok());
            }
        }
    }
}
