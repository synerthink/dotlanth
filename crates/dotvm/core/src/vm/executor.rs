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
    instruction::{
        instruction::Instruction,
        registry::{InstructionRegistry, Opcode},
    },
    memory::{Arch64, Architecture, MemoryHandle, MemoryManagement, MemoryManager, VirtualAddress},
    vm::errors::VMError,
};
use std::sync::Arc;

/// Executor struct handling the operand stack and instruction execution.
pub struct Executor {
    operand_stack: Vec<f64>,
    instruction_pointer: usize,
    instructions: Vec<Arc<dyn Instruction>>,
    memory_manager: MemoryManager<Arch64>,
}

impl Executor {
    /// Creates a new Executor instance with a 64-bit MemoryManager.
    pub fn new() -> Result<Self, VMError> {
        // Import the MemoryManagement trait to access the `new` method
        use crate::memory::MemoryManager;

        // Initialize the MemoryManager with Arch64
        let memory_manager = MemoryManager::<Arch64>::new()?;

        Ok(Executor {
            operand_stack: Vec::new(),
            instruction_pointer: 0,
            instructions: Vec::new(),
            memory_manager,
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

    /// Load instructions into the executor.
    pub fn load_instructions(&mut self, opcodes: Vec<(Opcode, Option<Vec<usize>>)>)
    where
        Self: Sized,
    {
        let registry = InstructionRegistry::new();
        for (opcode, args) in opcodes {
            match registry.create_instruction(opcode, args) {
                Ok(instr) => self.instructions.push(instr),
                Err(e) => {
                    // Handle error (e.g., log or panic). Here, we choose to panic for simplicity.
                    panic!("Failed to create instruction: {:?}", e);
                }
            }
        }
    }

    /// Execute all loaded instructions sequentially.
    pub fn execute(&mut self) -> Result<(), VMError> {
        while self.instruction_pointer < self.instructions.len() {
            let instruction = self.instructions[self.instruction_pointer].clone();
            instruction.execute(self)?;
            self.instruction_pointer += 1;
        }
        Ok(())
    }

    /// Get a reference to the MemoryManager.
    pub fn get_memory_manager(&self) -> &MemoryManager<Arch64> {
        &self.memory_manager
    }

    /// Get a mutable reference to the MemoryManager.
    pub fn get_memory_manager_mut(&mut self) -> &mut MemoryManager<Arch64> {
        &mut self.memory_manager
    }
}
