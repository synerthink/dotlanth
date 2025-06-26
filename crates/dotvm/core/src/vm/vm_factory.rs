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

use crate::bytecode::VmArchitecture;
use crate::vm::errors::VMError;
// use crate::memory::Architecture; // Unused: VmInstance doesn't need to expose HostArchitecture generic type itself

use std::any::Any; // Import Any

/// A placeholder trait representing a generic VM instance.
/// Specific VM implementations will implement this.
pub trait VmInstance: Any {
    // Add Any as supertrait
    /// Returns the architecture of this VM instance.
    fn architecture(&self) -> VmArchitecture;

    /// Executes the loaded bytecode.
    /// For now, this is a placeholder. It will eventually take bytecode and other params.
    fn run(&mut self) -> Result<(), VMError>;

    /// Resets the VM to its initial state, making it ready for reuse (especially for pooling).
    fn reset(&mut self) -> Result<(), VMError>;

    // More methods will be added here, e.g., load_bytecode, get_state, etc.
}

// Specific VM implementations are now MultiArchExecutor<A>.
// We do need MultiArchExecutor and the specific Arch types for the factory impl.
use crate::memory::{Arch32, Arch64, Arch128, Arch256, Arch512};
use crate::vm::architecture_detector::DetectedArch;
use crate::vm::multi_arch_executor::MultiArchExecutor; // Import DetectedArch

/// Defines the interface for a VM factory.
/// The factory is responsible for creating appropriate VM instances based on architecture requirements.
pub trait VMFactory {
    /// Creates a VM instance based on detected architecture information.
    ///
    /// # Arguments
    /// * `detected_arch`: The `DetectedArch` struct containing required and execution architectures.
    ///
    /// # Returns
    /// A `Result` containing a boxed `VmInstance` or a `VMError` if creation fails.
    fn create_vm_from_detected(&self, detected_arch: DetectedArch) -> Result<Box<dyn VmInstance>, VMError>;
}

/// A simple concrete implementation of the VMFactory trait.
/// It creates MultiArchExecutor instances based on the specified architecture.
pub struct SimpleVMFactory;

impl SimpleVMFactory {
    pub fn new() -> Self {
        SimpleVMFactory
    }
}

impl VMFactory for SimpleVMFactory {
    fn create_vm_from_detected(&self, detected_arch: DetectedArch) -> Result<Box<dyn VmInstance>, VMError> {
        let host_arch_label = detected_arch.execution;
        let guest_arch_label = detected_arch.required;

        match host_arch_label {
            VmArchitecture::Arch32 => {
                let vm = MultiArchExecutor::<Arch32>::new(host_arch_label, guest_arch_label)?;
                Ok(Box::new(vm))
            }
            VmArchitecture::Arch64 => {
                let vm = MultiArchExecutor::<Arch64>::new(host_arch_label, guest_arch_label)?;
                Ok(Box::new(vm))
            }
            VmArchitecture::Arch128 => {
                let vm = MultiArchExecutor::<Arch128>::new(host_arch_label, guest_arch_label)?;
                Ok(Box::new(vm))
            }
            VmArchitecture::Arch256 => {
                let vm = MultiArchExecutor::<Arch256>::new(host_arch_label, guest_arch_label)?;
                Ok(Box::new(vm))
            }
            VmArchitecture::Arch512 => {
                let vm = MultiArchExecutor::<Arch512>::new(host_arch_label, guest_arch_label)?;
                Ok(Box::new(vm))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::instruction::{ExecutorInterface, Instruction};
    // use std::sync::Arc; // Arc is used via MultiArchExecutor tests, not directly here for FactoryTestPushInstruction if not loaded
    // use crate::vm::architecture_detector::ArchitectureDetector; // Unused in these specific tests

    // Minimal test instruction
    #[derive(Debug, Clone)]
    struct FactoryTestPushInstruction {
        value: f64,
    }
    impl Instruction for FactoryTestPushInstruction {
        fn execute(&self, executor: &mut dyn crate::instruction::instruction::ExecutorInterface) -> Result<(), VMError> {
            executor.push_operand(self.value);
            Ok(())
        }
    }

    // Helper for creating DetectedArch for tests
    fn detected(required: VmArchitecture, execution: VmArchitecture) -> DetectedArch {
        DetectedArch {
            required,
            execution,
            compatibility_mode: required != execution,
        }
    }

    #[test]
    fn test_factory_creates_mae_arch32_native() {
        let factory = SimpleVMFactory::new();
        let da = detected(VmArchitecture::Arch32, VmArchitecture::Arch32);
        let vm_res = factory.create_vm_from_detected(da);
        assert!(vm_res.is_ok());
        let mut vm = vm_res.unwrap();
        assert_eq!(vm.architecture(), VmArchitecture::Arch32); // VmInstance::architecture reports host

        let exec_iface = (vm.as_mut() as &mut dyn Any).downcast_mut::<MultiArchExecutor<Arch32>>().expect("Should be MAE Arch32");
        assert_eq!(exec_iface.get_guest_architecture(), VmArchitecture::Arch32);
        assert!(!exec_iface.is_compatibility_mode());
        drop(exec_iface);

        assert!(vm.run().is_ok());
        assert!(vm.reset().is_ok());
    }

    #[test]
    fn test_factory_creates_mae_arch64_for_arch32_guest() {
        // Compatibility mode
        let factory = SimpleVMFactory::new();
        let da = detected(VmArchitecture::Arch32, VmArchitecture::Arch64); // Guest Arch32, Host Arch64
        let vm_res = factory.create_vm_from_detected(da);
        assert!(vm_res.is_ok());
        let mut vm = vm_res.unwrap();
        assert_eq!(vm.architecture(), VmArchitecture::Arch64); // Host arch

        let exec_iface = (vm.as_mut() as &mut dyn Any).downcast_mut::<MultiArchExecutor<Arch64>>().expect("Should be MAE Arch64");
        assert_eq!(exec_iface.get_guest_architecture(), VmArchitecture::Arch32); // Guest arch
        assert!(exec_iface.is_compatibility_mode());
        drop(exec_iface);

        assert!(vm.run().is_ok());
        assert!(vm.reset().is_ok());
    }

    #[test]
    fn test_factory_creates_mae_arch256_for_arch64_guest() {
        // Compatibility mode
        let factory = SimpleVMFactory::new();
        let da = detected(VmArchitecture::Arch64, VmArchitecture::Arch256); // Guest Arch64, Host Arch256
        let vm_res = factory.create_vm_from_detected(da);
        assert!(vm_res.is_ok());
        let mut vm = vm_res.unwrap();
        assert_eq!(vm.architecture(), VmArchitecture::Arch256);

        let exec_iface = (vm.as_mut() as &mut dyn Any).downcast_mut::<MultiArchExecutor<Arch256>>().expect("Should be MAE Arch256");
        assert_eq!(exec_iface.get_guest_architecture(), VmArchitecture::Arch64);
        assert!(exec_iface.is_compatibility_mode());
        drop(exec_iface);

        assert!(vm.run().is_ok());
        assert!(vm.reset().is_ok());
    }

    #[test]
    fn test_factory_with_all_host_architectures_native() {
        let factory = SimpleVMFactory::new();
        let architectures = [
            VmArchitecture::Arch32,
            VmArchitecture::Arch64,
            VmArchitecture::Arch128,
            VmArchitecture::Arch256,
            VmArchitecture::Arch512,
        ];

        for arch in architectures.iter() {
            let da = detected(*arch, *arch);
            let vm_res = factory.create_vm_from_detected(da);
            assert!(vm_res.is_ok(), "Failed for arch {:?}", arch);
            let mut vm = vm_res.unwrap();
            assert_eq!(vm.architecture(), *arch, "Host arch mismatch for {:?}", arch);

            // Downcasting to check guest_arch and compat_mode
            match arch {
                VmArchitecture::Arch32 => {
                    let ei = (vm.as_mut() as &mut dyn Any).downcast_mut::<MultiArchExecutor<Arch32>>().unwrap();
                    assert_eq!(ei.get_guest_architecture(), *arch);
                    assert!(!ei.is_compatibility_mode());
                }
                VmArchitecture::Arch64 => {
                    let ei = (vm.as_mut() as &mut dyn Any).downcast_mut::<MultiArchExecutor<Arch64>>().unwrap();
                    assert_eq!(ei.get_guest_architecture(), *arch);
                    assert!(!ei.is_compatibility_mode());
                }
                VmArchitecture::Arch128 => {
                    let ei = (vm.as_mut() as &mut dyn Any).downcast_mut::<MultiArchExecutor<Arch128>>().unwrap();
                    assert_eq!(ei.get_guest_architecture(), *arch);
                    assert!(!ei.is_compatibility_mode());
                }
                VmArchitecture::Arch256 => {
                    let ei = (vm.as_mut() as &mut dyn Any).downcast_mut::<MultiArchExecutor<Arch256>>().unwrap();
                    assert_eq!(ei.get_guest_architecture(), *arch);
                    assert!(!ei.is_compatibility_mode());
                }
                VmArchitecture::Arch512 => {
                    let ei = (vm.as_mut() as &mut dyn Any).downcast_mut::<MultiArchExecutor<Arch512>>().unwrap();
                    assert_eq!(ei.get_guest_architecture(), *arch);
                    assert!(!ei.is_compatibility_mode());
                }
            }
            assert!(vm.run().is_ok(), "Run failed for {:?}", arch);
            assert!(vm.reset().is_ok(), "Reset failed for {:?}", arch);
        }
    }
}
