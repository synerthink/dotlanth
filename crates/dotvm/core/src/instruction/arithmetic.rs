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

use crate::{opcode::arithmetic_opcodes::ArithmeticOpcode, vm::errors::VMError, vm::executor::Executor};

use super::instruction::Instruction;

pub struct ArithmeticInstruction {
    opcode: ArithmeticOpcode,
}

impl ArithmeticInstruction {
    pub fn new(opcode: ArithmeticOpcode) -> Self {
        ArithmeticInstruction { opcode }
    }
}

impl Instruction for ArithmeticInstruction {
    fn execute(&self, executor: &mut Executor) -> Result<(), VMError> {
        // Pop two operands from the stack
        let b = executor.pop_operand()?;
        let a = executor.pop_operand()?;

        // Perform operation based on opcode
        let result = match self.opcode {
            ArithmeticOpcode::Add => a + b,
            ArithmeticOpcode::Subtract => a - b,
            ArithmeticOpcode::Multiply => a * b,
            ArithmeticOpcode::Divide => {
                if b == 0.0 {
                    return Err(VMError::DivisionByZero);
                }
                a / b
            }
            ArithmeticOpcode::Modulus => {
                if b == 0.0 {
                    return Err(VMError::DivisionByZero);
                }
                a % b
            }
        };

        // Push the result back to the stack
        executor.push_operand(result);
        Ok(())
    }
}
