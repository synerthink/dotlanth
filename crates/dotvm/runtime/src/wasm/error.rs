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

//! WASM Runtime Error Types

use thiserror::Error;

/// Result type for WASM operations
pub type WasmResult<T> = Result<T, WasmError>;

/// WASM Runtime Error Types
#[derive(Error, Debug, Clone)]
pub enum WasmError {
    #[error("Module validation failed: {message}")]
    ValidationError { message: String },

    #[error("Module loading failed: {message}")]
    LoadingError { message: String },

    #[error("Instance creation failed: {message}")]
    InstantiationError { message: String },

    #[error("Execution failed: {message}")]
    ExecutionError { message: String },

    #[error("Memory access violation: {message}")]
    MemoryError { message: String },

    #[error("Stack overflow: current depth {current}, max depth {max}")]
    StackOverflow { current: usize, max: usize },

    #[error("Stack underflow: attempted to pop from empty stack")]
    StackUnderflow,

    #[error("Resource limit exceeded: {resource} - current: {current}, limit: {limit}")]
    ResourceLimitExceeded { resource: String, current: u64, limit: u64 },

    #[error("Security policy violation: {message}")]
    SecurityViolation { message: String },

    #[error("Function not found: {name}")]
    FunctionNotFound { name: String },

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Invalid instruction: {instruction} at offset {offset}")]
    InvalidInstruction { instruction: String, offset: usize },

    #[error("Trap occurred: {reason}")]
    Trap { reason: String },

    #[error("Timeout: execution exceeded {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Import resolution failed: module {module}, name {name}")]
    ImportResolutionError { module: String, name: String },

    #[error("Export not found: {name}")]
    ExportNotFound { name: String },

    #[error("Invalid module format: {message}")]
    InvalidFormat { message: String },

    #[error("Runtime internal error: {message}")]
    InternalError { message: String },
}

impl WasmError {
    /// Create a validation error
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::ValidationError { message: message.into() }
    }

    /// Create a loading error
    pub fn loading_error(message: impl Into<String>) -> Self {
        Self::LoadingError { message: message.into() }
    }

    /// Create an instantiation error
    pub fn instantiation_error(message: impl Into<String>) -> Self {
        Self::InstantiationError { message: message.into() }
    }

    /// Create an execution error
    pub fn execution_error(message: impl Into<String>) -> Self {
        Self::ExecutionError { message: message.into() }
    }

    /// Create a memory error
    pub fn memory_error(message: impl Into<String>) -> Self {
        Self::MemoryError { message: message.into() }
    }

    /// Create a security violation error
    pub fn security_violation(message: impl Into<String>) -> Self {
        Self::SecurityViolation { message: message.into() }
    }

    /// Create a function not found error
    pub fn function_not_found(name: impl Into<String>) -> Self {
        Self::FunctionNotFound { name: name.into() }
    }

    /// Create a type mismatch error
    pub fn type_mismatch(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::TypeMismatch {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create a trap error
    pub fn trap(reason: impl Into<String>) -> Self {
        Self::Trap { reason: reason.into() }
    }

    /// Create an internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError { message: message.into() }
    }

    /// Create resource limit exceeded error
    pub fn resource_limit_exceeded(resource: String, current: u64, limit: u64) -> Self {
        Self::ResourceLimitExceeded { resource, current, limit }
    }

    /// Create stack overflow error
    pub fn stack_overflow(current: usize, max: usize) -> Self {
        Self::StackOverflow { current, max }
    }

    /// Create timeout error
    pub fn timeout(timeout_ms: u64) -> Self {
        Self::Timeout { timeout_ms }
    }

    /// Create invalid instruction error
    pub fn invalid_instruction(instruction: String, offset: usize) -> Self {
        Self::InvalidInstruction { instruction, offset }
    }

    /// Create invalid format error
    pub fn invalid_format(message: impl Into<String>) -> Self {
        Self::InvalidFormat { message: message.into() }
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(self, Self::Timeout { .. } | Self::ResourceLimitExceeded { .. } | Self::StackOverflow { .. })
    }

    /// Check if this error indicates a security issue
    pub fn is_security_related(&self) -> bool {
        matches!(self, Self::SecurityViolation { .. } | Self::MemoryError { .. } | Self::ResourceLimitExceeded { .. })
    }

    /// Get error category for monitoring
    pub fn category(&self) -> &'static str {
        match self {
            Self::ValidationError { .. } => "validation",
            Self::LoadingError { .. } => "loading",
            Self::InstantiationError { .. } => "instantiation",
            Self::ExecutionError { .. } => "execution",
            Self::MemoryError { .. } => "memory",
            Self::StackOverflow { .. } => "stack_overflow",
            Self::StackUnderflow => "stack_underflow",
            Self::ResourceLimitExceeded { .. } => "resource_limit_exceeded",
            Self::SecurityViolation { .. } => "security",
            Self::FunctionNotFound { .. } => "function",
            Self::TypeMismatch { .. } => "type",
            Self::InvalidInstruction { .. } => "instruction",
            Self::Trap { .. } => "trap",
            Self::Timeout { .. } => "timeout",
            Self::ImportResolutionError { .. } => "import",
            Self::ExportNotFound { .. } => "export",
            Self::InvalidFormat { .. } => "format",
            Self::InternalError { .. } => "internal",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = WasmError::validation_error("test message");
        assert!(matches!(err, WasmError::ValidationError { .. }));
        assert_eq!(err.category(), "validation");
        assert!(!err.is_recoverable());
        assert!(!err.is_security_related());
    }

    #[test]
    fn test_security_error() {
        let err = WasmError::security_violation("unauthorized access");
        assert!(err.is_security_related());
        assert_eq!(err.category(), "security");
    }

    #[test]
    fn test_recoverable_error() {
        let err = WasmError::Timeout { timeout_ms: 1000 };
        assert!(err.is_recoverable());
        assert_eq!(err.category(), "timeout");
    }

    #[test]
    fn test_resource_limit_error() {
        let err = WasmError::ResourceLimitExceeded {
            resource: "memory".to_string(),
            current: 1000,
            limit: 500,
        };
        assert!(err.is_recoverable());
        assert!(err.is_security_related());
        assert_eq!(err.category(), "resource_limit_exceeded");
    }

    #[test]
    fn test_error_display() {
        let err = WasmError::function_not_found("main");
        assert_eq!(err.to_string(), "Function not found: main");
    }

    #[test]
    fn test_type_mismatch_error() {
        let err = WasmError::type_mismatch("i32", "f64");
        assert_eq!(err.to_string(), "Type mismatch: expected i32, got f64");
        assert_eq!(err.category(), "type");
    }
}
