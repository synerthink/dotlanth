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

//! Error types for bytecode generation

use thiserror::Error;

/// Errors that can occur during bytecode generation
#[derive(Error, Debug)]
pub enum BytecodeGenerationError {
    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid instruction at offset {offset}: {reason}")]
    InvalidInstruction { offset: u32, reason: String },

    #[error("Label resolution failed: {0}")]
    LabelResolutionFailed(String),

    #[error("Function index out of bounds: {0}")]
    FunctionIndexOutOfBounds(u32),

    #[error("Memory layout error: {0}")]
    MemoryLayoutError(String),

    #[error("Export resolution error: {0}")]
    ExportResolutionError(String),

    #[error("Import resolution error: {0}")]
    ImportResolutionError(String),

    #[error("Optimization error: {0}")]
    OptimizationError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Bytecode size limit exceeded: {actual} > {limit}")]
    BytecodeSizeLimitExceeded { actual: usize, limit: usize },
}

/// Result type for bytecode generation operations
pub type BytecodeResult<T> = Result<T, BytecodeGenerationError>;
