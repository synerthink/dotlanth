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

/// Enum representing memory opcodes.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum MemoryOpcode {
    Load = 0x20,
    Store = 0x21,
    Allocate = 0x22,
    Deallocate = 0x23,
    PointerOperation = 0x24,
}

impl MemoryOpcode {
    /// Converts a mnemonic to a `MemoryOpcode`.
    pub fn from_mnemonic(mnemonic: &str) -> Option<Self> {
        match mnemonic.to_uppercase().as_str() {
            "LOAD" => Some(Self::Load),
            "STORE" => Some(Self::Store),
            "ALLOCATE" => Some(Self::Allocate),
            "DEALLOCATE" => Some(Self::Deallocate),
            "POINTEROPERATION" => Some(Self::PointerOperation),
            _ => None,
        }
    }

    /// Converts a `MemoryOpcode` to its mnemonic.
    pub fn to_mnemonic(&self) -> &'static str {
        match self {
            MemoryOpcode::Load => "LOAD",
            MemoryOpcode::Store => "STORE",
            MemoryOpcode::Allocate => "ALLOCATE",
            MemoryOpcode::Deallocate => "DEALLOCATE",
            MemoryOpcode::PointerOperation => "POINTEROPERATION",
        }
    }

    /// Returns the opcode's numerical value.
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Converts a numerical value back to a MemoryOpcode.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x20 => Some(Self::Load),
            0x21 => Some(Self::Store),
            0x22 => Some(Self::Allocate),
            0x23 => Some(Self::Deallocate),
            0x24 => Some(Self::PointerOperation),
            _ => None,
        }
    }
}

/// Implement Display trait for MemoryOpcode.
impl fmt::Display for MemoryOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_mnemonic())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_values() {
        assert_eq!(MemoryOpcode::Load as u8, 0x20);
        assert_eq!(MemoryOpcode::Store as u8, 0x21);
        assert_eq!(MemoryOpcode::Allocate as u8, 0x22);
        assert_eq!(MemoryOpcode::Deallocate as u8, 0x23);
        assert_eq!(MemoryOpcode::PointerOperation as u8, 0x24);
    }

    #[test]
    fn test_mnemonic_conversions() {
        assert_eq!(MemoryOpcode::from_mnemonic("LOAD"), Some(MemoryOpcode::Load));
        assert_eq!(MemoryOpcode::from_mnemonic("store"), Some(MemoryOpcode::Store));
        assert_eq!(MemoryOpcode::from_mnemonic("Allocate"), Some(MemoryOpcode::Allocate));
        assert_eq!(MemoryOpcode::from_mnemonic("Deallocate"), Some(MemoryOpcode::Deallocate));
        assert_eq!(MemoryOpcode::from_mnemonic("pointeroperation"), Some(MemoryOpcode::PointerOperation));
        assert_eq!(MemoryOpcode::from_mnemonic("UNKNOWN"), None);
    }

    #[test]
    fn test_display_trait() {
        assert_eq!(MemoryOpcode::Load.to_string(), "LOAD");
        assert_eq!(MemoryOpcode::Store.to_string(), "STORE");
        assert_eq!(MemoryOpcode::Allocate.to_string(), "ALLOCATE");
        assert_eq!(MemoryOpcode::Deallocate.to_string(), "DEALLOCATE");
        assert_eq!(MemoryOpcode::PointerOperation.to_string(), "POINTEROPERATION");
    }
}
