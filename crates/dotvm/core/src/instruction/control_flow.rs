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

use super::instruction::{ExecutorInterface, Instruction};
use crate::opcode::control_flow_opcodes::ControlFlowOpcode;
use crate::vm::errors::VMError;

use std::sync::Arc;

/// Struct representing a conditional branch (`IfElse`) instruction.
pub struct IfElseInstruction {
    // The target instruction index to jump to if condition is false
    false_branch: usize,
}

impl IfElseInstruction {
    pub fn new(false_branch: usize) -> Self {
        IfElseInstruction { false_branch }
    }
}

impl Instruction for IfElseInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // Pop the condition from the stack
        let condition = executor.pop_operand()?;

        if condition == 0.0 {
            // If condition is false (0), jump to false_branch
            executor.set_instruction_pointer(self.false_branch)?;
        }

        Ok(())
    }
}

/// Struct representing a jump instruction.
pub struct JumpInstruction {
    // The target instruction index to jump to
    target: usize,
}

impl JumpInstruction {
    pub fn new(target: usize) -> Self {
        JumpInstruction { target }
    }
}

impl Instruction for JumpInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // Unconditionally jump to target
        executor.set_instruction_pointer(self.target)?;
        Ok(())
    }
}

/// Struct representing a loop instruction.
/// For simplicity, we'll implement `WhileLoop` and `ForLoop` using conditional branches.
pub enum LoopType {
    WhileLoop,
    DoWhileLoop,
    ForLoop, // For simplicity, treat as WhileLoop with a counter
}

/// Struct representing a loop instruction.
pub struct LoopInstruction {
    loop_type: LoopType,
    condition_start: usize,
    body_start: usize,
}

impl LoopInstruction {
    pub fn new(loop_type: LoopType, condition_start: usize, body_start: usize) -> Self {
        LoopInstruction {
            loop_type,
            condition_start,
            body_start,
        }
    }
}

impl Instruction for LoopInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        match self.loop_type {
            LoopType::WhileLoop => {
                // For WhileLoop, condition is already at condition_start
                executor.set_instruction_pointer(self.condition_start)?;
            }
            LoopType::DoWhileLoop => {
                // After body execution, jump back to condition_start
                executor.set_instruction_pointer(self.condition_start)?;
            }
            LoopType::ForLoop => {
                // Implement as a WhileLoop with a loop counter
                executor.set_instruction_pointer(self.condition_start)?;
            }
        }
        Ok(())
    }
}
