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

//! Vector processing opcodes for DotVM
//!
//! This module defines opcodes for large-scale vector operations
//! available on 512-bit architecture.

use std::fmt;

/// Vector processing opcodes for 512-bit architecture
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum VectorOpcode {
    // 512-bit vector arithmetic
    AddF32x16 = 0x01, // Add 16 x f32
    SubF32x16 = 0x02, // Subtract 16 x f32
    MulF32x16 = 0x03, // Multiply 16 x f32
    DivF32x16 = 0x04, // Divide 16 x f32

    AddF64x8 = 0x10, // Add 8 x f64
    SubF64x8 = 0x11, // Subtract 8 x f64
    MulF64x8 = 0x12, // Multiply 8 x f64
    DivF64x8 = 0x13, // Divide 8 x f64

    // Vector operations
    VectorAdd = 0x20,       // Generic vector addition
    VectorSub = 0x21,       // Generic vector subtraction
    VectorMul = 0x22,       // Element-wise vector multiplication
    VectorDiv = 0x23,       // Element-wise vector division
    DotProduct = 0x24,      // Vector dot product
    CrossProduct = 0x25,    // Vector cross product
    VectorNorm = 0x26,      // Vector norm/magnitude
    VectorNormalize = 0x27, // Vector normalization

    // Matrix operations
    MatrixMul = 0x30,         // Matrix multiplication
    MatrixTranspose = 0x31,   // Matrix transpose
    MatrixInverse = 0x32,     // Matrix inverse
    MatrixDeterminant = 0x33, // Matrix determinant
    MatrixTrace = 0x34,       // Matrix trace

    // Advanced vector operations
    DotProductF32x16 = 0x40,   // Dot product of 16 x f32 vectors
    CrossProductF32x16 = 0x41, // Cross product (3D vectors in 16-element format)
    NormalizeF32x16 = 0x42,    // Normalize 16 x f32 vector
    MagnitudeF32x16 = 0x43,    // Calculate magnitude of 16 x f32 vector

    // Reduction operations
    SumF32x16 = 0x50,     // Sum all elements in 16 x f32
    ProductF32x16 = 0x51, // Product of all elements in 16 x f32
    MinF32x16 = 0x52,     // Find minimum in 16 x f32
    MaxF32x16 = 0x53,     // Find maximum in 16 x f32

    // Transform operations
    FFT = 0x60,         // Fast Fourier Transform
    IFFT = 0x61,        // Inverse Fast Fourier Transform
    Convolution = 0x62, // Convolution operation
    Correlation = 0x63, // Correlation operation

    // Memory operations
    LoadVector = 0x70,    // Load vector from memory
    StoreVector = 0x71,   // Store vector to memory
    GatherVector = 0x72,  // Gather vector elements
    ScatterVector = 0x73, // Scatter vector elements

    // Comparison operations
    CompareEq = 0x80, // Vector equality comparison
    CompareNe = 0x81, // Vector inequality comparison
    CompareLt = 0x82, // Vector less than comparison
    CompareLe = 0x83, // Vector less equal comparison
    CompareGt = 0x84, // Vector greater than comparison
    CompareGe = 0x85, // Vector greater equal comparison

    // Conversion operations
    ConvertF32x16ToF64x8 = 0x90,  // Convert 16 x f32 to 8 x f64
    ConvertF64x8ToF32x16 = 0x91,  // Convert 8 x f64 to 16 x f32
    ConvertI32x16ToF32x16 = 0x92, // Convert 16 x i32 to 16 x f32
    ConvertF32x16ToI32x16 = 0x93, // Convert 16 x f32 to 16 x i32

    // Shuffle and permute
    ShuffleF32x16 = 0xA0, // Shuffle 16 x f32 elements
    PermuteF32x16 = 0xA1, // Permute 16 x f32 elements
    BlendF32x16 = 0xA2,   // Blend two 16 x f32 vectors
    SelectF32x16 = 0xA3,  // Select elements from two 16 x f32 vectors

    // Specialized operations
    Polynomial = 0xB0,  // Polynomial evaluation
    Interpolate = 0xB1, // Vector interpolation
    Extrapolate = 0xB2, // Vector extrapolation
    Smooth = 0xB3,      // Vector smoothing
}

impl VectorOpcode {
    /// Get the number of operands this opcode expects
    pub fn operand_count(&self) -> usize {
        match self {
            // Unary operations
            VectorOpcode::VectorNorm
            | VectorOpcode::VectorNormalize
            | VectorOpcode::MatrixTranspose
            | VectorOpcode::MatrixInverse
            | VectorOpcode::MatrixDeterminant
            | VectorOpcode::MatrixTrace
            | VectorOpcode::NormalizeF32x16
            | VectorOpcode::MagnitudeF32x16
            | VectorOpcode::SumF32x16
            | VectorOpcode::ProductF32x16
            | VectorOpcode::MinF32x16
            | VectorOpcode::MaxF32x16
            | VectorOpcode::FFT
            | VectorOpcode::IFFT
            | VectorOpcode::LoadVector
            | VectorOpcode::ConvertF32x16ToF64x8
            | VectorOpcode::ConvertF64x8ToF32x16
            | VectorOpcode::ConvertI32x16ToF32x16
            | VectorOpcode::ConvertF32x16ToI32x16
            | VectorOpcode::Polynomial
            | VectorOpcode::Smooth => 1,

            // Binary operations
            VectorOpcode::AddF32x16
            | VectorOpcode::SubF32x16
            | VectorOpcode::MulF32x16
            | VectorOpcode::DivF32x16
            | VectorOpcode::AddF64x8
            | VectorOpcode::SubF64x8
            | VectorOpcode::MulF64x8
            | VectorOpcode::DivF64x8
            | VectorOpcode::VectorAdd
            | VectorOpcode::VectorSub
            | VectorOpcode::VectorMul
            | VectorOpcode::VectorDiv
            | VectorOpcode::DotProduct
            | VectorOpcode::CrossProduct
            | VectorOpcode::MatrixMul
            | VectorOpcode::DotProductF32x16
            | VectorOpcode::CrossProductF32x16
            | VectorOpcode::Convolution
            | VectorOpcode::Correlation
            | VectorOpcode::StoreVector
            | VectorOpcode::GatherVector
            | VectorOpcode::ScatterVector
            | VectorOpcode::CompareEq
            | VectorOpcode::CompareNe
            | VectorOpcode::CompareLt
            | VectorOpcode::CompareLe
            | VectorOpcode::CompareGt
            | VectorOpcode::CompareGe
            | VectorOpcode::Interpolate
            | VectorOpcode::Extrapolate => 2,

            // Ternary operations
            VectorOpcode::ShuffleF32x16 | VectorOpcode::PermuteF32x16 | VectorOpcode::BlendF32x16 | VectorOpcode::SelectF32x16 => 3,
        }
    }

    /// Get the vector width in bits
    pub fn vector_width(&self) -> u32 {
        512 // All vector operations are 512-bit
    }

    /// Check if this operation is commutative
    pub fn is_commutative(&self) -> bool {
        matches!(
            self,
            VectorOpcode::AddF32x16
                | VectorOpcode::MulF32x16
                | VectorOpcode::AddF64x8
                | VectorOpcode::MulF64x8
                | VectorOpcode::VectorAdd
                | VectorOpcode::VectorMul
                | VectorOpcode::DotProduct
                | VectorOpcode::DotProductF32x16
                | VectorOpcode::CompareEq
                | VectorOpcode::CompareNe
        )
    }

    /// Check if this operation supports parallel execution
    pub fn supports_parallel(&self) -> bool {
        matches!(
            self,
            VectorOpcode::AddF32x16
                | VectorOpcode::SubF32x16
                | VectorOpcode::MulF32x16
                | VectorOpcode::DivF32x16
                | VectorOpcode::AddF64x8
                | VectorOpcode::SubF64x8
                | VectorOpcode::MulF64x8
                | VectorOpcode::DivF64x8
                | VectorOpcode::VectorAdd
                | VectorOpcode::VectorSub
                | VectorOpcode::VectorMul
                | VectorOpcode::VectorDiv
                | VectorOpcode::MatrixMul
                | VectorOpcode::Convolution
                | VectorOpcode::FFT
                | VectorOpcode::IFFT
        )
    }

    /// Get the computational complexity category
    pub fn complexity(&self) -> ComputationalComplexity {
        match self {
            // Linear operations
            VectorOpcode::AddF32x16
            | VectorOpcode::SubF32x16
            | VectorOpcode::AddF64x8
            | VectorOpcode::SubF64x8
            | VectorOpcode::VectorAdd
            | VectorOpcode::VectorSub
            | VectorOpcode::LoadVector
            | VectorOpcode::StoreVector => ComputationalComplexity::Linear,

            // Quadratic operations
            VectorOpcode::MulF32x16
            | VectorOpcode::DivF32x16
            | VectorOpcode::MulF64x8
            | VectorOpcode::DivF64x8
            | VectorOpcode::VectorMul
            | VectorOpcode::VectorDiv
            | VectorOpcode::DotProduct
            | VectorOpcode::DotProductF32x16 => ComputationalComplexity::Quadratic,

            // Cubic operations
            VectorOpcode::MatrixMul | VectorOpcode::MatrixInverse | VectorOpcode::MatrixDeterminant => ComputationalComplexity::Cubic,

            // Logarithmic operations
            VectorOpcode::FFT | VectorOpcode::IFFT => ComputationalComplexity::Logarithmic,

            // Complex operations
            VectorOpcode::Convolution | VectorOpcode::Correlation | VectorOpcode::Polynomial => ComputationalComplexity::Exponential,

            // Simple operations
            _ => ComputationalComplexity::Constant,
        }
    }
}

/// Computational complexity categories for vector operations
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ComputationalComplexity {
    Constant,    // O(1)
    Logarithmic, // O(log n)
    Linear,      // O(n)
    Quadratic,   // O(n²)
    Cubic,       // O(n³)
    Exponential, // O(2^n)
}

impl fmt::Display for VectorOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VectorOpcode::AddF32x16 => write!(f, "vec.add.f32x16"),
            VectorOpcode::SubF32x16 => write!(f, "vec.sub.f32x16"),
            VectorOpcode::MulF32x16 => write!(f, "vec.mul.f32x16"),
            VectorOpcode::DivF32x16 => write!(f, "vec.div.f32x16"),
            VectorOpcode::AddF64x8 => write!(f, "vec.add.f64x8"),
            VectorOpcode::SubF64x8 => write!(f, "vec.sub.f64x8"),
            VectorOpcode::MulF64x8 => write!(f, "vec.mul.f64x8"),
            VectorOpcode::DivF64x8 => write!(f, "vec.div.f64x8"),
            VectorOpcode::VectorAdd => write!(f, "vec.add"),
            VectorOpcode::VectorSub => write!(f, "vec.sub"),
            VectorOpcode::VectorMul => write!(f, "vec.mul"),
            VectorOpcode::VectorDiv => write!(f, "vec.div"),
            VectorOpcode::DotProduct => write!(f, "vec.dot"),
            VectorOpcode::CrossProduct => write!(f, "vec.cross"),
            VectorOpcode::VectorNorm => write!(f, "vec.norm"),
            VectorOpcode::VectorNormalize => write!(f, "vec.normalize"),
            VectorOpcode::MatrixMul => write!(f, "mat.mul"),
            VectorOpcode::MatrixTranspose => write!(f, "mat.transpose"),
            VectorOpcode::MatrixInverse => write!(f, "mat.inverse"),
            VectorOpcode::MatrixDeterminant => write!(f, "mat.det"),
            VectorOpcode::MatrixTrace => write!(f, "mat.trace"),
            VectorOpcode::DotProductF32x16 => write!(f, "vec.dot.f32x16"),
            VectorOpcode::CrossProductF32x16 => write!(f, "vec.cross.f32x16"),
            VectorOpcode::NormalizeF32x16 => write!(f, "vec.normalize.f32x16"),
            VectorOpcode::MagnitudeF32x16 => write!(f, "vec.magnitude.f32x16"),
            VectorOpcode::SumF32x16 => write!(f, "vec.sum.f32x16"),
            VectorOpcode::ProductF32x16 => write!(f, "vec.product.f32x16"),
            VectorOpcode::MinF32x16 => write!(f, "vec.min.f32x16"),
            VectorOpcode::MaxF32x16 => write!(f, "vec.max.f32x16"),
            VectorOpcode::FFT => write!(f, "vec.fft"),
            VectorOpcode::IFFT => write!(f, "vec.ifft"),
            VectorOpcode::Convolution => write!(f, "vec.conv"),
            VectorOpcode::Correlation => write!(f, "vec.corr"),
            VectorOpcode::LoadVector => write!(f, "vec.load"),
            VectorOpcode::StoreVector => write!(f, "vec.store"),
            VectorOpcode::GatherVector => write!(f, "vec.gather"),
            VectorOpcode::ScatterVector => write!(f, "vec.scatter"),
            VectorOpcode::CompareEq => write!(f, "vec.eq"),
            VectorOpcode::CompareNe => write!(f, "vec.ne"),
            VectorOpcode::CompareLt => write!(f, "vec.lt"),
            VectorOpcode::CompareLe => write!(f, "vec.le"),
            VectorOpcode::CompareGt => write!(f, "vec.gt"),
            VectorOpcode::CompareGe => write!(f, "vec.ge"),
            VectorOpcode::ConvertF32x16ToF64x8 => write!(f, "vec.convert.f32x16.f64x8"),
            VectorOpcode::ConvertF64x8ToF32x16 => write!(f, "vec.convert.f64x8.f32x16"),
            VectorOpcode::ConvertI32x16ToF32x16 => write!(f, "vec.convert.i32x16.f32x16"),
            VectorOpcode::ConvertF32x16ToI32x16 => write!(f, "vec.convert.f32x16.i32x16"),
            VectorOpcode::ShuffleF32x16 => write!(f, "vec.shuffle.f32x16"),
            VectorOpcode::PermuteF32x16 => write!(f, "vec.permute.f32x16"),
            VectorOpcode::BlendF32x16 => write!(f, "vec.blend.f32x16"),
            VectorOpcode::SelectF32x16 => write!(f, "vec.select.f32x16"),
            VectorOpcode::Polynomial => write!(f, "vec.polynomial"),
            VectorOpcode::Interpolate => write!(f, "vec.interpolate"),
            VectorOpcode::Extrapolate => write!(f, "vec.extrapolate"),
            VectorOpcode::Smooth => write!(f, "vec.smooth"),
        }
    }
}

impl From<u8> for VectorOpcode {
    fn from(value: u8) -> Self {
        match value {
            0x01 => VectorOpcode::AddF32x16,
            0x02 => VectorOpcode::SubF32x16,
            0x03 => VectorOpcode::MulF32x16,
            0x04 => VectorOpcode::DivF32x16,
            0x10 => VectorOpcode::AddF64x8,
            0x11 => VectorOpcode::SubF64x8,
            0x12 => VectorOpcode::MulF64x8,
            0x13 => VectorOpcode::DivF64x8,
            0x20 => VectorOpcode::VectorAdd,
            0x21 => VectorOpcode::VectorSub,
            0x22 => VectorOpcode::VectorMul,
            0x23 => VectorOpcode::VectorDiv,
            0x24 => VectorOpcode::DotProduct,
            0x25 => VectorOpcode::CrossProduct,
            0x26 => VectorOpcode::VectorNorm,
            0x27 => VectorOpcode::VectorNormalize,
            0x30 => VectorOpcode::MatrixMul,
            0x31 => VectorOpcode::MatrixTranspose,
            0x32 => VectorOpcode::MatrixInverse,
            0x33 => VectorOpcode::MatrixDeterminant,
            0x34 => VectorOpcode::MatrixTrace,
            0x40 => VectorOpcode::DotProductF32x16,
            0x41 => VectorOpcode::CrossProductF32x16,
            0x42 => VectorOpcode::NormalizeF32x16,
            0x43 => VectorOpcode::MagnitudeF32x16,
            0x50 => VectorOpcode::SumF32x16,
            0x51 => VectorOpcode::ProductF32x16,
            0x52 => VectorOpcode::MinF32x16,
            0x53 => VectorOpcode::MaxF32x16,
            0x60 => VectorOpcode::FFT,
            0x61 => VectorOpcode::IFFT,
            0x62 => VectorOpcode::Convolution,
            0x63 => VectorOpcode::Correlation,
            0x70 => VectorOpcode::LoadVector,
            0x71 => VectorOpcode::StoreVector,
            0x72 => VectorOpcode::GatherVector,
            0x73 => VectorOpcode::ScatterVector,
            0x80 => VectorOpcode::CompareEq,
            0x81 => VectorOpcode::CompareNe,
            0x82 => VectorOpcode::CompareLt,
            0x83 => VectorOpcode::CompareLe,
            0x84 => VectorOpcode::CompareGt,
            0x85 => VectorOpcode::CompareGe,
            0x90 => VectorOpcode::ConvertF32x16ToF64x8,
            0x91 => VectorOpcode::ConvertF64x8ToF32x16,
            0x92 => VectorOpcode::ConvertI32x16ToF32x16,
            0x93 => VectorOpcode::ConvertF32x16ToI32x16,
            0xA0 => VectorOpcode::ShuffleF32x16,
            0xA1 => VectorOpcode::PermuteF32x16,
            0xA2 => VectorOpcode::BlendF32x16,
            0xA3 => VectorOpcode::SelectF32x16,
            0xB0 => VectorOpcode::Polynomial,
            0xB1 => VectorOpcode::Interpolate,
            0xB2 => VectorOpcode::Extrapolate,
            0xB3 => VectorOpcode::Smooth,
            _ => VectorOpcode::AddF32x16, // Default fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operand_counts() {
        assert_eq!(VectorOpcode::AddF32x16.operand_count(), 2);
        assert_eq!(VectorOpcode::VectorNorm.operand_count(), 1);
        assert_eq!(VectorOpcode::BlendF32x16.operand_count(), 3);
    }

    #[test]
    fn test_vector_width() {
        assert_eq!(VectorOpcode::AddF32x16.vector_width(), 512);
        assert_eq!(VectorOpcode::MatrixMul.vector_width(), 512);
    }

    #[test]
    fn test_commutativity() {
        assert!(VectorOpcode::AddF32x16.is_commutative());
        assert!(VectorOpcode::DotProduct.is_commutative());
        assert!(!VectorOpcode::SubF32x16.is_commutative());
        assert!(!VectorOpcode::MatrixMul.is_commutative());
    }

    #[test]
    fn test_parallel_support() {
        assert!(VectorOpcode::AddF32x16.supports_parallel());
        assert!(VectorOpcode::MatrixMul.supports_parallel());
        assert!(!VectorOpcode::VectorNorm.supports_parallel());
    }

    #[test]
    fn test_complexity() {
        assert_eq!(VectorOpcode::AddF32x16.complexity(), ComputationalComplexity::Linear);
        assert_eq!(VectorOpcode::MatrixMul.complexity(), ComputationalComplexity::Cubic);
        assert_eq!(VectorOpcode::FFT.complexity(), ComputationalComplexity::Logarithmic);
    }
}
