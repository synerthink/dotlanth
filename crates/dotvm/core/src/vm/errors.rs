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
    CryptographicError(String),
    InvalidJumpTarget(usize),
    InvalidInstructionArguments,
    MissingInstructionArguments,
    MemoryManagerUnavailable,
    PointerOverflow,
    MemoryOperationError(String),
    SystemCallError(String),
    ProcessError(String),
    InvalidOperand(String),
    IntegerOverflow,
    ArchitectureMismatch(String), // For when a VmArchitecture label doesn't match a generic Arch type
    ConfigurationError(String),   // For general VM or component configuration issues
                                  // Add more error variants as needed
}

impl fmt::Display for VMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VMError::StackUnderflow => write!(f, "Stack underflow occurred"),
            VMError::DivisionByZero => write!(f, "Division by zero attempted"),
            VMError::UnknownOpcode => write!(f, "Unknown opcode encountered"),
            VMError::CryptographicError(msg) => write!(f, "Cryptographic error: {msg}"),
            VMError::InvalidJumpTarget(target) => write!(f, "Invalid jump target: {target}"),
            VMError::InvalidInstructionArguments => {
                write!(f, "Invalid instruction arguments provided")
            }
            VMError::MissingInstructionArguments => write!(f, "Missing instruction arguments"),
            VMError::MemoryManagerUnavailable => write!(f, "Memory manager is unavailable"),
            VMError::PointerOverflow => write!(f, "Pointer overflow occurred"),
            VMError::MemoryOperationError(msg) => write!(f, "Memory operation error: {msg}"),
            VMError::SystemCallError(msg) => write!(f, "System call error: {msg}"),
            VMError::ProcessError(msg) => write!(f, "Process error: {msg}"),
            VMError::InvalidOperand(msg) => write!(f, "Invalid operand: {msg}"),
            VMError::IntegerOverflow => write!(f, "Integer overflow occurred"),
            VMError::ArchitectureMismatch(msg) => write!(f, "Architecture mismatch: {msg}"),
            VMError::ConfigurationError(msg) => write!(f, "Configuration error: {msg}"),
        }
    }
}

impl std::error::Error for VMError {}

impl From<crate::memory::error::MemoryError> for VMError {
    fn from(err: crate::memory::error::MemoryError) -> Self {
        VMError::MemoryOperationError(err.to_string())
    }
}
