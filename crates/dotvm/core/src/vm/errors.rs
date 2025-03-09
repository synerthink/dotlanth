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
    MemoryManagerUnavailable,
    PointerOverflow,
    MemoryOperationError(String),
    // Add more error variants as needed
}
impl From<crate::memory::error::MemoryError> for VMError {
    fn from(err: crate::memory::error::MemoryError) -> Self {
        VMError::MemoryOperationError(err.to_string())
    }
}
