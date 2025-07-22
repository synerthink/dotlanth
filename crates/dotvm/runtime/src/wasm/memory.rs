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

//! WASM Memory Implementation

use crate::wasm::{WasmError, WasmResult};
use std::sync::atomic::{AtomicUsize, Ordering};

/// WASM Memory page size (64KB)
pub const PAGE_SIZE: usize = 65536;

/// WASM Memory
#[derive(Debug)]
pub struct WasmMemory {
    /// Memory data
    data: Vec<u8>,
    /// Current size in pages
    current_pages: u32,
    /// Maximum size in pages
    max_pages: Option<u32>,
    /// Memory protection enabled
    protected: bool,
    /// Access statistics
    stats: MemoryStats,
}

/// Memory access statistics
#[derive(Debug, Default)]
pub struct MemoryStats {
    /// Total reads
    pub reads: AtomicUsize,
    /// Total writes
    pub writes: AtomicUsize,
    /// Total grows
    pub grows: AtomicUsize,
    /// Bytes read
    pub bytes_read: AtomicUsize,
    /// Bytes written
    pub bytes_written: AtomicUsize,
    /// Out of bounds accesses
    pub out_of_bounds: AtomicUsize,
}

impl WasmMemory {
    /// Create a new memory instance
    pub fn new(initial_pages: u32, max_pages: Option<u32>) -> WasmResult<Self> {
        if let Some(max) = max_pages {
            if initial_pages > max {
                return Err(WasmError::validation_error("Initial pages exceed maximum pages"));
            }
        }

        let size = initial_pages as usize * PAGE_SIZE;
        let data = vec![0u8; size];

        Ok(Self {
            data,
            current_pages: initial_pages,
            max_pages,
            protected: false,
            stats: MemoryStats::default(),
        })
    }

    /// Get memory size in bytes
    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }

    /// Get memory size in pages
    pub fn size_pages(&self) -> u32 {
        self.current_pages
    }

    /// Get maximum pages
    pub fn max_pages(&self) -> Option<u32> {
        self.max_pages
    }

    /// Read a byte from memory
    pub fn read_u8(&self, addr: usize) -> WasmResult<u8> {
        self.stats.reads.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_read.fetch_add(1, Ordering::Relaxed);

        if addr >= self.data.len() {
            self.stats.out_of_bounds.fetch_add(1, Ordering::Relaxed);
            return Err(WasmError::memory_error(format!("Memory access out of bounds: {} >= {}", addr, self.data.len())));
        }

        Ok(self.data[addr])
    }

    /// Write a byte to memory
    pub fn write_u8(&mut self, addr: usize, value: u8) -> WasmResult<()> {
        self.stats.writes.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_written.fetch_add(1, Ordering::Relaxed);

        if addr >= self.data.len() {
            self.stats.out_of_bounds.fetch_add(1, Ordering::Relaxed);
            return Err(WasmError::memory_error(format!("Memory access out of bounds: {} >= {}", addr, self.data.len())));
        }

        if self.protected {
            return Err(WasmError::security_violation("Memory is write-protected"));
        }

        self.data[addr] = value;
        Ok(())
    }

    /// Read multiple bytes from memory
    pub fn read_bytes(&self, addr: usize, len: usize) -> WasmResult<&[u8]> {
        self.stats.reads.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_read.fetch_add(len, Ordering::Relaxed);

        if addr + len > self.data.len() {
            self.stats.out_of_bounds.fetch_add(1, Ordering::Relaxed);
            return Err(WasmError::memory_error(format!("Memory access out of bounds: {}..{} > {}", addr, addr + len, self.data.len())));
        }

        Ok(&self.data[addr..addr + len])
    }

    /// Write multiple bytes to memory
    pub fn write_bytes(&mut self, addr: usize, data: &[u8]) -> WasmResult<()> {
        self.stats.writes.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_written.fetch_add(data.len(), Ordering::Relaxed);

        if addr + data.len() > self.data.len() {
            self.stats.out_of_bounds.fetch_add(1, Ordering::Relaxed);
            return Err(WasmError::memory_error(format!("Memory access out of bounds: {}..{} > {}", addr, addr + data.len(), self.data.len())));
        }

        if self.protected {
            return Err(WasmError::security_violation("Memory is write-protected"));
        }

        self.data[addr..addr + data.len()].copy_from_slice(data);
        Ok(())
    }

    /// Read a 32-bit integer (little-endian)
    pub fn read_i32(&self, addr: usize) -> WasmResult<i32> {
        let bytes = self.read_bytes(addr, 4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Write a 32-bit integer (little-endian)
    pub fn write_i32(&mut self, addr: usize, value: i32) -> WasmResult<()> {
        let bytes = value.to_le_bytes();
        self.write_bytes(addr, &bytes)
    }

    /// Read a 64-bit integer (little-endian)
    pub fn read_i64(&self, addr: usize) -> WasmResult<i64> {
        let bytes = self.read_bytes(addr, 8)?;
        Ok(i64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]))
    }

    /// Write a 64-bit integer (little-endian)
    pub fn write_i64(&mut self, addr: usize, value: i64) -> WasmResult<()> {
        let bytes = value.to_le_bytes();
        self.write_bytes(addr, &bytes)
    }

    /// Read a 32-bit float (little-endian)
    pub fn read_f32(&self, addr: usize) -> WasmResult<f32> {
        let bytes = self.read_bytes(addr, 4)?;
        Ok(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Write a 32-bit float (little-endian)
    pub fn write_f32(&mut self, addr: usize, value: f32) -> WasmResult<()> {
        let bytes = value.to_le_bytes();
        self.write_bytes(addr, &bytes)
    }

    /// Read a 64-bit float (little-endian)
    pub fn read_f64(&self, addr: usize) -> WasmResult<f64> {
        let bytes = self.read_bytes(addr, 8)?;
        Ok(f64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]))
    }

    /// Write a 64-bit float (little-endian)
    pub fn write_f64(&mut self, addr: usize, value: f64) -> WasmResult<()> {
        let bytes = value.to_le_bytes();
        self.write_bytes(addr, &bytes)
    }

    /// Grow memory by specified number of pages
    pub fn grow(&mut self, delta_pages: u32) -> WasmResult<u32> {
        let old_pages = self.current_pages;
        let new_pages = old_pages + delta_pages;

        // Check maximum limit
        if let Some(max) = self.max_pages {
            if new_pages > max {
                return Err(WasmError::memory_error(format!("Memory growth would exceed maximum: {} > {}", new_pages, max)));
            }
        }

        // Check system limits (prevent excessive memory allocation)
        let new_size = new_pages as usize * PAGE_SIZE;
        if new_size > 2_147_483_648 {
            // 2GB limit
            return Err(WasmError::memory_error("Memory growth exceeds system limit"));
        }

        // Resize the data vector
        self.data.resize(new_size, 0);
        self.current_pages = new_pages;
        self.stats.grows.fetch_add(1, Ordering::Relaxed);

        Ok(old_pages)
    }

    /// Fill memory region with a value
    pub fn fill(&mut self, addr: usize, len: usize, value: u8) -> WasmResult<()> {
        if addr + len > self.data.len() {
            return Err(WasmError::memory_error("Memory fill out of bounds"));
        }

        if self.protected {
            return Err(WasmError::security_violation("Memory is write-protected"));
        }

        for i in addr..addr + len {
            self.data[i] = value;
        }

        self.stats.writes.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_written.fetch_add(len, Ordering::Relaxed);

        Ok(())
    }

    /// Copy memory region
    pub fn copy(&mut self, dst: usize, src: usize, len: usize) -> WasmResult<()> {
        if src + len > self.data.len() || dst + len > self.data.len() {
            return Err(WasmError::memory_error("Memory copy out of bounds"));
        }

        if self.protected {
            return Err(WasmError::security_violation("Memory is write-protected"));
        }

        // Handle overlapping regions
        if dst < src {
            for i in 0..len {
                self.data[dst + i] = self.data[src + i];
            }
        } else {
            for i in (0..len).rev() {
                self.data[dst + i] = self.data[src + i];
            }
        }

        self.stats.reads.fetch_add(1, Ordering::Relaxed);
        self.stats.writes.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_read.fetch_add(len, Ordering::Relaxed);
        self.stats.bytes_written.fetch_add(len, Ordering::Relaxed);

        Ok(())
    }

    /// Enable memory protection
    pub fn enable_protection(&mut self) {
        self.protected = true;
    }

    /// Disable memory protection
    pub fn disable_protection(&mut self) {
        self.protected = false;
    }

    /// Check if memory is protected
    pub fn is_protected(&self) -> bool {
        self.protected
    }

    /// Get memory statistics
    pub fn statistics(&self) -> MemoryStatistics {
        MemoryStatistics {
            size_bytes: self.size_bytes(),
            size_pages: self.size_pages(),
            max_pages: self.max_pages,
            protected: self.protected,
            reads: self.stats.reads.load(Ordering::Relaxed),
            writes: self.stats.writes.load(Ordering::Relaxed),
            grows: self.stats.grows.load(Ordering::Relaxed),
            bytes_read: self.stats.bytes_read.load(Ordering::Relaxed),
            bytes_written: self.stats.bytes_written.load(Ordering::Relaxed),
            out_of_bounds: self.stats.out_of_bounds.load(Ordering::Relaxed),
        }
    }

    /// Clear memory
    pub fn clear(&mut self) -> WasmResult<()> {
        if self.protected {
            return Err(WasmError::security_violation("Memory is write-protected"));
        }

        self.data.fill(0);
        Ok(())
    }

    /// Get raw data reference (unsafe)
    pub fn raw_data(&self) -> &[u8] {
        &self.data
    }

    /// Get mutable raw data reference (unsafe)
    pub fn raw_data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Check if address is valid
    pub fn is_valid_address(&self, addr: usize, len: usize) -> bool {
        addr + len <= self.data.len()
    }

    /// Get memory usage percentage
    pub fn usage_percentage(&self) -> f64 {
        if let Some(max) = self.max_pages { (self.current_pages as f64 / max as f64) * 100.0 } else { 0.0 }
    }
}

/// Memory statistics snapshot
#[derive(Debug, Clone)]
pub struct MemoryStatistics {
    pub size_bytes: usize,
    pub size_pages: u32,
    pub max_pages: Option<u32>,
    pub protected: bool,
    pub reads: usize,
    pub writes: usize,
    pub grows: usize,
    pub bytes_read: usize,
    pub bytes_written: usize,
    pub out_of_bounds: usize,
}

impl MemoryStatistics {
    /// Calculate read/write ratio
    pub fn read_write_ratio(&self) -> f64 {
        if self.writes == 0 {
            if self.reads == 0 { 0.0 } else { f64::INFINITY }
        } else {
            self.reads as f64 / self.writes as f64
        }
    }

    /// Calculate average read size
    pub fn average_read_size(&self) -> f64 {
        if self.reads == 0 { 0.0 } else { self.bytes_read as f64 / self.reads as f64 }
    }

    /// Calculate average write size
    pub fn average_write_size(&self) -> f64 {
        if self.writes == 0 { 0.0 } else { self.bytes_written as f64 / self.writes as f64 }
    }

    /// Get error rate
    pub fn error_rate(&self) -> f64 {
        let total_accesses = self.reads + self.writes;
        if total_accesses == 0 { 0.0 } else { self.out_of_bounds as f64 / total_accesses as f64 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_creation() {
        let memory = WasmMemory::new(1, Some(10)).unwrap();
        assert_eq!(memory.size_pages(), 1);
        assert_eq!(memory.size_bytes(), PAGE_SIZE);
        assert_eq!(memory.max_pages(), Some(10));
    }

    #[test]
    fn test_invalid_memory_creation() {
        let result = WasmMemory::new(10, Some(5));
        assert!(result.is_err());
    }

    #[test]
    fn test_byte_operations() {
        let mut memory = WasmMemory::new(1, None).unwrap();

        assert!(memory.write_u8(0, 42).is_ok());
        assert_eq!(memory.read_u8(0).unwrap(), 42);

        assert!(memory.read_u8(PAGE_SIZE).is_err());
        assert!(memory.write_u8(PAGE_SIZE, 0).is_err());
    }

    #[test]
    fn test_multi_byte_operations() {
        let mut memory = WasmMemory::new(1, None).unwrap();
        let data = vec![1, 2, 3, 4, 5];

        assert!(memory.write_bytes(0, &data).is_ok());
        let read_data = memory.read_bytes(0, 5).unwrap();
        assert_eq!(read_data, &data);
    }

    #[test]
    fn test_integer_operations() {
        let mut memory = WasmMemory::new(1, None).unwrap();

        assert!(memory.write_i32(0, -12345).is_ok());
        assert_eq!(memory.read_i32(0).unwrap(), -12345);

        assert!(memory.write_i64(8, -1234567890123456789).is_ok());
        assert_eq!(memory.read_i64(8).unwrap(), -1234567890123456789);
    }

    #[test]
    fn test_float_operations() {
        let mut memory = WasmMemory::new(1, None).unwrap();

        assert!(memory.write_f32(0, 3.14159).is_ok());
        assert!((memory.read_f32(0).unwrap() - 3.14159).abs() < f32::EPSILON);

        assert!(memory.write_f64(8, 2.718281828459045).is_ok());
        assert!((memory.read_f64(8).unwrap() - 2.718281828459045).abs() < f64::EPSILON);
    }

    #[test]
    fn test_memory_growth() {
        let mut memory = WasmMemory::new(1, Some(5)).unwrap();

        let old_pages = memory.grow(2).unwrap();
        assert_eq!(old_pages, 1);
        assert_eq!(memory.size_pages(), 3);
        assert_eq!(memory.size_bytes(), 3 * PAGE_SIZE);

        // Test growth limit
        let result = memory.grow(10);
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_fill() {
        let mut memory = WasmMemory::new(1, None).unwrap();

        assert!(memory.fill(0, 100, 0xFF).is_ok());
        for i in 0..100 {
            assert_eq!(memory.read_u8(i).unwrap(), 0xFF);
        }
    }

    #[test]
    fn test_memory_copy() {
        let mut memory = WasmMemory::new(1, None).unwrap();

        // Write some data
        let data = vec![1, 2, 3, 4, 5];
        memory.write_bytes(0, &data).unwrap();

        // Copy data
        assert!(memory.copy(10, 0, 5).is_ok());

        // Verify copy
        let copied_data = memory.read_bytes(10, 5).unwrap();
        assert_eq!(copied_data, &data);
    }

    #[test]
    fn test_memory_protection() {
        let mut memory = WasmMemory::new(1, None).unwrap();

        assert!(!memory.is_protected());
        assert!(memory.write_u8(0, 42).is_ok());

        memory.enable_protection();
        assert!(memory.is_protected());
        assert!(memory.write_u8(0, 43).is_err());

        // Read should still work
        assert_eq!(memory.read_u8(0).unwrap(), 42);

        memory.disable_protection();
        assert!(memory.write_u8(0, 43).is_ok());
    }

    #[test]
    fn test_memory_statistics() {
        let mut memory = WasmMemory::new(1, None).unwrap();

        memory.write_u8(0, 42).unwrap();
        memory.read_u8(0).unwrap();
        memory.grow(1).unwrap();

        let stats = memory.statistics();
        assert_eq!(stats.reads, 1);
        assert_eq!(stats.writes, 1);
        assert_eq!(stats.grows, 1);
        assert_eq!(stats.size_pages, 2);
    }

    #[test]
    fn test_overlapping_copy() {
        let mut memory = WasmMemory::new(1, None).unwrap();

        // Write test pattern
        let data = vec![1, 2, 3, 4, 5];
        memory.write_bytes(0, &data).unwrap();

        // Overlapping copy (forward)
        memory.copy(2, 0, 3).unwrap();
        assert_eq!(memory.read_bytes(0, 7).unwrap(), &[1, 2, 1, 2, 3, 0, 0]);
    }

    #[test]
    fn test_address_validation() {
        let memory = WasmMemory::new(1, None).unwrap();

        assert!(memory.is_valid_address(0, 100));
        assert!(memory.is_valid_address(PAGE_SIZE - 1, 1));
        assert!(!memory.is_valid_address(PAGE_SIZE, 1));
        assert!(!memory.is_valid_address(0, PAGE_SIZE + 1));
    }

    #[test]
    fn test_usage_percentage() {
        let memory = WasmMemory::new(2, Some(10)).unwrap();
        assert_eq!(memory.usage_percentage(), 20.0);

        let memory_no_max = WasmMemory::new(2, None).unwrap();
        assert_eq!(memory_no_max.usage_percentage(), 0.0);
    }

    #[test]
    fn test_statistics_calculations() {
        let mut memory = WasmMemory::new(1, None).unwrap();

        // Perform some operations
        memory.write_bytes(0, &[1, 2, 3, 4]).unwrap(); // 1 write, 4 bytes
        memory.read_bytes(0, 2).unwrap(); // 1 read, 2 bytes
        memory.read_u8(100000).unwrap_err(); // 1 out of bounds

        let stats = memory.statistics();
        assert_eq!(stats.average_write_size(), 4.0);
        assert_eq!(stats.average_read_size(), 1.5); // (2 + 1) / 2 reads
        assert_eq!(stats.read_write_ratio(), 2.0); // 2 reads / 1 write
        assert_eq!(stats.error_rate(), 1.0 / 3.0); // 1 error out of 3 total accesses
    }
}
