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

//! SIMD operation opcodes for DotVM
//!
//! This module defines opcodes for Single Instruction, Multiple Data operations
//! available on 256-bit+ architectures.

use std::fmt;

/// SIMD opcodes for 256-bit+ architectures
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum SimdOpcode {
    // 256-bit integer operations
    AddI32x8 = 0x01, // Add 8 x i32
    SubI32x8 = 0x02, // Subtract 8 x i32
    MulI32x8 = 0x03, // Multiply 8 x i32

    // 256-bit float operations
    AddF32x8 = 0x10, // Add 8 x f32
    SubF32x8 = 0x11, // Subtract 8 x f32
    MulF32x8 = 0x12, // Multiply 8 x f32
    DivF32x8 = 0x13, // Divide 8 x f32

    // 256-bit double operations
    AddF64x4 = 0x20, // Add 4 x f64
    SubF64x4 = 0x21, // Subtract 4 x f64
    MulF64x4 = 0x22, // Multiply 4 x f64
    DivF64x4 = 0x23, // Divide 4 x f64

    // Logical operations
    AndI32x8 = 0x30, // Bitwise AND 8 x i32
    OrI32x8 = 0x31,  // Bitwise OR 8 x i32
    XorI32x8 = 0x32, // Bitwise XOR 8 x i32
    NotI32x8 = 0x33, // Bitwise NOT 8 x i32

    // Comparison operations
    EqF32x8 = 0x40, // Equal comparison 8 x f32
    NeF32x8 = 0x41, // Not equal comparison 8 x f32
    LtF32x8 = 0x42, // Less than comparison 8 x f32
    LeF32x8 = 0x43, // Less equal comparison 8 x f32
    GtF32x8 = 0x44, // Greater than comparison 8 x f32
    GeF32x8 = 0x45, // Greater equal comparison 8 x f32

    // Data movement
    LoadF32x8 = 0x50,  // Load 8 x f32 from memory
    StoreF32x8 = 0x51, // Store 8 x f32 to memory
    LoadF64x4 = 0x52,  // Load 4 x f64 from memory
    StoreF64x4 = 0x53, // Store 4 x f64 to memory

    // Shuffle and permute
    ShuffleF32x8 = 0x60, // Shuffle 8 x f32
    PermuteF32x8 = 0x61, // Permute 8 x f32
    BlendF32x8 = 0x62,   // Blend 8 x f32

    // Reduction operations
    SumF32x8 = 0x70, // Sum all elements in 8 x f32
    MinF32x8 = 0x71, // Find minimum in 8 x f32
    MaxF32x8 = 0x72, // Find maximum in 8 x f32

    // Conversion operations
    ConvertI32x8ToF32x8 = 0x80, // Convert 8 x i32 to 8 x f32
    ConvertF32x8ToI32x8 = 0x81, // Convert 8 x f32 to 8 x i32
    ConvertF32x8ToF64x4 = 0x82, // Convert 8 x f32 to 4 x f64
    ConvertF64x4ToF32x8 = 0x83, // Convert 4 x f64 to 8 x f32
}

impl SimdOpcode {
    /// Get the number of operands this opcode expects
    pub fn operand_count(&self) -> usize {
        match self {
            // Unary operations
            SimdOpcode::NotI32x8
            | SimdOpcode::SumF32x8
            | SimdOpcode::MinF32x8
            | SimdOpcode::MaxF32x8
            | SimdOpcode::ConvertI32x8ToF32x8
            | SimdOpcode::ConvertF32x8ToI32x8
            | SimdOpcode::ConvertF32x8ToF64x4
            | SimdOpcode::ConvertF64x4ToF32x8
            | SimdOpcode::LoadF32x8
            | SimdOpcode::LoadF64x4 => 1,

            // Binary operations
            SimdOpcode::AddI32x8
            | SimdOpcode::SubI32x8
            | SimdOpcode::MulI32x8
            | SimdOpcode::AddF32x8
            | SimdOpcode::SubF32x8
            | SimdOpcode::MulF32x8
            | SimdOpcode::DivF32x8
            | SimdOpcode::AddF64x4
            | SimdOpcode::SubF64x4
            | SimdOpcode::MulF64x4
            | SimdOpcode::DivF64x4
            | SimdOpcode::AndI32x8
            | SimdOpcode::OrI32x8
            | SimdOpcode::XorI32x8
            | SimdOpcode::EqF32x8
            | SimdOpcode::NeF32x8
            | SimdOpcode::LtF32x8
            | SimdOpcode::LeF32x8
            | SimdOpcode::GtF32x8
            | SimdOpcode::GeF32x8
            | SimdOpcode::StoreF32x8
            | SimdOpcode::StoreF64x4 => 2,

            // Ternary operations
            SimdOpcode::ShuffleF32x8 | SimdOpcode::PermuteF32x8 | SimdOpcode::BlendF32x8 => 3,
        }
    }

    /// Get the vector width in bits
    pub fn vector_width(&self) -> u32 {
        256 // All SIMD operations are 256-bit
    }

    /// Check if this operation is commutative
    pub fn is_commutative(&self) -> bool {
        matches!(
            self,
            SimdOpcode::AddI32x8
                | SimdOpcode::MulI32x8
                | SimdOpcode::AddF32x8
                | SimdOpcode::MulF32x8
                | SimdOpcode::AddF64x4
                | SimdOpcode::MulF64x4
                | SimdOpcode::AndI32x8
                | SimdOpcode::OrI32x8
                | SimdOpcode::XorI32x8
                | SimdOpcode::EqF32x8
                | SimdOpcode::NeF32x8
        )
    }
}

impl fmt::Display for SimdOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimdOpcode::AddI32x8 => write!(f, "simd.add.i32x8"),
            SimdOpcode::SubI32x8 => write!(f, "simd.sub.i32x8"),
            SimdOpcode::MulI32x8 => write!(f, "simd.mul.i32x8"),
            SimdOpcode::AddF32x8 => write!(f, "simd.add.f32x8"),
            SimdOpcode::SubF32x8 => write!(f, "simd.sub.f32x8"),
            SimdOpcode::MulF32x8 => write!(f, "simd.mul.f32x8"),
            SimdOpcode::DivF32x8 => write!(f, "simd.div.f32x8"),
            SimdOpcode::AddF64x4 => write!(f, "simd.add.f64x4"),
            SimdOpcode::SubF64x4 => write!(f, "simd.sub.f64x4"),
            SimdOpcode::MulF64x4 => write!(f, "simd.mul.f64x4"),
            SimdOpcode::DivF64x4 => write!(f, "simd.div.f64x4"),
            SimdOpcode::AndI32x8 => write!(f, "simd.and.i32x8"),
            SimdOpcode::OrI32x8 => write!(f, "simd.or.i32x8"),
            SimdOpcode::XorI32x8 => write!(f, "simd.xor.i32x8"),
            SimdOpcode::NotI32x8 => write!(f, "simd.not.i32x8"),
            SimdOpcode::EqF32x8 => write!(f, "simd.eq.f32x8"),
            SimdOpcode::NeF32x8 => write!(f, "simd.ne.f32x8"),
            SimdOpcode::LtF32x8 => write!(f, "simd.lt.f32x8"),
            SimdOpcode::LeF32x8 => write!(f, "simd.le.f32x8"),
            SimdOpcode::GtF32x8 => write!(f, "simd.gt.f32x8"),
            SimdOpcode::GeF32x8 => write!(f, "simd.ge.f32x8"),
            SimdOpcode::LoadF32x8 => write!(f, "simd.load.f32x8"),
            SimdOpcode::StoreF32x8 => write!(f, "simd.store.f32x8"),
            SimdOpcode::LoadF64x4 => write!(f, "simd.load.f64x4"),
            SimdOpcode::StoreF64x4 => write!(f, "simd.store.f64x4"),
            SimdOpcode::ShuffleF32x8 => write!(f, "simd.shuffle.f32x8"),
            SimdOpcode::PermuteF32x8 => write!(f, "simd.permute.f32x8"),
            SimdOpcode::BlendF32x8 => write!(f, "simd.blend.f32x8"),
            SimdOpcode::SumF32x8 => write!(f, "simd.sum.f32x8"),
            SimdOpcode::MinF32x8 => write!(f, "simd.min.f32x8"),
            SimdOpcode::MaxF32x8 => write!(f, "simd.max.f32x8"),
            SimdOpcode::ConvertI32x8ToF32x8 => write!(f, "simd.convert.i32x8.f32x8"),
            SimdOpcode::ConvertF32x8ToI32x8 => write!(f, "simd.convert.f32x8.i32x8"),
            SimdOpcode::ConvertF32x8ToF64x4 => write!(f, "simd.convert.f32x8.f64x4"),
            SimdOpcode::ConvertF64x4ToF32x8 => write!(f, "simd.convert.f64x4.f32x8"),
        }
    }
}

impl From<u8> for SimdOpcode {
    fn from(value: u8) -> Self {
        match value {
            0x01 => SimdOpcode::AddI32x8,
            0x02 => SimdOpcode::SubI32x8,
            0x03 => SimdOpcode::MulI32x8,
            0x10 => SimdOpcode::AddF32x8,
            0x11 => SimdOpcode::SubF32x8,
            0x12 => SimdOpcode::MulF32x8,
            0x13 => SimdOpcode::DivF32x8,
            0x20 => SimdOpcode::AddF64x4,
            0x21 => SimdOpcode::SubF64x4,
            0x22 => SimdOpcode::MulF64x4,
            0x23 => SimdOpcode::DivF64x4,
            0x30 => SimdOpcode::AndI32x8,
            0x31 => SimdOpcode::OrI32x8,
            0x32 => SimdOpcode::XorI32x8,
            0x33 => SimdOpcode::NotI32x8,
            0x40 => SimdOpcode::EqF32x8,
            0x41 => SimdOpcode::NeF32x8,
            0x42 => SimdOpcode::LtF32x8,
            0x43 => SimdOpcode::LeF32x8,
            0x44 => SimdOpcode::GtF32x8,
            0x45 => SimdOpcode::GeF32x8,
            0x50 => SimdOpcode::LoadF32x8,
            0x51 => SimdOpcode::StoreF32x8,
            0x52 => SimdOpcode::LoadF64x4,
            0x53 => SimdOpcode::StoreF64x4,
            0x60 => SimdOpcode::ShuffleF32x8,
            0x61 => SimdOpcode::PermuteF32x8,
            0x62 => SimdOpcode::BlendF32x8,
            0x70 => SimdOpcode::SumF32x8,
            0x71 => SimdOpcode::MinF32x8,
            0x72 => SimdOpcode::MaxF32x8,
            0x80 => SimdOpcode::ConvertI32x8ToF32x8,
            0x81 => SimdOpcode::ConvertF32x8ToI32x8,
            0x82 => SimdOpcode::ConvertF32x8ToF64x4,
            0x83 => SimdOpcode::ConvertF64x4ToF32x8,
            _ => SimdOpcode::AddF32x8, // Default fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operand_counts() {
        assert_eq!(SimdOpcode::AddF32x8.operand_count(), 2);
        assert_eq!(SimdOpcode::NotI32x8.operand_count(), 1);
        assert_eq!(SimdOpcode::BlendF32x8.operand_count(), 3);
    }

    #[test]
    fn test_vector_width() {
        assert_eq!(SimdOpcode::AddF32x8.vector_width(), 256);
        assert_eq!(SimdOpcode::MulF64x4.vector_width(), 256);
    }

    #[test]
    fn test_commutativity() {
        assert!(SimdOpcode::AddF32x8.is_commutative());
        assert!(SimdOpcode::MulI32x8.is_commutative());
        assert!(!SimdOpcode::SubF32x8.is_commutative());
        assert!(!SimdOpcode::DivF64x4.is_commutative());
    }

    #[test]
    fn test_display() {
        assert_eq!(SimdOpcode::AddF32x8.to_string(), "simd.add.f32x8");
        assert_eq!(SimdOpcode::MulF64x4.to_string(), "simd.mul.f64x4");
    }
}
