use super::*;

#[test]
fn test_memory_manager_creation() {
    let result = MemoryManager::<Arch64>::new();
    assert!(result.is_ok(), "Memory manager creation should succeed");
}

#[test]
fn test_basic_memory_operations() {
    let mut manager = MemoryManager::<Arch64>::new()
        .expect("Memory manager creation failed");

    // Test basic allocation
    let handle = manager.allocate(4096)
        .expect("Should allocate memory");

    // Test protection setting
    manager.protect(handle, Protection::ReadWrite)
        .expect("Should set protection");

    // Test memory mapping
    let addr = manager.map(handle)
        .expect("Should map memory");

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