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

//! Architecture-specific opcode definitions for DotVM
//!
//! This module defines hierarchical opcode enums that expand with architecture size.
//! Each higher architecture includes all opcodes from lower architectures plus
//! architecture-specific extensions.

use super::{
    arithmetic_opcodes::ArithmeticOpcode, bigint_opcodes::BigIntOpcode, control_flow_opcodes::ControlFlowOpcode, crypto_opcodes::CryptoOpcode, math_opcodes::MathOpcode, memory_opcodes::MemoryOpcode,
    parallel_opcodes::ParallelOpcode, simd_opcodes::SimdOpcode, system_call_opcodes::SystemCallOpcode, vector_opcodes::VectorOpcode,
};
use std::fmt;

/// Base 64-bit architecture opcodes
/// This includes all fundamental operations that work with standard 64-bit values
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Opcode64 {
    /// Arithmetic operations (ADD, SUB, MUL, DIV, MOD)
    Arithmetic(ArithmeticOpcode),
    /// Control flow operations (IF, JUMP, LOOP, etc.)
    ControlFlow(ControlFlowOpcode),
    /// Memory operations (LOAD, STORE, ALLOCATE, etc.)
    Memory(MemoryOpcode),
    /// System call operations (READ, WRITE, NETWORK, etc.)
    SystemCall(SystemCallOpcode),
    /// Basic cryptographic operations (HASH, ENCRYPT, DECRYPT, etc.)
    Crypto(CryptoOpcode),
}

impl Opcode64 {
    /// Get the opcode's numerical value for bytecode generation
    pub fn as_u16(&self) -> u16 {
        match self {
            Opcode64::Arithmetic(op) => op.as_u8() as u16,
            Opcode64::ControlFlow(op) => 0x0100 + op.as_u8() as u16,
            Opcode64::Memory(op) => 0x0200 + op.as_u8() as u16,
            Opcode64::SystemCall(op) => 0x0300 + op.as_u8() as u16,
            Opcode64::Crypto(op) => 0x0400 + op.as_u8() as u16,
        }
    }

    /// Convert from numerical value back to opcode
    pub fn from_u16(value: u16) -> Option<Self> {
        match value & 0xFF00 {
            0x0000 => ArithmeticOpcode::from_u8((value & 0xFF) as u8).map(Opcode64::Arithmetic),
            0x0100 => ControlFlowOpcode::from_u8((value & 0xFF) as u8).map(Opcode64::ControlFlow),
            0x0200 => MemoryOpcode::from_u8((value & 0xFF) as u8).map(Opcode64::Memory),
            0x0300 => SystemCallOpcode::from_u8((value & 0xFF) as u8).map(Opcode64::SystemCall),
            0x0400 => CryptoOpcode::from_u8((value & 0xFF) as u8).map(Opcode64::Crypto),
            _ => None,
        }
    }

    /// Get human-readable mnemonic for the opcode
    pub fn mnemonic(&self) -> String {
        match self {
            Opcode64::Arithmetic(op) => format!("ARITH.{}", op.to_mnemonic()),
            Opcode64::ControlFlow(op) => format!("CTRL.{}", op.to_mnemonic()),
            Opcode64::Memory(op) => format!("MEM.{}", op.to_mnemonic()),
            Opcode64::SystemCall(op) => format!("SYS.{}", op.to_mnemonic()),
            Opcode64::Crypto(op) => format!("CRYPTO.{}", op.to_mnemonic()),
        }
    }
}

impl fmt::Display for Opcode64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.mnemonic())
    }
}

/// 128-bit architecture opcodes
/// Includes all 64-bit opcodes plus big integer operations
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Opcode128 {
    /// All base 64-bit opcodes are available
    Base(Opcode64),
    /// Big integer arithmetic operations for 128-bit values
    BigInt(BigIntOpcode),
}

impl Opcode128 {
    /// Get the opcode's numerical value for bytecode generation
    pub fn as_u16(&self) -> u16 {
        match self {
            Opcode128::Base(op) => op.as_u16(),
            Opcode128::BigInt(op) => 0x1000 + op.as_u8() as u16,
        }
    }

    /// Convert from numerical value back to opcode
    pub fn from_u16(value: u16) -> Option<Self> {
        match value & 0xF000 {
            0x0000..=0x0FFF => Opcode64::from_u16(value).map(Opcode128::Base),
            0x1000 => BigIntOpcode::from_u8((value & 0xFF) as u8).map(Opcode128::BigInt),
            _ => None,
        }
    }

    /// Get human-readable mnemonic for the opcode
    pub fn mnemonic(&self) -> String {
        match self {
            Opcode128::Base(op) => op.mnemonic(),
            Opcode128::BigInt(op) => format!("BIGINT.{op}"),
        }
    }

    /// Check if this opcode is available in 64-bit architecture
    pub fn is_64bit_compatible(&self) -> bool {
        matches!(self, Opcode128::Base(_))
    }
}

impl fmt::Display for Opcode128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.mnemonic())
    }
}

/// 256-bit architecture opcodes
/// Includes all 128-bit opcodes plus SIMD and advanced mathematical operations
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Opcode256 {
    /// All base 128-bit opcodes are available
    Base(Opcode128),
    /// SIMD operations for 256-bit vectors
    Simd(SimdOpcode),
    /// Advanced mathematical operations
    Math(MathOpcode),
}

impl Opcode256 {
    /// Get the opcode's numerical value for bytecode generation
    pub fn as_u16(&self) -> u16 {
        match self {
            Opcode256::Base(op) => op.as_u16(),
            Opcode256::Simd(op) => 0x2000 + *op as u16,
            Opcode256::Math(op) => 0x3000 + *op as u16,
        }
    }

    /// Convert from numerical value back to opcode
    pub fn from_u16(value: u16) -> Option<Self> {
        match value & 0xF000 {
            0x0000..=0x1FFF => Opcode128::from_u16(value).map(Opcode256::Base),
            0x2000 => Some(Opcode256::Simd(SimdOpcode::from((value & 0xFF) as u8))),
            0x3000 => Some(Opcode256::Math(MathOpcode::from((value & 0xFF) as u8))),
            _ => None,
        }
    }

    /// Get human-readable mnemonic for the opcode
    pub fn mnemonic(&self) -> String {
        match self {
            Opcode256::Base(op) => op.mnemonic(),
            Opcode256::Simd(op) => format!("SIMD.{op}"),
            Opcode256::Math(op) => format!("MATH.{op}"),
        }
    }

    /// Check if this opcode is available in 128-bit architecture
    pub fn is_128bit_compatible(&self) -> bool {
        matches!(self, Opcode256::Base(_))
    }
}

impl fmt::Display for Opcode256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.mnemonic())
    }
}

/// 512-bit architecture opcodes
/// Includes all 256-bit opcodes plus vector processing and parallel operations
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Opcode512 {
    /// All base 256-bit opcodes are available
    Base(Opcode256),
    /// Vector processing operations for 512-bit vectors
    Vector(VectorOpcode),
    /// Parallel processing operations
    Parallel(ParallelOpcode),
    /// Advanced cryptographic operations
    Crypto(CryptoOpcode),
}

impl Opcode512 {
    /// Get the opcode's numerical value for bytecode generation
    pub fn as_u16(&self) -> u16 {
        match self {
            Opcode512::Base(op) => op.as_u16(),
            Opcode512::Vector(op) => 0x4000 + *op as u16,
            Opcode512::Parallel(op) => 0x5000 + *op as u16,
            Opcode512::Crypto(op) => 0x6000 + *op as u16,
        }
    }

    /// Convert from numerical value back to opcode
    pub fn from_u16(value: u16) -> Option<Self> {
        match value & 0xF000 {
            0x0000..=0x3FFF => Opcode256::from_u16(value).map(Opcode512::Base),
            0x4000 => Some(Opcode512::Vector(VectorOpcode::from((value & 0xFF) as u8))),
            0x5000 => Some(Opcode512::Parallel(ParallelOpcode::from((value & 0xFF) as u8))),
            0x6000 => CryptoOpcode::from_u8((value & 0xFF) as u8).map(Opcode512::Crypto),
            _ => None,
        }
    }

    /// Get human-readable mnemonic for the opcode
    pub fn mnemonic(&self) -> String {
        match self {
            Opcode512::Base(op) => op.mnemonic(),
            Opcode512::Vector(op) => format!("VEC.{op}"),
            Opcode512::Parallel(op) => format!("PAR.{op}"),
            Opcode512::Crypto(op) => format!("CRYPTO.{op}"),
        }
    }

    /// Check if this opcode is available in 256-bit architecture
    pub fn is_256bit_compatible(&self) -> bool {
        matches!(self, Opcode512::Base(_))
    }
}

impl fmt::Display for Opcode512 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.mnemonic())
    }
}

/// Architecture trait for opcode support
pub trait ArchitectureOpcodes {
    type Opcode: Clone + Copy + fmt::Debug + fmt::Display;

    /// Check if an opcode is supported by this architecture
    fn supports_opcode(opcode: &Self::Opcode) -> bool;

    /// Get the maximum opcode value for this architecture
    fn max_opcode_value() -> u16;

    /// Get architecture name
    fn architecture_name() -> &'static str;
}

/// 64-bit architecture opcode support
pub struct Arch64Opcodes;

impl ArchitectureOpcodes for Arch64Opcodes {
    type Opcode = Opcode64;

    fn supports_opcode(_opcode: &Self::Opcode) -> bool {
        true // All Opcode64 variants are supported
    }

    fn max_opcode_value() -> u16 {
        0x04FF // Crypto opcodes are the highest in 64-bit
    }

    fn architecture_name() -> &'static str {
        "64-bit"
    }
}

/// 128-bit architecture opcode support
pub struct Arch128Opcodes;

impl ArchitectureOpcodes for Arch128Opcodes {
    type Opcode = Opcode128;

    fn supports_opcode(_opcode: &Self::Opcode) -> bool {
        true // All Opcode128 variants are supported
    }

    fn max_opcode_value() -> u16 {
        0x10FF // BigInt opcodes are the highest in 128-bit
    }

    fn architecture_name() -> &'static str {
        "128-bit"
    }
}

/// 256-bit architecture opcode support
pub struct Arch256Opcodes;

impl ArchitectureOpcodes for Arch256Opcodes {
    type Opcode = Opcode256;

    fn supports_opcode(_opcode: &Self::Opcode) -> bool {
        true // All Opcode256 variants are supported
    }

    fn max_opcode_value() -> u16 {
        0x3FFF // Math opcodes are the highest in 256-bit
    }

    fn architecture_name() -> &'static str {
        "256-bit"
    }
}

/// 512-bit architecture opcode support
pub struct Arch512Opcodes;

impl ArchitectureOpcodes for Arch512Opcodes {
    type Opcode = Opcode512;

    fn supports_opcode(_opcode: &Self::Opcode) -> bool {
        true // All Opcode512 variants are supported
    }

    fn max_opcode_value() -> u16 {
        0x6FFF // Crypto opcodes are the highest in 512-bit
    }

    fn architecture_name() -> &'static str {
        "512-bit"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode64_encoding() {
        let add_op = Opcode64::Arithmetic(ArithmeticOpcode::Add);
        let encoded = add_op.as_u16();
        let decoded = Opcode64::from_u16(encoded);
        assert_eq!(Some(add_op), decoded);
    }

    #[test]
    fn test_opcode128_encoding() {
        let bigint_add = Opcode128::BigInt(BigIntOpcode::Add);
        let encoded = bigint_add.as_u16();
        let decoded = Opcode128::from_u16(encoded);
        assert_eq!(Some(bigint_add), decoded);
    }

    #[test]
    fn test_opcode128_base_compatibility() {
        let base_op = Opcode128::Base(Opcode64::Arithmetic(ArithmeticOpcode::Add));
        assert!(base_op.is_64bit_compatible());

        let bigint_op = Opcode128::BigInt(BigIntOpcode::Add);
        assert!(!bigint_op.is_64bit_compatible());
    }

    #[test]
    fn test_bigint_opcode_mnemonics() {
        assert_eq!(BigIntOpcode::Add.to_mnemonic(), "ADD");
        assert_eq!(BigIntOpcode::from_mnemonic("ADD"), Some(BigIntOpcode::Add));
        assert_eq!(BigIntOpcode::from_mnemonic("INVALID"), None);
    }

    #[test]
    fn test_architecture_opcodes_traits() {
        assert_eq!(Arch64Opcodes::architecture_name(), "64-bit");
        assert_eq!(Arch128Opcodes::architecture_name(), "128-bit");
        assert!(Arch64Opcodes::max_opcode_value() < Arch128Opcodes::max_opcode_value());
    }
}
