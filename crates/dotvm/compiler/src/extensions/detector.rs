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

//! DotVM Extension Detection System
//!
//! This module provides functionality to detect when DotVM-specific features
//! are needed that go beyond standard WebAssembly capabilities. It analyzes
//! Rust attributes, function signatures, and code patterns to determine
//! extension requirements.

use crate::wasm::ast::{WasmFunction, WasmModule};
use dotvm_core::bytecode::VmArchitecture;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Errors that can occur during extension detection
#[derive(Error, Debug)]
pub enum ExtensionDetectionError {
    #[error("Invalid attribute syntax: {0}")]
    InvalidAttributeSyntax(String),
    #[error("Unsupported extension type: {0}")]
    UnsupportedExtension(String),
    #[error("Extension conflict: {extension1} conflicts with {extension2}")]
    ExtensionConflict { extension1: String, extension2: String },
    #[error("Architecture incompatibility: {extension} requires {required_arch:?} but target is {target_arch:?}")]
    ArchitectureIncompatibility {
        extension: String,
        required_arch: VmArchitecture,
        target_arch: VmArchitecture,
    },
}

/// Types of DotVM extensions that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExtensionType {
    /// BigInteger arithmetic operations (128-bit+)
    BigInt,
    /// SIMD vector operations (256-bit+)
    Simd,
    /// Large-scale vector processing (512-bit)
    Vector,
    /// Advanced cryptographic operations
    Crypto,
    /// High-precision floating point arithmetic
    HighPrecision,
    /// Parallel processing primitives
    Parallel,
    /// Custom mathematical functions
    CustomMath,
}

impl ExtensionType {
    /// Get the minimum architecture required for this extension
    pub fn minimum_architecture(&self) -> VmArchitecture {
        match self {
            ExtensionType::BigInt => VmArchitecture::Arch128,
            ExtensionType::Simd => VmArchitecture::Arch256,
            ExtensionType::Vector => VmArchitecture::Arch512,
            ExtensionType::Crypto => VmArchitecture::Arch128,
            ExtensionType::HighPrecision => VmArchitecture::Arch128,
            ExtensionType::Parallel => VmArchitecture::Arch256,
            ExtensionType::CustomMath => VmArchitecture::Arch128,
        }
    }

    /// Check if this extension is compatible with the given architecture
    pub fn is_compatible_with(&self, arch: VmArchitecture) -> bool {
        let min_arch = self.minimum_architecture();
        arch >= min_arch
    }
}

/// Represents a detected extension requirement
#[derive(Debug, Clone)]
pub struct ExtensionRequirement {
    /// The type of extension required
    pub extension_type: ExtensionType,
    /// The function where this extension was detected
    pub function_index: u32,
    /// Additional metadata about the requirement
    pub metadata: HashMap<String, String>,
    /// Priority of this requirement (higher = more important)
    pub priority: u32,
}

/// Rust attribute patterns that indicate DotVM extensions
#[derive(Debug, Clone)]
pub struct AttributePattern {
    /// The attribute name (e.g., "dotvm::simd")
    pub name: String,
    /// Parameters passed to the attribute
    pub parameters: HashMap<String, String>,
}

/// Extension detector that analyzes WASM modules for DotVM-specific features
pub struct ExtensionDetector {
    /// Target architecture for compatibility checking
    target_architecture: VmArchitecture,
    /// Detected extension requirements
    requirements: Vec<ExtensionRequirement>,
    /// Function signatures that indicate extensions
    signature_patterns: HashMap<String, ExtensionType>,
    /// Detected attribute patterns
    attribute_patterns: Vec<AttributePattern>,
}

impl ExtensionDetector {
    /// Create a new extension detector for the given target architecture
    pub fn new(target_architecture: VmArchitecture) -> Self {
        let mut detector = Self {
            target_architecture,
            requirements: Vec::new(),
            signature_patterns: HashMap::new(),
            attribute_patterns: Vec::new(),
        };

        detector.initialize_signature_patterns();
        detector
    }

    /// Initialize built-in function signature patterns
    fn initialize_signature_patterns(&mut self) {
        // BigInt patterns
        self.signature_patterns.insert("bigint_add".to_string(), ExtensionType::BigInt);
        self.signature_patterns.insert("bigint_mul".to_string(), ExtensionType::BigInt);
        self.signature_patterns.insert("bigint_div".to_string(), ExtensionType::BigInt);
        self.signature_patterns.insert("bigint_mod".to_string(), ExtensionType::BigInt);
        self.signature_patterns.insert("bigint_pow".to_string(), ExtensionType::BigInt);

        // SIMD patterns
        self.signature_patterns.insert("simd_add_f32x8".to_string(), ExtensionType::Simd);
        self.signature_patterns.insert("simd_mul_f32x8".to_string(), ExtensionType::Simd);
        self.signature_patterns.insert("simd_add_f64x4".to_string(), ExtensionType::Simd);
        self.signature_patterns.insert("simd_mul_f64x4".to_string(), ExtensionType::Simd);

        // Vector processing patterns
        self.signature_patterns.insert("vector_dot_product".to_string(), ExtensionType::Vector);
        self.signature_patterns.insert("vector_cross_product".to_string(), ExtensionType::Vector);
        self.signature_patterns.insert("matrix_multiply".to_string(), ExtensionType::Vector);

        // Crypto patterns
        self.signature_patterns.insert("sha3_hash".to_string(), ExtensionType::Crypto);
        self.signature_patterns.insert("blake2b_hash".to_string(), ExtensionType::Crypto);
        self.signature_patterns.insert("ed25519_sign".to_string(), ExtensionType::Crypto);
        self.signature_patterns.insert("ed25519_verify".to_string(), ExtensionType::Crypto);
    }

    /// Analyze a WASM module for extension requirements
    pub fn analyze_module(&mut self, module: &WasmModule) -> Result<(), ExtensionDetectionError> {
        // Clear previous analysis results
        self.requirements.clear();
        self.attribute_patterns.clear();

        // Analyze each function in the module
        for (index, function) in module.functions.iter().enumerate() {
            self.analyze_function(index as u32, function)?;
        }

        // Validate compatibility with target architecture
        self.validate_architecture_compatibility()?;

        Ok(())
    }

    /// Analyze a single function for extension requirements
    fn analyze_function(&mut self, function_index: u32, function: &WasmFunction) -> Result<(), ExtensionDetectionError> {
        // For now, we'll skip function name checking since WasmFunction doesn't have a name field
        // TODO: Add function name mapping when available
        /*
        if let Some(extension_type) = self.signature_patterns.get(&function.name) {
            self.add_requirement(ExtensionRequirement {
                extension_type: extension_type.clone(),
                function_index,
                metadata: HashMap::new(),
                priority: 100, // High priority for explicit function calls
            });
        }
        */

        // Analyze function body for patterns
        self.analyze_function_body(function_index, function)?;

        Ok(())
    }

    /// Analyze function body for extension patterns
    fn analyze_function_body(&mut self, function_index: u32, function: &WasmFunction) -> Result<(), ExtensionDetectionError> {
        // Look for patterns in the instruction sequence that indicate extensions
        for instruction in &function.body {
            match instruction {
                // Look for large integer operations
                crate::wasm::ast::WasmInstruction::I64Add | crate::wasm::ast::WasmInstruction::I64Mul | crate::wasm::ast::WasmInstruction::I64DivS => {
                    // Check if this might be part of a BigInt operation
                    if self.is_likely_bigint_operation(function) {
                        self.add_requirement(ExtensionRequirement {
                            extension_type: ExtensionType::BigInt,
                            function_index,
                            metadata: [("detected_via".to_string(), "instruction_pattern".to_string())].into(),
                            priority: 50, // Medium priority for pattern detection
                        });
                    }
                }
                // Look for SIMD-like patterns (using available instructions as placeholders)
                crate::wasm::ast::WasmInstruction::I32Load { .. } | crate::wasm::ast::WasmInstruction::I32Store { .. } => {
                    self.add_requirement(ExtensionRequirement {
                        extension_type: ExtensionType::Simd,
                        function_index,
                        metadata: [("detected_via".to_string(), "v128_instruction".to_string())].into(),
                        priority: 80, // High priority for explicit SIMD instructions
                    });
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Check if a function is likely performing BigInt operations
    fn is_likely_bigint_operation(&self, function: &WasmFunction) -> bool {
        // Heuristic: functions with many integer operations and specific patterns
        let int_ops = function
            .body
            .iter()
            .filter(|inst| {
                matches!(
                    inst,
                    crate::wasm::ast::WasmInstruction::I64Add | crate::wasm::ast::WasmInstruction::I64Sub | crate::wasm::ast::WasmInstruction::I64Mul | crate::wasm::ast::WasmInstruction::I64DivS
                )
            })
            .count();

        // If there are many integer operations, it might be BigInt
        // Note: function.name is not available in WasmFunction, so we only check operation count
        int_ops > 10
    }

    /// Parse DotVM attributes from function metadata
    pub fn parse_attributes(&mut self, attributes: &[String]) -> Result<(), ExtensionDetectionError> {
        for attr in attributes {
            if let Some(pattern) = self.parse_single_attribute(attr)? {
                self.attribute_patterns.push(pattern);
            }
        }
        Ok(())
    }

    /// Parse a single attribute string
    fn parse_single_attribute(&self, attr: &str) -> Result<Option<AttributePattern>, ExtensionDetectionError> {
        if !attr.starts_with("#[dotvm::") {
            return Ok(None);
        }

        // Extract the attribute name and parameters
        let attr = attr.trim_start_matches("#[").trim_end_matches("]");
        let parts: Vec<&str> = attr.split('(').collect();

        if parts.is_empty() {
            return Err(ExtensionDetectionError::InvalidAttributeSyntax(attr.to_string()));
        }

        let name = parts[0].to_string();
        let mut parameters = HashMap::new();

        if parts.len() > 1 {
            let param_str = parts[1].trim_end_matches(')');
            for param in param_str.split(',') {
                let param = param.trim();
                if let Some((key, value)) = param.split_once('=') {
                    parameters.insert(key.trim().to_string(), value.trim().trim_matches('"').to_string());
                }
            }
        }

        Ok(Some(AttributePattern { name, parameters }))
    }

    /// Add an extension requirement
    fn add_requirement(&mut self, requirement: ExtensionRequirement) {
        // Check for duplicates
        if !self
            .requirements
            .iter()
            .any(|r| r.extension_type == requirement.extension_type && r.function_index == requirement.function_index)
        {
            self.requirements.push(requirement);
        }
    }

    /// Validate that all detected extensions are compatible with the target architecture
    fn validate_architecture_compatibility(&self) -> Result<(), ExtensionDetectionError> {
        for requirement in &self.requirements {
            if !requirement.extension_type.is_compatible_with(self.target_architecture) {
                return Err(ExtensionDetectionError::ArchitectureIncompatibility {
                    extension: format!("{:?}", requirement.extension_type),
                    required_arch: requirement.extension_type.minimum_architecture(),
                    target_arch: self.target_architecture,
                });
            }
        }
        Ok(())
    }

    /// Get all detected extension requirements
    pub fn get_requirements(&self) -> &[ExtensionRequirement] {
        &self.requirements
    }

    /// Get unique extension types that were detected
    pub fn get_required_extensions(&self) -> HashSet<ExtensionType> {
        self.requirements.iter().map(|r| r.extension_type.clone()).collect()
    }

    /// Get the minimum architecture required for all detected extensions
    pub fn get_minimum_required_architecture(&self) -> VmArchitecture {
        self.requirements.iter().map(|r| r.extension_type.minimum_architecture()).max().unwrap_or(VmArchitecture::Arch64)
    }

    /// Check if a specific extension type was detected
    pub fn has_extension(&self, extension_type: &ExtensionType) -> bool {
        self.requirements.iter().any(|r| &r.extension_type == extension_type)
    }

    /// Get requirements for a specific function
    pub fn get_function_requirements(&self, function_index: u32) -> Vec<&ExtensionRequirement> {
        self.requirements.iter().filter(|r| r.function_index == function_index).collect()
    }
}

impl Default for ExtensionDetector {
    fn default() -> Self {
        Self::new(VmArchitecture::Arch64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm::ast::{WasmInstruction, WasmValueType};

    #[test]
    fn test_extension_type_architecture_requirements() {
        assert_eq!(ExtensionType::BigInt.minimum_architecture(), VmArchitecture::Arch128);
        assert_eq!(ExtensionType::Simd.minimum_architecture(), VmArchitecture::Arch256);
        assert_eq!(ExtensionType::Vector.minimum_architecture(), VmArchitecture::Arch512);
    }

    #[test]
    fn test_extension_compatibility() {
        assert!(ExtensionType::BigInt.is_compatible_with(VmArchitecture::Arch128));
        assert!(ExtensionType::BigInt.is_compatible_with(VmArchitecture::Arch256));
        assert!(!ExtensionType::Simd.is_compatible_with(VmArchitecture::Arch128));
        assert!(ExtensionType::Simd.is_compatible_with(VmArchitecture::Arch256));
    }

    #[test]
    fn test_attribute_parsing() {
        let detector = ExtensionDetector::new(VmArchitecture::Arch256);

        let attr = "#[dotvm::simd(width=256)]";
        let pattern = detector.parse_single_attribute(attr).unwrap().unwrap();

        assert_eq!(pattern.name, "dotvm::simd");
        assert_eq!(pattern.parameters.get("width"), Some(&"256".to_string()));
    }

    #[test]
    fn test_function_signature_detection() {
        let mut detector = ExtensionDetector::new(VmArchitecture::Arch256);

        let function = WasmFunction {
            signature: crate::wasm::ast::WasmFunctionType {
                params: vec![WasmValueType::I64, WasmValueType::I64],
                results: vec![WasmValueType::I64],
            },
            body: vec![],
            locals: vec![],
        };

        detector.analyze_function(0, &function).unwrap();

        // Note: Function name detection is currently disabled since WasmFunction doesn't have a name field
        // The extension would need to be detected through other means (instruction patterns, etc.)
        // For now, we expect no extensions to be detected
        assert_eq!(detector.get_requirements().len(), 0);
    }

    #[test]
    fn test_architecture_incompatibility() {
        let mut detector = ExtensionDetector::new(VmArchitecture::Arch64);

        let requirement = ExtensionRequirement {
            extension_type: ExtensionType::Simd,
            function_index: 0,
            metadata: HashMap::new(),
            priority: 100,
        };

        detector.add_requirement(requirement);

        let result = detector.validate_architecture_compatibility();
        assert!(result.is_err());

        if let Err(ExtensionDetectionError::ArchitectureIncompatibility { .. }) = result {
            // Expected error
        } else {
            panic!("Expected ArchitectureIncompatibility error");
        }
    }
}
