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

//! SIMD Extensions for DotVM
//!
//! This module handles Single Instruction, Multiple Data (SIMD) operations
//! for 256-bit and 512-bit architectures. It provides vectorized operations
//! that can process multiple data elements in parallel.

use crate::wasm::ast::{WasmFunction, WasmInstruction, WasmValueType};
use dotvm_core::{
    bytecode::VmArchitecture,
    opcode::architecture_opcodes::{Opcode256, Opcode512},
};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during SIMD extension processing
#[derive(Error, Debug)]
pub enum SimdExtensionError {
    #[error("SIMD operations require 256-bit+ architecture")]
    SimdArchitectureRequired,
    #[error("Unsupported SIMD operation: {0}")]
    UnsupportedOperation(String),
    #[error("Invalid vector width: {width} (supported: 128, 256, 512)")]
    InvalidVectorWidth { width: u32 },
    #[error("Vector size mismatch: operation requires {expected} elements, got {actual}")]
    VectorSizeMismatch { expected: usize, actual: usize },
    #[error("Incompatible vector types: {type1} and {type2}")]
    IncompatibleVectorTypes { type1: String, type2: String },
    #[error("SIMD operation overflow: result exceeds vector capacity")]
    SimdOverflow,
}

/// SIMD vector data types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimdVectorType {
    // 128-bit vectors (WebAssembly v128)
    I8x16, // 16 x 8-bit integers
    I16x8, // 8 x 16-bit integers
    I32x4, // 4 x 32-bit integers
    I64x2, // 2 x 64-bit integers
    F32x4, // 4 x 32-bit floats
    F64x2, // 2 x 64-bit floats

    // 256-bit vectors (DotVM extension)
    I8x32,  // 32 x 8-bit integers
    I16x16, // 16 x 16-bit integers
    I32x8,  // 8 x 32-bit integers
    I64x4,  // 4 x 64-bit integers
    F32x8,  // 8 x 32-bit floats
    F64x4,  // 4 x 64-bit floats

    // 512-bit vectors (DotVM extension)
    I8x64,  // 64 x 8-bit integers
    I16x32, // 32 x 16-bit integers
    I32x16, // 16 x 32-bit integers
    I64x8,  // 8 x 64-bit integers
    F32x16, // 16 x 32-bit floats
    F64x8,  // 8 x 64-bit floats
}

impl SimdVectorType {
    /// Get the bit width of this vector type
    pub fn bit_width(&self) -> u32 {
        match self {
            SimdVectorType::I8x16 | SimdVectorType::I16x8 | SimdVectorType::I32x4 | SimdVectorType::I64x2 | SimdVectorType::F32x4 | SimdVectorType::F64x2 => 128,

            SimdVectorType::I8x32 | SimdVectorType::I16x16 | SimdVectorType::I32x8 | SimdVectorType::I64x4 | SimdVectorType::F32x8 | SimdVectorType::F64x4 => 256,

            SimdVectorType::I8x64 | SimdVectorType::I16x32 | SimdVectorType::I32x16 | SimdVectorType::I64x8 | SimdVectorType::F32x16 | SimdVectorType::F64x8 => 512,
        }
    }

    /// Get the number of elements in this vector
    pub fn element_count(&self) -> usize {
        match self {
            SimdVectorType::I8x16 => 16,
            SimdVectorType::I16x8 => 8,
            SimdVectorType::I32x4 => 4,
            SimdVectorType::I64x2 => 2,
            SimdVectorType::F32x4 => 4,
            SimdVectorType::F64x2 => 2,

            SimdVectorType::I8x32 => 32,
            SimdVectorType::I16x16 => 16,
            SimdVectorType::I32x8 => 8,
            SimdVectorType::I64x4 => 4,
            SimdVectorType::F32x8 => 8,
            SimdVectorType::F64x4 => 4,

            SimdVectorType::I8x64 => 64,
            SimdVectorType::I16x32 => 32,
            SimdVectorType::I32x16 => 16,
            SimdVectorType::I64x8 => 8,
            SimdVectorType::F32x16 => 16,
            SimdVectorType::F64x8 => 8,
        }
    }

    /// Get the minimum architecture required for this vector type
    pub fn minimum_architecture(&self) -> VmArchitecture {
        match self.bit_width() {
            128 => VmArchitecture::Arch64, // WebAssembly v128 supported on 64-bit
            256 => VmArchitecture::Arch256,
            512 => VmArchitecture::Arch512,
            _ => VmArchitecture::Arch64,
        }
    }

    /// Check if this vector type is compatible with the given architecture
    pub fn is_compatible_with(&self, arch: VmArchitecture) -> bool {
        arch >= self.minimum_architecture()
    }
}

/// SIMD operations supported by DotVM
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimdOperation {
    // Arithmetic operations
    Add,
    Sub,
    Mul,
    Div,

    // Logical operations
    And,
    Or,
    Xor,
    Not,

    // Comparison operations
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,

    // Data movement operations
    Load,
    Store,
    Extract,
    Insert,
    Shuffle,

    // Reduction operations
    Sum,
    Product,
    Min,
    Max,

    // Advanced operations
    DotProduct,
    CrossProduct,
    Normalize,
    Magnitude,

    // Conversion operations
    Convert,
    Broadcast,
    Splat,
}

/// Represents a SIMD operation with its context
#[derive(Debug, Clone)]
pub struct SimdOperationContext {
    /// The SIMD operation to perform
    pub operation: SimdOperation,
    /// Input vector types
    pub input_types: Vec<SimdVectorType>,
    /// Output vector type
    pub output_type: SimdVectorType,
    /// Additional operation parameters
    pub parameters: HashMap<String, String>,
}

/// SIMD extension processor for DotVM
pub struct SimdExtension {
    /// Target architecture
    target_architecture: VmArchitecture,
    /// Operation mappings from function names
    operation_mappings: HashMap<String, (SimdOperation, SimdVectorType)>,
}

impl SimdExtension {
    /// Create a new SIMD extension processor
    pub fn new(target_architecture: VmArchitecture) -> Result<Self, SimdExtensionError> {
        if target_architecture < VmArchitecture::Arch256 {
            return Err(SimdExtensionError::SimdArchitectureRequired);
        }

        let mut extension = Self {
            target_architecture,
            operation_mappings: HashMap::new(),
        };

        extension.initialize_operation_mappings();
        Ok(extension)
    }

    /// Initialize built-in operation mappings
    fn initialize_operation_mappings(&mut self) {
        // 256-bit SIMD operations
        if self.target_architecture >= VmArchitecture::Arch256 {
            // Integer operations
            self.operation_mappings.insert("simd_add_i32x8".to_string(), (SimdOperation::Add, SimdVectorType::I32x8));
            self.operation_mappings.insert("simd_sub_i32x8".to_string(), (SimdOperation::Sub, SimdVectorType::I32x8));
            self.operation_mappings.insert("simd_mul_i32x8".to_string(), (SimdOperation::Mul, SimdVectorType::I32x8));

            // Float operations
            self.operation_mappings.insert("simd_add_f32x8".to_string(), (SimdOperation::Add, SimdVectorType::F32x8));
            self.operation_mappings.insert("simd_sub_f32x8".to_string(), (SimdOperation::Sub, SimdVectorType::F32x8));
            self.operation_mappings.insert("simd_mul_f32x8".to_string(), (SimdOperation::Mul, SimdVectorType::F32x8));
            self.operation_mappings.insert("simd_div_f32x8".to_string(), (SimdOperation::Div, SimdVectorType::F32x8));

            // Double precision
            self.operation_mappings.insert("simd_add_f64x4".to_string(), (SimdOperation::Add, SimdVectorType::F64x4));
            self.operation_mappings.insert("simd_mul_f64x4".to_string(), (SimdOperation::Mul, SimdVectorType::F64x4));

            // Logical operations
            self.operation_mappings.insert("simd_and_i32x8".to_string(), (SimdOperation::And, SimdVectorType::I32x8));
            self.operation_mappings.insert("simd_or_i32x8".to_string(), (SimdOperation::Or, SimdVectorType::I32x8));
            self.operation_mappings.insert("simd_xor_i32x8".to_string(), (SimdOperation::Xor, SimdVectorType::I32x8));
        }

        // 512-bit SIMD operations
        if self.target_architecture >= VmArchitecture::Arch512 {
            // High-performance operations
            self.operation_mappings.insert("simd_add_f32x16".to_string(), (SimdOperation::Add, SimdVectorType::F32x16));
            self.operation_mappings.insert("simd_mul_f32x16".to_string(), (SimdOperation::Mul, SimdVectorType::F32x16));
            self.operation_mappings.insert("simd_add_f64x8".to_string(), (SimdOperation::Add, SimdVectorType::F64x8));
            self.operation_mappings.insert("simd_mul_f64x8".to_string(), (SimdOperation::Mul, SimdVectorType::F64x8));

            // Advanced operations
            self.operation_mappings.insert("simd_dot_f32x16".to_string(), (SimdOperation::DotProduct, SimdVectorType::F32x16));
            self.operation_mappings.insert("simd_sum_f32x16".to_string(), (SimdOperation::Sum, SimdVectorType::F32x16));
        }
    }

    /// Detect SIMD operations in a function
    pub fn detect_operations(&self, function: &WasmFunction) -> Result<Vec<SimdOperationContext>, SimdExtensionError> {
        let mut operations = Vec::new();

        // TODO: Check if function name matches a known SIMD operation
        // Function name mapping will be added when export information is available
        /*
        if let Some((simd_op, vector_type)) = self.operation_mappings.get(&function.name) {
            let context = self.create_operation_context(simd_op, vector_type, function)?;
            operations.push(context);
        }
        */

        // Analyze function body for SIMD patterns
        operations.extend(self.analyze_function_body(function)?);

        Ok(operations)
    }

    /// Create operation context from function signature
    fn create_operation_context(&self, operation: &SimdOperation, vector_type: &SimdVectorType, function: &WasmFunction) -> Result<SimdOperationContext, SimdExtensionError> {
        // Validate architecture compatibility
        if !vector_type.is_compatible_with(self.target_architecture) {
            return Err(SimdExtensionError::SimdArchitectureRequired);
        }

        // Determine input and output types based on operation
        let (input_types, output_type) = self.infer_vector_types(operation, vector_type, function)?;

        Ok(SimdOperationContext {
            operation: operation.clone(),
            input_types,
            output_type,
            parameters: HashMap::new(),
        })
    }

    /// Infer vector types from operation and function signature
    fn infer_vector_types(&self, operation: &SimdOperation, default_type: &SimdVectorType, function: &WasmFunction) -> Result<(Vec<SimdVectorType>, SimdVectorType), SimdExtensionError> {
        match operation {
            // Binary operations: two inputs of same type, one output
            SimdOperation::Add | SimdOperation::Sub | SimdOperation::Mul | SimdOperation::Div | SimdOperation::And | SimdOperation::Or | SimdOperation::Xor => {
                Ok((vec![default_type.clone(), default_type.clone()], default_type.clone()))
            }

            // Unary operations: one input, one output
            SimdOperation::Not | SimdOperation::Normalize | SimdOperation::Magnitude => Ok((vec![default_type.clone()], default_type.clone())),

            // Reduction operations: one vector input, scalar output
            SimdOperation::Sum | SimdOperation::Product | SimdOperation::Min | SimdOperation::Max => {
                let scalar_type = self.vector_to_scalar_type(default_type)?;
                Ok((vec![default_type.clone()], scalar_type))
            }

            // Dot product: two vectors, scalar output
            SimdOperation::DotProduct => {
                let scalar_type = self.vector_to_scalar_type(default_type)?;
                Ok((vec![default_type.clone(), default_type.clone()], scalar_type))
            }

            // Default case
            _ => Ok((vec![default_type.clone()], default_type.clone())),
        }
    }

    /// Convert vector type to corresponding scalar type for reductions
    fn vector_to_scalar_type(&self, vector_type: &SimdVectorType) -> Result<SimdVectorType, SimdExtensionError> {
        match vector_type {
            // For reductions, we typically return a single element of the same base type
            SimdVectorType::F32x8 | SimdVectorType::F32x16 => Ok(SimdVectorType::F32x4), // Use smaller vector as "scalar"
            SimdVectorType::F64x4 | SimdVectorType::F64x8 => Ok(SimdVectorType::F64x2),
            SimdVectorType::I32x8 | SimdVectorType::I32x16 => Ok(SimdVectorType::I32x4),
            _ => Ok(vector_type.clone()), // Fallback
        }
    }

    /// Analyze function body for SIMD operation patterns
    fn analyze_function_body(&self, function: &WasmFunction) -> Result<Vec<SimdOperationContext>, SimdExtensionError> {
        let mut operations = Vec::new();

        // Look for WebAssembly v128 instructions that can be extended
        for instruction in &function.body {
            match instruction {
                WasmInstruction::I32Load { .. } => {
                    operations.push(SimdOperationContext {
                        operation: SimdOperation::Load,
                        input_types: vec![],
                        output_type: self.get_default_vector_type(),
                        parameters: [("detected_via".to_string(), "v128_load".to_string())].into(),
                    });
                }

                WasmInstruction::I32Store { .. } => {
                    operations.push(SimdOperationContext {
                        operation: SimdOperation::Store,
                        input_types: vec![self.get_default_vector_type()],
                        output_type: self.get_default_vector_type(),
                        parameters: [("detected_via".to_string(), "v128_store".to_string())].into(),
                    });
                }

                // Look for patterns that suggest SIMD operations
                WasmInstruction::F32Add | WasmInstruction::F64Add => {
                    if self.is_vectorizable_pattern(&function.body) {
                        operations.push(SimdOperationContext {
                            operation: SimdOperation::Add,
                            input_types: vec![self.get_default_vector_type(), self.get_default_vector_type()],
                            output_type: self.get_default_vector_type(),
                            parameters: [("detected_via".to_string(), "vectorizable_pattern".to_string())].into(),
                        });
                    }
                }

                _ => {}
            }
        }

        Ok(operations)
    }

    /// Get the default vector type for the current architecture
    fn get_default_vector_type(&self) -> SimdVectorType {
        match self.target_architecture {
            VmArchitecture::Arch256 => SimdVectorType::F32x8,
            VmArchitecture::Arch512 => SimdVectorType::F32x16,
            _ => SimdVectorType::F32x4, // Fallback to 128-bit
        }
    }

    /// Check if instruction sequence is vectorizable
    fn is_vectorizable_pattern(&self, instructions: &[WasmInstruction]) -> bool {
        // Look for repetitive arithmetic operations that could benefit from SIMD
        let arithmetic_ops = instructions
            .iter()
            .filter(|inst| {
                matches!(
                    inst,
                    WasmInstruction::F32Add
                        | WasmInstruction::F32Sub
                        | WasmInstruction::F32Mul
                        | WasmInstruction::F32Div
                        | WasmInstruction::F64Add
                        | WasmInstruction::F64Sub
                        | WasmInstruction::F64Mul
                        | WasmInstruction::F64Div
                )
            })
            .count();

        // Heuristic: if there are many arithmetic operations, it might benefit from SIMD
        arithmetic_ops >= 8
    }

    /// Generate DotVM bytecode for a SIMD operation
    pub fn generate_bytecode(&self, context: &SimdOperationContext) -> Result<Vec<u8>, SimdExtensionError> {
        match self.target_architecture {
            VmArchitecture::Arch256 => self.generate_256bit_simd_bytecode(context),
            VmArchitecture::Arch512 => self.generate_512bit_simd_bytecode(context),
            _ => Err(SimdExtensionError::SimdArchitectureRequired),
        }
    }

    /// Generate 256-bit SIMD bytecode
    fn generate_256bit_simd_bytecode(&self, context: &SimdOperationContext) -> Result<Vec<u8>, SimdExtensionError> {
        let mut bytecode = Vec::new();

        // Add vector type information
        bytecode.push(self.encode_vector_type(&context.output_type)?);

        match (&context.operation, &context.output_type) {
            (SimdOperation::Add, SimdVectorType::F32x8) => {
                bytecode.push((Opcode256::Simd(dotvm_core::opcode::simd_opcodes::SimdOpcode::AddF32x8).as_u16() & 0xFF) as u8);
            }
            (SimdOperation::Mul, SimdVectorType::F32x8) => {
                bytecode.push((Opcode256::Simd(dotvm_core::opcode::simd_opcodes::SimdOpcode::MulF32x8).as_u16() & 0xFF) as u8);
            }
            (SimdOperation::Add, SimdVectorType::F64x4) => {
                bytecode.push((Opcode256::Simd(dotvm_core::opcode::simd_opcodes::SimdOpcode::AddF64x4).as_u16() & 0xFF) as u8);
            }
            _ => {
                return Err(SimdExtensionError::UnsupportedOperation(format!(
                    "{:?} with {:?} not supported on 256-bit",
                    context.operation, context.output_type
                )));
            }
        }

        Ok(bytecode)
    }

    /// Generate 512-bit SIMD bytecode
    fn generate_512bit_simd_bytecode(&self, context: &SimdOperationContext) -> Result<Vec<u8>, SimdExtensionError> {
        let mut bytecode = Vec::new();

        // Add vector type information
        bytecode.push(self.encode_vector_type(&context.output_type)?);

        match (&context.operation, &context.output_type) {
            // 512-bit operations
            (SimdOperation::Add, SimdVectorType::F32x16) => {
                bytecode.push((Opcode512::Vector(dotvm_core::opcode::vector_opcodes::VectorOpcode::AddF32x16).as_u16() & 0xFF) as u8);
            }
            (SimdOperation::Mul, SimdVectorType::F32x16) => {
                bytecode.push((Opcode512::Vector(dotvm_core::opcode::vector_opcodes::VectorOpcode::MulF32x16).as_u16() & 0xFF) as u8);
            }
            (SimdOperation::DotProduct, SimdVectorType::F32x16) => {
                bytecode.push((Opcode512::Vector(dotvm_core::opcode::vector_opcodes::VectorOpcode::DotProductF32x16).as_u16() & 0xFF) as u8);
            }

            // Fallback to 256-bit operations if available
            _ => return self.generate_256bit_simd_bytecode(context),
        }

        Ok(bytecode)
    }

    /// Encode vector type as byte
    fn encode_vector_type(&self, vector_type: &SimdVectorType) -> Result<u8, SimdExtensionError> {
        match vector_type {
            // 128-bit vectors
            SimdVectorType::I8x16 => Ok(0x10),
            SimdVectorType::I16x8 => Ok(0x11),
            SimdVectorType::I32x4 => Ok(0x12),
            SimdVectorType::I64x2 => Ok(0x13),
            SimdVectorType::F32x4 => Ok(0x14),
            SimdVectorType::F64x2 => Ok(0x15),

            // 256-bit vectors
            SimdVectorType::I8x32 => Ok(0x20),
            SimdVectorType::I16x16 => Ok(0x21),
            SimdVectorType::I32x8 => Ok(0x22),
            SimdVectorType::I64x4 => Ok(0x23),
            SimdVectorType::F32x8 => Ok(0x24),
            SimdVectorType::F64x4 => Ok(0x25),

            // 512-bit vectors
            SimdVectorType::I8x64 => Ok(0x30),
            SimdVectorType::I16x32 => Ok(0x31),
            SimdVectorType::I32x16 => Ok(0x32),
            SimdVectorType::I64x8 => Ok(0x33),
            SimdVectorType::F32x16 => Ok(0x34),
            SimdVectorType::F64x8 => Ok(0x35),
        }
    }

    /// Get supported vector types for the current architecture
    pub fn get_supported_vector_types(&self) -> Vec<SimdVectorType> {
        let mut types = vec![
            // 128-bit types (always supported)
            SimdVectorType::I8x16,
            SimdVectorType::I16x8,
            SimdVectorType::I32x4,
            SimdVectorType::I64x2,
            SimdVectorType::F32x4,
            SimdVectorType::F64x2,
        ];

        if self.target_architecture >= VmArchitecture::Arch256 {
            types.extend(vec![
                SimdVectorType::I8x32,
                SimdVectorType::I16x16,
                SimdVectorType::I32x8,
                SimdVectorType::I64x4,
                SimdVectorType::F32x8,
                SimdVectorType::F64x4,
            ]);
        }

        if self.target_architecture >= VmArchitecture::Arch512 {
            types.extend(vec![
                SimdVectorType::I8x64,
                SimdVectorType::I16x32,
                SimdVectorType::I32x16,
                SimdVectorType::I64x8,
                SimdVectorType::F32x16,
                SimdVectorType::F64x8,
            ]);
        }

        types
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_type_properties() {
        assert_eq!(SimdVectorType::F32x8.bit_width(), 256);
        assert_eq!(SimdVectorType::F32x8.element_count(), 8);
        assert_eq!(SimdVectorType::F32x16.minimum_architecture(), VmArchitecture::Arch512);
    }

    #[test]
    fn test_vector_type_compatibility() {
        assert!(SimdVectorType::F32x8.is_compatible_with(VmArchitecture::Arch256));
        assert!(!SimdVectorType::F32x16.is_compatible_with(VmArchitecture::Arch256));
        assert!(SimdVectorType::F32x16.is_compatible_with(VmArchitecture::Arch512));
    }

    #[test]
    fn test_simd_extension_creation() {
        let extension = SimdExtension::new(VmArchitecture::Arch256).unwrap();
        assert!(extension.operation_mappings.contains_key("simd_add_f32x8"));

        let result = SimdExtension::new(VmArchitecture::Arch64);
        assert!(result.is_err());
    }

    #[test]
    fn test_operation_detection() {
        let extension = SimdExtension::new(VmArchitecture::Arch256).unwrap();

        let function = WasmFunction {
            signature: crate::wasm::ast::WasmFunctionType {
                params: vec![WasmValueType::V128, WasmValueType::V128],
                results: vec![WasmValueType::V128],
            },
            body: vec![],
            locals: vec![],
        };

        let operations = extension.detect_operations(&function).unwrap();
        // Note: Function name detection is currently disabled
        assert_eq!(operations.len(), 0);
    }

    #[test]
    fn test_vector_type_encoding() {
        let extension = SimdExtension::new(VmArchitecture::Arch256).unwrap();

        assert_eq!(extension.encode_vector_type(&SimdVectorType::F32x8).unwrap(), 0x24);
        assert_eq!(extension.encode_vector_type(&SimdVectorType::F64x4).unwrap(), 0x25);
    }
}
