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

use dotvm_core::memory::{
    Allocator, Arch32, Arch64, Architecture, MemoryError, MemoryHandle, MemoryManagement,
    MemoryManager, MemoryPool, PageTable, PhysicalAddress, Protection, VirtualAddress,
};

// Common test configurations
pub const TEST_PAGE_SIZE: usize = 4096;
pub const TEST_MEM_SIZE: usize = 1024 * 1024 * 16; // 16MB for testing

// Reexport test modules
pub mod memory;

// Common test utilities
pub mod test_utils {
    use super::*;

    pub fn align_address<A: Architecture>(addr: usize) -> usize {
        (addr + A::ALIGNMENT - 1) & !(A::ALIGNMENT - 1)
    }

    pub fn is_aligned<A: Architecture>(addr: usize) -> bool {
        addr % A::ALIGNMENT == 0
    }

    pub fn create_test_protection() -> Protection {
        Protection::ReadWrite
    }
}

// Common assertions for tests
#[macro_export]
macro_rules! assert_aligned {
    ($addr:expr, $arch:ty) => {
        assert_eq!(
            $addr % <$arch>::ALIGNMENT,
            0,
            "Address {:#x} is not aligned to {} bytes",
            $addr,
            <$arch>::ALIGNMENT
        );
    };
}

#[macro_export]
macro_rules! assert_page_aligned {
    ($addr:expr, $arch:ty) => {
        assert_eq!(
            $addr % <$arch>::PAGE_SIZE,
            0,
            "Address {:#x} is not page aligned to {} bytes",
            $addr,
            <$arch>::PAGE_SIZE
        );
    };
}

// Test fixtures and helpers
pub mod fixtures {
    use super::*;
    use std::marker::PhantomData;

    pub struct TestMemoryManager<A: Architecture> {
        pub manager: MemoryManager<A>,
    }

    impl<A: Architecture> TestMemoryManager<A> {
        pub fn new() -> Result<Self, MemoryError> {
            Ok(Self {
                manager: MemoryManager::new()?,
            })
        }

        pub fn allocate_test_pages(
            &mut self,
            count: usize,
        ) -> Result<Vec<MemoryHandle>, MemoryError> {
            let mut handles = Vec::new();
            for _ in 0..count {
                handles.push(self.manager.allocate(A::PAGE_SIZE)?);
            }
            Ok(handles)
        }

        pub fn cleanup(&mut self, handles: &[MemoryHandle]) {
            for &handle in handles {
                let _ = self.manager.deallocate(handle);
            }
        }
    }
}

// Re-export commonly used test utilities
pub use fixtures::*;
pub use test_utils::*;
