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

/// Memory management module for the database
///
/// This module provides a comprehensive memory management system including:
/// - Custom memory allocators with different allocation strategies
/// - Memory caching with configurable eviction policies
/// - Memory mapping for efficient file I/O
/// - Memory pools for reusing frequently allocated objects
/// - Common utilities for memory operations
pub mod allocator; // Custom memory allocation
pub mod cache; // Memory caching strategies
pub mod lib; // Shared utilities and helper functions
pub mod mmap; // Memory-mapped file operations
pub mod pool; // Memory pooling

// Re-export main components for easier access
pub use allocator::{AllocationStrategy, AllocatorError, CustomAllocator};
pub use cache::{Cache, CacheError, CacheStats, EvictionPolicy};
pub use lib::{align_to, is_power_of_two, next_power_of_two};
pub use mmap::{MappingStrategy, MemoryMap, MmapError};
pub use pool::{MemoryPool, PoolError, PoolStats, PoolType};
