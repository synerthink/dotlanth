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

//! Parallel processing opcodes for DotVM
//!
//! This module defines opcodes for parallel computation primitives
//! available on 512-bit architecture.

use std::fmt;

/// Parallel processing opcodes for 512-bit architecture
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ParallelOpcode {
    // Basic parallel operations
    Map = 0x01,              // Parallel map operation
    Reduce = 0x02,           // Parallel reduce operation
    Filter = 0x03,           // Parallel filter operation
    Scan = 0x04,             // Parallel scan (prefix sum)
    Sort = 0x05,             // Parallel sort operation
    
    // Parallel arithmetic
    ParallelAdd = 0x10,      // Parallel addition across multiple vectors
    ParallelMul = 0x11,      // Parallel multiplication across multiple vectors
    ParallelDot = 0x12,      // Parallel dot product computation
    ParallelSum = 0x13,      // Parallel summation
    
    // Work distribution
    Fork = 0x20,             // Fork execution into parallel threads
    Join = 0x21,             // Join parallel threads
    Barrier = 0x22,          // Synchronization barrier
    Spawn = 0x23,            // Spawn new parallel task
    
    // Memory operations
    ParallelLoad = 0x30,     // Parallel memory load
    ParallelStore = 0x31,    // Parallel memory store
    ParallelCopy = 0x32,     // Parallel memory copy
    ParallelSet = 0x33,      // Parallel memory set
    
    // Synchronization primitives
    Lock = 0x40,             // Acquire lock
    Unlock = 0x41,           // Release lock
    AtomicAdd = 0x42,        // Atomic addition
    AtomicSub = 0x43,        // Atomic subtraction
    AtomicCas = 0x44,        // Atomic compare-and-swap
    AtomicExchange = 0x45,   // Atomic exchange
    
    // Parallel algorithms
    ParallelSearch = 0x50,   // Parallel search operation
    ParallelMerge = 0x51,    // Parallel merge operation
    ParallelPartition = 0x52, // Parallel partition operation
    ParallelShuffle = 0x53,  // Parallel shuffle operation
    
    // Thread management
    GetThreadId = 0x60,      // Get current thread ID
    GetThreadCount = 0x61,   // Get total thread count
    SetThreadAffinity = 0x62, // Set thread CPU affinity
    YieldThread = 0x63,      // Yield current thread
    
    // Load balancing
    WorkSteal = 0x70,        // Work stealing operation
    LoadBalance = 0x71,      // Load balancing operation
    TaskQueue = 0x72,        // Task queue management
    TaskDequeue = 0x73,      // Task dequeue operation
    
    // Parallel I/O
    ParallelRead = 0x80,     // Parallel file read
    ParallelWrite = 0x81,    // Parallel file write
    ParallelFlush = 0x82,    // Parallel flush operation
    
    // Performance monitoring
    StartTimer = 0x90,       // Start performance timer
    StopTimer = 0x91,        // Stop performance timer
    GetCycles = 0x92,        // Get CPU cycle count
    GetCacheStats = 0x93,    // Get cache statistics
}

impl ParallelOpcode {
    /// Get the number of operands this opcode expects
    pub fn operand_count(&self) -> usize {
        match self {
            // No operand operations
            ParallelOpcode::Barrier |
            ParallelOpcode::GetThreadId |
            ParallelOpcode::GetThreadCount |
            ParallelOpcode::YieldThread |
            ParallelOpcode::StartTimer |
            ParallelOpcode::StopTimer |
            ParallelOpcode::GetCycles |
            ParallelOpcode::GetCacheStats => 0,
            
            // Unary operations
            ParallelOpcode::Map |
            ParallelOpcode::Reduce |
            ParallelOpcode::Filter |
            ParallelOpcode::Scan |
            ParallelOpcode::Sort |
            ParallelOpcode::ParallelSum |
            ParallelOpcode::Join |
            ParallelOpcode::Spawn |
            ParallelOpcode::ParallelLoad |
            ParallelOpcode::ParallelSet |
            ParallelOpcode::Lock |
            ParallelOpcode::Unlock |
            ParallelOpcode::ParallelSearch |
            ParallelOpcode::ParallelShuffle |
            ParallelOpcode::SetThreadAffinity |
            ParallelOpcode::WorkSteal |
            ParallelOpcode::LoadBalance |
            ParallelOpcode::TaskQueue |
            ParallelOpcode::TaskDequeue |
            ParallelOpcode::ParallelRead |
            ParallelOpcode::ParallelWrite |
            ParallelOpcode::ParallelFlush => 1,
            
            // Binary operations
            ParallelOpcode::ParallelAdd |
            ParallelOpcode::ParallelMul |
            ParallelOpcode::ParallelDot |
            ParallelOpcode::Fork |
            ParallelOpcode::ParallelStore |
            ParallelOpcode::ParallelCopy |
            ParallelOpcode::AtomicAdd |
            ParallelOpcode::AtomicSub |
            ParallelOpcode::AtomicExchange |
            ParallelOpcode::ParallelMerge |
            ParallelOpcode::ParallelPartition => 2,
            
            // Ternary operations
            ParallelOpcode::AtomicCas => 3,
        }
    }

    /// Check if this operation requires synchronization
    pub fn requires_synchronization(&self) -> bool {
        matches!(self,
            ParallelOpcode::Barrier |
            ParallelOpcode::Join |
            ParallelOpcode::Lock |
            ParallelOpcode::Unlock |
            ParallelOpcode::AtomicAdd |
            ParallelOpcode::AtomicSub |
            ParallelOpcode::AtomicCas |
            ParallelOpcode::AtomicExchange
        )
    }

    /// Check if this operation is thread-safe
    pub fn is_thread_safe(&self) -> bool {
        matches!(self,
            ParallelOpcode::Map |
            ParallelOpcode::Filter |
            ParallelOpcode::ParallelAdd |
            ParallelOpcode::ParallelMul |
            ParallelOpcode::ParallelDot |
            ParallelOpcode::ParallelLoad |
            ParallelOpcode::AtomicAdd |
            ParallelOpcode::AtomicSub |
            ParallelOpcode::AtomicCas |
            ParallelOpcode::AtomicExchange |
            ParallelOpcode::GetThreadId |
            ParallelOpcode::GetThreadCount |
            ParallelOpcode::GetCycles |
            ParallelOpcode::GetCacheStats
        )
    }

    /// Get the expected performance scaling factor
    pub fn scaling_factor(&self) -> ScalingFactor {
        match self {
            // Perfect scaling operations
            ParallelOpcode::Map |
            ParallelOpcode::Filter |
            ParallelOpcode::ParallelAdd |
            ParallelOpcode::ParallelMul |
            ParallelOpcode::ParallelLoad |
            ParallelOpcode::ParallelStore => ScalingFactor::Linear,
            
            // Good scaling operations
            ParallelOpcode::Reduce |
            ParallelOpcode::Scan |
            ParallelOpcode::ParallelDot |
            ParallelOpcode::ParallelSum |
            ParallelOpcode::ParallelSearch => ScalingFactor::Logarithmic,
            
            // Limited scaling operations
            ParallelOpcode::Sort |
            ParallelOpcode::ParallelMerge |
            ParallelOpcode::ParallelPartition => ScalingFactor::LogLinear,
            
            // Synchronization overhead
            ParallelOpcode::Fork |
            ParallelOpcode::Join |
            ParallelOpcode::Barrier |
            ParallelOpcode::Lock |
            ParallelOpcode::Unlock => ScalingFactor::Constant,
            
            // Atomic operations (contention dependent)
            ParallelOpcode::AtomicAdd |
            ParallelOpcode::AtomicSub |
            ParallelOpcode::AtomicCas |
            ParallelOpcode::AtomicExchange => ScalingFactor::Sublinear,
            
            // Other operations
            _ => ScalingFactor::Constant,
        }
    }

    /// Get the memory access pattern
    pub fn memory_pattern(&self) -> MemoryAccessPattern {
        match self {
            ParallelOpcode::ParallelLoad |
            ParallelOpcode::ParallelStore |
            ParallelOpcode::ParallelCopy => MemoryAccessPattern::Sequential,
            
            ParallelOpcode::Map |
            ParallelOpcode::Filter |
            ParallelOpcode::ParallelAdd |
            ParallelOpcode::ParallelMul => MemoryAccessPattern::Strided,
            
            ParallelOpcode::ParallelSearch |
            ParallelOpcode::ParallelShuffle |
            ParallelOpcode::WorkSteal => MemoryAccessPattern::Random,
            
            ParallelOpcode::Reduce |
            ParallelOpcode::Scan |
            ParallelOpcode::Sort => MemoryAccessPattern::Gather,
            
            _ => MemoryAccessPattern::Unknown,
        }
    }
}

/// Performance scaling factors for parallel operations
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ScalingFactor {
    Linear,      // O(1) - perfect scaling
    Logarithmic, // O(log n) - good scaling
    LogLinear,   // O(n log n) - limited scaling
    Sublinear,   // O(n^k) where k < 1 - poor scaling
    Constant,    // O(1) - no scaling benefit
}

/// Memory access patterns for parallel operations
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MemoryAccessPattern {
    Sequential, // Sequential memory access
    Strided,    // Strided memory access
    Random,     // Random memory access
    Gather,     // Gather/scatter pattern
    Unknown,    // Unknown or mixed pattern
}

impl fmt::Display for ParallelOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParallelOpcode::Map => write!(f, "par.map"),
            ParallelOpcode::Reduce => write!(f, "par.reduce"),
            ParallelOpcode::Filter => write!(f, "par.filter"),
            ParallelOpcode::Scan => write!(f, "par.scan"),
            ParallelOpcode::Sort => write!(f, "par.sort"),
            ParallelOpcode::ParallelAdd => write!(f, "par.add"),
            ParallelOpcode::ParallelMul => write!(f, "par.mul"),
            ParallelOpcode::ParallelDot => write!(f, "par.dot"),
            ParallelOpcode::ParallelSum => write!(f, "par.sum"),
            ParallelOpcode::Fork => write!(f, "par.fork"),
            ParallelOpcode::Join => write!(f, "par.join"),
            ParallelOpcode::Barrier => write!(f, "par.barrier"),
            ParallelOpcode::Spawn => write!(f, "par.spawn"),
            ParallelOpcode::ParallelLoad => write!(f, "par.load"),
            ParallelOpcode::ParallelStore => write!(f, "par.store"),
            ParallelOpcode::ParallelCopy => write!(f, "par.copy"),
            ParallelOpcode::ParallelSet => write!(f, "par.set"),
            ParallelOpcode::Lock => write!(f, "par.lock"),
            ParallelOpcode::Unlock => write!(f, "par.unlock"),
            ParallelOpcode::AtomicAdd => write!(f, "par.atomic.add"),
            ParallelOpcode::AtomicSub => write!(f, "par.atomic.sub"),
            ParallelOpcode::AtomicCas => write!(f, "par.atomic.cas"),
            ParallelOpcode::AtomicExchange => write!(f, "par.atomic.exchange"),
            ParallelOpcode::ParallelSearch => write!(f, "par.search"),
            ParallelOpcode::ParallelMerge => write!(f, "par.merge"),
            ParallelOpcode::ParallelPartition => write!(f, "par.partition"),
            ParallelOpcode::ParallelShuffle => write!(f, "par.shuffle"),
            ParallelOpcode::GetThreadId => write!(f, "par.thread.id"),
            ParallelOpcode::GetThreadCount => write!(f, "par.thread.count"),
            ParallelOpcode::SetThreadAffinity => write!(f, "par.thread.affinity"),
            ParallelOpcode::YieldThread => write!(f, "par.thread.yield"),
            ParallelOpcode::WorkSteal => write!(f, "par.worksteal"),
            ParallelOpcode::LoadBalance => write!(f, "par.loadbalance"),
            ParallelOpcode::TaskQueue => write!(f, "par.task.queue"),
            ParallelOpcode::TaskDequeue => write!(f, "par.task.dequeue"),
            ParallelOpcode::ParallelRead => write!(f, "par.read"),
            ParallelOpcode::ParallelWrite => write!(f, "par.write"),
            ParallelOpcode::ParallelFlush => write!(f, "par.flush"),
            ParallelOpcode::StartTimer => write!(f, "par.timer.start"),
            ParallelOpcode::StopTimer => write!(f, "par.timer.stop"),
            ParallelOpcode::GetCycles => write!(f, "par.cycles"),
            ParallelOpcode::GetCacheStats => write!(f, "par.cache.stats"),
        }
    }
}

impl From<u8> for ParallelOpcode {
    fn from(value: u8) -> Self {
        match value {
            0x01 => ParallelOpcode::Map,
            0x02 => ParallelOpcode::Reduce,
            0x03 => ParallelOpcode::Filter,
            0x04 => ParallelOpcode::Scan,
            0x05 => ParallelOpcode::Sort,
            0x10 => ParallelOpcode::ParallelAdd,
            0x11 => ParallelOpcode::ParallelMul,
            0x12 => ParallelOpcode::ParallelDot,
            0x13 => ParallelOpcode::ParallelSum,
            0x20 => ParallelOpcode::Fork,
            0x21 => ParallelOpcode::Join,
            0x22 => ParallelOpcode::Barrier,
            0x23 => ParallelOpcode::Spawn,
            0x30 => ParallelOpcode::ParallelLoad,
            0x31 => ParallelOpcode::ParallelStore,
            0x32 => ParallelOpcode::ParallelCopy,
            0x33 => ParallelOpcode::ParallelSet,
            0x40 => ParallelOpcode::Lock,
            0x41 => ParallelOpcode::Unlock,
            0x42 => ParallelOpcode::AtomicAdd,
            0x43 => ParallelOpcode::AtomicSub,
            0x44 => ParallelOpcode::AtomicCas,
            0x45 => ParallelOpcode::AtomicExchange,
            0x50 => ParallelOpcode::ParallelSearch,
            0x51 => ParallelOpcode::ParallelMerge,
            0x52 => ParallelOpcode::ParallelPartition,
            0x53 => ParallelOpcode::ParallelShuffle,
            0x60 => ParallelOpcode::GetThreadId,
            0x61 => ParallelOpcode::GetThreadCount,
            0x62 => ParallelOpcode::SetThreadAffinity,
            0x63 => ParallelOpcode::YieldThread,
            0x70 => ParallelOpcode::WorkSteal,
            0x71 => ParallelOpcode::LoadBalance,
            0x72 => ParallelOpcode::TaskQueue,
            0x73 => ParallelOpcode::TaskDequeue,
            0x80 => ParallelOpcode::ParallelRead,
            0x81 => ParallelOpcode::ParallelWrite,
            0x82 => ParallelOpcode::ParallelFlush,
            0x90 => ParallelOpcode::StartTimer,
            0x91 => ParallelOpcode::StopTimer,
            0x92 => ParallelOpcode::GetCycles,
            0x93 => ParallelOpcode::GetCacheStats,
            _ => ParallelOpcode::Map, // Default fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operand_counts() {
        assert_eq!(ParallelOpcode::Map.operand_count(), 1);
        assert_eq!(ParallelOpcode::ParallelAdd.operand_count(), 2);
        assert_eq!(ParallelOpcode::AtomicCas.operand_count(), 3);
        assert_eq!(ParallelOpcode::Barrier.operand_count(), 0);
    }

    #[test]
    fn test_synchronization_requirements() {
        assert!(ParallelOpcode::Barrier.requires_synchronization());
        assert!(ParallelOpcode::AtomicAdd.requires_synchronization());
        assert!(!ParallelOpcode::Map.requires_synchronization());
        assert!(!ParallelOpcode::ParallelAdd.requires_synchronization());
    }

    #[test]
    fn test_thread_safety() {
        assert!(ParallelOpcode::Map.is_thread_safe());
        assert!(ParallelOpcode::AtomicAdd.is_thread_safe());
        assert!(!ParallelOpcode::Lock.is_thread_safe());
    }

    #[test]
    fn test_scaling_factors() {
        assert_eq!(ParallelOpcode::Map.scaling_factor(), ScalingFactor::Linear);
        assert_eq!(ParallelOpcode::Reduce.scaling_factor(), ScalingFactor::Logarithmic);
        assert_eq!(ParallelOpcode::Sort.scaling_factor(), ScalingFactor::LogLinear);
        assert_eq!(ParallelOpcode::AtomicAdd.scaling_factor(), ScalingFactor::Sublinear);
    }

    #[test]
    fn test_memory_patterns() {
        assert_eq!(ParallelOpcode::ParallelLoad.memory_pattern(), MemoryAccessPattern::Sequential);
        assert_eq!(ParallelOpcode::Map.memory_pattern(), MemoryAccessPattern::Strided);
        assert_eq!(ParallelOpcode::ParallelSearch.memory_pattern(), MemoryAccessPattern::Random);
        assert_eq!(ParallelOpcode::Reduce.memory_pattern(), MemoryAccessPattern::Gather);
    }

    #[test]
    fn test_display() {
        assert_eq!(ParallelOpcode::Map.to_string(), "par.map");
        assert_eq!(ParallelOpcode::AtomicCas.to_string(), "par.atomic.cas");
        assert_eq!(ParallelOpcode::GetThreadId.to_string(), "par.thread.id");
    }
}