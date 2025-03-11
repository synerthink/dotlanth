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
        if block_size == 0 || total_size == 0 {
            return Err(MemoryError::AllocationFailed(block_size.to_string()));
        }
        if total_size % block_size != 0 {
            return Err(MemoryError::InvalidAlignment(total_size % block_size));
        }

        let total_blocks = total_size / block_size;
        let mut free_blocks = VecDeque::with_capacity(total_blocks);

        // Create addresses starting from 0, incrementing by the block size
        for i in 0..total_blocks {
            free_blocks.push_back(PoolBlock {
                address: PhysicalAddress(i * block_size),
                size: block_size,
            });
        }

        Ok(MemoryPool {
            block_size,
            total_blocks,
            free_blocks,
            used_blocks: 0,
        })
    }

    pub fn allocate(&mut self) -> Result<PoolBlock, MemoryError> {
        match self.free_blocks.pop_front() {
            Some(block) => {
                self.used_blocks += 1;
                Ok(block)
            }
            None => Err(MemoryError::OutOfMemory {
                requested: self.block_size,
                available: 0,
            }),
        }
    }

    pub fn deallocate(&mut self, block: PoolBlock) -> Result<(), MemoryError> {
        // Change control orders
        if block.size != self.block_size {
            return Err(MemoryError::InvalidAddress(block.address.0));
        }
        if block.address.0 / self.block_size >= self.total_blocks {
            return Err(MemoryError::InvalidAddress(block.address.0));
        }
        if block.address.0 % self.block_size != 0 {
            return Err(MemoryError::InvalidAlignment(block.address.0));
        }
        if self.free_blocks.iter().any(|b| b.address == block.address) {
            return Err(MemoryError::InvalidAddress(block.address.0));
        }

        self.used_blocks -= 1;
        // Add the block to the front of the queue
        self.free_blocks.push_front(block);
        Ok(())
    }

    pub fn get_stats(&self) -> PoolStats {
        let free_blocks = self.free_blocks.len();
        let utilization = if self.total_blocks > 0 { self.used_blocks as f64 / self.total_blocks as f64 } else { 0.0 };

        PoolStats {
            block_size: self.block_size,
            total_blocks: self.total_blocks,
            used_blocks: self.used_blocks,
            free_blocks,
            utilization,
        }
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
        Self {
            pools: Vec::new(),
            size_classes: Vec::new(),
        }
    }

    pub fn get_pool(&mut self, size: usize) -> Option<&mut MemoryPool> {
        let mut best = None;
        for (i, &class) in self.size_classes.iter().enumerate() {
            if class >= size {
                if let Some((current_size, _)) = best {
                    if class < current_size {
                        best = Some((class, i));
                    }
                } else {
                    best = Some((class, i));
                }
            }
        }
        best.and_then(|(_, i)| self.pools.get_mut(i))
    }

    pub fn create_pool(&mut self, block_size: usize, total_size: usize) -> Result<(), MemoryError> {
        if self.size_classes.contains(&block_size) {
            return Err(MemoryError::PoolError("Size class already exists".to_string()));
        }
        let pool = MemoryPool::new(block_size, total_size)?;
        self.size_classes.push(block_size);
        self.pools.push(pool);
        Ok(())
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
            assert!(matches!(MemoryPool::new(0, 1024), Err(MemoryError::AllocationFailed(_))));

            // Test zero total size
            assert!(matches!(MemoryPool::new(64, 0), Err(MemoryError::AllocationFailed(_))));

            // Test total size not multiple of block size
            assert!(matches!(MemoryPool::new(64, 100), Err(MemoryError::InvalidAlignment(_))));
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
            assert!(matches!(pool.allocate(), Err(MemoryError::OutOfMemory { requested: 64, available: 0 })));
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

            assert!(matches!(pool.deallocate(invalid_block), Err(MemoryError::InvalidAddress(_))));
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
            assert!(matches!(manager.create_pool(64, 2048), Err(MemoryError::PoolError(_))));
        }

        #[test]
        fn test_pool_selection() {
            let mut manager = PoolManager::new();

            // Create pools with different size classes
            manager.create_pool(32, 1024).expect("Failed to create 32-byte pool");
            manager.create_pool(64, 1024).expect("Failed to create 64-byte pool");
            manager.create_pool(128, 1024).expect("Failed to create 128-byte pool");

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
            manager.create_pool(64, 1024).expect("Failed to create pool");

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
            manager.create_pool(32, 1024).expect("Failed to create 32-byte pool");
            manager.create_pool(64, 1024).expect("Failed to create 64-byte pool");

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
