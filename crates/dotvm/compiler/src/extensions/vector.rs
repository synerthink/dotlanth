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

//! Vector Processing Extensions for DotVM
//!
//! This module handles large-scale vector operations for 512-bit architecture,
//! providing high-performance computing features including parallel computation
//! primitives, matrix operations, and advanced vector processing.

use crate::wasm::ast::{WasmFunction, WasmInstruction, WasmValueType};
use dotvm_core::{
    bytecode::VmArchitecture,
    opcode::architecture_opcodes::Opcode512,
};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during vector processing extension
#[derive(Error, Debug)]
pub enum VectorExtensionError {
    #[error("Vector processing requires 512-bit architecture")]
    VectorArchitectureRequired,
    #[error("Unsupported vector operation: {0}")]
    UnsupportedOperation(String),
    #[error("Matrix dimension mismatch: {operation} requires {expected_dims}, got {actual_dims}")]
    MatrixDimensionMismatch {
        operation: String,
        expected_dims: String,
        actual_dims: String,
    },
    #[error("Vector length mismatch: expected {expected}, got {actual}")]
    VectorLengthMismatch { expected: usize, actual: usize },
    #[error("Parallel operation overflow: operation exceeds processing capacity")]
    ParallelOverflow,
    #[error("Invalid tensor shape: {details}")]
    InvalidTensorShape { details: String },
    #[error("Memory alignment error: vector operations require {required}-byte alignment")]
    MemoryAlignmentError { required: usize },
}

/// Vector data types for high-performance computing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VectorDataType {
    // Basic vector types
    Vector2D,    // 2D vector (x, y)
    Vector3D,    // 3D vector (x, y, z)
    Vector4D,    // 4D vector (x, y, z, w)
    
    // Matrix types
    Matrix2x2,   // 2x2 matrix
    Matrix3x3,   // 3x3 matrix
    Matrix4x4,   // 4x4 matrix
    MatrixNxM,   // Variable size matrix
    
    // Tensor types
    Tensor1D,    // 1D tensor (vector)
    Tensor2D,    // 2D tensor (matrix)
    Tensor3D,    // 3D tensor
    Tensor4D,    // 4D tensor
    TensorND,    // N-dimensional tensor
    
    // Specialized types
    Quaternion,  // Quaternion for 3D rotations
    Complex,     // Complex number
    Polynomial,  // Polynomial coefficients
}

impl VectorDataType {
    /// Get the memory size in bytes for this data type
    pub fn memory_size(&self) -> usize {
        match self {
            VectorDataType::Vector2D => 2 * 8,      // 2 x f64
            VectorDataType::Vector3D => 3 * 8,      // 3 x f64
            VectorDataType::Vector4D => 4 * 8,      // 4 x f64
            VectorDataType::Matrix2x2 => 4 * 8,     // 4 x f64
            VectorDataType::Matrix3x3 => 9 * 8,     // 9 x f64
            VectorDataType::Matrix4x4 => 16 * 8,    // 16 x f64
            VectorDataType::Quaternion => 4 * 8,    // 4 x f64
            VectorDataType::Complex => 2 * 8,       // 2 x f64
            _ => 64, // Default size for variable types
        }
    }

    /// Get the required memory alignment for this data type
    pub fn memory_alignment(&self) -> usize {
        match self {
            VectorDataType::Vector2D | VectorDataType::Complex => 16,
            VectorDataType::Vector3D => 32,
            VectorDataType::Vector4D | VectorDataType::Quaternion => 32,
            VectorDataType::Matrix2x2 => 32,
            VectorDataType::Matrix3x3 => 64,
            VectorDataType::Matrix4x4 => 64,
            _ => 64, // Conservative alignment for complex types
        }
    }
}

/// Vector processing operations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VectorOperation {
    // Basic vector operations
    VectorAdd,
    VectorSub,
    VectorMul,      // Element-wise multiplication
    VectorDiv,      // Element-wise division
    DotProduct,
    CrossProduct,
    VectorNorm,
    VectorNormalize,
    VectorDistance,
    VectorAngle,
    
    // Matrix operations
    MatrixAdd,
    MatrixSub,
    MatrixMul,      // Matrix multiplication
    MatrixTranspose,
    MatrixInverse,
    MatrixDeterminant,
    MatrixEigenvalues,
    MatrixEigenvectors,
    MatrixDecomposition, // LU, QR, SVD
    
    // Tensor operations
    TensorAdd,
    TensorSub,
    TensorMul,
    TensorContraction,
    TensorTranspose,
    TensorReshape,
    TensorSlice,
    
    // Advanced operations
    ConvolutionND,
    FourierTransform,
    InverseFourierTransform,
    WaveletTransform,
    PrincipalComponentAnalysis,
    
    // Parallel operations
    ParallelMap,
    ParallelReduce,
    ParallelFilter,
    ParallelSort,
    ParallelScan,
    
    // Specialized operations
    QuaternionMul,
    QuaternionConjugate,
    QuaternionToMatrix,
    ComplexMul,
    ComplexDiv,
    PolynomialEval,
    PolynomialRoots,
}

impl VectorOperation {
    /// Get the minimum number of operands required
    pub fn min_operands(&self) -> usize {
        match self {
            // Unary operations
            VectorOperation::VectorNorm |
            VectorOperation::VectorNormalize |
            VectorOperation::MatrixTranspose |
            VectorOperation::MatrixInverse |
            VectorOperation::MatrixDeterminant |
            VectorOperation::MatrixEigenvalues |
            VectorOperation::MatrixEigenvectors |
            VectorOperation::QuaternionConjugate |
            VectorOperation::QuaternionToMatrix |
            VectorOperation::InverseFourierTransform |
            VectorOperation::FourierTransform => 1,
            
            // Binary operations
            VectorOperation::VectorAdd |
            VectorOperation::VectorSub |
            VectorOperation::VectorMul |
            VectorOperation::VectorDiv |
            VectorOperation::DotProduct |
            VectorOperation::CrossProduct |
            VectorOperation::VectorDistance |
            VectorOperation::VectorAngle |
            VectorOperation::MatrixAdd |
            VectorOperation::MatrixSub |
            VectorOperation::MatrixMul |
            VectorOperation::QuaternionMul |
            VectorOperation::ComplexMul |
            VectorOperation::ComplexDiv => 2,
            
            // Variable operand operations
            _ => 1,
        }
    }

    /// Check if this operation supports parallel execution
    pub fn supports_parallel(&self) -> bool {
        matches!(self,
            VectorOperation::VectorAdd |
            VectorOperation::VectorSub |
            VectorOperation::VectorMul |
            VectorOperation::VectorDiv |
            VectorOperation::MatrixAdd |
            VectorOperation::MatrixSub |
            VectorOperation::TensorAdd |
            VectorOperation::TensorSub |
            VectorOperation::ParallelMap |
            VectorOperation::ParallelReduce |
            VectorOperation::ParallelFilter |
            VectorOperation::ParallelSort |
            VectorOperation::ParallelScan |
            VectorOperation::ConvolutionND
        )
    }
}

/// Represents a vector operation with its context
#[derive(Debug, Clone)]
pub struct VectorOperationContext {
    /// The vector operation to perform
    pub operation: VectorOperation,
    /// Input data types
    pub input_types: Vec<VectorDataType>,
    /// Output data type
    pub output_type: VectorDataType,
    /// Dimensions for matrix/tensor operations
    pub dimensions: Vec<usize>,
    /// Parallel execution parameters
    pub parallel_config: Option<ParallelConfig>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Configuration for parallel execution
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Number of parallel threads to use
    pub thread_count: usize,
    /// Chunk size for parallel processing
    pub chunk_size: usize,
    /// Memory access pattern optimization
    pub memory_pattern: MemoryPattern,
}

/// Memory access patterns for optimization
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryPattern {
    Sequential,    // Sequential access
    Strided,       // Strided access with fixed stride
    Random,        // Random access pattern
    Blocked,       // Block-based access for cache efficiency
}

/// Vector processing extension for DotVM
pub struct VectorExtension {
    /// Target architecture (must be 512-bit)
    target_architecture: VmArchitecture,
    /// Operation mappings from function names
    operation_mappings: HashMap<String, VectorOperation>,
    /// Default parallel configuration
    default_parallel_config: ParallelConfig,
}

impl VectorExtension {
    /// Create a new vector processing extension
    pub fn new(target_architecture: VmArchitecture) -> Result<Self, VectorExtensionError> {
        if target_architecture != VmArchitecture::Arch512 {
            return Err(VectorExtensionError::VectorArchitectureRequired);
        }

        let mut extension = Self {
            target_architecture,
            operation_mappings: HashMap::new(),
            default_parallel_config: ParallelConfig {
                thread_count: 8, // Default to 8 threads
                chunk_size: 1024,
                memory_pattern: MemoryPattern::Sequential,
            },
        };
        
        extension.initialize_operation_mappings();
        Ok(extension)
    }

    /// Initialize built-in operation mappings
    fn initialize_operation_mappings(&mut self) {
        // Vector operations
        self.operation_mappings.insert("vector_add".to_string(), VectorOperation::VectorAdd);
        self.operation_mappings.insert("vector_sub".to_string(), VectorOperation::VectorSub);
        self.operation_mappings.insert("vector_mul".to_string(), VectorOperation::VectorMul);
        self.operation_mappings.insert("vector_div".to_string(), VectorOperation::VectorDiv);
        self.operation_mappings.insert("dot_product".to_string(), VectorOperation::DotProduct);
        self.operation_mappings.insert("cross_product".to_string(), VectorOperation::CrossProduct);
        self.operation_mappings.insert("vector_norm".to_string(), VectorOperation::VectorNorm);
        self.operation_mappings.insert("vector_normalize".to_string(), VectorOperation::VectorNormalize);
        self.operation_mappings.insert("vector_distance".to_string(), VectorOperation::VectorDistance);
        self.operation_mappings.insert("vector_angle".to_string(), VectorOperation::VectorAngle);

        // Matrix operations
        self.operation_mappings.insert("matrix_add".to_string(), VectorOperation::MatrixAdd);
        self.operation_mappings.insert("matrix_sub".to_string(), VectorOperation::MatrixSub);
        self.operation_mappings.insert("matrix_mul".to_string(), VectorOperation::MatrixMul);
        self.operation_mappings.insert("matrix_transpose".to_string(), VectorOperation::MatrixTranspose);
        self.operation_mappings.insert("matrix_inverse".to_string(), VectorOperation::MatrixInverse);
        self.operation_mappings.insert("matrix_determinant".to_string(), VectorOperation::MatrixDeterminant);
        self.operation_mappings.insert("matrix_eigenvalues".to_string(), VectorOperation::MatrixEigenvalues);
        self.operation_mappings.insert("matrix_eigenvectors".to_string(), VectorOperation::MatrixEigenvectors);

        // Tensor operations
        self.operation_mappings.insert("tensor_add".to_string(), VectorOperation::TensorAdd);
        self.operation_mappings.insert("tensor_mul".to_string(), VectorOperation::TensorMul);
        self.operation_mappings.insert("tensor_contraction".to_string(), VectorOperation::TensorContraction);
        self.operation_mappings.insert("tensor_transpose".to_string(), VectorOperation::TensorTranspose);

        // Advanced operations
        self.operation_mappings.insert("convolution".to_string(), VectorOperation::ConvolutionND);
        self.operation_mappings.insert("fft".to_string(), VectorOperation::FourierTransform);
        self.operation_mappings.insert("ifft".to_string(), VectorOperation::InverseFourierTransform);
        self.operation_mappings.insert("wavelet".to_string(), VectorOperation::WaveletTransform);
        self.operation_mappings.insert("pca".to_string(), VectorOperation::PrincipalComponentAnalysis);

        // Parallel operations
        self.operation_mappings.insert("parallel_map".to_string(), VectorOperation::ParallelMap);
        self.operation_mappings.insert("parallel_reduce".to_string(), VectorOperation::ParallelReduce);
        self.operation_mappings.insert("parallel_filter".to_string(), VectorOperation::ParallelFilter);
        self.operation_mappings.insert("parallel_sort".to_string(), VectorOperation::ParallelSort);

        // Specialized operations
        self.operation_mappings.insert("quaternion_mul".to_string(), VectorOperation::QuaternionMul);
        self.operation_mappings.insert("quaternion_conjugate".to_string(), VectorOperation::QuaternionConjugate);
        self.operation_mappings.insert("complex_mul".to_string(), VectorOperation::ComplexMul);
        self.operation_mappings.insert("polynomial_eval".to_string(), VectorOperation::PolynomialEval);
    }

    /// Detect vector operations in a function
    pub fn detect_operations(&self, function: &WasmFunction) -> Result<Vec<VectorOperationContext>, VectorExtensionError> {
        let mut operations = Vec::new();

        // TODO: Check if function name matches a known vector operation
        // Function name mapping will be added when export information is available
        /*
        if let Some(vector_op) = self.operation_mappings.get(&function.name) {
            let context = self.create_operation_context(vector_op, function)?;
            operations.push(context);
        }
        */

        // Analyze function body for vector processing patterns
        operations.extend(self.analyze_function_body(function)?);

        Ok(operations)
    }

    /// Create operation context from function signature
    fn create_operation_context(
        &self,
        operation: &VectorOperation,
        function: &WasmFunction,
    ) -> Result<VectorOperationContext, VectorExtensionError> {
        let (input_types, output_type, dimensions) = self.infer_data_types(operation, function)?;
        
        let parallel_config = if operation.supports_parallel() {
            Some(self.default_parallel_config.clone())
        } else {
            None
        };

        Ok(VectorOperationContext {
            operation: operation.clone(),
            input_types,
            output_type,
            dimensions,
            parallel_config,
            metadata: HashMap::new(),
        })
    }

    /// Infer data types and dimensions from operation and function signature
    fn infer_data_types(
        &self,
        operation: &VectorOperation,
        function: &WasmFunction,
    ) -> Result<(Vec<VectorDataType>, VectorDataType, Vec<usize>), VectorExtensionError> {
        match operation {
            // Vector operations
            VectorOperation::VectorAdd | VectorOperation::VectorSub |
            VectorOperation::VectorMul | VectorOperation::VectorDiv => {
                let vector_type = self.infer_vector_type_from_params(&function.signature.params)?;
                Ok((vec![vector_type.clone(), vector_type.clone()], vector_type, vec![]))
            }
            
            VectorOperation::DotProduct => {
                let vector_type = self.infer_vector_type_from_params(&function.signature.params)?;
                Ok((vec![vector_type.clone(), vector_type], VectorDataType::Vector2D, vec![]))
            }
            
            VectorOperation::CrossProduct => {
                Ok((vec![VectorDataType::Vector3D, VectorDataType::Vector3D], VectorDataType::Vector3D, vec![]))
            }
            
            // Matrix operations
            VectorOperation::MatrixMul => {
                let matrix_type = self.infer_matrix_type_from_params(&function.signature.params)?;
                Ok((vec![matrix_type.clone(), matrix_type.clone()], matrix_type, vec![]))
            }
            
            VectorOperation::MatrixTranspose => {
                let matrix_type = self.infer_matrix_type_from_params(&function.signature.params)?;
                Ok((vec![matrix_type.clone()], matrix_type, vec![]))
            }
            
            // Default case
            _ => {
                let default_type = VectorDataType::Vector4D;
                Ok((vec![default_type.clone()], default_type, vec![]))
            }
        }
    }

    /// Infer vector type from function parameters
    fn infer_vector_type_from_params(&self, params: &[WasmValueType]) -> Result<VectorDataType, VectorExtensionError> {
        match params.len() {
            2 => Ok(VectorDataType::Vector2D),
            3 => Ok(VectorDataType::Vector3D),
            4 => Ok(VectorDataType::Vector4D),
            _ => Ok(VectorDataType::Vector4D), // Default
        }
    }

    /// Infer matrix type from function parameters
    fn infer_matrix_type_from_params(&self, params: &[WasmValueType]) -> Result<VectorDataType, VectorExtensionError> {
        match params.len() {
            4 => Ok(VectorDataType::Matrix2x2),
            9 => Ok(VectorDataType::Matrix3x3),
            16 => Ok(VectorDataType::Matrix4x4),
            _ => Ok(VectorDataType::MatrixNxM), // Variable size
        }
    }

    /// Analyze function body for vector processing patterns
    fn analyze_function_body(&self, function: &WasmFunction) -> Result<Vec<VectorOperationContext>, VectorExtensionError> {
        let mut operations = Vec::new();

        // Look for patterns that indicate vector processing
        if self.has_matrix_multiplication_pattern(&function.body) {
            operations.push(VectorOperationContext {
                operation: VectorOperation::MatrixMul,
                input_types: vec![VectorDataType::Matrix4x4, VectorDataType::Matrix4x4],
                output_type: VectorDataType::Matrix4x4,
                dimensions: vec![4, 4],
                parallel_config: Some(self.default_parallel_config.clone()),
                metadata: [("detected_via".to_string(), "pattern_analysis".to_string())].into(),
            });
        }

        if self.has_parallel_processing_pattern(&function.body) {
            operations.push(VectorOperationContext {
                operation: VectorOperation::ParallelMap,
                input_types: vec![VectorDataType::TensorND],
                output_type: VectorDataType::TensorND,
                dimensions: vec![],
                parallel_config: Some(self.default_parallel_config.clone()),
                metadata: [("detected_via".to_string(), "parallel_pattern".to_string())].into(),
            });
        }

        Ok(operations)
    }

    /// Check if instruction sequence indicates matrix multiplication
    fn has_matrix_multiplication_pattern(&self, instructions: &[WasmInstruction]) -> bool {
        // Look for nested loops with multiply-accumulate operations
        let mul_ops = instructions.iter()
            .filter(|inst| matches!(inst, 
                WasmInstruction::F64Mul | WasmInstruction::F32Mul
            ))
            .count();
        
        let add_ops = instructions.iter()
            .filter(|inst| matches!(inst,
                WasmInstruction::F64Add | WasmInstruction::F32Add
            ))
            .count();

        // Heuristic: matrix multiplication has many multiply-add pairs
        mul_ops >= 16 && add_ops >= 16 && (mul_ops as f32 / add_ops as f32).abs() < 2.0
    }

    /// Check if instruction sequence indicates parallel processing
    fn has_parallel_processing_pattern(&self, instructions: &[WasmInstruction]) -> bool {
        // Look for repetitive operations that could benefit from parallelization
        let total_ops = instructions.len();
        let arithmetic_ops = instructions.iter()
            .filter(|inst| matches!(inst,
                WasmInstruction::F64Add | WasmInstruction::F64Sub |
                WasmInstruction::F64Mul | WasmInstruction::F64Div |
                WasmInstruction::F32Add | WasmInstruction::F32Sub |
                WasmInstruction::F32Mul | WasmInstruction::F32Div
            ))
            .count();

        // Heuristic: if >50% of instructions are arithmetic, it might benefit from parallelization
        total_ops > 100 && (arithmetic_ops as f32 / total_ops as f32) > 0.5
    }

    /// Generate DotVM bytecode for a vector operation
    pub fn generate_bytecode(&self, context: &VectorOperationContext) -> Result<Vec<u8>, VectorExtensionError> {
        let mut bytecode = Vec::new();

        // Add operation metadata
        bytecode.push(self.encode_vector_operation(&context.operation)?);
        
        // Add data type information
        for input_type in &context.input_types {
            bytecode.push(self.encode_data_type(input_type)?);
        }
        bytecode.push(self.encode_data_type(&context.output_type)?);

        // Add parallel configuration if present
        if let Some(parallel_config) = &context.parallel_config {
            bytecode.push(0xFF); // Parallel marker
            bytecode.extend_from_slice(&(parallel_config.thread_count as u32).to_le_bytes());
            bytecode.extend_from_slice(&(parallel_config.chunk_size as u32).to_le_bytes());
        }

        // Generate operation-specific bytecode
        match context.operation {
            VectorOperation::VectorAdd => {
                bytecode.push((Opcode512::Vector(dotvm_core::opcode::vector_opcodes::VectorOpcode::VectorAdd).as_u16() & 0xFF) as u8);
            }
            VectorOperation::MatrixMul => {
                bytecode.push((Opcode512::Vector(dotvm_core::opcode::vector_opcodes::VectorOpcode::MatrixMul).as_u16() & 0xFF) as u8);
            }
            VectorOperation::DotProduct => {
                bytecode.push((Opcode512::Vector(dotvm_core::opcode::vector_opcodes::VectorOpcode::DotProduct).as_u16() & 0xFF) as u8);
            }
            VectorOperation::FourierTransform => {
                bytecode.push((Opcode512::Vector(dotvm_core::opcode::vector_opcodes::VectorOpcode::FFT).as_u16() & 0xFF) as u8);
            }
            VectorOperation::ParallelMap => {
                bytecode.push((Opcode512::Parallel(dotvm_core::opcode::parallel_opcodes::ParallelOpcode::Map).as_u16() & 0xFF) as u8);
            }
            _ => return Err(VectorExtensionError::UnsupportedOperation(
                format!("{:?} not yet implemented", context.operation)
            )),
        }

        Ok(bytecode)
    }

    /// Encode vector operation as byte
    fn encode_vector_operation(&self, operation: &VectorOperation) -> Result<u8, VectorExtensionError> {
        match operation {
            VectorOperation::VectorAdd => Ok(0x01),
            VectorOperation::VectorSub => Ok(0x02),
            VectorOperation::VectorMul => Ok(0x03),
            VectorOperation::DotProduct => Ok(0x04),
            VectorOperation::CrossProduct => Ok(0x05),
            VectorOperation::MatrixMul => Ok(0x10),
            VectorOperation::MatrixTranspose => Ok(0x11),
            VectorOperation::MatrixInverse => Ok(0x12),
            VectorOperation::FourierTransform => Ok(0x20),
            VectorOperation::ParallelMap => Ok(0x30),
            VectorOperation::ParallelReduce => Ok(0x31),
            _ => Err(VectorExtensionError::UnsupportedOperation(
                format!("Encoding not implemented for {:?}", operation)
            )),
        }
    }

    /// Encode data type as byte
    fn encode_data_type(&self, data_type: &VectorDataType) -> Result<u8, VectorExtensionError> {
        match data_type {
            VectorDataType::Vector2D => Ok(0x01),
            VectorDataType::Vector3D => Ok(0x02),
            VectorDataType::Vector4D => Ok(0x03),
            VectorDataType::Matrix2x2 => Ok(0x10),
            VectorDataType::Matrix3x3 => Ok(0x11),
            VectorDataType::Matrix4x4 => Ok(0x12),
            VectorDataType::Quaternion => Ok(0x20),
            VectorDataType::Complex => Ok(0x21),
            _ => Ok(0xFF), // Generic type
        }
    }

    /// Get supported operations for 512-bit architecture
    pub fn get_supported_operations(&self) -> Vec<VectorOperation> {
        self.operation_mappings.values().cloned().collect()
    }

    /// Set parallel configuration
    pub fn set_parallel_config(&mut self, config: ParallelConfig) {
        self.default_parallel_config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_extension_creation() {
        let extension = VectorExtension::new(VmArchitecture::Arch512).unwrap();
        assert!(extension.operation_mappings.contains_key("vector_add"));
        assert!(extension.operation_mappings.contains_key("matrix_mul"));
        
        let result = VectorExtension::new(VmArchitecture::Arch256);
        assert!(result.is_err());
    }

    #[test]
    fn test_data_type_properties() {
        assert_eq!(VectorDataType::Vector3D.memory_size(), 24);
        assert_eq!(VectorDataType::Matrix4x4.memory_size(), 128);
        assert_eq!(VectorDataType::Vector4D.memory_alignment(), 32);
    }

    #[test]
    fn test_operation_properties() {
        assert_eq!(VectorOperation::VectorAdd.min_operands(), 2);
        assert_eq!(VectorOperation::VectorNorm.min_operands(), 1);
        assert!(VectorOperation::VectorAdd.supports_parallel());
        assert!(!VectorOperation::MatrixInverse.supports_parallel());
    }

    #[test]
    fn test_operation_detection() {
        let extension = VectorExtension::new(VmArchitecture::Arch512).unwrap();
        
        let function = WasmFunction {
            signature: crate::wasm::ast::WasmFunctionType {
                params: vec![WasmValueType::F64, WasmValueType::F64, WasmValueType::F64],
                results: vec![WasmValueType::F64, WasmValueType::F64, WasmValueType::F64],
            },
            body: vec![],
            locals: vec![],
        };

        let operations = extension.detect_operations(&function).unwrap();
        // Note: Function name detection is currently disabled
        assert_eq!(operations.len(), 0);
    }

    #[test]
    fn test_parallel_config() {
        let mut extension = VectorExtension::new(VmArchitecture::Arch512).unwrap();
        
        let new_config = ParallelConfig {
            thread_count: 16,
            chunk_size: 2048,
            memory_pattern: MemoryPattern::Blocked,
        };
        
        extension.set_parallel_config(new_config.clone());
        assert_eq!(extension.default_parallel_config.thread_count, 16);
        assert_eq!(extension.default_parallel_config.chunk_size, 2048);
    }
}