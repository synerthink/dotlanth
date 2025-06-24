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

//! Multi-architecture executor for DotVM
//!
//! This module provides executors that are aware of the target architecture
//! and can handle different opcode sets and instruction types based on the
//! architecture configuration.

use crate::{
    instruction::{
        architecture_registry::{ArchitectureRegistry, Registry64, Registry128, RegistryFactory},
        instruction::{ExecutorInterface, Instruction},
    },
    memory::{Arch64, Arch128, MemoryManagement, MemoryManager},
    opcode::architecture_opcodes::{Opcode64, Opcode128},
    vm::errors::VMError,
};
use std::{marker::PhantomData, sync::Arc};

/// Multi-architecture executor trait
pub trait MultiArchExecutor {
    type Opcode: Clone + Copy + std::fmt::Debug + std::fmt::Display;

    /// Push an operand onto the stack
    fn push_operand(&mut self, value: f64);

    /// Pop an operand from the stack
    fn pop_operand(&mut self) -> Result<f64, VMError>;

    /// Set the instruction pointer to a specific index (for external control)
    fn set_instruction_pointer_external(&mut self, target: usize) -> Result<(), VMError>;

    /// Load instructions into the executor
    fn load_instructions(&mut self, opcodes: Vec<(Self::Opcode, Option<Vec<usize>>)>);

    /// Execute all loaded instructions sequentially
    fn execute(&mut self) -> Result<(), VMError>;

    /// Get the current instruction pointer
    fn get_instruction_pointer(&self) -> usize;

    /// Get the current stack size
    fn get_stack_size(&self) -> usize;

    /// Get architecture name
    fn architecture_name(&self) -> &'static str;
}

/// 64-bit architecture executor
pub struct Executor64 {
    operand_stack: Vec<f64>,
    instruction_pointer: usize,
    instructions: Vec<Arc<dyn Instruction>>,
    memory_manager: MemoryManager<Arch64>,
    registry: Registry64,
}

impl Executor64 {
    /// Creates a new 64-bit executor instance
    pub fn new() -> Result<Self, VMError> {
        let memory_manager = MemoryManager::<Arch64>::new()?;

        Ok(Executor64 {
            operand_stack: Vec::new(),
            instruction_pointer: 0,
            instructions: Vec::new(),
            memory_manager,
            registry: RegistryFactory::create_64bit_registry(),
        })
    }

    /// Get a reference to the MemoryManager
    pub fn get_memory_manager(&self) -> &MemoryManager<Arch64> {
        &self.memory_manager
    }

    /// Get a mutable reference to the MemoryManager
    pub fn get_memory_manager_mut(&mut self) -> &mut MemoryManager<Arch64> {
        &mut self.memory_manager
    }
}

// Implement the same interface as the original Executor for compatibility
impl Executor64 {
    /// Push an operand onto the stack (compatibility method)
    pub fn push_operand(&mut self, value: f64) {
        self.operand_stack.push(value);
    }

    /// Pop an operand from the stack (compatibility method)
    pub fn pop_operand(&mut self) -> Result<f64, VMError> {
        self.operand_stack.pop().ok_or(VMError::StackUnderflow)
    }
}

impl ExecutorInterface for Executor64 {
    fn push_operand(&mut self, value: f64) {
        self.operand_stack.push(value);
    }

    fn pop_operand(&mut self) -> Result<f64, VMError> {
        self.operand_stack.pop().ok_or(VMError::StackUnderflow)
    }

    fn set_instruction_pointer(&mut self, target: usize) -> Result<(), VMError> {
        if target >= self.instructions.len() {
            return Err(VMError::InvalidJumpTarget(target));
        }
        self.instruction_pointer = target;
        Ok(())
    }

    fn get_memory_manager_mut(&mut self) -> &mut dyn crate::instruction::instruction::MemoryManagerInterface {
        &mut self.memory_manager
    }
}

impl MultiArchExecutor for Executor64 {
    type Opcode = Opcode64;

    fn push_operand(&mut self, value: f64) {
        self.operand_stack.push(value);
    }

    fn pop_operand(&mut self) -> Result<f64, VMError> {
        self.operand_stack.pop().ok_or(VMError::StackUnderflow)
    }

    fn set_instruction_pointer_external(&mut self, target: usize) -> Result<(), VMError> {
        if target >= self.instructions.len() {
            return Err(VMError::InvalidJumpTarget(target));
        }
        self.instruction_pointer = target;
        Ok(())
    }

    fn load_instructions(&mut self, opcodes: Vec<(Self::Opcode, Option<Vec<usize>>)>) {
        self.instructions.clear();
        for (opcode, args) in opcodes {
            match self.registry.create_instruction(opcode, args) {
                Ok(instr) => self.instructions.push(instr),
                Err(e) => {
                    // In a production system, you might want to handle this more gracefully
                    panic!("Failed to create instruction: {:?}", e);
                }
            }
        }
    }

    fn execute(&mut self) -> Result<(), VMError> {
        while self.instruction_pointer < self.instructions.len() {
            let instruction = self.instructions[self.instruction_pointer].clone();
            instruction.execute(self)?;
            self.instruction_pointer += 1;
        }
        Ok(())
    }

    fn get_instruction_pointer(&self) -> usize {
        self.instruction_pointer
    }

    fn get_stack_size(&self) -> usize {
        self.operand_stack.len()
    }

    fn architecture_name(&self) -> &'static str {
        "64-bit"
    }
}

/// 128-bit architecture executor
pub struct Executor128 {
    operand_stack: Vec<f64>,
    instruction_pointer: usize,
    instructions: Vec<Arc<dyn Instruction>>,
    memory_manager: MemoryManager<Arch128>,
    registry: Registry128,
}

impl Executor128 {
    /// Creates a new 128-bit executor instance
    pub fn new() -> Result<Self, VMError> {
        let memory_manager = MemoryManager::<Arch128>::new()?;

        Ok(Executor128 {
            operand_stack: Vec::new(),
            instruction_pointer: 0,
            instructions: Vec::new(),
            memory_manager,
            registry: RegistryFactory::create_128bit_registry(),
        })
    }

    /// Get a reference to the MemoryManager
    pub fn get_memory_manager(&self) -> &MemoryManager<Arch128> {
        &self.memory_manager
    }

    /// Get a mutable reference to the MemoryManager
    pub fn get_memory_manager_mut(&mut self) -> &mut MemoryManager<Arch128> {
        &mut self.memory_manager
    }

    /// Check if an opcode is backward compatible (can run 64-bit code)
    pub fn is_backward_compatible(&self, opcode: &Opcode128) -> bool {
        opcode.is_64bit_compatible()
    }
}

// Implement the same interface as the original Executor for compatibility
impl Executor128 {
    /// Push an operand onto the stack (compatibility method)
    pub fn push_operand(&mut self, value: f64) {
        self.operand_stack.push(value);
    }

    /// Pop an operand from the stack (compatibility method)
    pub fn pop_operand(&mut self) -> Result<f64, VMError> {
        self.operand_stack.pop().ok_or(VMError::StackUnderflow)
    }
}

impl ExecutorInterface for Executor128 {
    fn push_operand(&mut self, value: f64) {
        self.operand_stack.push(value);
    }

    fn pop_operand(&mut self) -> Result<f64, VMError> {
        self.operand_stack.pop().ok_or(VMError::StackUnderflow)
    }

    fn set_instruction_pointer(&mut self, target: usize) -> Result<(), VMError> {
        if target >= self.instructions.len() {
            return Err(VMError::InvalidJumpTarget(target));
        }
        self.instruction_pointer = target;
        Ok(())
    }

    fn get_memory_manager_mut(&mut self) -> &mut dyn crate::instruction::instruction::MemoryManagerInterface {
        &mut self.memory_manager
    }
}

impl MultiArchExecutor for Executor128 {
    type Opcode = Opcode128;

    fn push_operand(&mut self, value: f64) {
        self.operand_stack.push(value);
    }

    fn pop_operand(&mut self) -> Result<f64, VMError> {
        self.operand_stack.pop().ok_or(VMError::StackUnderflow)
    }

    fn set_instruction_pointer_external(&mut self, target: usize) -> Result<(), VMError> {
        if target >= self.instructions.len() {
            return Err(VMError::InvalidJumpTarget(target));
        }
        self.instruction_pointer = target;
        Ok(())
    }

    fn load_instructions(&mut self, opcodes: Vec<(Self::Opcode, Option<Vec<usize>>)>) {
        self.instructions.clear();
        for (opcode, args) in opcodes {
            match self.registry.create_instruction(opcode, args) {
                Ok(instr) => self.instructions.push(instr),
                Err(e) => {
                    // In a production system, you might want to handle this more gracefully
                    panic!("Failed to create instruction: {:?}", e);
                }
            }
        }
    }

    fn execute(&mut self) -> Result<(), VMError> {
        while self.instruction_pointer < self.instructions.len() {
            let instruction = self.instructions[self.instruction_pointer].clone();
            instruction.execute(self)?;
            self.instruction_pointer += 1;
        }
        Ok(())
    }

    fn get_instruction_pointer(&self) -> usize {
        self.instruction_pointer
    }

    fn get_stack_size(&self) -> usize {
        self.operand_stack.len()
    }

    fn architecture_name(&self) -> &'static str {
        "128-bit"
    }
}

/// Executor factory for creating architecture-specific executors
pub struct ExecutorFactory;

impl ExecutorFactory {
    /// Create a 64-bit executor
    pub fn create_64bit_executor() -> Result<Executor64, VMError> {
        Executor64::new()
    }

    /// Create a 128-bit executor
    pub fn create_128bit_executor() -> Result<Executor128, VMError> {
        Executor128::new()
    }

    /// Create an executor based on architecture name
    pub fn create_executor_for_architecture(arch_name: &str) -> Result<Box<dyn std::any::Any>, VMError> {
        match arch_name {
            "64-bit" => Ok(Box::new(Self::create_64bit_executor()?)),
            "128-bit" => Ok(Box::new(Self::create_128bit_executor()?)),
            _ => Err(VMError::UnknownOpcode),
        }
    }
}

/// Backward compatibility utilities
pub mod compatibility {
    use super::*;
    use crate::opcode::architecture_opcodes::{Opcode64, Opcode128};

    /// Convert 64-bit opcodes to 128-bit opcodes for backward compatibility
    pub fn convert_64bit_opcodes_to_128bit(opcodes: Vec<(Opcode64, Option<Vec<usize>>)>) -> Vec<(Opcode128, Option<Vec<usize>>)> {
        opcodes.into_iter().map(|(opcode, args)| (Opcode128::Base(opcode), args)).collect()
    }

    /// Run 64-bit bytecode on a 128-bit executor
    pub fn run_64bit_on_128bit_executor(executor: &mut Executor128, opcodes: Vec<(Opcode64, Option<Vec<usize>>)>) -> Result<(), VMError> {
        let converted_opcodes = convert_64bit_opcodes_to_128bit(opcodes);
        executor.load_instructions(converted_opcodes);
        executor.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcode::{
        architecture_opcodes::{BigIntOpcode, Opcode64, Opcode128},
        arithmetic_opcodes::ArithmeticOpcode,
    };

    #[test]
    fn test_64bit_executor_creation() {
        let executor = ExecutorFactory::create_64bit_executor();
        assert!(executor.is_ok());

        let executor = executor.unwrap();
        assert_eq!(executor.architecture_name(), "64-bit");
        assert_eq!(executor.get_instruction_pointer(), 0);
        assert_eq!(executor.get_stack_size(), 0);
    }

    #[test]
    fn test_128bit_executor_creation() {
        let executor = ExecutorFactory::create_128bit_executor();
        assert!(executor.is_ok());

        let executor = executor.unwrap();
        assert_eq!(executor.architecture_name(), "128-bit");
        assert_eq!(executor.get_instruction_pointer(), 0);
        assert_eq!(executor.get_stack_size(), 0);
    }

    #[test]
    fn test_64bit_executor_basic_operations() {
        let mut executor = ExecutorFactory::create_64bit_executor().unwrap();

        // Test stack operations
        executor.push_operand(42.0);
        executor.push_operand(24.0);
        assert_eq!(executor.get_stack_size(), 2);

        let value = executor.pop_operand().unwrap();
        assert_eq!(value, 24.0);
        assert_eq!(executor.get_stack_size(), 1);

        let value = executor.pop_operand().unwrap();
        assert_eq!(value, 42.0);
        assert_eq!(executor.get_stack_size(), 0);
    }

    #[test]
    fn test_128bit_executor_basic_operations() {
        let mut executor = ExecutorFactory::create_128bit_executor().unwrap();

        // Test stack operations
        executor.push_operand(100.0);
        executor.push_operand(200.0);
        assert_eq!(executor.get_stack_size(), 2);

        let value = executor.pop_operand().unwrap();
        assert_eq!(value, 200.0);
        assert_eq!(executor.get_stack_size(), 1);
    }

    #[test]
    fn test_64bit_arithmetic_execution() {
        let mut executor = ExecutorFactory::create_64bit_executor().unwrap();

        // Load a simple addition instruction
        let opcodes = vec![(Opcode64::Arithmetic(ArithmeticOpcode::Add), None)];
        executor.load_instructions(opcodes);

        // Set up operands for addition
        executor.push_operand(10.0);
        executor.push_operand(20.0);

        // Execute the instruction
        let result = executor.execute();
        assert!(result.is_ok());

        // Check the result
        assert_eq!(executor.get_stack_size(), 1);
        let result = executor.pop_operand().unwrap();
        assert_eq!(result, 30.0);
    }

    #[test]
    fn test_128bit_bigint_execution() {
        let mut executor = ExecutorFactory::create_128bit_executor().unwrap();

        // Load a BigInt addition instruction
        let opcodes = vec![(Opcode128::BigInt(BigIntOpcode::Add), None)];
        executor.load_instructions(opcodes);

        // Set up operands for BigInt addition
        executor.push_operand(100.0);
        executor.push_operand(200.0);

        // Execute the instruction
        let result = executor.execute();
        assert!(result.is_ok());

        // Check the result
        assert_eq!(executor.get_stack_size(), 1);
        let result = executor.pop_operand().unwrap();
        assert_eq!(result, 300.0);
    }

    #[test]
    fn test_backward_compatibility() {
        let mut executor = ExecutorFactory::create_128bit_executor().unwrap();

        // Test running 64-bit code on 128-bit executor
        let opcodes_64bit = vec![(Opcode64::Arithmetic(ArithmeticOpcode::Add), None)];

        executor.push_operand(15.0);
        executor.push_operand(25.0);

        let result = compatibility::run_64bit_on_128bit_executor(&mut executor, opcodes_64bit);
        assert!(result.is_ok());

        // Check the result
        assert_eq!(executor.get_stack_size(), 1);
        let result = executor.pop_operand().unwrap();
        assert_eq!(result, 40.0);
    }

    #[test]
    fn test_opcode_conversion() {
        let opcodes_64bit = vec![
            (Opcode64::Arithmetic(ArithmeticOpcode::Add), None),
            (Opcode64::Arithmetic(ArithmeticOpcode::Multiply), Some(vec![1, 2])),
        ];

        let converted = compatibility::convert_64bit_opcodes_to_128bit(opcodes_64bit.clone());
        assert_eq!(converted.len(), 2);

        // Check that the conversion preserves the opcodes and arguments
        match &converted[0] {
            (Opcode128::Base(Opcode64::Arithmetic(ArithmeticOpcode::Add)), None) => {}
            _ => panic!("Conversion failed for first opcode"),
        }

        match &converted[1] {
            (Opcode128::Base(Opcode64::Arithmetic(ArithmeticOpcode::Multiply)), Some(args)) => {
                assert_eq!(args, &vec![1, 2]);
            }
            _ => panic!("Conversion failed for second opcode"),
        }
    }

    #[test]
    fn test_stack_underflow() {
        let mut executor = ExecutorFactory::create_64bit_executor().unwrap();

        // Try to pop from empty stack
        let result = executor.pop_operand();
        assert!(matches!(result, Err(VMError::StackUnderflow)));
    }

    #[test]
    fn test_invalid_jump_target() {
        let mut executor = ExecutorFactory::create_64bit_executor().unwrap();

        // Try to jump to invalid target
        let result = executor.set_instruction_pointer_external(100);
        assert!(matches!(result, Err(VMError::InvalidJumpTarget(100))));
    }
}
