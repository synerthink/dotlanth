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

//! Comprehensive error handling for the transpiler module

use crate::wasm::WasmError;
use thiserror::Error;

/// Errors that can occur during transpilation
#[derive(Error, Debug)]
pub enum TranspilationError {
    // Input/Parsing Errors
    #[error("WASM error: {0}")]
    WasmError(#[from] WasmError),

    #[error("Invalid input format: {0}")]
    InvalidInputFormat(String),

    #[error("Malformed WASM module: {0}")]
    MalformedModule(String),

    // Opcode Mapping Errors
    #[error("Opcode mapping error: {0}")]
    MappingError(String),

    #[error("Unsupported WASM instruction: {instruction} at position {position}")]
    UnsupportedInstruction { instruction: String, position: u32 },

    // Control Flow Errors
    #[error("Control flow error: {0}")]
    ControlFlowError(String),

    #[error("Invalid control flow structure at function {function}: {details}")]
    InvalidControlFlow { function: u32, details: String },

    #[error("Unreachable code detected in function {function} at position {position}")]
    UnreachableCode { function: u32, position: u32 },

    // Memory Model Errors
    #[error("Memory model incompatibility: {0}")]
    MemoryModelError(String),

    #[error("Invalid memory access pattern: {details}")]
    InvalidMemoryAccess { details: String },

    #[error("Memory layout conflict: {0}")]
    MemoryLayoutConflict(String),

    // Function Processing Errors
    #[error("Function not found: {0}")]
    FunctionNotFound(u32),

    #[error("Type mismatch in function {function}: {details}")]
    TypeMismatch { function: u32, details: String },

    #[error("Invalid function signature for function {function}: {details}")]
    InvalidFunctionSignature { function: u32, details: String },

    #[error("Function too large: {function} has {size} instructions (max: {max_size})")]
    FunctionTooLarge { function: u32, size: u32, max_size: u32 },

    // Architecture Compatibility Errors
    #[error("Architecture incompatibility: {0}")]
    ArchitectureIncompatibility(String),

    #[error("Target architecture {target} cannot support required features: {features:?}")]
    UnsupportedArchitectureFeatures { target: String, features: Vec<String> },

    #[error("Instruction requires architecture {required} but target is {target}")]
    InstructionArchitectureMismatch { required: String, target: String },

    // Module Processing Errors
    #[error("Invalid module structure: {0}")]
    InvalidModuleStructure(String),

    #[error("Export/import conflict: {name} is both exported and imported")]
    ExportImportConflict { name: String },

    #[error("Missing required import: {name} from module {module}")]
    MissingRequiredImport { name: String, module: String },

    #[error("Invalid export: {name} of type {export_type} references non-existent index {index}")]
    InvalidExport { name: String, export_type: String, index: u32 },

    // Global Variable Errors
    #[error("Global variable error: {0}")]
    GlobalVariableError(String),

    #[error("Invalid global initialization for global {index}: {details}")]
    InvalidGlobalInitialization { index: u32, details: String },

    // Pipeline Processing Errors
    #[error("Preprocessing failed: {stage} - {details}")]
    PreprocessingError { stage: String, details: String },

    #[error("Analysis failed: {analyzer} - {details}")]
    AnalysisError { analyzer: String, details: String },

    #[error("Translation failed: {component} - {details}")]
    TranslationError { component: String, details: String },

    #[error("Postprocessing failed: {stage} - {details}")]
    PostprocessingError { stage: String, details: String },

    // Configuration Errors
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Configuration validation failed: {field} - {details}")]
    ConfigurationValidationError { field: String, details: String },

    // Feature Support Errors
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),

    #[error("Feature {feature} requires {requirement} but it is not available")]
    FeatureRequirementNotMet { feature: String, requirement: String },

    // Internal Errors
    #[error("Internal transpiler error: {0}")]
    InternalError(String),

    #[error("Assertion failed: {condition} - {context}")]
    AssertionFailed { condition: String, context: String },
}

impl TranspilationError {
    /// Create a control flow error with context
    pub fn control_flow_error(message: impl Into<String>) -> Self {
        Self::ControlFlowError(message.into())
    }

    /// Create a memory model error with context
    pub fn memory_model_error(message: impl Into<String>) -> Self {
        Self::MemoryModelError(message.into())
    }

    /// Create an architecture incompatibility error
    pub fn architecture_incompatibility(message: impl Into<String>) -> Self {
        Self::ArchitectureIncompatibility(message.into())
    }

    /// Create an unsupported feature error
    pub fn unsupported_feature(feature: impl Into<String>) -> Self {
        Self::UnsupportedFeature(feature.into())
    }

    /// Create an internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError(message.into())
    }

    /// Create a preprocessing error
    pub fn preprocessing_error(stage: impl Into<String>, details: impl Into<String>) -> Self {
        Self::PreprocessingError {
            stage: stage.into(),
            details: details.into(),
        }
    }

    /// Create an analysis error
    pub fn analysis_error(analyzer: impl Into<String>, details: impl Into<String>) -> Self {
        Self::AnalysisError {
            analyzer: analyzer.into(),
            details: details.into(),
        }
    }

    /// Create a translation error
    pub fn translation_error(component: impl Into<String>, details: impl Into<String>) -> Self {
        Self::TranslationError {
            component: component.into(),
            details: details.into(),
        }
    }

    /// Create a postprocessing error
    pub fn postprocessing_error(stage: impl Into<String>, details: impl Into<String>) -> Self {
        Self::PostprocessingError {
            stage: stage.into(),
            details: details.into(),
        }
    }

    /// Check if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::WasmError(_) => false,
            Self::MalformedModule(_) => false,
            Self::ArchitectureIncompatibility(_) => false,
            Self::UnsupportedArchitectureFeatures { .. } => false,
            Self::InternalError(_) => false,
            Self::AssertionFailed { .. } => false,
            _ => true,
        }
    }

    /// Get the error category
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::WasmError(_) | Self::InvalidInputFormat(_) | Self::MalformedModule(_) => ErrorCategory::Input,
            Self::MappingError(_) | Self::UnsupportedInstruction { .. } => ErrorCategory::Mapping,
            Self::ControlFlowError(_) | Self::InvalidControlFlow { .. } | Self::UnreachableCode { .. } => ErrorCategory::ControlFlow,
            Self::MemoryModelError(_) | Self::InvalidMemoryAccess { .. } | Self::MemoryLayoutConflict(_) => ErrorCategory::Memory,
            Self::FunctionNotFound(_) | Self::TypeMismatch { .. } | Self::InvalidFunctionSignature { .. } | Self::FunctionTooLarge { .. } => ErrorCategory::Function,
            Self::ArchitectureIncompatibility(_) | Self::UnsupportedArchitectureFeatures { .. } | Self::InstructionArchitectureMismatch { .. } => ErrorCategory::Architecture,
            Self::InvalidModuleStructure(_) | Self::ExportImportConflict { .. } | Self::MissingRequiredImport { .. } | Self::InvalidExport { .. } => ErrorCategory::Module,
            Self::GlobalVariableError(_) | Self::InvalidGlobalInitialization { .. } => ErrorCategory::Global,
            Self::PreprocessingError { .. } | Self::AnalysisError { .. } | Self::TranslationError { .. } | Self::PostprocessingError { .. } => ErrorCategory::Pipeline,
            Self::InvalidConfiguration(_) | Self::ConfigurationValidationError { .. } => ErrorCategory::Configuration,
            Self::UnsupportedFeature(_) | Self::FeatureRequirementNotMet { .. } => ErrorCategory::Feature,
            Self::InternalError(_) | Self::AssertionFailed { .. } => ErrorCategory::Internal,
        }
    }

    /// Get a user-friendly error message
    pub fn user_message(&self) -> String {
        match self.category() {
            ErrorCategory::Input => "There was a problem with the input WASM file. Please check that it is a valid WebAssembly module.".to_string(),
            ErrorCategory::Mapping => "Failed to convert WASM instructions to DotVM bytecode. This may be due to unsupported WASM features.".to_string(),
            ErrorCategory::ControlFlow => "The control flow structure of the WASM module is invalid or unsupported.".to_string(),
            ErrorCategory::Memory => "There was a problem with memory layout or access patterns in the WASM module.".to_string(),
            ErrorCategory::Function => "A function in the WASM module has invalid structure or exceeds size limits.".to_string(),
            ErrorCategory::Architecture => "The WASM module requires features not supported by the target architecture.".to_string(),
            ErrorCategory::Module => "The WASM module structure is invalid or has conflicting exports/imports.".to_string(),
            ErrorCategory::Global => "There was a problem with global variable definitions in the WASM module.".to_string(),
            ErrorCategory::Pipeline => "An error occurred during the transpilation pipeline processing.".to_string(),
            ErrorCategory::Configuration => "The transpilation configuration is invalid or incomplete.".to_string(),
            ErrorCategory::Feature => "The WASM module uses features that are not supported by this transpiler.".to_string(),
            ErrorCategory::Internal => "An internal error occurred in the transpiler. This is likely a bug.".to_string(),
        }
    }
}

/// Error categories for better error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Input,
    Mapping,
    ControlFlow,
    Memory,
    Function,
    Architecture,
    Module,
    Global,
    Pipeline,
    Configuration,
    Feature,
    Internal,
}

impl ErrorCategory {
    /// Get the category name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Mapping => "mapping",
            Self::ControlFlow => "control_flow",
            Self::Memory => "memory",
            Self::Function => "function",
            Self::Architecture => "architecture",
            Self::Module => "module",
            Self::Global => "global",
            Self::Pipeline => "pipeline",
            Self::Configuration => "configuration",
            Self::Feature => "feature",
            Self::Internal => "internal",
        }
    }
}

/// Result type alias for transpilation operations
pub type TranspilationResult<T> = Result<T, TranspilationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categories() {
        let error = TranspilationError::WasmError(WasmError::InvalidBinary("test".to_string()));
        assert_eq!(error.category(), ErrorCategory::Input);
        assert!(!error.is_recoverable());

        let error = TranspilationError::control_flow_error("test");
        assert_eq!(error.category(), ErrorCategory::ControlFlow);
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_error_construction() {
        let error = TranspilationError::preprocessing_error("validation", "invalid input");
        match error {
            TranspilationError::PreprocessingError { stage, details } => {
                assert_eq!(stage, "validation");
                assert_eq!(details, "invalid input");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_user_messages() {
        let error = TranspilationError::internal_error("test");
        let message = error.user_message();
        assert!(message.contains("internal error"));
    }
}
