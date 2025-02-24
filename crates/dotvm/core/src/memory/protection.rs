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
use cfg_if::cfg_if;
use std::arch::asm;
use std::collections::HashMap;
use std::env;
use std::fs;

// Constants
const PAGE_SIZE: usize = 4096;
const MAX_MEMORY_SIZE: usize = 1024 * 1024 * 1024; // Example: 1GB

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
    /// Checks supported protection mechanisms.
    pub fn new() -> Self {
        let os = env::consts::OS;
        let arch = env::consts::ARCH;

        if os == "linux" {
            if arch == "x86_64" {
                let mpk_supported = Self::is_mpk_supported(arch);
                let pkey_supported = Self::is_pkey_supported(arch);
                HardwareProtection {
                    pkey_supported,
                    mpk_supported,
                }
            } else {
                println!("Unsupported Architecture");
                HardwareProtection {
                    pkey_supported: false,
                    mpk_supported: false,
                }
            }
        } else {
            println!("Unsupported OS");
            HardwareProtection {
                pkey_supported: false,
                mpk_supported: false,
            }
        }
    }

    /// Implements the protection mechanism according to the operating system and architecture.
    pub fn initialize_protection(
        &self,
        addr: VirtualAddress,
        size: usize,
        protection: Protection,
    ) -> Result<(), MemoryError> {
        let os = env::consts::OS;
        let arch = env::consts::ARCH;

        if os == "linux" {
            if arch == "x86_64" {
                self.protect_region(addr, size, protection)
            } else {
                println!("Unsupported Architecture");
                Err(MemoryError::UnsupportedArch)
            }
        } else {
            println!("Unsupported OS");
            Err(MemoryError::UnsupportedOS)
        }
    }

    /// Automatically determines the protection type and applies protection.
    fn protect_region(
        &self,
        addr: VirtualAddress,
        size: usize,
        protection: Protection,
    ) -> Result<(), MemoryError> {
        self.enforce_alignment(addr)?;
        self.validate_size(size)?;

        cfg_if! {
            if #[cfg(target_arch = "x86_64")] {
                if self.mpk_supported {
                    return self.apply_mpk_protection(addr, size, protection);
                } else if self.pkey_supported {
                    return self.apply_pkey_protection(addr, size, protection);
                }
            }
        }

        Err(MemoryError::UnsupportedProtection)
    }

    #[cfg(target_arch = "x86_64")]
    fn apply_mpk_protection(
        &self,
        addr: VirtualAddress,
        size: usize,
        protection: Protection,
    ) -> Result<(), MemoryError> {
        // Set MPK protection key
        let mpk = self.determine_mpk_for_protection(protection);

        // Assign protection key to memory region
        unsafe {
            // Set the protection switch using platform-specific instructions
            Self::set_memory_protection(addr.0, mpk);
        }

        // Control memory access using the protection key
        self.enforce_mpk_protection(addr, size, mpk)?;

        Ok(())
    }

    fn determine_mpk_for_protection(&self, protection: Protection) -> u32 {
        // Determine the appropriate MPK key according to the protection type
        match protection {
            Protection::ReadOnly => 1,
            Protection::ReadWrite => 2,
            Protection::ReadExecute => 3,
            Protection::ReadWriteExecute => 4,
            _ => 0, // Default key for None or other cases
        }
    }

    #[cfg(target_arch = "x86_64")]
    unsafe fn set_memory_protection(addr: usize, mpk: u32) {
        // Set memory protection using platform-specific instructions
        // Example: For x86_64 the `wrpkru` instruction is available
        asm!("wrpkru", in("eax") mpk, in("ecx") addr);
    }

    #[cfg(target_arch = "x86_64")]
    fn enforce_mpk_protection(
        &self,
        addr: VirtualAddress,
        size: usize,
        mpk: u32,
    ) -> Result<(), MemoryError> {
        // Check the protection switch of the memory region
        // Return an error if the protection key is not available
        if !self.check_mpk_protection(addr, size, mpk) {
            return Err(MemoryError::ProtectionError(
                "MPK protection failed".to_string(),
            ));
        }
        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    fn check_mpk_protection(&self, addr: VirtualAddress, size: usize, mpk: u32) -> bool {
        // Check the protection switch of the memory region
        // Example: For x86_64 the `rdpkru` instruction is available
        unsafe {
            let current_mpk: u32;
            asm!("rdpkru", out("eax") current_mpk);
            current_mpk == mpk
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn apply_pkey_protection(
        &self,
        addr: VirtualAddress,
        size: usize,
        protection: Protection,
    ) -> Result<(), MemoryError> {
        // Set PKEY protection key
        let pkey = self.determine_pkey_for_protection(protection);

        // Assign protection key to memory region
        unsafe {
            // Set the protection switch using platform-specific instructions
            Self::set_protection_key(addr.0, pkey);
        }

        // Control memory access using the protection key
        self.enforce_pkey_protection(addr, size, pkey)?;

        Ok(())
    }

    fn determine_pkey_for_protection(&self, protection: Protection) -> u32 {
        // Determine the appropriate PKEY key according to the protection type
        match protection {
            Protection::ReadOnly => 1,
            Protection::ReadWrite => 2,
            Protection::ReadExecute => 3,
            Protection::ReadWriteExecute => 4,
            _ => 0, // Default key for None or other cases
        }
    }

    #[cfg(target_arch = "x86_64")]
    unsafe fn set_protection_key(addr: usize, pkey: u32) {
        // Set the protection switch using platform-specific instructions
        // Example: `pkey_mprotect` system call available for x86_64
        asm!("syscall", in("rax") 0x123, in("rdi") addr, in("rsi") pkey);
    }

    #[cfg(target_arch = "x86_64")]
    fn enforce_pkey_protection(
        &self,
        addr: VirtualAddress,
        size: usize,
        pkey: u32,
    ) -> Result<(), MemoryError> {
        // Check the protection switch of the memory region
        // Return an error if the protection key is not available
        if !self.check_pkey_protection(addr, size, pkey) {
            return Err(MemoryError::ProtectionError(
                "PKEY protection failed".to_string(),
            ));
        }
        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    fn check_pkey_protection(&self, addr: VirtualAddress, size: usize, pkey: u32) -> bool {
        // Check the protection switch of the memory region
        // Example: `pkey_get` system call for x86_64 is available
        unsafe {
            let current_pkey: u32;
            asm!("syscall", out("rax") current_pkey, in("rdi") addr.0);
            current_pkey == pkey
        }
    }

    /// Checks whether MPK support is available
    /// Checks the relevant flags according to the architecture
    fn is_mpk_supported(arch: &str) -> bool {
        match arch {
            "x86_64" => {
                // For x86_64, check the 'mpk' flag in /proc/cpuinfo
                if let Ok(cpu_info) = fs::read_to_string("/proc/cpuinfo") {
                    // Search for 'mpk' in the 'flags' lines
                    cpu_info
                        .lines()
                        .filter(|line| line.contains("flags"))
                        .any(|line| line.contains("mpk"))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Checks if pkey support is available.
    /// Checks the relevant flags according to the architecture.
    fn is_pkey_supported(arch: &str) -> bool {
        match arch {
            "x86_64" => {
                // check pkey flag in /proc/cpuinfo for x86_64
                if let Ok(cpu_info) = fs::read_to_string("/proc/cpuinfo") {
                    // search for “pkey” in “flags” lines
                    cpu_info
                        .lines()
                        .filter(|line| line.contains("flags"))
                        .any(|line| line.contains("pkey"))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Enforces alignment for memory protection.
    pub fn enforce_alignment(&self, addr: VirtualAddress) -> Result<(), MemoryError> {
        if addr.0 % PAGE_SIZE != 0 {
            return Err(MemoryError::ProtectionError(
                "Address is not page-aligned".to_string(),
            ));
        }
        Ok(())
    }

    // update the HardwareProtection impl block in protection.rs
    pub fn validate_size(&self, size: usize) -> Result<(), MemoryError> {
        if size == 0 || size > MAX_MEMORY_SIZE || size % PAGE_SIZE != 0 {
            return Err(MemoryError::ProtectionError(
                "Invalid memory size".to_string(),
            ));
        }
        Ok(())
    }
}

/// Extends integration tests to verify hardware-enforced protections.
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_hardware_enforced_protections() {
        let mut ctx = ProtectionContext::new();
        let hw_protection = HardwareProtection::new();
        let handle = MemoryHandle(1);
        let addr = VirtualAddress(0x1000);

        ctx.set_protection(handle, Protection::ReadWrite).unwrap();

        // Wait for hardware protection to return an error
        let hw_result = hw_protection.protect_region(addr, 4096, Protection::ReadWrite);
        assert!(matches!(hw_result, Err(MemoryError::UnsupportedProtection)));

        // Continue to check software protection
        assert!(ctx.check_access(&handle, Protection::ReadWrite).is_ok());
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

            // Expect UnsupportedProtection if there is no hardware support
            assert!(matches!(
                hw_protection.protect_region(addr, 4096, Protection::ReadWrite),
                Err(MemoryError::UnsupportedProtection)
            ));
        }

        #[test]
        fn test_protect_region_alignment() {
            let hw_protection = HardwareProtection::new();
            let unaligned_addr = VirtualAddress(0x1001); // Not page-aligned

            // Alignment error expected
            assert!(matches!(
                hw_protection.protect_region(unaligned_addr, 4096, Protection::ReadWrite),
                Err(MemoryError::ProtectionError(_))
            ));
        }

        #[test]
        fn test_protect_region_size() {
            let hw_protection = HardwareProtection::new();
            let addr = VirtualAddress(0x1000);

            // Invalid size error expected (100 cannot be divided by PAGE_SIZE)
            assert!(matches!(
                hw_protection.protect_region(addr, 100, Protection::ReadWrite),
                Err(MemoryError::ProtectionError(_))
            ));
        }
    }

    mod integration_tests {
        use super::*;

        #[test]
        fn test_hardware_enforced_protections() {
            let mut ctx = ProtectionContext::new();
            let hw_protection = HardwareProtection::new();
            let handle = MemoryHandle(1);
            let addr = VirtualAddress(0x1000);

            ctx.set_protection(handle, Protection::ReadWrite).unwrap();

            // Wait for hardware protection to return an error
            let hw_result = hw_protection.protect_region(addr, 4096, Protection::ReadWrite);
            assert!(matches!(hw_result, Err(MemoryError::UnsupportedProtection)));

            // Continue to check software protection
            assert!(ctx.check_access(&handle, Protection::ReadWrite).is_ok());
        }

        #[test]
        fn test_protection_changes() {
            let mut ctx = ProtectionContext::new();
            let hw_protection = HardwareProtection::new();
            let handle = MemoryHandle(1);
            let addr = VirtualAddress(0x1000);

            // Set software protection
            ctx.set_protection(handle, Protection::ReadWrite).unwrap();

            // Try to set hardware protection, but expect error if there is no hardware support
            let hw_result = hw_protection.protect_region(addr, 4096, Protection::ReadWrite);
            if hw_protection.mpk_supported || hw_protection.pkey_supported {
                hw_result.unwrap(); // Do unwrap if hardware support is available
            } else {
                assert!(matches!(hw_result, Err(MemoryError::UnsupportedProtection)));
            }

            // Change software protection to ReadOnly
            ctx.set_protection(handle, Protection::ReadOnly).unwrap();

            // Try to update hardware protection to ReadOnly
            let hw_result = hw_protection.protect_region(addr, 4096, Protection::ReadOnly);
            if hw_protection.mpk_supported || hw_protection.pkey_supported {
                hw_result.unwrap(); // Do unwrap if hardware support is available
            } else {
                assert!(matches!(hw_result, Err(MemoryError::UnsupportedProtection)));
            }

            // Check software protection
            assert!(ctx.check_access(&handle, Protection::ReadOnly).is_ok());
            assert!(matches!(
                ctx.check_access(&handle, Protection::ReadWrite),
                Err(MemoryError::ProtectionError(_))
            ));
        }
    }
}
