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
use std::collections::HashMap;

/// Page table entry flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageFlags {
    pub present: bool,
    pub writable: bool,
    pub executable: bool,
    pub user_accessible: bool,
    pub cached: bool,
}

impl PageFlags {
    pub fn to_protection(&self) -> Protection {
        match (self.writable, self.executable) {
            (true, true) => Protection::ReadWriteExecute,
            (true, false) => Protection::ReadWrite,
            (false, true) => Protection::ReadExecute,
            (false, false) if self.present => Protection::ReadOnly,
            _ => Protection::None,
        }
    }

    pub fn check_protection(&self, required: Protection) -> bool {
        let current = self.to_protection();
        current >= required
    }
}

/// Page table entry
pub struct PageTableEntry {
    physical_address: PhysicalAddress,
    flags: PageFlags,
}

/// Page table structure supporting multiple levels
pub struct PageTable<A: Architecture> {
    entries: HashMap<VirtualAddress, PageTableEntry>,
    free_pages: Vec<PhysicalAddress>,
    _phantom: PhantomData<A>,
}

impl<A: Architecture> PageTable<A> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            free_pages: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn map(&mut self, virtual_addr: VirtualAddress, physical_addr: PhysicalAddress, flags: PageFlags) -> Result<(), MemoryError> {
        // Check virtual address alignment
        if virtual_addr.0 % A::PAGE_SIZE != 0 {
            return Err(MemoryError::InvalidAlignment(virtual_addr.0));
        }

        // Check for existing mapping
        if self.entries.contains_key(&virtual_addr) {
            return Err(MemoryError::PageTableError("Virtual address already mapped".to_string()));
        }

        // Insert new entry
        self.entries.insert(virtual_addr, PageTableEntry {
            physical_address: physical_addr,
            flags,
        });

        Ok(())
    }

    pub fn unmap(&mut self, virtual_addr: VirtualAddress) -> Result<(), MemoryError> {
        if let Some(entry) = self.entries.remove(&virtual_addr) {
            self.free_pages.push(entry.physical_address);
            Ok(())
        } else {
            Err(MemoryError::PageTableError("Virtual address not mapped".into()))
        }
    }

    pub fn translate(&self, virtual_addr: VirtualAddress) -> Option<(PhysicalAddress, PageFlags)> {
        self.entries.get(&virtual_addr).map(|entry| (entry.physical_address, entry.flags))
    }

    pub fn update_flags(&mut self, virtual_addr: VirtualAddress, flags: PageFlags) -> Result<(), MemoryError> {
        if let Some(entry) = self.entries.get_mut(&virtual_addr) {
            entry.flags = flags;
            Ok(())
        } else {
            Err(MemoryError::PageTableError("Virtual address not mapped".to_string()))
        }
    }

    pub fn reverse_mapping(&self, phys_addr: PhysicalAddress) -> Option<(VirtualAddress, PageFlags)> {
        self.entries.iter().find(|(_, entry)| entry.physical_address == phys_addr).map(|(virt, entry)| (*virt, entry.flags))
    }

    pub fn find_contiguous_virtual_space(&self, size: usize) -> Result<VirtualAddress, MemoryError> {
        let mut current_addr = VirtualAddress::new(0);
        let end_addr = VirtualAddress::new(A::MAX_MEMORY);
        let required_pages = size / A::PAGE_SIZE;
        let mut found_pages = 0;

        while current_addr < end_addr {
            if !self.entries.contains_key(&current_addr) {
                found_pages += 1;
                if found_pages == required_pages {
                    return Ok(VirtualAddress::new(current_addr.0 - (required_pages - 1) * A::PAGE_SIZE));
                }
            } else {
                found_pages = 0;
            }
            current_addr = VirtualAddress::new(current_addr.0 + A::PAGE_SIZE);
        }

        Err(MemoryError::OutOfVirtualMemory)
    }
}

/// TLB (Translation Lookaside Buffer) implementation
pub struct TLB<A: Architecture> {
    entries: HashMap<VirtualAddress, (PhysicalAddress, PageFlags)>,
    order: Vec<VirtualAddress>,
    capacity: usize,
    _phantom: PhantomData<A>,
}

impl<A: Architecture> TLB<A> {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
            order: Vec::with_capacity(capacity),
            capacity,
            _phantom: PhantomData,
        }
    }

    pub fn lookup(&self, virtual_addr: VirtualAddress) -> Option<(PhysicalAddress, PageFlags)> {
        self.entries.get(&virtual_addr).copied()
    }

    pub fn insert(&mut self, virtual_addr: VirtualAddress, physical_addr: PhysicalAddress, flags: PageFlags) {
        // Remove the old entry (if it exists)
        if self.entries.contains_key(&virtual_addr) {
            if let Some(pos) = self.order.iter().position(|&x| x == virtual_addr) {
                self.order.remove(pos);
            }
        }

        // If capacity is exceeded, remove the oldest entry
        if self.order.len() >= self.capacity {
            if let Some(oldest) = self.order.pop() {
                self.entries.remove(&oldest);
            }
        }

        // Insert the new entry
        self.order.insert(0, virtual_addr); // The newest is added to the front
        self.entries.insert(virtual_addr, (physical_addr, flags));
    }

    pub fn flush(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod page_table_tests {
    use super::*;

    // Helper functions
    fn create_test_flags() -> PageFlags {
        PageFlags {
            present: true,
            writable: true,
            executable: false,
            user_accessible: true,
            cached: true,
        }
    }

    fn create_aligned_address<A: Architecture>(addr: usize) -> VirtualAddress {
        VirtualAddress(addr - (addr % A::PAGE_SIZE))
    }

    mod page_table_basic_tests {
        use super::*;

        #[test]
        fn test_new_page_table() {
            let table = PageTable::<Arch64>::new();
            assert!(table.entries.is_empty());
            assert!(table.free_pages.is_empty());
        }

        #[test]
        fn test_basic_mapping() {
            let mut table = PageTable::<Arch64>::new();
            let vaddr = create_aligned_address::<Arch64>(0x1000);
            let paddr = PhysicalAddress(0x2000);
            let flags = create_test_flags();

            assert!(table.map(vaddr, paddr, flags).is_ok());
            assert_eq!(table.translate(vaddr), Some((paddr, flags)));
        }

        #[test]
        fn test_unmap() {
            let mut table = PageTable::<Arch64>::new();
            let vaddr = create_aligned_address::<Arch64>(0x1000);
            let paddr = PhysicalAddress(0x2000);
            let flags = create_test_flags();

            table.map(vaddr, paddr, flags).expect("Failed to map page");
            assert!(table.unmap(vaddr).is_ok());
            assert_eq!(table.translate(vaddr), None);
        }

        #[test]
        fn test_update_flags() {
            let mut table = PageTable::<Arch64>::new();
            let vaddr = create_aligned_address::<Arch64>(0x1000);
            let paddr = PhysicalAddress(0x2000);
            let flags = create_test_flags();

            table.map(vaddr, paddr, flags).expect("Failed to map page");

            let new_flags = PageFlags { writable: false, ..flags };

            assert!(table.update_flags(vaddr, new_flags).is_ok());
            assert_eq!(table.translate(vaddr), Some((paddr, new_flags)));
        }
    }

    mod page_table_error_tests {
        use super::*;

        #[test]
        fn test_double_mapping() {
            let mut table = PageTable::<Arch64>::new();
            let vaddr = create_aligned_address::<Arch64>(0x1000);
            let paddr = PhysicalAddress(0x2000);
            let flags = create_test_flags();

            assert!(table.map(vaddr, paddr, flags).is_ok());
            assert!(matches!(table.map(vaddr, paddr, flags), Err(MemoryError::PageTableError(_))));
        }

        #[test]
        fn test_unmap_unmapped() {
            let mut table = PageTable::<Arch64>::new();
            let vaddr = create_aligned_address::<Arch64>(0x1000);

            assert!(matches!(table.unmap(vaddr), Err(MemoryError::PageTableError(_))));
        }

        #[test]
        fn test_update_flags_unmapped() {
            let mut table = PageTable::<Arch64>::new();
            let vaddr = create_aligned_address::<Arch64>(0x1000);
            let flags = create_test_flags();

            assert!(matches!(table.update_flags(vaddr, flags), Err(MemoryError::PageTableError(_))));
        }

        #[test]
        fn test_unaligned_mapping() {
            let mut table = PageTable::<Arch64>::new();
            let vaddr = VirtualAddress(0x1001); // Unaligned address
            let paddr = PhysicalAddress(0x2000);
            let flags = create_test_flags();

            assert!(matches!(table.map(vaddr, paddr, flags), Err(MemoryError::InvalidAlignment(_))));
        }
    }

    mod page_protection_tests {
        use super::*;

        #[test]
        fn test_read_only_protection() {
            let mut table = PageTable::<Arch64>::new();
            let vaddr = create_aligned_address::<Arch64>(0x1000);
            let paddr = PhysicalAddress(0x2000);
            let flags = PageFlags {
                writable: false,
                ..create_test_flags()
            };

            table.map(vaddr, paddr, flags).expect("Failed to map page");
            let (_, retrieved_flags) = table.translate(vaddr).unwrap();
            assert!(!retrieved_flags.writable);
        }

        #[test]
        fn test_execute_protection() {
            let mut table = PageTable::<Arch64>::new();
            let vaddr = create_aligned_address::<Arch64>(0x1000);
            let paddr = PhysicalAddress(0x2000);
            let flags = PageFlags {
                executable: false,
                ..create_test_flags()
            };

            table.map(vaddr, paddr, flags).expect("Failed to map page");
            let (_, retrieved_flags) = table.translate(vaddr).unwrap();
            assert!(!retrieved_flags.executable);
        }

        #[test]
        fn test_user_access_protection() {
            let mut table = PageTable::<Arch64>::new();
            let vaddr = create_aligned_address::<Arch64>(0x1000);
            let paddr = PhysicalAddress(0x2000);
            let flags = PageFlags {
                user_accessible: false,
                ..create_test_flags()
            };

            table.map(vaddr, paddr, flags).expect("Failed to map page");
            let (_, retrieved_flags) = table.translate(vaddr).unwrap();
            assert!(!retrieved_flags.user_accessible);
        }
    }

    mod tlb_tests {
        use super::*;

        #[test]
        fn test_tlb_basic_functionality() {
            let mut tlb = TLB::<Arch64>::new(4);
            let vaddr = create_aligned_address::<Arch64>(0x1000);
            let paddr = PhysicalAddress(0x2000);
            let flags = create_test_flags();

            tlb.insert(vaddr, paddr, flags);
            assert_eq!(tlb.lookup(vaddr), Some((paddr, flags)));
        }

        #[test]
        fn test_tlb_capacity() {
            let mut tlb = TLB::<Arch64>::new(2);
            let flags = create_test_flags();

            // Insert more entries than capacity
            for i in 0..4 {
                let vaddr = create_aligned_address::<Arch64>(i * 0x1000);
                let paddr = PhysicalAddress(i * 0x2000);
                tlb.insert(vaddr, paddr, flags);
            }

            // Verify only most recent entries are present
            let first_vaddr = create_aligned_address::<Arch64>(0);
            assert_eq!(tlb.lookup(first_vaddr), None);
        }

        #[test]
        fn test_tlb_flush() {
            let mut tlb = TLB::<Arch64>::new(4);
            let vaddr = create_aligned_address::<Arch64>(0x1000);
            let paddr = PhysicalAddress(0x2000);
            let flags = create_test_flags();

            tlb.insert(vaddr, paddr, flags);
            tlb.flush();
            assert_eq!(tlb.lookup(vaddr), None);
        }

        #[test]
        fn test_tlb_update() {
            let mut tlb = TLB::<Arch64>::new(4);
            let vaddr = create_aligned_address::<Arch64>(0x1000);
            let paddr = PhysicalAddress(0x2000);
            let flags = create_test_flags();

            tlb.insert(vaddr, paddr, flags);

            // Update with new flags
            let new_flags = PageFlags { writable: false, ..flags };
            tlb.insert(vaddr, paddr, new_flags);

            assert_eq!(tlb.lookup(vaddr), Some((paddr, new_flags)));
        }
    }

    mod integration_tests {
        use super::*;

        #[test]
        fn test_page_table_tlb_integration() {
            let mut table = PageTable::<Arch64>::new();
            let mut tlb = TLB::<Arch64>::new(4);
            let vaddr = create_aligned_address::<Arch64>(0x1000);
            let paddr = PhysicalAddress(0x2000);
            let flags = create_test_flags();

            // Map in page table
            table.map(vaddr, paddr, flags).expect("Failed to map page");

            // Insert in TLB
            if let Some((paddr, flags)) = table.translate(vaddr) {
                tlb.insert(vaddr, paddr, flags);
            }

            // Verify TLB lookup matches page table
            assert_eq!(tlb.lookup(vaddr), table.translate(vaddr));
        }

        #[test]
        fn test_protection_changes_propagation() {
            let mut table = PageTable::<Arch64>::new();
            let mut tlb = TLB::<Arch64>::new(4);
            let vaddr = create_aligned_address::<Arch64>(0x1000);
            let paddr = PhysicalAddress(0x2000);
            let flags = create_test_flags();

            // Initial mapping
            table.map(vaddr, paddr, flags).expect("Failed to map page");
            tlb.insert(vaddr, paddr, flags);

            // Update protection
            let new_flags = PageFlags { writable: false, ..flags };
            table.update_flags(vaddr, new_flags).expect("Failed to update flags");

            // TLB should be flushed and updated
            tlb.flush();
            if let Some((paddr, flags)) = table.translate(vaddr) {
                tlb.insert(vaddr, paddr, flags);
            }

            assert_eq!(tlb.lookup(vaddr), Some((paddr, new_flags)));
        }
    }
}
