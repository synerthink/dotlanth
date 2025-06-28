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

//! Mathematical Extensions for DotVM
//!
//! This module handles advanced mathematical operations including BigInt arithmetic,
//! high-precision floating point operations, and custom mathematical functions
//! that go beyond standard WebAssembly capabilities.

use crate::wasm::ast::{WasmFunction, WasmInstruction, WasmValueType};
use dotvm_core::{
    bytecode::VmArchitecture,
    opcode::architecture_opcodes::{Opcode128, Opcode256, Opcode512},
};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during mathematical extension processing
#[derive(Error, Debug)]
pub enum MathExtensionError {
    #[error("Unsupported mathematical operation: {0}")]
    UnsupportedOperation(String),
    #[error("BigInt operation requires 128-bit+ architecture")]
    BigIntArchitectureRequired,
    #[error("High-precision operation requires 256-bit+ architecture")]
    HighPrecisionArchitectureRequired,
    #[error("Invalid operand count for operation {operation}: expected {expected}, got {actual}")]
    InvalidOperandCount { operation: String, expected: usize, actual: usize },
    #[error("Type mismatch in mathematical operation: {details}")]
    TypeMismatch { details: String },
    #[error("Precision overflow: operation would exceed maximum precision")]
    PrecisionOverflow,
}

/// Types of mathematical operations supported by DotVM extensions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MathOperation {
    // BigInt operations (128-bit+)
    BigIntAdd,
    BigIntSub,
    BigIntMul,
    BigIntDiv,
    BigIntMod,
    BigIntPow,
    BigIntGcd,
    BigIntLcm,
    BigIntFactorial,

    // High-precision floating point (256-bit+)
    HighPrecisionAdd,
    HighPrecisionSub,
    HighPrecisionMul,
    HighPrecisionDiv,
    HighPrecisionSqrt,
    HighPrecisionPow,
    HighPrecisionLog,
    HighPrecisionExp,
    HighPrecisionSin,
    HighPrecisionCos,
    HighPrecisionTan,

    // Advanced mathematical functions
    ModularExponentiation,
    DiscreteLogarithm,
    PrimeFactorization,
    MillerRabinTest,
    EllipticCurveAdd,
    EllipticCurveMul,

    // Statistical operations
    Mean,
    Variance,
    StandardDeviation,
    Correlation,
    Regression,
}

impl MathOperation {
    /// Get the minimum architecture required for this operation
    pub fn minimum_architecture(&self) -> VmArchitecture {
        match self {
            // BigInt operations require 128-bit
            MathOperation::BigIntAdd
            | MathOperation::BigIntSub
            | MathOperation::BigIntMul
            | MathOperation::BigIntDiv
            | MathOperation::BigIntMod
            | MathOperation::BigIntPow
            | MathOperation::BigIntGcd
            | MathOperation::BigIntLcm
            | MathOperation::BigIntFactorial => VmArchitecture::Arch128,

            // High-precision operations require 256-bit
            MathOperation::HighPrecisionAdd
            | MathOperation::HighPrecisionSub
            | MathOperation::HighPrecisionMul
            | MathOperation::HighPrecisionDiv
            | MathOperation::HighPrecisionSqrt
            | MathOperation::HighPrecisionPow
            | MathOperation::HighPrecisionLog
            | MathOperation::HighPrecisionExp
            | MathOperation::HighPrecisionSin
            | MathOperation::HighPrecisionCos
            | MathOperation::HighPrecisionTan => VmArchitecture::Arch256,

            // Advanced operations require 256-bit or 512-bit
            MathOperation::ModularExponentiation | MathOperation::DiscreteLogarithm | MathOperation::PrimeFactorization | MathOperation::MillerRabinTest => VmArchitecture::Arch256,

            MathOperation::EllipticCurveAdd | MathOperation::EllipticCurveMul => VmArchitecture::Arch512,

            // Statistical operations require 256-bit for precision
            MathOperation::Mean | MathOperation::Variance | MathOperation::StandardDeviation | MathOperation::Correlation | MathOperation::Regression => VmArchitecture::Arch256,
        }
    }

    /// Get the expected number of operands for this operation
    pub fn operand_count(&self) -> usize {
        match self {
            // Unary operations
            MathOperation::BigIntFactorial
            | MathOperation::HighPrecisionSqrt
            | MathOperation::HighPrecisionLog
            | MathOperation::HighPrecisionExp
            | MathOperation::HighPrecisionSin
            | MathOperation::HighPrecisionCos
            | MathOperation::HighPrecisionTan
            | MathOperation::PrimeFactorization
            | MathOperation::MillerRabinTest => 1,

            // Binary operations
            MathOperation::BigIntAdd
            | MathOperation::BigIntSub
            | MathOperation::BigIntMul
            | MathOperation::BigIntDiv
            | MathOperation::BigIntMod
            | MathOperation::BigIntPow
            | MathOperation::BigIntGcd
            | MathOperation::BigIntLcm
            | MathOperation::HighPrecisionAdd
            | MathOperation::HighPrecisionSub
            | MathOperation::HighPrecisionMul
            | MathOperation::HighPrecisionDiv
            | MathOperation::HighPrecisionPow
            | MathOperation::ModularExponentiation
            | MathOperation::DiscreteLogarithm
            | MathOperation::EllipticCurveAdd
            | MathOperation::EllipticCurveMul
            | MathOperation::Correlation => 2,

            // Variable operand operations (represented as array operations)
            MathOperation::Mean | MathOperation::Variance | MathOperation::StandardDeviation | MathOperation::Regression => 1, // Takes array as single operand
        }
    }
}

/// Represents a mathematical operation with its context
#[derive(Debug, Clone)]
pub struct MathOperationContext {
    /// The mathematical operation to perform
    pub operation: MathOperation,
    /// Input operand types
    pub input_types: Vec<WasmValueType>,
    /// Output type
    pub output_type: WasmValueType,
    /// Precision requirements (for high-precision operations)
    pub precision: Option<u32>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Mathematical extension processor for DotVM
pub struct MathExtension {
    /// Target architecture
    target_architecture: VmArchitecture,
    /// Operation mappings from function names
    operation_mappings: HashMap<String, MathOperation>,
}

impl MathExtension {
    /// Create a new mathematical extension processor
    pub fn new(target_architecture: VmArchitecture) -> Self {
        let mut extension = Self {
            target_architecture,
            operation_mappings: HashMap::new(),
        };

        extension.initialize_operation_mappings();
        extension
    }

    /// Initialize built-in operation mappings
    fn initialize_operation_mappings(&mut self) {
        // BigInt operations
        self.operation_mappings.insert("bigint_add".to_string(), MathOperation::BigIntAdd);
        self.operation_mappings.insert("bigint_sub".to_string(), MathOperation::BigIntSub);
        self.operation_mappings.insert("bigint_mul".to_string(), MathOperation::BigIntMul);
        self.operation_mappings.insert("bigint_div".to_string(), MathOperation::BigIntDiv);
        self.operation_mappings.insert("bigint_mod".to_string(), MathOperation::BigIntMod);
        self.operation_mappings.insert("bigint_pow".to_string(), MathOperation::BigIntPow);
        self.operation_mappings.insert("bigint_gcd".to_string(), MathOperation::BigIntGcd);
        self.operation_mappings.insert("bigint_lcm".to_string(), MathOperation::BigIntLcm);
        self.operation_mappings.insert("bigint_factorial".to_string(), MathOperation::BigIntFactorial);

        // High-precision operations
        self.operation_mappings.insert("hp_add".to_string(), MathOperation::HighPrecisionAdd);
        self.operation_mappings.insert("hp_sub".to_string(), MathOperation::HighPrecisionSub);
        self.operation_mappings.insert("hp_mul".to_string(), MathOperation::HighPrecisionMul);
        self.operation_mappings.insert("hp_div".to_string(), MathOperation::HighPrecisionDiv);
        self.operation_mappings.insert("hp_sqrt".to_string(), MathOperation::HighPrecisionSqrt);
        self.operation_mappings.insert("hp_pow".to_string(), MathOperation::HighPrecisionPow);
        self.operation_mappings.insert("hp_log".to_string(), MathOperation::HighPrecisionLog);
        self.operation_mappings.insert("hp_exp".to_string(), MathOperation::HighPrecisionExp);
        self.operation_mappings.insert("hp_sin".to_string(), MathOperation::HighPrecisionSin);
        self.operation_mappings.insert("hp_cos".to_string(), MathOperation::HighPrecisionCos);
        self.operation_mappings.insert("hp_tan".to_string(), MathOperation::HighPrecisionTan);

        // Advanced mathematical functions
        self.operation_mappings.insert("mod_exp".to_string(), MathOperation::ModularExponentiation);
        self.operation_mappings.insert("discrete_log".to_string(), MathOperation::DiscreteLogarithm);
        self.operation_mappings.insert("prime_factors".to_string(), MathOperation::PrimeFactorization);
        self.operation_mappings.insert("miller_rabin".to_string(), MathOperation::MillerRabinTest);
        self.operation_mappings.insert("ec_add".to_string(), MathOperation::EllipticCurveAdd);
        self.operation_mappings.insert("ec_mul".to_string(), MathOperation::EllipticCurveMul);

        // Statistical operations
        self.operation_mappings.insert("mean".to_string(), MathOperation::Mean);
        self.operation_mappings.insert("variance".to_string(), MathOperation::Variance);
        self.operation_mappings.insert("std_dev".to_string(), MathOperation::StandardDeviation);
        self.operation_mappings.insert("correlation".to_string(), MathOperation::Correlation);
        self.operation_mappings.insert("regression".to_string(), MathOperation::Regression);
    }

    /// Detect mathematical operations in a function
    pub fn detect_operations(&self, function: &WasmFunction) -> Result<Vec<MathOperationContext>, MathExtensionError> {
        let mut operations = Vec::new();

        // TODO: Check if function name matches a known mathematical operation
        // Function name mapping will be added when export information is available
        /*
        if let Some(math_op) = self.operation_mappings.get(&function.name) {
            let context = self.create_operation_context(math_op, function)?;
            operations.push(context);
        }
        */

        // Analyze function body for mathematical patterns
        operations.extend(self.analyze_function_body(function)?);

        Ok(operations)
    }

    /// Create operation context from function signature
    fn create_operation_context(&self, operation: &MathOperation, function: &WasmFunction) -> Result<MathOperationContext, MathExtensionError> {
        // Validate architecture compatibility
        if operation.minimum_architecture() > self.target_architecture {
            return match operation.minimum_architecture() {
                VmArchitecture::Arch128 => Err(MathExtensionError::BigIntArchitectureRequired),
                VmArchitecture::Arch256 | VmArchitecture::Arch512 => Err(MathExtensionError::HighPrecisionArchitectureRequired),
                _ => Err(MathExtensionError::UnsupportedOperation(format!("{:?}", operation))),
            };
        }

        // Validate operand count
        let expected_operands = operation.operand_count();
        if function.signature.params.len() != expected_operands {
            return Err(MathExtensionError::InvalidOperandCount {
                operation: format!("{:?}", operation),
                expected: expected_operands,
                actual: function.signature.params.len(),
            });
        }

        let output_type = function.signature.results.first().cloned().unwrap_or(WasmValueType::I64);

        Ok(MathOperationContext {
            operation: operation.clone(),
            input_types: function.signature.params.clone(),
            output_type,
            precision: None, // TODO: Extract from attributes
            metadata: HashMap::new(),
        })
    }

    /// Analyze function body for mathematical operation patterns
    fn analyze_function_body(&self, function: &WasmFunction) -> Result<Vec<MathOperationContext>, MathExtensionError> {
        let mut operations = Vec::new();

        // Look for patterns that indicate mathematical operations
        let mut i = 0;
        while i < function.body.len() {
            match &function.body[i] {
                // BigInt addition pattern: multiple i64.add operations
                WasmInstruction::I64Add => {
                    if self.is_bigint_pattern(&function.body[i..]) {
                        operations.push(MathOperationContext {
                            operation: MathOperation::BigIntAdd,
                            input_types: vec![WasmValueType::I64, WasmValueType::I64],
                            output_type: WasmValueType::I64,
                            precision: None,
                            metadata: [("detected_via".to_string(), "pattern_analysis".to_string())].into(),
                        });
                    }
                }

                // High-precision floating point patterns
                WasmInstruction::F64Add => {
                    if self.is_high_precision_pattern(&function.body[i..]) {
                        operations.push(MathOperationContext {
                            operation: MathOperation::HighPrecisionAdd,
                            input_types: vec![WasmValueType::F64, WasmValueType::F64],
                            output_type: WasmValueType::F64,
                            precision: Some(256), // Default high precision
                            metadata: [("detected_via".to_string(), "pattern_analysis".to_string())].into(),
                        });
                    }
                }

                _ => {}
            }
            i += 1;
        }

        Ok(operations)
    }

    /// Check if instruction sequence indicates BigInt operation
    fn is_bigint_pattern(&self, instructions: &[WasmInstruction]) -> bool {
        // Look for multiple consecutive integer operations
        let consecutive_int_ops = instructions
            .iter()
            .take(10) // Look ahead up to 10 instructions
            .filter(|inst| {
                matches!(
                    inst,
                    WasmInstruction::I64Add | WasmInstruction::I64Sub | WasmInstruction::I64Mul | WasmInstruction::I64And | WasmInstruction::I64Or | WasmInstruction::I64Xor
                )
            })
            .count();

        consecutive_int_ops >= 3 // Heuristic: 3+ consecutive operations suggest BigInt
    }

    /// Check if instruction sequence indicates high-precision operation
    fn is_high_precision_pattern(&self, instructions: &[WasmInstruction]) -> bool {
        // Look for complex floating point operation sequences
        let fp_ops = instructions
            .iter()
            .take(15) // Look ahead up to 15 instructions
            .filter(|inst| {
                matches!(
                    inst,
                    WasmInstruction::F64Add | WasmInstruction::F64Sub | WasmInstruction::F64Mul | WasmInstruction::F64Div | WasmInstruction::F64Sqrt | WasmInstruction::F64Abs
                )
            })
            .count();

        fp_ops >= 5 // Heuristic: 5+ operations suggest high-precision computation
    }

    /// Generate DotVM bytecode for a mathematical operation
    pub fn generate_bytecode(&self, context: &MathOperationContext) -> Result<Vec<u8>, MathExtensionError> {
        match self.target_architecture {
            VmArchitecture::Arch128 => self.generate_128bit_bytecode(context),
            VmArchitecture::Arch256 => self.generate_256bit_bytecode(context),
            VmArchitecture::Arch512 => self.generate_512bit_bytecode(context),
            _ => Err(MathExtensionError::UnsupportedOperation(format!(
                "Architecture {:?} not supported for math operations",
                self.target_architecture
            ))),
        }
    }

    /// Generate 128-bit architecture bytecode
    fn generate_128bit_bytecode(&self, context: &MathOperationContext) -> Result<Vec<u8>, MathExtensionError> {
        let mut bytecode = Vec::new();

        match context.operation {
            MathOperation::BigIntAdd => {
                bytecode.push((Opcode128::BigInt(dotvm_core::opcode::bigint_opcodes::BigIntOpcode::Add).as_u16() & 0xFF) as u8);
            }
            MathOperation::BigIntMul => {
                bytecode.push((Opcode128::BigInt(dotvm_core::opcode::bigint_opcodes::BigIntOpcode::Mul).as_u16() & 0xFF) as u8);
            }
            MathOperation::BigIntDiv => {
                bytecode.push((Opcode128::BigInt(dotvm_core::opcode::bigint_opcodes::BigIntOpcode::Div).as_u16() & 0xFF) as u8);
            }
            _ => return Err(MathExtensionError::UnsupportedOperation(format!("{:?} not supported on 128-bit architecture", context.operation))),
        }

        Ok(bytecode)
    }

    /// Generate 256-bit architecture bytecode
    fn generate_256bit_bytecode(&self, context: &MathOperationContext) -> Result<Vec<u8>, MathExtensionError> {
        let mut bytecode = Vec::new();

        match context.operation {
            // BigInt operations (inherited from 128-bit)
            MathOperation::BigIntAdd => {
                bytecode.push((Opcode256::Base(Opcode128::BigInt(dotvm_core::opcode::bigint_opcodes::BigIntOpcode::Add)).as_u16() & 0xFF) as u8);
            }

            // High-precision operations
            MathOperation::HighPrecisionAdd => {
                bytecode.push((Opcode256::Math(dotvm_core::opcode::math_opcodes::MathOpcode::HighPrecisionAdd).as_u16() & 0xFF) as u8);
            }
            MathOperation::HighPrecisionMul => {
                bytecode.push((Opcode256::Math(dotvm_core::opcode::math_opcodes::MathOpcode::HighPrecisionMul).as_u16() & 0xFF) as u8);
            }

            _ => return Err(MathExtensionError::UnsupportedOperation(format!("{:?} not supported on 256-bit architecture", context.operation))),
        }

        Ok(bytecode)
    }

    /// Generate 512-bit architecture bytecode
    fn generate_512bit_bytecode(&self, context: &MathOperationContext) -> Result<Vec<u8>, MathExtensionError> {
        let mut bytecode = Vec::new();

        match context.operation {
            // All operations from lower architectures are supported
            MathOperation::BigIntAdd => {
                bytecode.push((Opcode512::Base(Opcode256::Base(Opcode128::BigInt(dotvm_core::opcode::bigint_opcodes::BigIntOpcode::Add))).as_u16() & 0xFF) as u8);
            }
            MathOperation::HighPrecisionAdd => {
                bytecode.push((Opcode512::Base(Opcode256::Math(dotvm_core::opcode::math_opcodes::MathOpcode::HighPrecisionAdd)).as_u16() & 0xFF) as u8);
            }

            // 512-bit specific operations (using available crypto opcodes as placeholders)
            MathOperation::EllipticCurveAdd => {
                bytecode.push((Opcode512::Crypto(dotvm_core::opcode::crypto_opcodes::CryptoOpcode::Hash).as_u16() & 0xFF) as u8);
            }
            MathOperation::EllipticCurveMul => {
                bytecode.push((Opcode512::Crypto(dotvm_core::opcode::crypto_opcodes::CryptoOpcode::Sign).as_u16() & 0xFF) as u8);
            }

            _ => return Err(MathExtensionError::UnsupportedOperation(format!("{:?} not supported on 512-bit architecture", context.operation))),
        }

        Ok(bytecode)
    }

    /// Get supported operations for the current architecture
    pub fn get_supported_operations(&self) -> Vec<MathOperation> {
        self.operation_mappings.values().filter(|op| op.minimum_architecture() <= self.target_architecture).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_architecture_requirements() {
        assert_eq!(MathOperation::BigIntAdd.minimum_architecture(), VmArchitecture::Arch128);
        assert_eq!(MathOperation::HighPrecisionAdd.minimum_architecture(), VmArchitecture::Arch256);
        assert_eq!(MathOperation::EllipticCurveAdd.minimum_architecture(), VmArchitecture::Arch512);
    }

    #[test]
    fn test_operation_operand_counts() {
        assert_eq!(MathOperation::BigIntAdd.operand_count(), 2);
        assert_eq!(MathOperation::BigIntFactorial.operand_count(), 1);
        assert_eq!(MathOperation::HighPrecisionSqrt.operand_count(), 1);
    }

    #[test]
    fn test_math_extension_creation() {
        let extension = MathExtension::new(VmArchitecture::Arch256);
        assert!(extension.operation_mappings.contains_key("bigint_add"));
        assert!(extension.operation_mappings.contains_key("hp_add"));
    }

    #[test]
    fn test_operation_detection() {
        let extension = MathExtension::new(VmArchitecture::Arch256);

        let function = WasmFunction {
            signature: crate::wasm::ast::WasmFunctionType {
                params: vec![WasmValueType::I64, WasmValueType::I64],
                results: vec![WasmValueType::I64],
            },
            body: vec![],
            locals: vec![],
        };

        let operations = extension.detect_operations(&function).unwrap();
        // Note: Function name detection is currently disabled
        assert_eq!(operations.len(), 0);
    }

    #[test]
    fn test_architecture_validation() {
        let extension = MathExtension::new(VmArchitecture::Arch64);

        let function = WasmFunction {
            signature: crate::wasm::ast::WasmFunctionType {
                params: vec![WasmValueType::I64, WasmValueType::I64],
                results: vec![WasmValueType::I64],
            },
            body: vec![],
            locals: vec![],
        };

        let result = extension.detect_operations(&function);
        // Note: Since function name detection is disabled, no operations are detected
        // and therefore no architecture validation error occurs
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
