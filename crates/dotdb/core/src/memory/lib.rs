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

/// Aligns a size to the specified alignment boundary
/// This is critical for memory operations that require specific alignment
/// The function rounds up the size to the next multiple of alignment
pub fn align_to(size: usize, alignment: usize) -> usize {
    (size + alignment - 1) & !(alignment - 1)
}

/// Checks if a number is a power of two
/// Powers of two have only one bit set to 1, which this function efficiently tests
pub fn is_power_of_two(n: usize) -> bool {
    n != 0 && (n & (n - 1)) == 0
}

/// Gets the next power of two for a given number
/// Used for efficient memory allocation that requires power-of-two sizes
pub fn next_power_of_two(n: usize) -> usize {
    if n == 0 {
        return 1;
    }

    let mut power = 1;
    while power < n {
        power <<= 1;
    }
    power
}

/// Gets the system page size in a platform-independent way
/// Page size is important for memory-mapped operations and allocation optimization
pub fn get_page_size() -> usize {
    unsafe {
        #[cfg(unix)]
        {
            libc::sysconf(libc::_SC_PAGESIZE) as usize
        }
        #[cfg(windows)]
        {
            let mut info = mem::zeroed();
            windows_sys::Win32::System::SystemInformation::GetSystemInfo(&mut info);
            info.dwPageSize as usize
        }
        #[cfg(not(any(unix, windows)))]
        {
            4096 // Default page size
        }
    }
}

/// A struct to track memory usage statistics
/// Useful for monitoring and optimizing memory consumption
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub allocated: usize,        // Total bytes allocated
    pub deallocated: usize,      // Total bytes deallocated
    pub current_usage: usize,    // Current memory in use (allocated - deallocated)
    pub peak_usage: usize,       // Maximum memory usage recorded
    pub allocation_count: u64,   // Number of allocation operations
    pub deallocation_count: u64, // Number of deallocation operations
}

impl MemoryStats {
    /// Records a memory allocation and updates statistics
    pub fn record_allocation(&mut self, size: usize) {
        self.allocated += size;
        self.current_usage += size;
        self.allocation_count += 1;

        if self.current_usage > self.peak_usage {
            self.peak_usage = self.current_usage;
        }
    }

    /// Records a memory deallocation and updates statistics
    pub fn record_deallocation(&mut self, size: usize) {
        self.deallocated += size;
        self.current_usage = self.current_usage.saturating_sub(size);
        self.deallocation_count += 1;
    }

    /// Calculates memory fragmentation ratio
    /// A higher ratio indicates more fragmentation
    pub fn fragmentation_ratio(&self) -> f64 {
        if self.allocated == 0 {
            return 0.0;
        }

        let wasted = self.allocated.saturating_sub(self.current_usage);
        wasted as f64 / self.allocated as f64
    }
}
