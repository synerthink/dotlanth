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

//! BigInteger operation opcodes for DotVM
//!
//! This module defines opcodes for arbitrary precision integer arithmetic
//! operations that are available on 128-bit+ architectures.

use std::fmt;

/// BigInteger arithmetic opcodes for 128-bit+ architectures
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum BigIntOpcode {
    // Basic arithmetic operations
    Add = 0x01, // BigInt addition
    Sub = 0x02, // BigInt subtraction (also known as Subtract)
    Mul = 0x03, // BigInt multiplication (also known as Multiply)
    Div = 0x04, // BigInt division (also known as Divide)
    Mod = 0x05, // BigInt modulo

    // Power operations
    Pow = 0x10,    // BigInt exponentiation (also known as Power)
    ModPow = 0x11, // Modular exponentiation

    // Number theory operations
    Gcd = 0x20,        // Greatest common divisor
    Lcm = 0x21,        // Least common multiple
    ModInverse = 0x22, // Modular multiplicative inverse

    // Comparison operations
    Cmp = 0x30,    // Compare two BigInts (also known as Compare)
    IsZero = 0x31, // Check if BigInt is zero
    IsOne = 0x32,  // Check if BigInt is one
    IsEven = 0x33, // Check if BigInt is even
    IsOdd = 0x34,  // Check if BigInt is odd

    // Bitwise operations
    And = 0x40, // Bitwise AND
    Or = 0x41,  // Bitwise OR
    Xor = 0x42, // Bitwise XOR
    Not = 0x43, // Bitwise NOT
    Shl = 0x44, // Shift left
    Shr = 0x45, // Shift right

    // Conversion operations
    FromI64 = 0x50,    // Convert from i64
    ToI64 = 0x51,      // Convert to i64 (if possible)
    FromBytes = 0x52,  // Create from byte array
    ToBytes = 0x53,    // Convert to byte array
    FromString = 0x54, // Parse from string
    ToString = 0x55,   // Convert to string

    // Advanced operations
    Factorial = 0x60, // Factorial
    Fibonacci = 0x61, // Fibonacci number
    IsPrime = 0x62,   // Primality test
    NextPrime = 0x63, // Next prime number
    Random = 0x64,    // Generate random BigInt
    Abs = 0x65,       // Absolute value

    // Memory operations
    Load = 0x70,  // Load BigInt from memory
    Store = 0x71, // Store BigInt to memory
    Copy = 0x72,  // Copy BigInt
    Swap = 0x73,  // Swap two BigInts
}

// Compatibility aliases for common alternative names
impl BigIntOpcode {
    pub const Subtract: Self = Self::Sub;
    pub const Multiply: Self = Self::Mul;
    pub const Divide: Self = Self::Div;
    pub const Power: Self = Self::Pow;
    pub const Compare: Self = Self::Cmp;
}

impl BigIntOpcode {
    /// Get the number of operands this opcode expects
    pub fn operand_count(&self) -> usize {
        match self {
            // Unary operations
            BigIntOpcode::Not
            | BigIntOpcode::IsZero
            | BigIntOpcode::IsOne
            | BigIntOpcode::IsEven
            | BigIntOpcode::IsOdd
            | BigIntOpcode::FromI64
            | BigIntOpcode::ToI64
            | BigIntOpcode::FromBytes
            | BigIntOpcode::ToBytes
            | BigIntOpcode::FromString
            | BigIntOpcode::ToString
            | BigIntOpcode::Factorial
            | BigIntOpcode::Fibonacci
            | BigIntOpcode::IsPrime
            | BigIntOpcode::NextPrime
            | BigIntOpcode::Load
            | BigIntOpcode::Copy
            | BigIntOpcode::Abs => 1,

            // Binary operations
            BigIntOpcode::Add
            | BigIntOpcode::Sub
            | BigIntOpcode::Mul
            | BigIntOpcode::Div
            | BigIntOpcode::Mod
            | BigIntOpcode::Pow
            | BigIntOpcode::Gcd
            | BigIntOpcode::Lcm
            | BigIntOpcode::ModInverse
            | BigIntOpcode::Cmp
            | BigIntOpcode::And
            | BigIntOpcode::Or
            | BigIntOpcode::Xor
            | BigIntOpcode::Shl
            | BigIntOpcode::Shr
            | BigIntOpcode::Store
            | BigIntOpcode::Swap => 2,

            // Ternary operations
            BigIntOpcode::ModPow => 3,

            // Special operations
            BigIntOpcode::Random => 1, // Takes bit length as parameter
        }
    }

    /// Check if this operation modifies its operands
    pub fn is_mutating(&self) -> bool {
        matches!(self, BigIntOpcode::Store | BigIntOpcode::Swap)
    }

    /// Check if this operation is commutative
    pub fn is_commutative(&self) -> bool {
        matches!(
            self,
            BigIntOpcode::Add | BigIntOpcode::Mul | BigIntOpcode::Gcd | BigIntOpcode::Lcm | BigIntOpcode::And | BigIntOpcode::Or | BigIntOpcode::Xor
        )
    }

    /// Get the opcode's numerical value
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Convert from numerical value back to opcode
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(Self::from(value))
    }

    /// Convert opcode to mnemonic string
    pub fn to_mnemonic(&self) -> &'static str {
        match self {
            BigIntOpcode::Add => "ADD",
            BigIntOpcode::Sub => "SUB",
            BigIntOpcode::Mul => "MUL",
            BigIntOpcode::Div => "DIV",
            BigIntOpcode::Mod => "MOD",
            BigIntOpcode::Pow => "POW",
            BigIntOpcode::ModPow => "MODPOW",
            BigIntOpcode::Gcd => "GCD",
            BigIntOpcode::Lcm => "LCM",
            BigIntOpcode::ModInverse => "MODINV",
            BigIntOpcode::Cmp => "CMP",
            BigIntOpcode::IsZero => "ISZERO",
            BigIntOpcode::IsOne => "ISONE",
            BigIntOpcode::IsEven => "ISEVEN",
            BigIntOpcode::IsOdd => "ISODD",
            BigIntOpcode::And => "AND",
            BigIntOpcode::Or => "OR",
            BigIntOpcode::Xor => "XOR",
            BigIntOpcode::Not => "NOT",
            BigIntOpcode::Shl => "SHL",
            BigIntOpcode::Shr => "SHR",
            BigIntOpcode::FromI64 => "FROMI64",
            BigIntOpcode::ToI64 => "TOI64",
            BigIntOpcode::FromBytes => "FROMBYTES",
            BigIntOpcode::ToBytes => "TOBYTES",
            BigIntOpcode::FromString => "FROMSTRING",
            BigIntOpcode::ToString => "TOSTRING",
            BigIntOpcode::Factorial => "FACTORIAL",
            BigIntOpcode::Fibonacci => "FIBONACCI",
            BigIntOpcode::IsPrime => "ISPRIME",
            BigIntOpcode::NextPrime => "NEXTPRIME",
            BigIntOpcode::Random => "RANDOM",
            BigIntOpcode::Abs => "ABS",
            BigIntOpcode::Load => "LOAD",
            BigIntOpcode::Store => "STORE",
            BigIntOpcode::Copy => "COPY",
            BigIntOpcode::Swap => "SWAP",
        }
    }

    /// Convert mnemonic string to opcode
    pub fn from_mnemonic(mnemonic: &str) -> Option<Self> {
        match mnemonic {
            "ADD" => Some(BigIntOpcode::Add),
            "SUB" => Some(BigIntOpcode::Sub),
            "MUL" => Some(BigIntOpcode::Mul),
            "DIV" => Some(BigIntOpcode::Div),
            "MOD" => Some(BigIntOpcode::Mod),
            "POW" => Some(BigIntOpcode::Pow),
            "MODPOW" => Some(BigIntOpcode::ModPow),
            "GCD" => Some(BigIntOpcode::Gcd),
            "LCM" => Some(BigIntOpcode::Lcm),
            "MODINV" => Some(BigIntOpcode::ModInverse),
            "CMP" => Some(BigIntOpcode::Cmp),
            "ISZERO" => Some(BigIntOpcode::IsZero),
            "ISONE" => Some(BigIntOpcode::IsOne),
            "ISEVEN" => Some(BigIntOpcode::IsEven),
            "ISODD" => Some(BigIntOpcode::IsOdd),
            "AND" => Some(BigIntOpcode::And),
            "OR" => Some(BigIntOpcode::Or),
            "XOR" => Some(BigIntOpcode::Xor),
            "NOT" => Some(BigIntOpcode::Not),
            "SHL" => Some(BigIntOpcode::Shl),
            "SHR" => Some(BigIntOpcode::Shr),
            "FROMI64" => Some(BigIntOpcode::FromI64),
            "TOI64" => Some(BigIntOpcode::ToI64),
            "FROMBYTES" => Some(BigIntOpcode::FromBytes),
            "TOBYTES" => Some(BigIntOpcode::ToBytes),
            "FROMSTRING" => Some(BigIntOpcode::FromString),
            "TOSTRING" => Some(BigIntOpcode::ToString),
            "FACTORIAL" => Some(BigIntOpcode::Factorial),
            "FIBONACCI" => Some(BigIntOpcode::Fibonacci),
            "ISPRIME" => Some(BigIntOpcode::IsPrime),
            "NEXTPRIME" => Some(BigIntOpcode::NextPrime),
            "RANDOM" => Some(BigIntOpcode::Random),
            "ABS" => Some(BigIntOpcode::Abs),
            "LOAD" => Some(BigIntOpcode::Load),
            "STORE" => Some(BigIntOpcode::Store),
            "COPY" => Some(BigIntOpcode::Copy),
            "SWAP" => Some(BigIntOpcode::Swap),
            _ => None,
        }
    }

    /// Get the computational complexity category
    pub fn complexity(&self) -> ComputationalComplexity {
        match self {
            // O(1) operations
            BigIntOpcode::IsZero | BigIntOpcode::IsOne | BigIntOpcode::IsEven | BigIntOpcode::IsOdd | BigIntOpcode::Load | BigIntOpcode::Store | BigIntOpcode::Copy | BigIntOpcode::Swap => {
                ComputationalComplexity::Constant
            }

            // O(n) operations
            BigIntOpcode::Add
            | BigIntOpcode::Sub
            | BigIntOpcode::Cmp
            | BigIntOpcode::And
            | BigIntOpcode::Or
            | BigIntOpcode::Xor
            | BigIntOpcode::Not
            | BigIntOpcode::Shl
            | BigIntOpcode::Shr
            | BigIntOpcode::FromI64
            | BigIntOpcode::ToI64
            | BigIntOpcode::FromBytes
            | BigIntOpcode::ToBytes
            | BigIntOpcode::FromString
            | BigIntOpcode::ToString
            | BigIntOpcode::Abs => ComputationalComplexity::Linear,

            // O(n²) operations
            BigIntOpcode::Mul | BigIntOpcode::Div | BigIntOpcode::Mod => ComputationalComplexity::Quadratic,

            // O(n³) operations
            BigIntOpcode::Pow | BigIntOpcode::ModPow => ComputationalComplexity::Cubic,

            // Complex operations
            BigIntOpcode::Gcd | BigIntOpcode::Lcm | BigIntOpcode::ModInverse => ComputationalComplexity::Logarithmic,

            // Very complex operations
            BigIntOpcode::Factorial | BigIntOpcode::Fibonacci | BigIntOpcode::IsPrime | BigIntOpcode::NextPrime => ComputationalComplexity::Exponential,

            BigIntOpcode::Random => ComputationalComplexity::Linear,
        }
    }
}

/// Computational complexity categories for BigInt operations
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ComputationalComplexity {
    Constant,    // O(1)
    Logarithmic, // O(log n)
    Linear,      // O(n)
    Quadratic,   // O(n²)
    Cubic,       // O(n³)
    Exponential, // O(2^n) or worse
}

impl fmt::Display for BigIntOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BigIntOpcode::Add => write!(f, "bigint.add"),
            BigIntOpcode::Sub => write!(f, "bigint.sub"),
            BigIntOpcode::Mul => write!(f, "bigint.mul"),
            BigIntOpcode::Div => write!(f, "bigint.div"),
            BigIntOpcode::Mod => write!(f, "bigint.mod"),
            BigIntOpcode::Pow => write!(f, "bigint.pow"),
            BigIntOpcode::ModPow => write!(f, "bigint.modpow"),
            BigIntOpcode::Gcd => write!(f, "bigint.gcd"),
            BigIntOpcode::Lcm => write!(f, "bigint.lcm"),
            BigIntOpcode::ModInverse => write!(f, "bigint.modinv"),
            BigIntOpcode::Cmp => write!(f, "bigint.cmp"),
            BigIntOpcode::IsZero => write!(f, "bigint.iszero"),
            BigIntOpcode::IsOne => write!(f, "bigint.isone"),
            BigIntOpcode::IsEven => write!(f, "bigint.iseven"),
            BigIntOpcode::IsOdd => write!(f, "bigint.isodd"),
            BigIntOpcode::And => write!(f, "bigint.and"),
            BigIntOpcode::Or => write!(f, "bigint.or"),
            BigIntOpcode::Xor => write!(f, "bigint.xor"),
            BigIntOpcode::Not => write!(f, "bigint.not"),
            BigIntOpcode::Shl => write!(f, "bigint.shl"),
            BigIntOpcode::Shr => write!(f, "bigint.shr"),
            BigIntOpcode::FromI64 => write!(f, "bigint.fromi64"),
            BigIntOpcode::ToI64 => write!(f, "bigint.toi64"),
            BigIntOpcode::FromBytes => write!(f, "bigint.frombytes"),
            BigIntOpcode::ToBytes => write!(f, "bigint.tobytes"),
            BigIntOpcode::FromString => write!(f, "bigint.fromstring"),
            BigIntOpcode::ToString => write!(f, "bigint.tostring"),
            BigIntOpcode::Factorial => write!(f, "bigint.factorial"),
            BigIntOpcode::Fibonacci => write!(f, "bigint.fibonacci"),
            BigIntOpcode::IsPrime => write!(f, "bigint.isprime"),
            BigIntOpcode::NextPrime => write!(f, "bigint.nextprime"),
            BigIntOpcode::Random => write!(f, "bigint.random"),
            BigIntOpcode::Abs => write!(f, "bigint.abs"),
            BigIntOpcode::Load => write!(f, "bigint.load"),
            BigIntOpcode::Store => write!(f, "bigint.store"),
            BigIntOpcode::Copy => write!(f, "bigint.copy"),
            BigIntOpcode::Swap => write!(f, "bigint.swap"),
        }
    }
}

impl From<u8> for BigIntOpcode {
    fn from(value: u8) -> Self {
        match value {
            0x01 => BigIntOpcode::Add,
            0x02 => BigIntOpcode::Sub,
            0x03 => BigIntOpcode::Mul,
            0x04 => BigIntOpcode::Div,
            0x05 => BigIntOpcode::Mod,
            0x10 => BigIntOpcode::Pow,
            0x11 => BigIntOpcode::ModPow,
            0x20 => BigIntOpcode::Gcd,
            0x21 => BigIntOpcode::Lcm,
            0x22 => BigIntOpcode::ModInverse,
            0x30 => BigIntOpcode::Cmp,
            0x31 => BigIntOpcode::IsZero,
            0x32 => BigIntOpcode::IsOne,
            0x33 => BigIntOpcode::IsEven,
            0x34 => BigIntOpcode::IsOdd,
            0x40 => BigIntOpcode::And,
            0x41 => BigIntOpcode::Or,
            0x42 => BigIntOpcode::Xor,
            0x43 => BigIntOpcode::Not,
            0x44 => BigIntOpcode::Shl,
            0x45 => BigIntOpcode::Shr,
            0x50 => BigIntOpcode::FromI64,
            0x51 => BigIntOpcode::ToI64,
            0x52 => BigIntOpcode::FromBytes,
            0x53 => BigIntOpcode::ToBytes,
            0x54 => BigIntOpcode::FromString,
            0x55 => BigIntOpcode::ToString,
            0x60 => BigIntOpcode::Factorial,
            0x61 => BigIntOpcode::Fibonacci,
            0x62 => BigIntOpcode::IsPrime,
            0x63 => BigIntOpcode::NextPrime,
            0x64 => BigIntOpcode::Random,
            0x65 => BigIntOpcode::Abs,
            0x70 => BigIntOpcode::Load,
            0x71 => BigIntOpcode::Store,
            0x72 => BigIntOpcode::Copy,
            0x73 => BigIntOpcode::Swap,
            _ => BigIntOpcode::Add, // Default fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operand_counts() {
        assert_eq!(BigIntOpcode::Add.operand_count(), 2);
        assert_eq!(BigIntOpcode::Not.operand_count(), 1);
        assert_eq!(BigIntOpcode::ModPow.operand_count(), 3);
    }

    #[test]
    fn test_commutativity() {
        assert!(BigIntOpcode::Add.is_commutative());
        assert!(BigIntOpcode::Mul.is_commutative());
        assert!(!BigIntOpcode::Sub.is_commutative());
        assert!(!BigIntOpcode::Div.is_commutative());
    }

    #[test]
    fn test_complexity() {
        assert_eq!(BigIntOpcode::Add.complexity(), ComputationalComplexity::Linear);
        assert_eq!(BigIntOpcode::Mul.complexity(), ComputationalComplexity::Quadratic);
        assert_eq!(BigIntOpcode::IsPrime.complexity(), ComputationalComplexity::Exponential);
    }

    #[test]
    fn test_display() {
        assert_eq!(BigIntOpcode::Add.to_string(), "bigint.add");
        assert_eq!(BigIntOpcode::ModPow.to_string(), "bigint.modpow");
    }

    #[test]
    fn test_from_u8() {
        assert_eq!(BigIntOpcode::from(0x01), BigIntOpcode::Add);
        assert_eq!(BigIntOpcode::from(0x11), BigIntOpcode::ModPow);
        assert_eq!(BigIntOpcode::from(0x62), BigIntOpcode::IsPrime);
    }
}
