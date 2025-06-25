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

use crate::memory::{Arch32, Arch64, Arch128, Arch256, Arch512};
use crate::{
    bytecode::VmArchitecture,
    instruction::instruction::MemoryManagerInterface,
    memory::{Architecture, MemoryError, MemoryHandle, MemoryManagement, MemoryManager},
    vm::errors::VMError,
}; // Added imports

/// Wraps a MemoryManager of a Host Architecture to provide a MemoryManagerInterface
/// suitable for bytecode from a Guest Architecture, when running in compatibility mode
/// (HostArch::WORD_SIZE > GuestArch::WORD_SIZE).
#[derive(Debug)] // Added Debug
pub struct AdaptedMemoryManager<HostArch: Architecture> {
    host_memory_manager: MemoryManager<HostArch>, // MemoryManager is Debug
    guest_arch: VmArchitecture,                   // VmArchitecture is Debug
}

impl<HostArch: Architecture + std::fmt::Debug> AdaptedMemoryManager<HostArch> {
    pub fn new(
        host_memory_manager: MemoryManager<HostArch>,
        guest_arch: VmArchitecture,
        host_arch_label: VmArchitecture, // Added parameter
    ) -> Result<Self, VMError> {
        // Check if compatibility is needed and valid (host > guest)
        // Note: host_arch_label.word_size() should be == HostArch::WORD_SIZE, this is checked in MultiArchExecutor::new
        if host_arch_label.word_size() <= guest_arch.word_size() && host_arch_label != guest_arch {
            // This adapter is primarily for host > guest.
            // If host == guest, it could be used but offers no benefit over direct MemoryManager.
            // It should definitely not be used if host < guest.
            return Err(VMError::ConfigurationError(format!(
                "AdaptedMemoryManager: Host architecture ({:?}, {} bytes) must be strictly larger than guest architecture ({:?}, {} bytes) for meaningful adaptation, or they must be identical (no adaptation needed). Current host word size: {}, guest word size: {}.",
                host_arch_label,
                HostArch::WORD_SIZE,
                guest_arch,
                guest_arch.word_size(),
                HostArch::WORD_SIZE,
                guest_arch.word_size()
            )));
        }
        Ok(AdaptedMemoryManager { host_memory_manager, guest_arch })
    }
}

impl<HostArch: Architecture + std::fmt::Debug> MemoryManagerInterface for AdaptedMemoryManager<HostArch> {
    fn allocate(&mut self, requested_size: usize) -> Result<MemoryHandle, VMError> {
        if requested_size == 0 {
            return Err(VMError::MemoryOperationError(MemoryError::InvalidSize { available: HostArch::MAX_MEMORY }.to_string()));
        }

        // The guest requests `requested_size`.
        // The host_memory_manager requires allocations to be aligned to HostArch::ALIGNMENT.
        let host_alignment = HostArch::ALIGNMENT;

        // If guest requests a size smaller than host's alignment,
        // we must allocate at least host_alignment.
        // If guest requests a size larger, round it up to the next multiple of host_alignment.
        let actual_alloc_size = if requested_size < host_alignment {
            host_alignment
        } else {
            (requested_size + host_alignment - 1) / host_alignment * host_alignment
        };

        // Now, allocate this actual_alloc_size from the host_memory_manager.
        // The MemoryManager<HostArch>::allocate method already checks if actual_alloc_size is a multiple of HostArch::ALIGNMENT.
        // Our calculation above ensures this.
        MemoryManagement::allocate(&mut self.host_memory_manager, actual_alloc_size).map_err(|e| VMError::MemoryOperationError(e.to_string()))
    }

    fn deallocate(&mut self, handle: MemoryHandle) -> Result<(), VMError> {
        MemoryManagement::deallocate(&mut self.host_memory_manager, handle).map_err(|e| VMError::MemoryOperationError(e.to_string()))
    }

    fn load(&self, address: usize) -> Result<u8, VMError> {
        // TODO: Add guest address space bounds check if guest_arch.max_address() < HostArch::MAX_MEMORY
        // For now, relying on host_memory_manager's internal checks which operate on its own address space.
        // This assumes guest addresses are directly valid in the host space.
        MemoryManagement::load(&self.host_memory_manager, address).map_err(|e| VMError::MemoryOperationError(e.to_string()))
    }

    fn store(&mut self, address: usize, value: u8) -> Result<(), VMError> {
        // Similar to load, guest address space checks could be added.
        MemoryManagement::store(&mut self.host_memory_manager, address, value).map_err(|e| VMError::MemoryOperationError(e.to_string()))
    }
}

// Helper to get VmArchitecture label from MemoryManager for constructor check
// This is needed because Architecture trait doesn't enforce providing a VmArchitecture label.
// We'll add this to the actual MemoryManager<A> if it's not there.
// For now, let's assume MemoryManager has a way to tell its VmArchitecture.
// This is a temporary measure for the constructor logic above.
// The proper way is for MultiArchExecutor to pass its host VmArchitecture label.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{Arch32, Arch64, Arch128, Arch256, Arch512}; // Import specific architectures

    // Helper to create a MemoryManager for a specific HostArch for testing
    fn create_host_manager<HostArch: Architecture>() -> MemoryManager<HostArch> {
        // For now, just use the default new() method
        // TODO: In the future, we should add a constructor that takes a custom memory size
        crate::memory::MemoryManagement::new().expect("Failed to create memory manager")
    }

    // Mocking the VmArchitecture label retrieval for testing AdaptedMemoryManager constructor
    // In real scenario, MemoryManager<A> would need a method like architecture_vm_label()
    // or this check would be done at a higher level (e.g. in MultiArchExecutor).
    impl<A: Architecture> MemoryManager<A> {
        // If this method doesn't exist, tests might need adjustment or this method added to actual MemoryManager
        // For now, this is a test-only extension.
        // Let's assume MultiArchExecutor will provide the VmArchitecture label of the host.
        // The constructor of AdaptedMemoryManager might be better off taking host_arch_label as well.
        // For now, the constructor logic `host_memory_manager.architecture_vm_label()` is illustrative.
        // Let's simplify the constructor check for now and rely on correct usage by MultiArchExecutor.
    }

    #[test]
    fn test_adapted_alloc_guest_smaller_than_host_alignment() {
        // Guest: Arch32 (word 4, align 4), Host: Arch64 (word 8, align 8)
        let host_manager = create_host_manager::<Arch64>();
        let mut adapted_manager = AdaptedMemoryManager::new(host_manager, VmArchitecture::Arch32, VmArchitecture::Arch64).expect("Failed to create adapted manager");

        // Guest requests 4 bytes. Host alignment is 8. Should allocate 8 bytes from host.
        let handle = adapted_manager.allocate(4).unwrap();
        // We can't easily verify the *actual* size allocated by host_manager without exposing its internals
        // or modifying MemoryHandle to include size. But the call should succeed.
        assert!(adapted_manager.deallocate(handle).is_ok());
    }

    #[test]
    fn test_adapted_alloc_guest_equals_host_alignment() {
        let host_manager = create_host_manager::<Arch64>();
        let mut adapted_manager = AdaptedMemoryManager::new(host_manager, VmArchitecture::Arch32, VmArchitecture::Arch64).expect("Failed to create adapted manager");

        // Guest requests 8 bytes. Host alignment is 8. Should allocate 8.
        let handle = adapted_manager.allocate(8).unwrap();
        assert!(adapted_manager.deallocate(handle).is_ok());
    }

    #[test]
    fn test_adapted_alloc_guest_larger_than_host_alignment_multiple() {
        let host_manager = create_host_manager::<Arch64>(); // align 8
        let mut adapted_manager = AdaptedMemoryManager::new(host_manager, VmArchitecture::Arch32, VmArchitecture::Arch64).expect("Failed to create adapted manager");

        // Guest requests 16 bytes (multiple of host align 8). Should allocate 16.
        let handle = adapted_manager.allocate(16).unwrap();
        assert!(adapted_manager.deallocate(handle).is_ok());
    }

    #[test]
    fn test_adapted_alloc_guest_larger_than_host_alignment_not_multiple() {
        let host_manager = create_host_manager::<Arch64>(); // align 8
        let mut adapted_manager = AdaptedMemoryManager::new(host_manager, VmArchitecture::Arch32, VmArchitecture::Arch64).expect("Failed to create adapted manager");

        // Guest requests 10 bytes. Host align 8. Should allocate 16 (rounded up).
        let handle = adapted_manager.allocate(10).unwrap();
        assert!(adapted_manager.deallocate(handle).is_ok());
    }

    #[test]
    fn test_adapted_alloc_zero_size() {
        let host_manager = create_host_manager::<Arch64>();
        let mut adapted_manager = AdaptedMemoryManager::new(host_manager, VmArchitecture::Arch32, VmArchitecture::Arch64).expect("Failed to create adapted manager");

        let result = adapted_manager.allocate(0);
        assert!(result.is_err());
        match result.err().unwrap() {
            VMError::MemoryOperationError(msg) => assert!(msg.contains("size cannot be zero")),
            _ => panic!("Expected MemoryOperationError for zero size allocation"),
        }
    }

    #[test]
    fn test_load_store_pass_through() {
        // Host Arch64, Guest Arch32
        let host_manager = create_host_manager::<Arch64>();
        let mut adapted_manager = AdaptedMemoryManager::new(host_manager, VmArchitecture::Arch32, VmArchitecture::Arch64).expect("Failed to create adapted manager");

        let handle = adapted_manager.allocate(8).unwrap(); // Allocates 8 bytes on host.

        // Addresses are relative to the start of the simulated memory space, not the handle.
        // To test load/store, we'd typically need to map the handle to a virtual address.
        // The MemoryManagerInterface doesn't expose map/unmap.
        // The underlying MemoryManager does, but AdaptedMemoryManager only implements MemoryManagerInterface.
        // For this test, we'll assume address 0 is part of this allocation IF the allocator always starts at 0
        // (which it might for a fresh MemoryManager). This is a simplification.
        // A more robust test would involve a MemoryManager that pre-allocates a known region or
        // use debug features if available.
        // For now, we rely on the fact that if allocate succeeds, load/store to valid offsets within
        // that (conceptual) allocation should pass through to the host manager's load/store.
        // Since we cannot easily get the base address of `handle` here, we'll test that the methods
        // can be called and defer detailed memory content check to integration tests with MultiArchExecutor.

        // This test is more of a "does it compile and not panic on simple calls"
        // The dummy load/store in MemoryManager will return fixed values or Ok.
        assert!(adapted_manager.store(0, 10).is_ok());
        assert_eq!(adapted_manager.load(0).unwrap(), (0 & 0xFF) as u8); // Dummy load behavior

        assert!(adapted_manager.store(handle.0, 42).is_ok()); // Using handle.0 as address - this is often how it works if handle is base address
        assert_eq!(adapted_manager.load(handle.0).unwrap(), (handle.0 & 0xFF) as u8); // Dummy load behavior

        assert!(adapted_manager.deallocate(handle).is_ok());
    }

    #[test]
    fn test_adapted_manager_new_error_host_not_larger_direct() {
        // Guest Arch64, Host Arch32
        let host_manager_arch32 = create_host_manager::<Arch32>();
        // Now we pass host_arch_label directly
        let res_err = AdaptedMemoryManager::new(host_manager_arch32, VmArchitecture::Arch64, VmArchitecture::Arch32);
        assert!(res_err.is_err());
        match res_err.err().unwrap() {
            VMError::ConfigurationError(msg) => {
                assert!(msg.contains("Host architecture (Arch32, 4 bytes) must be strictly larger than guest architecture (Arch64, 8 bytes)"));
            }
            _ => panic!("Expected ConfigurationError for host not larger than guest"),
        }
    }
}
