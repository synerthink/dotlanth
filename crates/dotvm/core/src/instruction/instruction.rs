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

use crate::vm::errors::VMError;
use std::fmt::Debug; // Import Debug

/// Trait for types that can execute instructions (executor interface)
pub trait ExecutorInterface: Debug {
    // Add Debug supertrait
    /// Push an operand onto the stack
    fn push_operand(&mut self, value: f64);

    /// Pop an operand from the stack
    fn pop_operand(&mut self) -> Result<f64, VMError>;

    /// Set the instruction pointer to a specific index
    fn set_instruction_pointer(&mut self, target: usize) -> Result<(), VMError>;

    /// Get access to memory manager for memory operations
    /// Returns a trait object that can handle memory operations
    fn get_memory_manager_mut(&mut self) -> &mut dyn MemoryManagerInterface;

    /// Gets the architecture for which the currently running bytecode was intended.
    /// This may differ from the actual execution architecture if in compatibility mode.
    fn get_guest_architecture(&self) -> crate::bytecode::VmArchitecture;

    /// Checks if the executor is currently running in compatibility mode.
    fn is_compatibility_mode(&self) -> bool;
}

/// Trait for memory manager interface that instructions can use
pub trait MemoryManagerInterface: Debug {
    // Add Debug supertrait
    /// Allocate memory and return a handle
    fn allocate(&mut self, size: usize) -> Result<crate::memory::MemoryHandle, VMError>;

    /// Deallocate memory using a handle
    fn deallocate(&mut self, handle: crate::memory::MemoryHandle) -> Result<(), VMError>;

    /// Load a byte from memory
    fn load(&self, address: usize) -> Result<u8, VMError>;

    /// Store a byte to memory
    fn store(&mut self, address: usize, value: u8) -> Result<(), VMError>;
}

/// Trait representing a generic instruction.
pub trait Instruction {
    /// Execute the instruction using the provided executor.
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError>;
}
