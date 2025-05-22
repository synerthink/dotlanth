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
    collections::VecDeque,
    ptr::NonNull,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::memory::lib::{MemoryStats, align_to};

/// Types of memory pools that can be created
/// Controls the size of blocks allocated in the pool
#[derive(Debug, Clone)]
pub enum PoolType {
    FixedSize { block_size: usize },                   // All blocks are the same size
    VariableSize { min_size: usize, max_size: usize }, // Blocks can vary between min and max size
}

/// Errors that can occur during memory pool operations
#[derive(Debug)]
pub enum PoolError {
    OutOfMemory,          // Not enough memory to allocate blocks
    InvalidSize,          // Requested size is invalid for the pool
    InvalidConfiguration, // Pool configuration is invalid
    PoolExhausted,        // No more blocks available in the pool
}

/// Statistics about memory pool usage and performance
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_blocks: usize,       // Total number of blocks in the pool
    pub free_blocks: usize,        // Number of blocks available for allocation
    pub allocated_blocks: usize,   // Number of blocks currently in use
    pub peak_usage: usize,         // Maximum number of blocks used at once
    pub allocation_count: u64,     // Total number of allocation operations
    pub deallocation_count: u64,   // Total number of deallocation operations
    pub pool_grows: u64,           // Number of times the pool had to grow
    pub pool_shrinks: u64,         // Number of times the pool was shrunk
    pub hit_rate: f64,             // Ratio of successful allocations from pool vs growth
    pub memory_stats: MemoryStats, // Detailed memory usage statistics
}

impl Default for PoolStats {
    fn default() -> Self {
        Self {
            total_blocks: 0,
            free_blocks: 0,
            allocated_blocks: 0,
            peak_usage: 0,
            allocation_count: 0,
            deallocation_count: 0,
            pool_grows: 0,
            pool_shrinks: 0,
            hit_rate: 0.0,
            memory_stats: MemoryStats::default(),
        }
    }
}

/// Internal representation of a memory block in the pool
#[derive(Debug)]
struct PoolBlock {
    data: NonNull<u8>,             // Pointer to the actual memory block
    size: usize,                   // Size of the block in bytes
    allocated_at: Option<Instant>, // When this block was last allocated
    last_used: Instant,            // When this block was last accessed
}

// Implement thread safety for PoolBlock
unsafe impl Send for PoolBlock {}
unsafe impl Sync for PoolBlock {}

/// Memory pool for efficiently reusing allocated memory blocks
/// Reduces allocation overhead for frequently used memory sizes
pub struct MemoryPool {
    pool_type: PoolType,                          // Type of pool (fixed or variable size)
    free_blocks: Arc<Mutex<VecDeque<PoolBlock>>>, // Available blocks for allocation
    allocated_blocks: Arc<Mutex<Vec<PoolBlock>>>, // Currently allocated blocks
    stats: Arc<Mutex<PoolStats>>,                 // Usage statistics
    initial_capacity: usize,                      // Initial number of blocks
    max_capacity: usize,                          // Maximum number of blocks
    grow_factor: f64,                             // How much to grow when needed
    shrink_threshold: f64,                        // When to shrink the pool
    alignment: usize,                             // Memory alignment for blocks
    auto_shrink: bool,                            // Whether to automatically shrink
    shrink_interval: Duration,                    // How often to check for shrinking
    last_shrink: Arc<Mutex<Instant>>,             // When the pool was last shrunk
}

impl MemoryPool {
    /// Creates a new memory pool with the specified type and initial capacity
    ///
    /// # Arguments
    /// * `pool_type` - Type of pool to create (fixed or variable size)
    /// * `initial_capacity` - Initial number of blocks to allocate
    ///
    /// # Returns
    /// * `Result<Self, PoolError>` - The new pool or an error
    pub fn new(pool_type: PoolType, initial_capacity: usize) -> Result<Self, PoolError> {
        if initial_capacity == 0 {
            return Err(PoolError::InvalidConfiguration);
        }

        let pool = Self {
            pool_type,
            free_blocks: Arc::new(Mutex::new(VecDeque::new())),
            allocated_blocks: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(PoolStats::default())),
            initial_capacity,
            max_capacity: initial_capacity * 10,      // Default max is 10x initial
            grow_factor: 1.5,                         // Grow by 50% when needed
            shrink_threshold: 0.25,                   // Shrink when usage below 25%
            alignment: std::mem::align_of::<usize>(), // Default to pointer alignment
            auto_shrink: true,
            shrink_interval: Duration::from_secs(60), // Check for shrinking every minute
            last_shrink: Arc::new(Mutex::new(Instant::now())),
        };

        // Prepopulate the pool with blocks
        pool.initialize_pool()?;
        Ok(pool)
    }

    /// Sets the maximum capacity for this pool
    ///
    /// # Arguments
    /// * `max_capacity` - Maximum number of blocks the pool can contain
    pub fn with_max_capacity(mut self, max_capacity: usize) -> Self {
        self.max_capacity = max_capacity;
        self
    }

    /// Sets the growth factor for when the pool needs to expand
    ///
    /// # Arguments
    /// * `grow_factor` - Multiplier for pool size when growing (e.g., 1.5 = grow by 50%)
    pub fn with_grow_factor(mut self, grow_factor: f64) -> Self {
        self.grow_factor = grow_factor;
        self
    }

    /// Sets the threshold at which the pool should shrink
    ///
    /// # Arguments
    /// * `threshold` - Usage ratio below which to shrink (e.g., 0.25 = shrink when < 25% used)
    pub fn with_shrink_threshold(mut self, threshold: f64) -> Self {
        self.shrink_threshold = threshold;
        self
    }

    /// Sets the memory alignment for allocated blocks
    ///
    /// # Arguments
    /// * `alignment` - Memory alignment in bytes (must be power of 2)
    pub fn with_alignment(mut self, alignment: usize) -> Self {
        self.alignment = alignment;
        self
    }

    /// Configures automatic shrinking of the pool
    ///
    /// # Arguments
    /// * `enabled` - Whether to enable automatic shrinking
    /// * `interval` - How often to check for shrinking opportunities
    pub fn with_auto_shrink(mut self, enabled: bool, interval: Duration) -> Self {
        self.auto_shrink = enabled;
        self.shrink_interval = interval;
        self
    }

    /// Initializes the pool by creating the initial blocks
    ///
    /// # Returns
    /// * `Result<(), PoolError>` - Success or an error
    fn initialize_pool(&self) -> Result<(), PoolError> {
        let mut free_blocks = self.free_blocks.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        // Create the initial blocks
        for _ in 0..self.initial_capacity {
            let block = self.allocate_block()?;
            free_blocks.push_back(block);
        }

        // Update statistics
        stats.total_blocks = self.initial_capacity;
        stats.free_blocks = self.initial_capacity;
        Ok(())
    }

    /// Allocates a new memory block for the pool
    ///
    /// # Returns
    /// * `Result<PoolBlock, PoolError>` - The new block or an error
    fn allocate_block(&self) -> Result<PoolBlock, PoolError> {
        // Determine the block size based on pool type
        let size = match &self.pool_type {
            PoolType::FixedSize { block_size } => *block_size,
            PoolType::VariableSize { min_size, .. } => *min_size,
        };

        // Ensure proper alignment
        let aligned_size = align_to(size, self.alignment);

        unsafe {
            // Create a layout with the required size and alignment
            let layout = std::alloc::Layout::from_size_align(aligned_size, self.alignment).map_err(|_| PoolError::InvalidSize)?;

            // Allocate the memory
            let ptr = std::alloc::alloc(layout);
            if ptr.is_null() {
                return Err(PoolError::OutOfMemory);
            }

            // Create and return the PoolBlock
            Ok(PoolBlock {
                data: NonNull::new_unchecked(ptr),
                size: aligned_size,
                allocated_at: None,
                last_used: Instant::now(),
            })
        }
    }

    /// Allocates a block from the pool
    ///
    /// # Arguments
    /// * `requested_size` - Size of memory needed
    ///
    /// # Returns
    /// * `Result<NonNull<u8>, PoolError>` - Pointer to allocated memory or error
    pub fn allocate(&self, requested_size: usize) -> Result<NonNull<u8>, PoolError> {
        if requested_size == 0 {
            return Err(PoolError::InvalidSize);
        }

        // Check if size is compatible with pool type
        match &self.pool_type {
            PoolType::FixedSize { block_size } => {
                if requested_size > *block_size {
                    return Err(PoolError::InvalidSize);
                }
            }
            PoolType::VariableSize { min_size, max_size } => {
                if requested_size < *min_size || requested_size > *max_size {
                    return Err(PoolError::InvalidSize);
                }
            }
        }

        let mut free_blocks = self.free_blocks.lock().unwrap();
        let mut allocated_blocks = self.allocated_blocks.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        // Try to get a block from the free pool
        if let Some(mut block) = free_blocks.pop_front() {
            // Update block metadata
            block.allocated_at = Some(Instant::now());
            block.last_used = Instant::now();

            let ptr = block.data;

            // Move block to the allocated list
            allocated_blocks.push(block);

            // Update stats
            stats.free_blocks -= 1;
            stats.allocated_blocks += 1;
            stats.allocation_count += 1;

            if stats.allocated_blocks > stats.peak_usage {
                stats.peak_usage = stats.allocated_blocks;
            }

            // Track hit rate
            let total_ops = stats.allocation_count;
            let pool_hits = total_ops - stats.pool_grows;
            stats.hit_rate = pool_hits as f64 / total_ops as f64;

            return Ok(ptr);
        }

        // If no free blocks are available, try to grow the pool
        drop(free_blocks);
        drop(allocated_blocks);
        drop(stats);

        if self.try_grow_pool()? {
            // Retry allocation after growing
            let mut free_blocks = self.free_blocks.lock().unwrap();
            let mut allocated_blocks = self.allocated_blocks.lock().unwrap();
            let mut stats = self.stats.lock().unwrap();

            if let Some(mut block) = free_blocks.pop_front() {
                block.allocated_at = Some(Instant::now());
                block.last_used = Instant::now();

                let ptr = block.data;
                allocated_blocks.push(block);

                stats.free_blocks -= 1;
                stats.allocated_blocks += 1;
                stats.allocation_count += 1;
                stats.memory_stats.record_allocation(requested_size);

                if stats.allocated_blocks > stats.peak_usage {
                    stats.peak_usage = stats.allocated_blocks;
                }

                stats.hit_rate = stats.allocation_count as f64 / (stats.allocation_count + stats.pool_grows) as f64;

                return Ok(ptr);
            }
        }

        Err(PoolError::PoolExhausted)
    }

    /// Returns a block to the pool for reuse
    ///
    /// # Arguments
    /// * `ptr` - Pointer to the block being returned
    ///
    /// # Returns
    /// * `Result<(), PoolError>` - Success or an error if the pointer is invalid
    ///
    /// # Errors
    /// * `PoolError::InvalidSize` - If the pointer was not found in allocated blocks
    pub fn deallocate(&self, ptr: NonNull<u8>) -> Result<(), PoolError> {
        let mut free_blocks = self.free_blocks.lock().unwrap();
        let mut allocated_blocks = self.allocated_blocks.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        // Find the block in allocated blocks
        if let Some(pos) = allocated_blocks.iter().position(|block| block.data == ptr) {
            let mut block = allocated_blocks.swap_remove(pos);
            block.allocated_at = None;
            block.last_used = Instant::now();

            let size = block.size;
            free_blocks.push_back(block);

            stats.free_blocks += 1;
            stats.allocated_blocks -= 1;
            stats.deallocation_count += 1;
            stats.memory_stats.record_deallocation(size);

            stats.hit_rate = stats.allocation_count as f64 / (stats.allocation_count + stats.pool_grows) as f64;

            // Check if we should shrink the pool
            drop(free_blocks);
            drop(allocated_blocks);
            drop(stats);

            if self.auto_shrink {
                let _ = self.try_shrink_pool();
            }

            Ok(())
        } else {
            Err(PoolError::InvalidSize) // Block not found
        }
    }

    fn try_grow_pool(&self) -> Result<bool, PoolError> {
        let mut stats = self.stats.lock().unwrap();

        if stats.total_blocks >= self.max_capacity {
            return Ok(false);
        }

        let grow_size = ((stats.total_blocks as f64 * self.grow_factor) as usize)
            .saturating_sub(stats.total_blocks)
            .min(self.max_capacity - stats.total_blocks)
            .max(1);

        drop(stats);

        let mut free_blocks = self.free_blocks.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        for _ in 0..grow_size {
            match self.allocate_block() {
                Ok(block) => {
                    free_blocks.push_back(block);
                    stats.total_blocks += 1;
                    stats.free_blocks += 1;
                }
                Err(_) => break,
            }
        }

        stats.pool_grows += 1;
        Ok(true)
    }

    fn try_shrink_pool(&self) -> Result<bool, PoolError> {
        let now = Instant::now();
        let mut last_shrink = self.last_shrink.lock().unwrap();

        if now.duration_since(*last_shrink) < self.shrink_interval {
            return Ok(false);
        }

        let mut free_blocks = self.free_blocks.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        let usage_ratio = stats.allocated_blocks as f64 / stats.total_blocks as f64;

        if usage_ratio > self.shrink_threshold || stats.total_blocks <= self.initial_capacity {
            return Ok(false);
        }

        let target_free = (stats.total_blocks as f64 * self.shrink_threshold) as usize;
        let shrink_count = stats.free_blocks.saturating_sub(target_free);

        if shrink_count == 0 {
            return Ok(false);
        }

        let mut shrunk = 0;
        let cutoff_time = now - Duration::from_secs(300); // 5 minutes

        // Remove blocks that haven't been used recently
        free_blocks.retain(|block| {
            if shrunk >= shrink_count || block.last_used > cutoff_time {
                true
            } else {
                unsafe {
                    let layout = std::alloc::Layout::from_size_align_unchecked(block.size, self.alignment);
                    std::alloc::dealloc(block.data.as_ptr(), layout);
                }
                shrunk += 1;
                false
            }
        });

        stats.total_blocks -= shrunk;
        stats.free_blocks -= shrunk;
        stats.pool_shrinks += 1;
        *last_shrink = now;

        Ok(shrunk > 0)
    }

    /// Gets current statistics about the memory pool's usage
    ///
    /// # Returns
    /// * `PoolStats` - Current statistics including allocation counts, block counts, and memory usage
    pub fn get_stats(&self) -> PoolStats {
        self.stats.lock().unwrap().clone()
    }

    /// Forces the pool to immediately shrink by freeing unused blocks
    /// Unlike automatic shrinking, this doesn't wait for the shrink interval
    ///
    /// # Returns
    /// * `Result<usize, PoolError>` - Number of blocks that were freed or an error
    pub fn force_shrink(&self) -> Result<usize, PoolError> {
        let mut free_blocks = self.free_blocks.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        let initial_count = free_blocks.len();
        let keep_count = self.initial_capacity.min(stats.allocated_blocks + 10);
        let shrink_count = initial_count.saturating_sub(keep_count);

        for _ in 0..shrink_count {
            if let Some(block) = free_blocks.pop_back() {
                unsafe {
                    let layout = std::alloc::Layout::from_size_align_unchecked(block.size, self.alignment);
                    std::alloc::dealloc(block.data.as_ptr(), layout);
                }
                stats.total_blocks -= 1;
                stats.free_blocks -= 1;
            }
        }

        stats.pool_shrinks += 1;
        Ok(shrink_count)
    }

    /// Deallocates all blocks in the pool
    /// Can only be called when no blocks are currently allocated
    ///
    /// # Returns
    /// * `Result<(), PoolError>` - Success or an error if blocks are still allocated
    ///
    /// # Errors
    /// * `PoolError::InvalidConfiguration` - If there are still allocated blocks
    pub fn clear(&self) -> Result<(), PoolError> {
        let mut free_blocks = self.free_blocks.lock().unwrap();
        let allocated_blocks = self.allocated_blocks.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        if !allocated_blocks.is_empty() {
            return Err(PoolError::InvalidConfiguration); // Can't clear with allocated blocks
        }

        // Deallocate all free blocks
        while let Some(block) = free_blocks.pop_front() {
            unsafe {
                let layout = std::alloc::Layout::from_size_align_unchecked(block.size, self.alignment);
                std::alloc::dealloc(block.data.as_ptr(), layout);
            }
        }

        stats.total_blocks = 0;
        stats.free_blocks = 0;
        stats.allocated_blocks = 0;

        Ok(())
    }

    /// Pre-allocates additional blocks for the pool
    /// Useful for warming up the pool before heavy usage
    ///
    /// # Arguments
    /// * `count` - Number of blocks to pre-allocate
    ///
    /// # Returns
    /// * `Result<(), PoolError>` - Success or an error if allocation fails
    ///
    /// # Errors
    /// * `PoolError::OutOfMemory` - If memory allocation fails
    pub fn prealloc(&self, count: usize) -> Result<(), PoolError> {
        let mut free_blocks = self.free_blocks.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        let current_total = stats.total_blocks;
        let target_total = (current_total + count).min(self.max_capacity);
        let actual_count = target_total - current_total;

        for _ in 0..actual_count {
            match self.allocate_block() {
                Ok(block) => {
                    free_blocks.push_back(block);
                    stats.total_blocks += 1;
                    stats.free_blocks += 1;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }
}

impl Drop for MemoryPool {
    fn drop(&mut self) {
        let _ = self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_size_pool_creation() {
        let pool_type = PoolType::FixedSize { block_size: 1024 };
        let pool = MemoryPool::new(pool_type, 10).unwrap();

        let stats = pool.get_stats();
        assert_eq!(stats.total_blocks, 10);
        assert_eq!(stats.free_blocks, 10);
        assert_eq!(stats.allocated_blocks, 0);
    }

    #[test]
    fn test_variable_size_pool_creation() {
        let pool_type = PoolType::VariableSize { min_size: 512, max_size: 2048 };
        let pool = MemoryPool::new(pool_type, 5).unwrap();

        let stats = pool.get_stats();
        assert_eq!(stats.total_blocks, 5);
        assert_eq!(stats.free_blocks, 5);
    }

    #[test]
    fn test_allocation_and_deallocation() {
        let pool_type = PoolType::FixedSize { block_size: 1024 };
        let pool = MemoryPool::new(pool_type, 5).unwrap();

        let ptr1 = pool.allocate(512).unwrap();
        let ptr2 = pool.allocate(1024).unwrap();

        let stats = pool.get_stats();
        assert_eq!(stats.allocated_blocks, 2);
        assert_eq!(stats.free_blocks, 3);

        pool.deallocate(ptr1).unwrap();
        pool.deallocate(ptr2).unwrap();

        let final_stats = pool.get_stats();
        assert_eq!(final_stats.allocated_blocks, 0);
        assert_eq!(final_stats.free_blocks, 5);
    }

    #[test]
    fn test_pool_growth() {
        let pool_type = PoolType::FixedSize { block_size: 1024 };
        let pool = MemoryPool::new(pool_type, 2).unwrap().with_max_capacity(10);

        // Allocate all initial blocks
        let ptr1 = pool.allocate(1024).unwrap();
        let ptr2 = pool.allocate(1024).unwrap();

        // This should trigger pool growth
        let ptr3 = pool.allocate(1024).unwrap();

        let stats = pool.get_stats();
        assert!(stats.total_blocks > 2);
        assert_eq!(stats.allocated_blocks, 3);

        pool.deallocate(ptr1).unwrap();
        pool.deallocate(ptr2).unwrap();
        pool.deallocate(ptr3).unwrap();
    }

    #[test]
    fn test_invalid_size_allocation() {
        let pool_type = PoolType::FixedSize { block_size: 1024 };
        let pool = MemoryPool::new(pool_type, 5).unwrap();

        // Request size larger than block size
        assert!(pool.allocate(2048).is_err());

        // Zero size allocation
        assert!(pool.allocate(0).is_err());
    }

    #[test]
    fn test_variable_size_constraints() {
        let pool_type = PoolType::VariableSize { min_size: 512, max_size: 2048 };
        let pool = MemoryPool::new(pool_type, 5).unwrap();

        // Valid sizes
        assert!(pool.allocate(512).is_ok());
        assert!(pool.allocate(1024).is_ok());
        assert!(pool.allocate(2048).is_ok());

        // Invalid sizes
        assert!(pool.allocate(256).is_err()); // Too small
        assert!(pool.allocate(4096).is_err()); // Too large
    }

    #[test]
    fn test_pool_exhaustion() {
        let pool_type = PoolType::FixedSize { block_size: 1024 };
        let pool = MemoryPool::new(pool_type, 2).unwrap().with_max_capacity(2); // Prevent growth

        let _ptr1 = pool.allocate(1024).unwrap();
        let _ptr2 = pool.allocate(1024).unwrap();

        // Pool should be exhausted
        assert!(pool.allocate(1024).is_err());
    }

    #[test]
    fn test_stats_tracking() {
        let pool_type = PoolType::FixedSize { block_size: 1024 };
        let pool = MemoryPool::new(pool_type, 3).unwrap();

        let ptr1 = pool.allocate(1024).unwrap();
        let ptr2 = pool.allocate(512).unwrap();

        let stats = pool.get_stats();
        assert_eq!(stats.allocation_count, 2);
        assert_eq!(stats.peak_usage, 2);

        pool.deallocate(ptr1).unwrap();

        let final_stats = pool.get_stats();
        assert_eq!(final_stats.deallocation_count, 1);
        assert_eq!(final_stats.peak_usage, 2); // Peak should remain

        pool.deallocate(ptr2).unwrap();
    }

    #[test]
    fn test_force_shrink() {
        let pool_type = PoolType::FixedSize { block_size: 1024 };
        let pool = MemoryPool::new(pool_type, 10).unwrap();

        let initial_stats = pool.get_stats();
        assert_eq!(initial_stats.total_blocks, 10);

        pool.prealloc(10).unwrap();
        let stats_after_prealloc = pool.get_stats();
        assert_eq!(stats_after_prealloc.total_blocks, 20);

        let shrunk = pool.force_shrink().unwrap();
        assert!(shrunk > 0);

        let final_stats = pool.get_stats();
        assert!(final_stats.total_blocks < stats_after_prealloc.total_blocks);
    }

    #[test]
    fn test_preallocation() {
        let pool_type = PoolType::FixedSize { block_size: 1024 };
        let pool = MemoryPool::new(pool_type, 5).unwrap();

        pool.prealloc(10).unwrap();

        let stats = pool.get_stats();
        assert_eq!(stats.total_blocks, 15);
        assert_eq!(stats.free_blocks, 15);
    }

    #[test]
    fn test_pool_configuration() {
        let pool_type = PoolType::FixedSize { block_size: 1024 };
        let pool = MemoryPool::new(pool_type, 5)
            .unwrap()
            .with_max_capacity(20)
            .with_grow_factor(2.0)
            .with_shrink_threshold(0.1)
            .with_alignment(16);

        let stats = pool.get_stats();
        assert_eq!(stats.total_blocks, 5);

        // Test that configuration is applied
        let ptr = pool.allocate(1024).unwrap();
        assert!(!ptr.as_ptr().is_null());

        pool.deallocate(ptr).unwrap();
    }

    #[test]
    fn test_clear_pool() {
        let pool_type = PoolType::FixedSize { block_size: 1024 };
        let pool = MemoryPool::new(pool_type, 5).unwrap();

        // Can't clear with allocated blocks
        let ptr = pool.allocate(1024).unwrap();
        assert!(pool.clear().is_err());

        pool.deallocate(ptr).unwrap();

        // Now should be able to clear
        assert!(pool.clear().is_ok());

        let stats = pool.get_stats();
        assert_eq!(stats.total_blocks, 0);
        assert_eq!(stats.free_blocks, 0);
    }
}
