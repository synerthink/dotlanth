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

use super::{arithmetic_opcodes::ArithmeticOpcode, control_flow_opcodes::ControlFlowOpcode, crypto_opcodes::CryptoOpcode, memory_opcodes::MemoryOpcode, system_call_opcodes::SystemCallOpcode};
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
            Opcode64::Arithmetic(op) => 0x0000 + op.as_u8() as u16,
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
            Opcode128::BigInt(op) => format!("BIGINT.{}", op.to_mnemonic()),
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

/// Big integer opcodes for 128-bit+ architectures
/// These operations work with arbitrary precision integers
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum BigIntOpcode {
    /// Add two big integers
    Add = 0x01,
    /// Subtract two big integers
    Subtract = 0x02,
    /// Multiply two big integers
    Multiply = 0x03,
    /// Divide two big integers
    Divide = 0x04,
    /// Modulus operation on big integers
    Modulus = 0x05,
    /// Power operation (base^exponent)
    Power = 0x06,
    /// Square root of big integer
    SquareRoot = 0x07,
    /// Greatest common divisor
    Gcd = 0x08,
    /// Least common multiple
    Lcm = 0x09,
    /// Convert from regular integer to big integer
    FromInt = 0x0A,
    /// Convert big integer to regular integer (with overflow check)
    ToInt = 0x0B,
    /// Compare two big integers (-1, 0, 1)
    Compare = 0x0C,
    /// Check if big integer is zero
    IsZero = 0x0D,
    /// Check if big integer is negative
    IsNegative = 0x0E,
    /// Absolute value of big integer
    Abs = 0x0F,
}

impl BigIntOpcode {
    /// Convert from mnemonic string to opcode
    pub fn from_mnemonic(mnemonic: &str) -> Option<Self> {
        match mnemonic.to_uppercase().as_str() {
            "ADD" => Some(Self::Add),
            "SUB" => Some(Self::Subtract),
            "MUL" => Some(Self::Multiply),
            "DIV" => Some(Self::Divide),
            "MOD" => Some(Self::Modulus),
            "POW" => Some(Self::Power),
            "SQRT" => Some(Self::SquareRoot),
            "GCD" => Some(Self::Gcd),
            "LCM" => Some(Self::Lcm),
            "FROMINT" => Some(Self::FromInt),
            "TOINT" => Some(Self::ToInt),
            "CMP" => Some(Self::Compare),
            "ISZERO" => Some(Self::IsZero),
            "ISNEG" => Some(Self::IsNegative),
            "ABS" => Some(Self::Abs),
            _ => None,
        }
    }

    /// Convert opcode to mnemonic string
    pub fn to_mnemonic(&self) -> &'static str {
        match self {
            BigIntOpcode::Add => "ADD",
            BigIntOpcode::Subtract => "SUB",
            BigIntOpcode::Multiply => "MUL",
            BigIntOpcode::Divide => "DIV",
            BigIntOpcode::Modulus => "MOD",
            BigIntOpcode::Power => "POW",
            BigIntOpcode::SquareRoot => "SQRT",
            BigIntOpcode::Gcd => "GCD",
            BigIntOpcode::Lcm => "LCM",
            BigIntOpcode::FromInt => "FROMINT",
            BigIntOpcode::ToInt => "TOINT",
            BigIntOpcode::Compare => "CMP",
            BigIntOpcode::IsZero => "ISZERO",
            BigIntOpcode::IsNegative => "ISNEG",
            BigIntOpcode::Abs => "ABS",
        }
    }

    /// Get the opcode's numerical value
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Convert from numerical value back to opcode
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::Add),
            0x02 => Some(Self::Subtract),
            0x03 => Some(Self::Multiply),
            0x04 => Some(Self::Divide),
            0x05 => Some(Self::Modulus),
            0x06 => Some(Self::Power),
            0x07 => Some(Self::SquareRoot),
            0x08 => Some(Self::Gcd),
            0x09 => Some(Self::Lcm),
            0x0A => Some(Self::FromInt),
            0x0B => Some(Self::ToInt),
            0x0C => Some(Self::Compare),
            0x0D => Some(Self::IsZero),
            0x0E => Some(Self::IsNegative),
            0x0F => Some(Self::Abs),
            _ => None,
        }
    }
}

impl fmt::Display for BigIntOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_mnemonic())
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
