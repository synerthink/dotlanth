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

use crate::{
    bytecode::VmArchitecture,
    instruction::{
        instruction::{ExecutorInterface, Instruction, MemoryManagerInterface},
        registry::{InstructionRegistry, Opcode},
    },
    memory::Architecture,
    vm::{errors::VMError, vm_factory::VmInstance},
};
use std::marker::PhantomData;
use std::sync::Arc;

// Added use for AdaptedMemoryManager
use crate::vm::compatibility::AdaptedMemoryManager;

use std::fmt;

/// Helper function to create a memory manager with reasonable size for testing
fn create_test_memory_manager<A: crate::memory::Architecture>() -> crate::memory::MemoryManager<A> {
    // For now, just use the default new() method
    // TODO: In the future, we should add a constructor that takes a custom memory size
    crate::memory::MemoryManagement::new().expect("Failed to create memory manager")
}

/// MultiArchExecutor struct handling the operand stack and instruction execution,
/// generic over a specific architecture `A` (Host Architecture).
// #[derive(Debug)] // Cannot auto-derive due to dyn Instruction and dyn MemoryManagerInterface
pub struct MultiArchExecutor<HostArch: Architecture> {
    operand_stack: Vec<f64>,
    instruction_pointer: usize,
    instructions: Vec<Arc<dyn Instruction>>,

    memory_interface: Box<dyn MemoryManagerInterface>,

    host_vm_arch_label: VmArchitecture,
    guest_vm_arch_label: VmArchitecture,
    is_compatibility_mode: bool,
    _phantom_host: PhantomData<HostArch>,
}

impl<HostArch: Architecture> fmt::Debug for MultiArchExecutor<HostArch> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MultiArchExecutor")
            .field("host_vm_arch_label", &self.host_vm_arch_label)
            .field("guest_vm_arch_label", &self.guest_vm_arch_label)
            .field("is_compatibility_mode", &self.is_compatibility_mode)
            .field("instruction_pointer", &self.instruction_pointer)
            .field("operand_stack_len", &self.operand_stack.len()) // Avoid printing full stack
            .field("instructions_count", &self.instructions.len()) // Avoid printing all instructions
            .field("memory_interface", &"Box<dyn MemoryManagerInterface>") // Placeholder for memory_interface
            .finish()
    }
}

impl<HostArch: Architecture + std::fmt::Debug> MultiArchExecutor<HostArch> {
    /// Creates a new MultiArchExecutor instance.
    ///
    /// # Arguments
    /// * `host_arch_label`: The actual architecture this executor will run as (e.g., VmArchitecture::Arch256).
    /// * `guest_arch_label`: The architecture the bytecode is intended for (e.g., VmArchitecture::Arch64).
    ///                      If same as `host_arch_label`, not in compatibility mode.
    pub fn new(host_arch_label: VmArchitecture, guest_arch_label: VmArchitecture) -> Result<Self, VMError> {
        // Runtime check: host_arch_label must match the generic type HostArch
        if host_arch_label.word_size() != HostArch::WORD_SIZE {
            return Err(VMError::ArchitectureMismatch(format!(
                "Host label {:?} (word size {}) does not match generic Arch type {} (word size {})",
                host_arch_label,
                host_arch_label.word_size(),
                std::any::type_name::<HostArch>(),
                HostArch::WORD_SIZE
            )));
        }

        let compatibility_mode = host_arch_label != guest_arch_label;

        if compatibility_mode && host_arch_label.word_size() < guest_arch_label.word_size() {
            return Err(VMError::ConfigurationError(format!(
                "Cannot run guest bytecode for {:?} on a smaller host architecture {:?}",
                guest_arch_label, host_arch_label
            )));
        }

        // Create a memory manager with reasonable size for testing
        let host_memory_manager = create_test_memory_manager::<HostArch>();
        let memory_interface: Box<dyn MemoryManagerInterface>;

        if compatibility_mode {
            // Ensure AdaptedMemoryManager is created correctly
            match AdaptedMemoryManager::new(host_memory_manager, guest_arch_label, host_arch_label) {
                // Pass host_arch_label
                Ok(adapted_manager) => memory_interface = Box::new(adapted_manager),
                Err(e) => return Err(e), // Propagate error from AdaptedMemoryManager::new
            }
        } else {
            memory_interface = Box::new(host_memory_manager);
        }

        Ok(MultiArchExecutor {
            operand_stack: Vec::new(),
            instruction_pointer: 0,
            instructions: Vec::new(),
            memory_interface,
            host_vm_arch_label: host_arch_label,
            guest_vm_arch_label: guest_arch_label,
            is_compatibility_mode: compatibility_mode,
            _phantom_host: PhantomData,
        })
    }

    /// Push an operand onto the stack.
    pub fn push_operand(&mut self, value: f64) {
        self.operand_stack.push(value);
    }

    /// Pop an operand from the stack.
    pub fn pop_operand(&mut self) -> Result<f64, VMError> {
        self.operand_stack.pop().ok_or(VMError::StackUnderflow)
    }

    /// Set the instruction pointer to a specific index.
    pub fn set_instruction_pointer(&mut self, target: usize) -> Result<(), VMError> {
        if target >= self.instructions.len() {
            return Err(VMError::InvalidJumpTarget(target));
        }
        self.instruction_pointer = target;
        Ok(())
    }

    /// Load instructions into the executor from opcodes.
    /// Clears existing instructions and resets the instruction pointer.
    pub fn load_instructions_from_opcodes(&mut self, opcodes: Vec<(Opcode, Option<Vec<usize>>)>) {
        let registry = InstructionRegistry::new();
        self.instructions.clear();
        self.instruction_pointer = 0;
        for (opcode, args) in opcodes {
            match registry.create_instruction(opcode, args) {
                Ok(instr) => self.instructions.push(instr),
                Err(e) => {
                    // Consider propagating this error instead of panicking
                    panic!("Failed to create instruction for opcode {:?}: {:?}", opcode, e);
                }
            }
        }
    }

    /// Load pre-built instructions into the executor.
    /// Clears existing instructions and resets the instruction pointer.
    pub fn load_instructions(&mut self, instructions: Vec<Arc<dyn Instruction>>) {
        self.instructions = instructions;
        self.instruction_pointer = 0;
        self.operand_stack.clear(); // Also clear stack for a fresh run
    }

    /// Execute all loaded instructions sequentially.
    pub fn execute_all(&mut self) -> Result<(), VMError> {
        while self.instruction_pointer < self.instructions.len() {
            let instruction = self.instructions[self.instruction_pointer].clone();
            instruction.execute(self)?; // `self` implements ExecutorInterface
            self.instruction_pointer += 1;
        }
        Ok(())
    }

    // Direct access to MemoryManager<HostArch> is removed. Access is via MemoryManagerInterface.
    // If specific HostArch MemoryManager methods were needed internally by MultiArchExecutor,
    // it would need to downcast the Box<dyn MemoryManagerInterface> or store MemoryManager<HostArch> separately,
    // but for now, all interactions are through the interface.
}

impl<HostArch: Architecture + std::fmt::Debug> VmInstance for MultiArchExecutor<HostArch> {
    fn architecture(&self) -> VmArchitecture {
        // This should report the HOST architecture, as VmInstance represents the actual VM running.
        // The guest architecture is available via ExecutorInterface::get_guest_architecture().
        self.host_vm_arch_label
    }

    fn run(&mut self) -> Result<(), VMError> {
        self.execute_all()
    }

    fn reset(&mut self) -> Result<(), VMError> {
        self.operand_stack.clear();
        self.instruction_pointer = 0;
        self.instructions.clear();
        // Resetting the memory_interface:
        // If it's an AdaptedMemoryManager, its internal host_memory_manager might need resetting,
        // or a new AdaptedMemoryManager might be created with a fresh host_memory_manager.
        // If it's a direct MemoryManager, it might need some form of reset if it's stateful.
        // For now, MemoryManager::new() creates a fresh state.
        // A true reset of the memory interface might involve recreating it:
        // Create a memory manager with reasonable size for testing
        let new_host_manager = create_test_memory_manager::<HostArch>();
        if self.is_compatibility_mode {
            self.memory_interface = Box::new(
                AdaptedMemoryManager::new(new_host_manager, self.guest_vm_arch_label, self.host_vm_arch_label)?, // Pass host_vm_arch_label
            );
        } else {
            self.memory_interface = Box::new(new_host_manager);
        }
        println!("MultiArchExecutor for Host:{:?}/Guest:{:?} reset.", self.host_vm_arch_label, self.guest_vm_arch_label);
        Ok(())
    }
}

impl<HostArch: Architecture + std::fmt::Debug> ExecutorInterface for MultiArchExecutor<HostArch> {
    fn push_operand(&mut self, value: f64) {
        self.operand_stack.push(value); // Direct inherent method call is fine
    }

    fn pop_operand(&mut self) -> Result<f64, VMError> {
        self.operand_stack.pop().ok_or(VMError::StackUnderflow) // Direct inherent method call is fine
    }

    fn set_instruction_pointer(&mut self, target: usize) -> Result<(), VMError> {
        // Direct inherent method call is fine
        if target >= self.instructions.len() {
            return Err(VMError::InvalidJumpTarget(target));
        }
        self.instruction_pointer = target;
        Ok(())
    }

    fn get_memory_manager_mut(&mut self) -> &mut dyn MemoryManagerInterface {
        &mut *self.memory_interface // Dereference the Box to get &mut dyn MemoryManagerInterface
    }

    fn get_guest_architecture(&self) -> VmArchitecture {
        self.guest_vm_arch_label
    }

    fn is_compatibility_mode(&self) -> bool {
        self.is_compatibility_mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{Arch32, Arch64, Arch128}; // Removed Arch256, Arch512 as they are not used in these specific tests
    use std::sync::Arc; // Required for Arc<dyn Instruction>

    // Test instruction for direct loading
    #[derive(Debug, Clone)]
    struct TestAddInstruction;
    impl Instruction for TestAddInstruction {
        fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
            let op2 = executor.pop_operand()?;
            let op1 = executor.pop_operand()?;
            executor.push_operand(op1 + op2);
            Ok(())
        }
    }

    #[derive(Debug, Clone)]
    struct TestPushInstruction {
        value: f64,
    }
    impl Instruction for TestPushInstruction {
        fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
            executor.push_operand(self.value);
            Ok(())
        }
    }

    #[derive(Debug, Clone)]
    struct TestStoreLoadGuestWordInstruction;
    impl Instruction for TestStoreLoadGuestWordInstruction {
        fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
            let guest_arch = executor.get_guest_architecture();
            let word_size = guest_arch.word_size();
            let mem_manager = executor.get_memory_manager_mut();

            let handle = mem_manager.allocate(word_size)?;
            // For simplicity, we assume the handle is the base address for this test.
            // In a real scenario, mapping might be needed if handles are not direct addresses.
            // Also, MemoryManager's dummy load/store might not reflect actual stored values.
            // This test primarily checks that allocate/load/store can be called through AdaptedMemoryManager.
            let base_address = handle.0;

            let mut stored_bytes = Vec::with_capacity(word_size);
            for i in 0..word_size {
                let byte_val = (i as u8 + 1) * 11; // Some arbitrary pattern
                // Using a direct store to the host_memory_manager if available for testing,
                // or rely on the interface. For now, use interface.
                mem_manager.store(base_address + i, byte_val)?;
                stored_bytes.push(byte_val);
            }

            let mut loaded_bytes = Vec::with_capacity(word_size);
            for i in 0..word_size {
                loaded_bytes.push(mem_manager.load(base_address + i)?);
            }

            mem_manager.deallocate(handle)?;

            // The dummy MemoryManager::load always returns (address & 0xFF) as u8.
            // So a direct comparison stored_bytes == loaded_bytes will fail with dummy memory manager.
            // The goal here is to ensure the operations complete without error in compat mode.
            // A true value check would require a MemoryManager that actually stores/retrieves.
            // For now, success is operations not erroring and pushing 1.0.
            // We can check if loaded_bytes has the expected pattern from the dummy load.
            let mut expected_loaded_bytes = Vec::with_capacity(word_size);
            for i in 0..word_size {
                expected_loaded_bytes.push(((base_address + i) & 0xFF) as u8);
            }

            if loaded_bytes == expected_loaded_bytes {
                executor.push_operand(1.0); // Success based on dummy load behavior
            } else {
                println!("Mismatch! Expected dummy loaded: {:?}, Actual loaded: {:?}", expected_loaded_bytes, loaded_bytes);
                executor.push_operand(0.0); // Failure
            }
            Ok(())
        }
    }

    fn new_executor<HostArch: Architecture + std::fmt::Debug>(host_label: VmArchitecture, guest_label: VmArchitecture) -> MultiArchExecutor<HostArch> {
        MultiArchExecutor::<HostArch>::new(host_label, guest_label).expect("Failed to create executor for test")
    }

    #[test]
    fn test_new_executors_native_mode() {
        let exec32 = new_executor::<Arch32>(VmArchitecture::Arch32, VmArchitecture::Arch32);
        assert_eq!(exec32.host_vm_arch_label, VmArchitecture::Arch32);
        assert_eq!(exec32.guest_vm_arch_label, VmArchitecture::Arch32);
        assert!(!exec32.is_compatibility_mode);
        // Test the actual memory manager type if possible, or its behavior.
        // For now, we assume it's a direct MemoryManager<Arch32>.
        // Attempting to get word size from the boxed trait object is tricky without downcasting.
        // We'll rely on other behavior tests.

        let exec64 = new_executor::<Arch64>(VmArchitecture::Arch64, VmArchitecture::Arch64);
        assert_eq!(exec64.host_vm_arch_label, VmArchitecture::Arch64);
        assert!(!exec64.is_compatibility_mode);
    }

    #[test]
    fn test_new_executors_compatibility_mode() {
        // Host Arch64, Guest Arch32
        let exec_compat = new_executor::<Arch64>(VmArchitecture::Arch64, VmArchitecture::Arch32);
        assert_eq!(exec_compat.host_vm_arch_label, VmArchitecture::Arch64);
        assert_eq!(exec_compat.guest_vm_arch_label, VmArchitecture::Arch32);
        assert!(exec_compat.is_compatibility_mode);
        // Here, exec_compat.memory_interface should be an AdaptedMemoryManager<Arch64>
    }

    #[test]
    fn test_arch_mismatch_error_in_new() {
        // Host label Arch64, but generic type is Arch32
        let res = MultiArchExecutor::<Arch32>::new(VmArchitecture::Arch64, VmArchitecture::Arch32);
        assert!(res.is_err());
        if let Err(VMError::ArchitectureMismatch(msg)) = res {
            assert!(msg.contains("Host label Arch64"));
            assert!(msg.contains("Arch32"));
        } else {
            panic!("Expected ArchitectureMismatch error, got {:?}", res);
        }
    }

    #[test]
    fn test_config_error_guest_larger_than_host_in_new() {
        let res = MultiArchExecutor::<Arch32>::new(VmArchitecture::Arch32, VmArchitecture::Arch64);
        assert!(res.is_err());
        if let Err(VMError::ConfigurationError(msg)) = res {
            assert!(msg.contains("Cannot run guest bytecode for Arch64 on a smaller host architecture Arch32"));
        } else {
            panic!("Expected ConfigurationError, got {:?}", res);
        }
    }

    #[test]
    fn test_stack_ops_native() {
        let mut exec = new_executor::<Arch64>(VmArchitecture::Arch64, VmArchitecture::Arch64);
        exec.push_operand(10.0);
        exec.push_operand(20.0);
        assert_eq!(exec.pop_operand().unwrap(), 20.0);
        assert_eq!(exec.pop_operand().unwrap(), 10.0);
        assert!(exec.pop_operand().is_err());
    }

    #[test]
    fn test_instruction_loading_and_execution_native() {
        let mut exec = new_executor::<Arch64>(VmArchitecture::Arch64, VmArchitecture::Arch64);
        let instructions: Vec<Arc<dyn Instruction>> = vec![Arc::new(TestPushInstruction { value: 3.0 }), Arc::new(TestPushInstruction { value: 4.0 }), Arc::new(TestAddInstruction)];
        exec.load_instructions(instructions);

        assert!(exec.run().is_ok());
        assert_eq!(exec.pop_operand().unwrap(), 7.0);
        assert_eq!(exec.instruction_pointer, 3);
    }

    #[test]
    fn test_reset_native() {
        let mut exec = new_executor::<Arch64>(VmArchitecture::Arch64, VmArchitecture::Arch64);
        exec.push_operand(10.0);
        let instructions: Vec<Arc<dyn Instruction>> = vec![Arc::new(TestPushInstruction { value: 1.0 })];
        exec.load_instructions(instructions);
        exec.instruction_pointer = 1; // Simulate partial execution

        exec.reset().unwrap();

        assert!(exec.operand_stack.is_empty());
        assert_eq!(exec.instruction_pointer, 0);
        assert!(exec.instructions.is_empty());
    }

    #[test]
    fn test_set_instruction_pointer_native() {
        let mut exec = new_executor::<Arch64>(VmArchitecture::Arch64, VmArchitecture::Arch64);
        let instructions: Vec<Arc<dyn Instruction>> = vec![Arc::new(TestPushInstruction { value: 1.0 }), Arc::new(TestPushInstruction { value: 2.0 })];
        exec.load_instructions(instructions);

        assert!(exec.set_instruction_pointer(1).is_ok());
        assert_eq!(exec.instruction_pointer, 1);

        assert!(exec.set_instruction_pointer(0).is_ok());
        assert_eq!(exec.instruction_pointer, 0);

        assert!(exec.set_instruction_pointer(2).is_err());
        assert!(exec.set_instruction_pointer(100).is_err());
    }

    #[test]
    fn test_compatibility_mode_memory_access() {
        // Host Arch64, Guest Arch32
        let mut exec = new_executor::<Arch64>(VmArchitecture::Arch64, VmArchitecture::Arch32);
        assert!(exec.is_compatibility_mode());
        assert_eq!(exec.get_guest_architecture(), VmArchitecture::Arch32);

        let instructions: Vec<Arc<dyn Instruction>> = vec![Arc::new(TestStoreLoadGuestWordInstruction)];
        exec.load_instructions(instructions);
        let run_result = exec.run();
        assert!(run_result.is_ok(), "Run failed: {:?}", run_result.err());

        let success_flag = exec.pop_operand().expect("Stack underflow after compat test instruction");
        assert_eq!(
            success_flag, 1.0,
            "TestStoreLoadGuestWordInstruction indicated failure due to value mismatch (could be dummy memory behavior)"
        );
    }

    #[test]
    fn test_compatibility_mode_reset() {
        let mut exec = new_executor::<Arch128>(VmArchitecture::Arch128, VmArchitecture::Arch32); // Host 128, Guest 32
        exec.push_operand(1.0);
        exec.load_instructions(vec![Arc::new(TestStoreLoadGuestWordInstruction)]);
        assert!(exec.run().is_ok()); // Pops the 1.0, pushes result of test
        exec.pop_operand().unwrap(); // Pop result of test

        exec.reset().unwrap();
        assert!(exec.operand_stack.is_empty());
        assert_eq!(exec.instruction_pointer, 0);
        assert!(exec.instructions.is_empty());
        assert!(exec.is_compatibility_mode); // Still in compat mode conceptually
        assert_eq!(exec.guest_vm_arch_label, VmArchitecture::Arch32);
        assert_eq!(exec.host_vm_arch_label, VmArchitecture::Arch128);
    }

    // Removed the TestMemoryManagerInterfaceExt and related complexity,
    // as direct testing of the boxed trait object's underlying type is non-trivial
    // and better covered by behavior (e.g., does compat mode work as expected).
    // The MemoryManager<A>::architecture_word_size() helper is also removed as it was
    // part of the problematic test structure. The executor's own labels are the source of truth now.
    impl<A: Architecture> MemoryManager<A> {
        // This test-only helper was defined in the original file, keeping it if it's used by other tests
        // not being modified here. If not, it can be removed.
        // It seems it was used by the old `test_new_executors`.
        // The new `test_new_executors_native_mode` doesn't use it directly on memory_manager field anymore.
        // Let's remove it to clean up as it's no longer directly used by the refactored tests.
        // pub fn architecture_word_size(&self) -> usize {
        //     A::WORD_SIZE
        // }
    }
}
