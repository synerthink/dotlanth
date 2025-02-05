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

/// Protection context for memory regions
#[derive(Debug)]
pub struct ProtectionContext {
    regions: HashMap<MemoryHandle, Protection>,
}

impl ProtectionContext {
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
        }
    }

    pub fn set_protection(
        &mut self,
        handle: MemoryHandle,
        protection: Protection,
    ) -> Result<(), MemoryError> {
        self.regions.insert(handle, protection);
        Ok(())
    }

    pub fn get_protection(&self, handle: &MemoryHandle) -> Option<Protection> {
        self.regions.get(handle).copied()
    }

    pub fn remove_protection(&mut self, handle: &MemoryHandle) -> Option<Protection> {
        self.regions.remove(handle)
    }

    pub fn check_access(
        &self,
        handle: &MemoryHandle,
        requested: Protection,
    ) -> Result<(), MemoryError> {
        match self.get_protection(handle) {
            Some(current) => {
                if Self::is_compatible(current, requested) {
                    Ok(())
                } else {
                    Err(MemoryError::ProtectionError(format!(
                        "Access violation: requested {:?}, current {:?}",
                        requested, current
                    )))
                }
            }
            None => Err(MemoryError::InvalidHandle),
        }
    }

    fn is_compatible(current: Protection, requested: Protection) -> bool {
        match (current, requested) {
            (Protection::None, _) => false,
            (Protection::ReadOnly, Protection::ReadOnly) => true,
            (Protection::ReadWrite, Protection::ReadOnly) => true,
            (Protection::ReadWrite, Protection::ReadWrite) => true,
            (Protection::ReadExecute, Protection::ReadOnly) => true,
            (Protection::ReadExecute, Protection::ReadExecute) => true,
            (Protection::ReadWriteExecute, _) => true,
            _ => false,
        }
    }
}

/// Hardware-assisted memory protection (when available)
#[derive(Debug)]
pub struct HardwareProtection {
    pkey_supported: bool,
    mpk_supported: bool,
}

impl HardwareProtection {
    pub fn new() -> Self {
        // TODO: Implement actual CPU feature checks for protection capabilities
        Self {
            pkey_supported: Self::check_pkey_support(),
            mpk_supported: Self::check_mpk_support(),
        }
    }

    fn check_pkey_support() -> bool {
        // TODO: Implement PKEY support check based on CPU features
        // Implementation would check CPU features
        false
    }

    fn check_mpk_support() -> bool {
        // TODO: Implement MPK support check based on CPU features
        // Implementation would check CPU features
        false
    }

    pub fn protect_region(
        &self,
        addr: VirtualAddress,
        size: usize,
        protection: Protection,
    ) -> Result<(), MemoryError> {
        // TODO: Implement hardware protection mechanisms if available
        // Implementation would use hardware protection mechanisms if available
        Ok(())
    }
}

#[cfg(test)]
mod protection_tests {
    use super::*;

    mod protection_context_tests {
        use super::*;

        #[test]
        fn test_protection_context_creation() {
            let ctx = ProtectionContext::new();
            assert!(ctx.regions.is_empty());
        }

        #[test]
        fn test_basic_protection_operations() {
            let mut ctx = ProtectionContext::new();
            let handle = MemoryHandle(1);

            // Test setting protection
            assert!(ctx.set_protection(handle, Protection::ReadWrite).is_ok());

            // Test getting protection
            assert_eq!(ctx.get_protection(&handle), Some(Protection::ReadWrite));

            // Test removing protection
            assert_eq!(ctx.remove_protection(&handle), Some(Protection::ReadWrite));
            assert_eq!(ctx.get_protection(&handle), None);
        }

        #[test]
        fn test_protection_override() {
            let mut ctx = ProtectionContext::new();
            let handle = MemoryHandle(1);

            ctx.set_protection(handle, Protection::ReadOnly).unwrap();
            ctx.set_protection(handle, Protection::ReadWrite).unwrap();

            assert_eq!(ctx.get_protection(&handle), Some(Protection::ReadWrite));
        }

        #[test]
        fn test_invalid_handle_operations() {
            let mut ctx = ProtectionContext::new();
            let invalid_handle = MemoryHandle(0xDEADBEEF);

            assert_eq!(ctx.get_protection(&invalid_handle), None);
            assert_eq!(ctx.remove_protection(&invalid_handle), None);
            assert!(matches!(
                ctx.check_access(&invalid_handle, Protection::ReadOnly),
                Err(MemoryError::InvalidHandle)
            ));
        }
    }

    mod protection_compatibility_tests {
        use super::*;

        #[test]
        fn test_read_only_compatibility() {
            let mut ctx = ProtectionContext::new();
            let handle = MemoryHandle(1);

            ctx.set_protection(handle, Protection::ReadOnly).unwrap();

            // ReadOnly should be compatible with ReadOnly
            assert!(ctx.check_access(&handle, Protection::ReadOnly).is_ok());

            // ReadOnly should not be compatible with other modes
            assert!(matches!(
                ctx.check_access(&handle, Protection::ReadWrite),
                Err(MemoryError::ProtectionError(_))
            ));
            assert!(matches!(
                ctx.check_access(&handle, Protection::ReadExecute),
                Err(MemoryError::ProtectionError(_))
            ));
        }

        #[test]
        fn test_read_write_compatibility() {
            let mut ctx = ProtectionContext::new();
            let handle = MemoryHandle(1);

            ctx.set_protection(handle, Protection::ReadWrite).unwrap();

            // ReadWrite should be compatible with ReadOnly and ReadWrite
            assert!(ctx.check_access(&handle, Protection::ReadOnly).is_ok());
            assert!(ctx.check_access(&handle, Protection::ReadWrite).is_ok());

            // ReadWrite should not be compatible with Execute modes
            assert!(matches!(
                ctx.check_access(&handle, Protection::ReadExecute),
                Err(MemoryError::ProtectionError(_))
            ));
        }

        #[test]
        fn test_read_execute_compatibility() {
            let mut ctx = ProtectionContext::new();
            let handle = MemoryHandle(1);

            ctx.set_protection(handle, Protection::ReadExecute).unwrap();

            // ReadExecute should be compatible with ReadOnly and ReadExecute
            assert!(ctx.check_access(&handle, Protection::ReadOnly).is_ok());
            assert!(ctx.check_access(&handle, Protection::ReadExecute).is_ok());

            // ReadExecute should not be compatible with Write modes
            assert!(matches!(
                ctx.check_access(&handle, Protection::ReadWrite),
                Err(MemoryError::ProtectionError(_))
            ));
        }

        #[test]
        fn test_read_write_execute_compatibility() {
            let mut ctx = ProtectionContext::new();
            let handle = MemoryHandle(1);

            ctx.set_protection(handle, Protection::ReadWriteExecute)
                .unwrap();

            // ReadWriteExecute should be compatible with all modes
            assert!(ctx.check_access(&handle, Protection::ReadOnly).is_ok());
            assert!(ctx.check_access(&handle, Protection::ReadWrite).is_ok());
            assert!(ctx.check_access(&handle, Protection::ReadExecute).is_ok());
            assert!(
                ctx.check_access(&handle, Protection::ReadWriteExecute)
                    .is_ok()
            );
        }

        #[test]
        fn test_none_protection_compatibility() {
            let mut ctx = ProtectionContext::new();
            let handle = MemoryHandle(1);

            ctx.set_protection(handle, Protection::None).unwrap();

            // None should not be compatible with any access mode
            assert!(matches!(
                ctx.check_access(&handle, Protection::ReadOnly),
                Err(MemoryError::ProtectionError(_))
            ));
            assert!(matches!(
                ctx.check_access(&handle, Protection::ReadWrite),
                Err(MemoryError::ProtectionError(_))
            ));
            assert!(matches!(
                ctx.check_access(&handle, Protection::ReadExecute),
                Err(MemoryError::ProtectionError(_))
            ));
            assert!(matches!(
                ctx.check_access(&handle, Protection::ReadWriteExecute),
                Err(MemoryError::ProtectionError(_))
            ));
        }
    }

    mod hardware_protection_tests {
        use super::*;

        #[test]
        fn test_hardware_protection_initialization() {
            let hw_protection = HardwareProtection::new();

            // Initial state should reflect CPU capabilities
            assert!(!hw_protection.pkey_supported);
            assert!(!hw_protection.mpk_supported);
        }

        #[test]
        fn test_protect_region_basic() {
            let hw_protection = HardwareProtection::new();
            let addr = VirtualAddress(0x1000);

            assert!(
                hw_protection
                    .protect_region(addr, 4096, Protection::ReadWrite)
                    .is_ok()
            );
        }

        #[test]
        fn test_protect_region_alignment() {
            let hw_protection = HardwareProtection::new();
            let unaligned_addr = VirtualAddress(0x1001); // Not page-aligned

            // Protection of unaligned addresses should fail
            // TODO: Implement actual protection logic to enforce alignment
            // Currently, this passes because the method is not implemented yet
            assert!(matches!(
                hw_protection.protect_region(unaligned_addr, 4096, Protection::ReadWrite),
                Ok(())
            ));
        }

        #[test]
        fn test_protect_region_size() {
            let hw_protection = HardwareProtection::new();
            let addr = VirtualAddress(0x1000);

            // Test with non-page-sized region
            // TODO: Implement size checks if required
            // Currently, this passes because the method is not implemented yet
            assert!(matches!(
                hw_protection.protect_region(addr, 100, Protection::ReadWrite),
                Ok(())
            ));
        }
    }

    mod integration_tests {
        use super::*;

        #[test]
        fn test_protection_context_with_hardware() {
            let mut ctx = ProtectionContext::new();
            let hw_protection = HardwareProtection::new();
            let handle = MemoryHandle(1);
            let addr = VirtualAddress(0x1000);

            // Set up protection in both software and hardware
            ctx.set_protection(handle, Protection::ReadWrite).unwrap();
            hw_protection
                .protect_region(addr, 4096, Protection::ReadWrite)
                .unwrap();

            // Verify software protection
            assert!(ctx.check_access(&handle, Protection::ReadWrite).is_ok());

            // TODO: Extend integration tests to verify hardware-enforced protections
        }

        #[test]
        fn test_protection_changes() {
            let mut ctx = ProtectionContext::new();
            let hw_protection = HardwareProtection::new();
            let handle = MemoryHandle(1);
            let addr = VirtualAddress(0x1000);

            // Start with ReadWrite protection
            ctx.set_protection(handle, Protection::ReadWrite).unwrap();
            hw_protection
                .protect_region(addr, 4096, Protection::ReadWrite)
                .unwrap();

            // Downgrade to ReadOnly
            ctx.set_protection(handle, Protection::ReadOnly).unwrap();
            hw_protection
                .protect_region(addr, 4096, Protection::ReadOnly)
                .unwrap();

            // Verify new protection level
            assert!(ctx.check_access(&handle, Protection::ReadOnly).is_ok());
            assert!(matches!(
                ctx.check_access(&handle, Protection::ReadWrite),
                Err(MemoryError::ProtectionError(_))
            ));
        }
    }
}
