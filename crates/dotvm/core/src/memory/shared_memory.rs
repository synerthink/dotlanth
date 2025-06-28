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

/// Represents a shared memory region.
#[derive(Debug, Clone)]
pub struct SharedMemoryRegion {
    pub id: u64,
    pub memory_handle: MemoryHandle,
    pub access_list: HashMap<u64, Protection>, // Contract ID -> Permissions
}

impl SharedMemoryRegion {
    pub fn new(id: u64, handle: MemoryHandle) -> Self {
        SharedMemoryRegion {
            id,
            memory_handle: handle,
            access_list: HashMap::new(),
        }
    }

    pub fn share(&mut self, contract_id: u64, protection: Protection) {
        self.access_list.insert(contract_id, protection);
    }

    pub fn unshare(&mut self, contract_id: u64) {
        self.access_list.remove(&contract_id);
    }
}

/// Manages shared memory regions and ensures isolation between contracts.
pub struct SharedMemoryManager<A: Architecture> {
    regions: HashMap<u64, SharedMemoryRegion>,
    allocator: Allocator<A>,
}

impl<A: Architecture> Default for SharedMemoryManager<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: Architecture> SharedMemoryManager<A> {
    /// Creates a new SharedMemoryManager instance.
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
            allocator: Allocator::new(1024 * 1024), // Use 1MB for testing
        }
    }

    /// Creates a new shared memory region.
    pub fn create_region(&mut self, id: u64, size: usize) -> Result<(), MemoryError> {
        let handle = self.allocator.allocate(size)?;
        let region = SharedMemoryRegion::new(id, handle);
        self.regions.insert(id, region);
        Ok(())
    }

    /// Removes a shared memory region.
    pub fn remove_region(&mut self, id: u64) -> Result<(), MemoryError> {
        if let Some(region) = self.regions.remove(&id) {
            self.allocator
                .deallocate(region.memory_handle)
                .map_err(|e| MemoryError::AllocationError(format!("Failed to deallocate region {id}: {e:?}")))?;
            Ok(())
        } else {
            Err(MemoryError::InvalidRegion(format!("Region {id} not found")))
        }
    }

    /// Shares a memory region with a contract.
    pub fn share_region(&mut self, region_id: u64, contract_id: u64, protection: Protection) -> Result<(), MemoryError> {
        if let Some(region) = self.regions.get_mut(&region_id) {
            region.share(contract_id, protection);
            Ok(())
        } else {
            Err(MemoryError::InvalidRegion(format!("Region {region_id} does not exist")))
        }
    }

    /// Unshares a memory region from a contract.
    pub fn unshare_region(&mut self, region_id: u64, contract_id: u64) -> Result<(), MemoryError> {
        self.regions
            .get_mut(&region_id)
            .map(|region| region.unshare(contract_id))
            .ok_or_else(|| MemoryError::InvalidRegion(format!("Region {region_id} not found")))
    }

    /// Retrieves a shared memory region by ID.
    pub fn get_region(&self, id: u64) -> Option<&SharedMemoryRegion> {
        self.regions.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_memory_region_creation() {
        let handle = MemoryHandle(123); // Fake handle for testing
        let region = SharedMemoryRegion::new(1, handle);
        assert_eq!(region.id, 1);
        assert_eq!(region.memory_handle.0, 123, "Memory handle should be initialized correctly.");
    }

    #[test]
    fn test_shared_memory_manager_add_remove() {
        let mut manager = SharedMemoryManager::<Arch64>::new();

        // Create Zones
        manager.create_region(1, 1024).expect("Failed to create region 1");
        manager.create_region(2, 2048).expect("Failed to create region 2");

        // Sharing and unsharing
        manager.share_region(1, 100, Protection::ReadWrite).unwrap();
        manager.unshare_region(1, 100).unwrap();

        // Remove the zone
        manager.remove_region(1).expect("Failed to remove region 1");
        assert!(manager.get_region(1).is_none());
    }

    #[test]
    fn test_memory_isolation_between_shared_regions() {
        let mut manager = SharedMemoryManager::<Arch64>::new();

        manager.create_region(1, 1024).unwrap();
        manager.create_region(2, 2048).unwrap();

        let region1 = manager.get_region(1).unwrap();
        let region2 = manager.get_region(2).unwrap();
        assert_ne!(region1.memory_handle.0, region2.memory_handle.0, "Shared memory regions should have different handles");
    }
}
