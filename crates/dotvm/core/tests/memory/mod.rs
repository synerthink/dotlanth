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

#[test]
fn test_memory_manager_creation() {
    let result = MemoryManager::<Arch64>::new();
    assert!(result.is_ok(), "Memory manager creation should succeed");
}

#[test]
fn test_basic_memory_operations() {
    let mut manager = MemoryManager::<Arch64>::new().expect("Memory manager creation failed");

    // Test basic allocation
    let handle = manager.allocate(4096).expect("Should allocate memory");

    // Test protection setting
    manager.protect(handle, Protection::ReadWrite).expect("Should set protection");

    // Test memory mapping
    let addr = manager.map(handle).expect("Should map memory");

    // Test permission checking
    assert!(manager.check_permission(&handle, Protection::ReadWrite).is_ok());

    // Test unmapping
    manager.unmap(addr).expect("Should unmap memory");

    // Test deallocation
    manager.deallocate(handle).expect("Should deallocate memory");
}

// Helper function for tests
pub fn create_test_manager<A: Architecture>() -> Result<MemoryManager<A>, MemoryError> {
    MemoryManager::new()
}
