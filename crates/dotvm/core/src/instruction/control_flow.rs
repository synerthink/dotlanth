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

use crate::instruction::arithmetic::{Instruction, Operand, Operator};

/// Enum representing different control flow instructions.
#[derive(Debug, PartialEq)]
pub enum ControlFlowInstruction {
    /// Conditional branching with if-else structures.
    IfElse {
        condition: Condition,
        if_branch: Vec<Instruction>,
        else_branch: Vec<Instruction>,
    },
    /// For loop supporting initializer, condition, updater, and body.
    ForLoop {
        initializer: Instruction,
        condition: Condition,
        updater: Instruction,
        body: Vec<Instruction>,
    },
    /// While loop that continues as long as the condition is true.
    WhileLoop {
        condition: Condition,
        body: Vec<Instruction>,
    },
    /// Do-while loop that executes the body at least once.
    DoWhileLoop {
        body: Vec<Instruction>,
        condition: Condition,
    },
    /// Unconditional jump to a specific instruction index.
    Jump { target: usize },
}

/// Struct representing a condition used in control flow instructions.
#[derive(Debug, PartialEq)]
pub struct Condition {
    pub left: Operand,
    pub operator: Operator,
    pub right: Operand,
}

/// Executes an if-else control flow instruction.
pub fn execute_if_else(
    condition: &Condition,
    if_branch: &Vec<Instruction>,
    else_branch: &Vec<Instruction>,
) -> Result<(), String> {
    // Implement the if-else logic here
    todo!("Implement IfElse instruction")
}

/// Executes a for loop control flow instruction.
pub fn execute_for_loop(
    initializer: &Instruction,
    condition: &Condition,
    updater: &Instruction,
    body: &Vec<Instruction>,
) -> Result<(), String> {
    // Initialize, evaluate condition, execute body, and perform updater
    todo!("Implement ForLoop execution logic")
}

/// Executes a while loop control flow instruction.
pub fn execute_while_loop(condition: &Condition, body: &Vec<Instruction>) -> Result<(), String> {
    // Evaluate condition and execute body repeatedly
    todo!("Implement WhileLoop execution logic")
}

/// Executes a do-while loop control flow instruction.
pub fn execute_do_while_loop(body: &Vec<Instruction>, condition: &Condition) -> Result<(), String> {
    // Execute body once and then evaluate condition
    todo!("Implement DoWhileLoop execution logic")
}

/// Executes an unconditional jump control flow instruction.
pub fn execute_jump(target: usize) -> Result<(), String> {
    // Perform an unconditional jump to the target instruction index
    todo!("Implement Jump execution logic")
}

/// Executes a given control flow instruction.
pub fn execute_control_flow(instruction: &ControlFlowInstruction) -> Result<(), String> {
    match instruction {
        ControlFlowInstruction::IfElse {
            condition,
            if_branch,
            else_branch,
        } => execute_if_else(condition, if_branch, else_branch),
        ControlFlowInstruction::ForLoop {
            initializer,
            condition,
            updater,
            body,
        } => execute_for_loop(initializer, condition, updater, body),
        ControlFlowInstruction::WhileLoop { condition, body } => {
            execute_while_loop(condition, body)
        }
        ControlFlowInstruction::DoWhileLoop { body, condition } => {
            execute_do_while_loop(body, condition)
        }
        ControlFlowInstruction::Jump { target } => execute_jump(*target),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_if_else_instruction() {
        // Arrange
        let condition = Condition {
            left: Operand::Integer(1),
            operator: Operator::GreaterThan,
            right: Operand::Integer(0),
        };
        let if_branch = vec![
            Instruction::Initialize, // Replace with actual instructions
        ];
        let else_branch = vec![
            Instruction::Initialize, // Replace with actual instructions
        ];
        let instruction = ControlFlowInstruction::IfElse {
            condition,
            if_branch,
            else_branch,
        };

        // Act
        let result = execute_control_flow(&instruction);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_for_loop_instruction() {
        let initializer = Instruction::Initialize; // Replace with actual instruction
        let condition = Condition {
            left: Operand::Integer(0),
            operator: Operator::LessThan,
            right: Operand::Integer(10),
        };
        let updater = Instruction::Increment; // Replace with actual instruction
        let body = vec![
            Instruction::SomeInstruction, // Replace with actual instructions
        ];
        let instruction = ControlFlowInstruction::ForLoop {
            initializer,
            condition,
            updater,
            body,
        };

        let result = execute_control_flow(&instruction);
        assert!(result.is_ok());
    }

    #[test]
    fn test_while_loop_instruction() {
        let condition = Condition {
            left: Operand::Integer(5),
            operator: Operator::LessThanOrEqual,
            right: Operand::Integer(15),
        };
        let body = vec![Instruction::SomeInstruction];
        let instruction = ControlFlowInstruction::WhileLoop { condition, body };

        let result = execute_control_flow(&instruction);
        assert!(result.is_ok());
    }

    #[test]
    fn test_do_while_loop_instruction() {
        let condition = Condition {
            left: Operand::Integer(10),
            operator: Operator::LessThan,
            right: Operand::Integer(20),
        };
        let body = vec![Instruction::SomeInstruction];
        let instruction = ControlFlowInstruction::DoWhileLoop { body, condition };

        let result = execute_control_flow(&instruction);
        assert!(result.is_ok());
    }

    #[test]
    fn test_jump_instruction() {
        let target = 42;
        let instruction = ControlFlowInstruction::Jump { target };

        let result = execute_control_flow(&instruction);
        assert!(result.is_ok());
    }
}
