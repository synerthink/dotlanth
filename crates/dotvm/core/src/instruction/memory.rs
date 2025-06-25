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

// use crate::memory::MemoryManagement; // Unused
use crate::memory::MemoryHandle; // VirtualAddress removed
// use crate::opcode::memory_opcodes::MemoryOpcode; // Unused
// use crate::operand::operands::Operand; // Unused
use crate::vm::errors::VMError;

// use std::fmt; // Unused

use super::instruction::{ExecutorInterface, Instruction};

/// Struct representing a LOAD instruction.
pub struct LoadInstruction {
    address: usize,
}

impl LoadInstruction {
    pub fn new(address: usize) -> Self {
        LoadInstruction { address }
    }
}

impl Instruction for LoadInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        let value = {
            let memory_manager = executor.get_memory_manager_mut();
            memory_manager.load(self.address)?
        };
        executor.push_operand(value as f64); // Assuming value is u8, cast to f64
        Ok(())
    }
}

/// Struct representing a STORE instruction.
pub struct StoreInstruction {
    address: usize,
}

impl StoreInstruction {
    pub fn new(address: usize) -> Self {
        StoreInstruction { address }
    }
}

impl Instruction for StoreInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        let value = executor.pop_operand()? as u8; // Store as u8
        {
            let memory_manager = executor.get_memory_manager_mut();
            memory_manager.store(self.address, value)?;
        }
        Ok(())
    }
}

/// Struct representing an ALLOCATE instruction.
pub struct AllocateInstruction {
    size: usize,
}

impl AllocateInstruction {
    pub fn new(size: usize) -> Self {
        AllocateInstruction { size }
    }
}

impl Instruction for AllocateInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        let handle = {
            let memory_manager = executor.get_memory_manager_mut();
            memory_manager.allocate(self.size)?
        };
        executor.push_operand(handle.0 as f64); // Push handle address as f64
        Ok(())
    }
}

/// Struct representing a DEALLOCATE instruction.
pub struct DeallocateInstruction {
    handle: usize,
}

impl DeallocateInstruction {
    pub fn new(handle: usize) -> Self {
        DeallocateInstruction { handle }
    }
}

impl Instruction for DeallocateInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        {
            let mm = executor.get_memory_manager_mut();
            mm.deallocate(MemoryHandle(self.handle))?;
        }
        Ok(())
    }
}

/// Enum for Pointer Operations.
pub enum PointerOperationType {
    Add,
    Subtract,
}

/// Struct representing a POINTEROPERATION instruction.
pub struct PointerOperationInstruction {
    operation: PointerOperationType,
    offset: isize,
}

impl PointerOperationInstruction {
    pub fn new(operation: PointerOperationType, offset: isize) -> Self {
        PointerOperationInstruction { operation, offset }
    }
}

impl Instruction for PointerOperationInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // Pop a pointer (address) from the stack
        let pointer = executor.pop_operand()? as isize;
        let new_pointer = match self.operation {
            PointerOperationType::Add => pointer.checked_add(self.offset).ok_or(VMError::PointerOverflow)?,
            PointerOperationType::Subtract => pointer.checked_sub(self.offset).ok_or(VMError::PointerOverflow)?,
        };
        executor.push_operand(new_pointer as f64);
        Ok(())
    }
}
