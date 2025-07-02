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

//! Comprehensive error handling for WASM module operations

use dotvm_core::bytecode::VmArchitecture;
use thiserror::Error;

/// Errors that can occur during WASM operations
#[derive(Error, Debug)]
pub enum WasmError {
    // Parsing errors
    #[error("Invalid WASM binary: {0}")]
    InvalidBinary(String),

    #[error("Unsupported WASM version: {version} (expected 1)")]
    UnsupportedVersion { version: u32 },

    #[error("Malformed section {section}: {details}")]
    MalformedSection { section: String, details: String },

    #[error("Parser error: {0}")]
    ParserError(#[from] wasmparser::BinaryReaderError),

    #[error("String conversion error: {0}")]
    StringConversionError(#[from] std::str::Utf8Error),

    // Type system errors
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    #[error("Invalid type index: {index}")]
    InvalidTypeIndex { index: u32 },

    #[error("Invalid function index: {index}")]
    InvalidFunctionIndex { index: u32 },

    #[error("Invalid global index: {index}")]
    InvalidGlobalIndex { index: u32 },

    #[error("Invalid memory index: {index}")]
    InvalidMemoryIndex { index: u32 },

    #[error("Invalid table index: {index}")]
    InvalidTableIndex { index: u32 },

    // Feature support errors
    #[error("Unsupported WASM feature: {feature}")]
    UnsupportedFeature { feature: String },

    #[error("Feature {feature} requires {requirement} but it is not enabled")]
    FeatureRequirementNotMet { feature: String, requirement: String },

    #[error("WASM proposal {proposal} is not supported")]
    UnsupportedProposal { proposal: String },

    // Validation errors
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Invalid function body: {details}")]
    InvalidFunctionBody { details: String },

    #[error("Invalid instruction at position {position}: {details}")]
    InvalidInstruction { position: u32, details: String },

    #[error("Stack underflow in function {function} at instruction {instruction}")]
    StackUnderflow { function: u32, instruction: u32 },

    #[error("Stack overflow in function {function} at instruction {instruction}")]
    StackOverflow { function: u32, instruction: u32 },

    // Opcode mapping errors
    #[error("Unsupported WASM instruction for architecture {arch:?}: {instruction}")]
    UnsupportedInstruction { instruction: String, arch: VmArchitecture },

    #[error("Invalid operand for instruction {instruction}: {reason}")]
    InvalidOperand { instruction: String, reason: String },

    #[error("Architecture {arch:?} cannot handle instruction {instruction}")]
    IncompatibleArchitecture { instruction: String, arch: VmArchitecture },

    #[error("Mapping failed for instruction {instruction}: {details}")]
    MappingFailed { instruction: String, details: String },

    // Module structure errors
    #[error("Invalid module structure: {0}")]
    InvalidModuleStructure(String),

    #[error("Missing required section: {section}")]
    MissingRequiredSection { section: String },

    #[error("Duplicate section: {section}")]
    DuplicateSection { section: String },

    #[error("Section {section} appears in wrong order")]
    InvalidSectionOrder { section: String },

    // Memory and limits errors
    #[error("Memory limit exceeded: {current} > {limit}")]
    MemoryLimitExceeded { current: u64, limit: u64 },

    #[error("Invalid memory access: offset {offset} + size {size} exceeds bounds")]
    InvalidMemoryAccess { offset: u64, size: u32 },

    #[error("Table limit exceeded: {current} > {limit}")]
    TableLimitExceeded { current: u32, limit: u32 },

    // Import/Export errors
    #[error("Import resolution failed: {module}::{name} - {reason}")]
    ImportResolutionFailed { module: String, name: String, reason: String },

    #[error("Export conflict: {name} is exported multiple times")]
    ExportConflict { name: String },

    #[error("Invalid export: {name} references non-existent {kind} at index {index}")]
    InvalidExport { name: String, kind: String, index: u32 },

    // Custom section errors
    #[error("Invalid custom section {name}: {details}")]
    InvalidCustomSection { name: String, details: String },

    #[error("Name section parsing failed: {0}")]
    NameSectionError(String),

    // Internal errors
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Not implemented: {feature}")]
    NotImplemented { feature: String },
}

impl WasmError {
    /// Create an unsupported feature error
    pub fn unsupported_feature(feature: impl Into<String>) -> Self {
        Self::UnsupportedFeature { feature: feature.into() }
    }

    /// Create a validation error
    pub fn validation_failed(message: impl Into<String>) -> Self {
        Self::ValidationFailed(message.into())
    }

    /// Create an invalid binary error
    pub fn invalid_binary(message: impl Into<String>) -> Self {
        Self::InvalidBinary(message.into())
    }

    /// Create a malformed section error
    pub fn malformed_section(section: impl Into<String>, details: impl Into<String>) -> Self {
        Self::MalformedSection {
            section: section.into(),
            details: details.into(),
        }
    }

    /// Create an internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError(message.into())
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::InvalidBinary(_) => false,
            Self::UnsupportedVersion { .. } => false,
            Self::ParserError(_) => false,
            Self::InvalidModuleStructure(_) => false,
            Self::InternalError(_) => false,
            _ => true,
        }
    }

    /// Get the error category
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::InvalidBinary(_) | Self::UnsupportedVersion { .. } | Self::ParserError(_) | Self::StringConversionError(_) => ErrorCategory::Parsing,
            Self::TypeMismatch { .. }
            | Self::InvalidTypeIndex { .. }
            | Self::InvalidFunctionIndex { .. }
            | Self::InvalidGlobalIndex { .. }
            | Self::InvalidMemoryIndex { .. }
            | Self::InvalidTableIndex { .. } => ErrorCategory::Type,
            Self::UnsupportedFeature { .. } | Self::FeatureRequirementNotMet { .. } | Self::UnsupportedProposal { .. } => ErrorCategory::Feature,
            Self::ValidationFailed(_) | Self::InvalidFunctionBody { .. } | Self::InvalidInstruction { .. } | Self::StackUnderflow { .. } | Self::StackOverflow { .. } => ErrorCategory::Validation,
            Self::UnsupportedInstruction { .. } | Self::InvalidOperand { .. } | Self::IncompatibleArchitecture { .. } | Self::MappingFailed { .. } => ErrorCategory::Mapping,
            Self::InvalidModuleStructure(_) | Self::MissingRequiredSection { .. } | Self::DuplicateSection { .. } | Self::InvalidSectionOrder { .. } => ErrorCategory::Module,
            Self::MemoryLimitExceeded { .. } | Self::InvalidMemoryAccess { .. } | Self::TableLimitExceeded { .. } => ErrorCategory::Limits,
            Self::ImportResolutionFailed { .. } | Self::ExportConflict { .. } | Self::InvalidExport { .. } => ErrorCategory::ImportExport,
            Self::InvalidCustomSection { .. } | Self::NameSectionError(_) => ErrorCategory::CustomSection,
            Self::InternalError(_) | Self::NotImplemented { .. } => ErrorCategory::Internal,
            Self::MalformedSection { .. } => ErrorCategory::Parsing,
        }
    }

    /// Get a user-friendly error message
    pub fn user_message(&self) -> String {
        match self.category() {
            ErrorCategory::Parsing => "The WASM binary file is invalid or corrupted. Please check that it's a valid WebAssembly module.".to_string(),
            ErrorCategory::Type => "There's a type system error in the WASM module. This usually indicates a malformed or invalid module.".to_string(),
            ErrorCategory::Feature => "The WASM module uses features that are not supported or enabled.".to_string(),
            ErrorCategory::Validation => "The WASM module failed validation. The module structure or instructions are invalid.".to_string(),
            ErrorCategory::Mapping => "Failed to convert WASM instructions to DotVM bytecode. This may be due to architecture limitations.".to_string(),
            ErrorCategory::Module => "The WASM module structure is invalid or has missing/duplicate sections.".to_string(),
            ErrorCategory::Limits => "The WASM module exceeds memory or table limits.".to_string(),
            ErrorCategory::ImportExport => "There's a problem with the module's imports or exports.".to_string(),
            ErrorCategory::CustomSection => "A custom section in the WASM module is invalid.".to_string(),
            ErrorCategory::Internal => "An internal error occurred. This is likely a bug in the WASM processor.".to_string(),
        }
    }
}

/// Error categories for better error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Parsing,
    Type,
    Feature,
    Validation,
    Mapping,
    Module,
    Limits,
    ImportExport,
    CustomSection,
    Internal,
}

impl ErrorCategory {
    /// Get the category name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Parsing => "parsing",
            Self::Type => "type",
            Self::Feature => "feature",
            Self::Validation => "validation",
            Self::Mapping => "mapping",
            Self::Module => "module",
            Self::Limits => "limits",
            Self::ImportExport => "import_export",
            Self::CustomSection => "custom_section",
            Self::Internal => "internal",
        }
    }
}

/// Result type alias for WASM operations
pub type WasmResult<T> = Result<T, WasmError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categories() {
        let error = WasmError::invalid_binary("test");
        assert_eq!(error.category(), ErrorCategory::Parsing);
        assert!(!error.is_recoverable());

        let error = WasmError::validation_failed("test");
        assert_eq!(error.category(), ErrorCategory::Validation);
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_error_construction() {
        let error = WasmError::malformed_section("code", "invalid instruction");
        match error {
            WasmError::MalformedSection { section, details } => {
                assert_eq!(section, "code");
                assert_eq!(details, "invalid instruction");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_user_messages() {
        let error = WasmError::internal_error("test");
        let message = error.user_message();
        assert!(message.contains("internal error"));
    }
}
