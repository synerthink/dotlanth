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

use std::fmt;

/// Enum representing the arithmetic opcodes.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ArithmeticOpcode {
    Add = 0x01,
    Subtract = 0x02,
    Multiply = 0x03,
    Divide = 0x04,
    Modulus = 0x05,
}

impl ArithmeticOpcode {
    /// Converts a mnemonic to an `ArithmeticOpcode`.
    pub fn from_mnemonic(mnemonic: &str) -> Option<Self> {
        match mnemonic.to_uppercase().as_str() {
            "ADD" => Some(Self::Add),
            "SUB" => Some(Self::Subtract),
            "MUL" => Some(Self::Multiply),
            "DIV" => Some(Self::Divide),
            "MOD" => Some(Self::Modulus),
            _ => None,
        }
    }

    /// Converts an `ArithmeticOpcode` to its mnemonic.
    pub fn to_mnemonic(&self) -> &'static str {
        match self {
            ArithmeticOpcode::Add => "ADD",
            ArithmeticOpcode::Subtract => "SUB",
            ArithmeticOpcode::Multiply => "MUL",
            ArithmeticOpcode::Divide => "DIV",
            ArithmeticOpcode::Modulus => "MOD",
        }
    }

    /// Returns the opcode's numerical value.
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

impl fmt::Display for ArithmeticOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_mnemonic())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_mnemonic_valid() {
        assert_eq!(
            ArithmeticOpcode::from_mnemonic("ADD"),
            Some(ArithmeticOpcode::Add)
        );
        assert_eq!(
            ArithmeticOpcode::from_mnemonic("sub"),
            Some(ArithmeticOpcode::Subtract)
        );
        assert_eq!(
            ArithmeticOpcode::from_mnemonic("MuL"),
            Some(ArithmeticOpcode::Multiply)
        );
        assert_eq!(
            ArithmeticOpcode::from_mnemonic("div"),
            Some(ArithmeticOpcode::Divide)
        );
        assert_eq!(
            ArithmeticOpcode::from_mnemonic("MOD"),
            Some(ArithmeticOpcode::Modulus)
        );
    }

    #[test]
    fn test_from_mnemonic_invalid() {
        assert_eq!(ArithmeticOpcode::from_mnemonic("UNKNOWN"), None);
        assert_eq!(ArithmeticOpcode::from_mnemonic(""), None);
        assert_eq!(ArithmeticOpcode::from_mnemonic("ADDX"), None);
        assert_eq!(ArithmeticOpcode::from_mnemonic("SUBTRACT"), None);
        assert_eq!(ArithmeticOpcode::from_mnemonic("multiplication"), None);
    }

    #[test]
    fn test_to_mnemonic() {
        assert_eq!(ArithmeticOpcode::Add.to_mnemonic(), "ADD");
        assert_eq!(ArithmeticOpcode::Subtract.to_mnemonic(), "SUB");
        assert_eq!(ArithmeticOpcode::Multiply.to_mnemonic(), "MUL");
        assert_eq!(ArithmeticOpcode::Divide.to_mnemonic(), "DIV");
        assert_eq!(ArithmeticOpcode::Modulus.to_mnemonic(), "MOD");
    }

    #[test]
    fn test_display_trait() {
        assert_eq!(ArithmeticOpcode::Add.to_string(), "ADD");
        assert_eq!(ArithmeticOpcode::Subtract.to_string(), "SUB");
        assert_eq!(ArithmeticOpcode::Multiply.to_string(), "MUL");
        assert_eq!(ArithmeticOpcode::Divide.to_string(), "DIV");
        assert_eq!(ArithmeticOpcode::Modulus.to_string(), "MOD");
    }

    #[test]
    fn test_opcode_values() {
        assert_eq!(ArithmeticOpcode::Add as u8, 0x01);
        assert_eq!(ArithmeticOpcode::Subtract as u8, 0x02);
        assert_eq!(ArithmeticOpcode::Multiply as u8, 0x03);
        assert_eq!(ArithmeticOpcode::Divide as u8, 0x04);
        assert_eq!(ArithmeticOpcode::Modulus as u8, 0x05);
    }
}
