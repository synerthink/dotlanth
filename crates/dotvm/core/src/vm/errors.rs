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

use std::fmt;

/// Enum representing possible VM errors.
#[derive(Debug)]
pub enum VMError {
    StackUnderflow,
    DivisionByZero,
    UnknownOpcode,
    InvalidJumpTarget(usize),
    InvalidInstructionArguments,
    MissingInstructionArguments,
    // Add more error variants as needed
}

impl fmt::Display for VMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VMError::StackUnderflow => write!(f, "Stack underflow occurred."),
            VMError::DivisionByZero => write!(f, "Attempted division by zero."),
            VMError::UnknownOpcode => write!(f, "Encountered unknown opcode."),
            VMError::InvalidJumpTarget(target) => {
                write!(f, "Invalid jump target: {}.", target)
            }
            VMError::InvalidInstructionArguments => {
                write!(f, "Invalid instruction arguments provided.")
            }
            VMError::MissingInstructionArguments => {
                write!(f, "Missing instruction arguments.")
            }
        }
    }
}

impl std::error::Error for VMError {}
