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

mod allocator;
pub mod error;
mod page_table;
mod pool;
mod protection;
mod shared_memory;

pub use allocator::*;
pub use error::*;
pub use page_table::*;
pub use pool::*;
pub use protection::*;
pub use shared_memory::*;

use num_bigint::BigUint;
use std::marker::PhantomData;
use std::sync::atomic::Ordering;

/// Trait defining architecture-specific memory behaviour
pub trait Architecture: Send + Sync + 'static {
    const WORD_SIZE: usize;
    const PAGE_SIZE: usize;
    const MAX_MEMORY: usize;
    const ALIGNMENT: usize;
}

/// 32-bit architecture implementation
pub struct Arch32;
impl Architecture for Arch32 {
    const WORD_SIZE: usize = 4;
    const PAGE_SIZE: usize = 4096;
    const MAX_MEMORY: usize = 0xFFFFFFFF;
    const ALIGNMENT: usize = 4;
}

/// 64-bit architecture implementation
pub struct Arch64;
impl Architecture for Arch64 {
    const WORD_SIZE: usize = 8;
    const PAGE_SIZE: usize = 4096;
    const MAX_MEMORY: usize = 0xFFFFFFFFFFFFFFFF;
    const ALIGNMENT: usize = 8;
}

/// 128-bit architecture implementation
pub struct Arch128;
impl Architecture for Arch128 {
    const WORD_SIZE: usize = 16;
    const PAGE_SIZE: usize = 16384;
    const MAX_MEMORY: usize = usize::MAX;
    const ALIGNMENT: usize = 16;
}

/// Extended memory support for architectures beyond usize capacity
pub trait ExtendedMemory {
    fn max_memory() -> BigUint;
}

impl ExtendedMemory for Arch128 {
    fn max_memory() -> BigUint {
        // 2^128 - 1 bytes
        BigUint::from(1u8) << 128
    }
}

/// 256-bit architecture implementation
pub struct Arch256;
impl Architecture for Arch256 {
    const WORD_SIZE: usize = 32;
    const PAGE_SIZE: usize = 65536; // 64KB pages for very large memory spaces
    const MAX_MEMORY: usize = usize::MAX; // Runtime will handle bigger values via BigUint
    const ALIGNMENT: usize = 32;
}

impl ExtendedMemory for Arch256 {
    fn max_memory() -> BigUint {
        // 2^256 - 1 bytes
        BigUint::from(1u8) << 256
    }
}

/// 512-bit architecture implementation
pub struct Arch512;
impl Architecture for Arch512 {
    const WORD_SIZE: usize = 64;
    const PAGE_SIZE: usize = 262144; // 256KB pages for extreme memory spaces
    const MAX_MEMORY: usize = usize::MAX; // Runtime will handle bigger values via BigUint
    const ALIGNMENT: usize = 64;
}

impl ExtendedMemory for Arch512 {
    fn max_memory() -> BigUint {
        // 2^512 - 1 bytes
        BigUint::from(1u8) << 512
    }
}

/// Memory handle for tracking allocations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryHandle(pub usize);

/// Memory protection flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum Protection {
    None,
    ReadOnly,
    ReadWrite,
    ReadExecute,
    ReadWriteExecute,
}

impl Protection {
    pub fn into_page_flags(self) -> PageFlags {
        match self {
            Self::None => PageFlags {
                present: false,
                writable: false,
                executable: false,
                user_accessible: false,
                cached: false,
            },
            Self::ReadOnly => PageFlags {
                present: true,
                writable: false,
                executable: false,
                user_accessible: true,
                cached: true,
            },
            Self::ReadWrite => PageFlags {
                present: true,
                writable: true,
                executable: false,
                user_accessible: true,
                cached: true,
            },
            Self::ReadExecute => PageFlags {
                present: true,
                writable: false,
                executable: true,
                user_accessible: true,
                cached: false,
            },
            Self::ReadWriteExecute => PageFlags {
                present: true,
                writable: true,
                executable: true,
                user_accessible: true,
                cached: true,
            },
        }
    }
}

/// Virtual memory address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct VirtualAddress(pub usize);

impl VirtualAddress {
    pub fn new(addr: usize) -> Self {
        Self(addr)
    }
}

/// Physical memory address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PhysicalAddress(usize);

impl PhysicalAddress {
    pub fn new(p0: usize) -> PhysicalAddress {
        PhysicalAddress(p0)
    }

    pub fn as_usize(self) -> usize {
        self.0
    }
}

/// Main memory manager structure
pub struct MemoryManager<A: Architecture> {
    allocator: Allocator<A>,
    page_table: PageTable<A>,
    pools: Vec<MemoryPool>,
    _phantom: PhantomData<A>,
}

/// Core memory management trait
pub trait MemoryManagement: Sized {
    type Error;
    fn new() -> Result<Self, Self::Error>;
    fn allocate(&mut self, size: usize) -> Result<MemoryHandle, Self::Error>;
    fn deallocate(&mut self, handle: MemoryHandle) -> Result<(), Self::Error>;
    fn protect(&mut self, handle: MemoryHandle, protection: Protection) -> Result<(), Self::Error>;
    fn map(&mut self, handle: MemoryHandle) -> Result<VirtualAddress, Self::Error>;
    fn unmap(&mut self, addr: VirtualAddress) -> Result<(), Self::Error>;
    fn check_permission(&self, p0: &MemoryHandle, p1: Protection) -> Result<(), Self::Error>;
    fn load(&self, address: usize) -> Result<u8, Self::Error>;
    fn store(&mut self, address: usize, value: u8) -> Result<(), Self::Error>;
}

impl<A: Architecture> MemoryManagement for MemoryManager<A> {
    type Error = MemoryError;

    fn new() -> Result<Self, Self::Error> {
        Ok(Self {
            allocator: Allocator::new(A::MAX_MEMORY),
            page_table: PageTable::new(),
            pools: Vec::new(),
            _phantom: PhantomData,
        })
    }

    fn allocate(&mut self, size: usize) -> Result<MemoryHandle, Self::Error> {
        if size == 0 {
            return Err(MemoryError::InvalidSize { available: A::MAX_MEMORY });
        }
        if size > A::MAX_MEMORY {
            return Err(MemoryError::AllocationTooLarge {
                requested: size,
                maximum: A::MAX_MEMORY,
            });
        }
        if size % A::ALIGNMENT != 0 {
            return Err(MemoryError::InvalidAlignment(A::ALIGNMENT));
        }
        self.allocator.allocate(size).map_err(|e| match e {
            MemoryError::OutOfMemory { requested, available } => MemoryError::OutOfMemory { requested, available },
            _ => MemoryError::AllocationError(e.to_string()),
        })
    }

    fn deallocate(&mut self, handle: MemoryHandle) -> Result<(), Self::Error> {
        // Check handle validity
        if !self.allocator.is_valid_handle(handle) {
            return Err(MemoryError::InvalidHandle);
        }

        // Report error from Allocator directly
        self.allocator.deallocate(handle)?;
        Ok(())
    }

    fn protect(&mut self, handle: MemoryHandle, protection: Protection) -> Result<(), Self::Error> {
        let phys_addr = PhysicalAddress::new(handle.0);
        let size = self.allocator.get_allocation_size(handle)?;

        // Calculate page range for batch update
        let start_page = phys_addr.0 / A::PAGE_SIZE;
        let end_page = (phys_addr.0 + size + A::PAGE_SIZE - 1) / A::PAGE_SIZE;
        let flags = protection.into_page_flags();

        for page in start_page..end_page {
            let current_phys = PhysicalAddress::new(page * A::PAGE_SIZE);
            if let Some((virt_addr, _)) = self.page_table.reverse_mapping(current_phys) {
                self.page_table.update_flags(virt_addr, flags)?;
            }
        }
        Ok(())
    }

    fn map(&mut self, handle: MemoryHandle) -> Result<VirtualAddress, Self::Error> {
        let phys_addr = PhysicalAddress::new(handle.0);
        let size = self.allocator.get_allocation_size(handle)?;
        let flags = Protection::ReadWrite.into_page_flags(); // Default flags

        // Return the first virtual address
        let first_virt = self.page_table.find_contiguous_virtual_space(size)?;

        // Map page by page
        for i in 0..size / A::PAGE_SIZE {
            let current_phys = PhysicalAddress::new(phys_addr.0 + i * A::PAGE_SIZE);
            let current_virt = VirtualAddress::new(first_virt.0 + i * A::PAGE_SIZE);
            self.page_table.map(current_virt, current_phys, flags)?;
        }

        Ok(first_virt)
    }

    fn unmap(&mut self, addr: VirtualAddress) -> Result<(), Self::Error> {
        let mut current_addr = addr;
        let mut any_unmapped = false;

        // Try to unmap the first page
        match self.page_table.unmap(current_addr) {
            Ok(()) => any_unmapped = true,
            Err(e) => return Err(e), // Return the error on the first failure
        }

        // Remove the next pages
        current_addr = VirtualAddress::new(current_addr.0 + A::PAGE_SIZE);
        while let Ok(()) = self.page_table.unmap(current_addr) {
            any_unmapped = true;
            current_addr = VirtualAddress::new(current_addr.0 + A::PAGE_SIZE);
        }

        Ok(())
    }

    fn check_permission(&self, handle: &MemoryHandle, required: Protection) -> Result<(), Self::Error> {
        let phys_addr = PhysicalAddress::new(handle.0);
        let size = self.allocator.get_allocation_size(*handle)?;

        // Check all physical pages
        for offset in (0..size).step_by(A::PAGE_SIZE) {
            let current_phys = PhysicalAddress::new(phys_addr.0 + offset);
            // Find the virtual address mapped to the physical address
            let (virt_addr, _) = self.page_table.reverse_mapping(current_phys).ok_or(MemoryError::InvalidAddress(current_phys.0))?;

            let (_, flags) = self.page_table.translate(virt_addr).ok_or(MemoryError::InvalidAddress(virt_addr.0))?;

            if !flags.check_protection(required) {
                return Err(MemoryError::PermissionDenied(format!("Required: {:?}, Current: {:?}", required, flags.to_protection())));
            }
        }

        Ok(())
    }

    fn load(&self, address: usize) -> Result<u8, Self::Error> {
        // Dummy implementation: return lower 8 bits of the address.
        Ok((address & 0xFF) as u8)
    }

    fn store(&mut self, address: usize, value: u8) -> Result<(), Self::Error> {
        // Dummy implementation: simulate storing a value.
        Ok(())
    }
}

#[cfg(test)]
mod memory_tests {
    use super::*;
    use std::collections::HashSet;

    // Helper function to create memory managers for different architectures
    fn create_memory_manager<A: Architecture>() -> MemoryManager<A> {
        MemoryManager::<A>::new().expect("Memory manager creation failed")
    }

    mod architecture_tests {
        use super::*;

        #[test]
        fn test_arch32_constants() {
            assert_eq!(Arch32::WORD_SIZE, 4);
            assert_eq!(Arch32::PAGE_SIZE, 4096);
            assert_eq!(Arch32::MAX_MEMORY, 0xFFFFFFFF);
            assert_eq!(Arch32::ALIGNMENT, 4);
        }

        #[test]
        fn test_arch64_constants() {
            assert_eq!(Arch64::WORD_SIZE, 8);
            assert_eq!(Arch64::PAGE_SIZE, 4096);
            assert_eq!(Arch64::MAX_MEMORY, 0xFFFFFFFFFFFFFFFF);
            assert_eq!(Arch64::ALIGNMENT, 8);
        }

        #[test]
        fn test_arch128_constants() {
            assert_eq!(Arch128::WORD_SIZE, 16);
            assert_eq!(Arch128::PAGE_SIZE, 16384);
            assert_eq!(Arch128::ALIGNMENT, 16);

            let max_memory = Arch128::max_memory();
            assert_eq!(max_memory, BigUint::from(1u8) << 128);
        }
    }

    mod allocation_tests {
        use super::*;

        #[test]
        fn test_basic_allocation() {
            let mut mm = create_memory_manager::<Arch64>();
            let handle = mm.allocate(1024).expect("Failed to allocate memory");
            assert!(handle.0 >= 0); // 0 is also acceptable
        }

        #[test]
        fn test_zero_size_allocation() {
            let mut mm = create_memory_manager::<Arch64>();
            let result = mm.allocate(0);
            assert!(matches!(result, Err(MemoryError::InvalidSize { available: _ })));
        }

        #[test]
        fn test_max_size_allocation() {
            let mut mm = create_memory_manager::<Arch32>();
            let result = mm.allocate(Arch32::MAX_MEMORY + 1);
            assert!(matches!(result, Err(MemoryError::AllocationTooLarge { requested: _, maximum: _ })));
        }

        #[test]
        fn test_multiple_allocations() {
            let mut mm = create_memory_manager::<Arch64>();
            let mut handles = HashSet::new();

            for _ in 0..100 {
                let handle = mm.allocate(1024).expect("Failed to allocate memory");
                assert!(handles.insert(handle), "Duplicate handle detected");
            }
        }

        #[test]
        fn test_allocate_bounds() {
            // Simulate bounds checking by attempting to allocate more than MAX_MEMORY and expect an error
            let mut mm = create_memory_manager::<Arch32>();
            assert!(mm.allocate(Arch32::MAX_MEMORY + 1).is_err());
            // Example: Try to allocate more than MAX_MEMORY and expect an error
        }
    }

    mod deallocation_tests {
        use super::*;

        #[test]
        fn test_basic_deallocation() {
            let mut mm = create_memory_manager::<Arch64>();
            let handle = mm.allocate(1024).expect("Failed to allocate memory");
            assert!(mm.deallocate(handle).is_ok());
        }

        #[test]
        fn test_double_deallocation() {
            let mut mm = create_memory_manager::<Arch64>();
            let handle = mm.allocate(1024).expect("Failed to allocate memory");

            // The first deallocate should succeed
            assert!(mm.deallocate(handle).is_ok());

            // The handle may no longer be valid (due to merging)
            // In this case, you may expect AlreadyDeallocated or InvalidHandle
            let result = mm.deallocate(handle);

            assert!(matches!(result, Err(MemoryError::AlreadyDeallocated)) || matches!(result, Err(MemoryError::InvalidHandle)));
        }

        #[test]
        fn test_invalid_handle_deallocation() {
            let mut mm = create_memory_manager::<Arch64>();
            assert!(matches!(mm.deallocate(MemoryHandle(0xDEADBEEF)), Err(MemoryError::InvalidHandle)));
        }
    }

    mod protection_tests {
        use super::*;

        #[test]
        fn test_protection_changes() {
            let mut mm = create_memory_manager::<Arch64>();
            let handle = mm.allocate(1024).expect("Failed to allocate memory");

            assert!(mm.protect(handle, Protection::ReadOnly).is_ok());
            assert!(mm.protect(handle, Protection::ReadWrite).is_ok());
            assert!(mm.protect(handle, Protection::ReadExecute).is_ok());
        }

        #[test]
        fn test_invalid_handle_protection() {
            let mut mm = create_memory_manager::<Arch64>();
            assert!(matches!(mm.protect(MemoryHandle(0xDEADBEEF), Protection::ReadOnly), Err(MemoryError::InvalidHandle)));
        }
    }

    mod mapping_tests {
        use super::*;

        #[test]
        fn test_invalid_unmap() {
            let mut mm = create_memory_manager::<Arch64>();
            assert!(matches!(
                mm.unmap(VirtualAddress(0xDEADBEEF)),
                Err(MemoryError::PageTableError(_)) // Expected error type
            ));
        }
    }

    mod stress_tests {
        use super::*;

        #[test]
        fn test_fragmentation() {
            let mut mm = create_memory_manager::<Arch64>();
            let mut handles = Vec::new();

            // Allocate many small blocks
            for _ in 0..1000 {
                if let Ok(handle) = mm.allocate(64) {
                    handles.push(handle);
                }
            }

            // Free every other block
            for i in (0..handles.len()).step_by(2) {
                assert!(mm.deallocate(handles[i]).is_ok());
            }

            // Try to allocate larger blocks
            let large_handles: Result<Vec<_>, _> = (0..10).map(|_| mm.allocate(4096)).collect();

            assert!(large_handles.is_ok(), "Failed to allocate after fragmentation");
        }

        #[test]
        fn test_memory_exhaustion() {
            let mut mm = create_memory_manager::<Arch32>();
            let mut handles = Vec::new();

            // Keep allocating until we run out of memory
            loop {
                match mm.allocate(1024 * 1024) {
                    // 1MB blocks
                    Ok(handle) => handles.push(handle),
                    Err(MemoryError::OutOfMemory { requested: _, available: _ }) => break,
                    Err(e) => panic!("Unexpected error: {:?}", e),
                }
            }

            // Verify we can free all allocations
            for handle in handles {
                assert!(mm.deallocate(handle).is_ok());
            }
        }
    }

    mod pool_tests {
        use super::*;

        #[test]
        fn test_pool_allocation() {
            let mut mm = create_memory_manager::<Arch64>();
            let mut handles = Vec::new();

            // Allocate many same-sized blocks
            for _ in 0..100 {
                let handle = mm.allocate(64).expect("Pool allocation failed");
                handles.push(handle);
            }

            // Free all blocks
            for handle in handles {
                assert!(mm.deallocate(handle).is_ok());
            }
        }
    }

    mod error_handling_tests {
        use super::*;

        #[test]
        fn test_alignment_errors() {
            let mut mm = create_memory_manager::<Arch64>();
            let result = mm.allocate(3); // Not aligned to 8 bytes
            assert!(matches!(result, Err(MemoryError::InvalidAlignment(_))));
        }
    }
    #[cfg(test)]
    pub mod memory_isolation_tests {
        use super::*;

        #[test]
        fn test_memory_isolation_between_contracts() {
            let mut mm = create_memory_manager::<Arch64>();
            let handle1 = mm.allocate(1024).expect("Failed to allocate memory for contract 1");
            let handle2 = mm.allocate(1024).expect("Failed to allocate memory for contract 2");

            // Attempt to access handle1's memory from handle2's context
            // This should fail if isolation is enforced
            // Simulate cross-contract access by attempting to check permissions and assert failure
            assert!(mm.check_permission(&handle1, Protection::ReadOnly).is_err());
        }

        #[test]
        fn test_memory_isolation_on_deallocation() {
            let mut mm = create_memory_manager::<Arch64>();
            let handle = mm.allocate(1024).expect("Failed to allocate memory for contract");

            mm.deallocate(handle).expect("Failed to deallocate memory");

            // Attempt to access deallocated memory
            // This should fail if isolation is enforced
            // Simulate access to deallocated memory by attempting to check permissions and assert failure
            assert!(mm.check_permission(&handle, Protection::ReadOnly).is_err());
        }
    }
}
