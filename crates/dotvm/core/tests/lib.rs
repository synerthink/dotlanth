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
