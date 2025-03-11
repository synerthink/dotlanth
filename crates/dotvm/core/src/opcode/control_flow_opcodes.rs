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

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ControlFlowOpcode {
    IfElse = 0x10,
    ForLoop = 0x11,
    WhileLoop = 0x12,
    DoWhileLoop = 0x13,
    Jump = 0x14,
}

impl ControlFlowOpcode {
    /// Converts a `ControlFlowOpcode` to its mnemonic.
    pub fn from_mnemonic(mnemonic: &str) -> Option<Self> {
        match mnemonic.to_uppercase().as_str() {
            "IFELSE" => Some(Self::IfElse),
            "FORLOOP" => Some(Self::ForLoop),
            "WHILELOOP" => Some(Self::WhileLoop),
            "DOWHILELOOP" => Some(Self::DoWhileLoop),
            "JUMP" => Some(Self::Jump),
            _ => None,
        }
    }

    /// Converts a `ControlFlowOpcode` to its mnemonic.
    pub fn to_mnemonic(&self) -> &'static str {
        match self {
            ControlFlowOpcode::IfElse => "IFELSE",
            ControlFlowOpcode::ForLoop => "FORLOOP",
            ControlFlowOpcode::WhileLoop => "WHILELOOP",
            ControlFlowOpcode::DoWhileLoop => "DOWHILELOOP",
            ControlFlowOpcode::Jump => "JUMP",
        }
    }

    /// Returns the opcode's numerical value.
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

impl fmt::Display for ControlFlowOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_mnemonic())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_mnemonic() {
        assert_eq!(ControlFlowOpcode::from_mnemonic("IFELSE"), Some(ControlFlowOpcode::IfElse));
        assert_eq!(ControlFlowOpcode::from_mnemonic("forloop"), Some(ControlFlowOpcode::ForLoop));
        assert_eq!(ControlFlowOpcode::from_mnemonic("WhileLoop"), Some(ControlFlowOpcode::WhileLoop));
        assert_eq!(ControlFlowOpcode::from_mnemonic("DoWhileLoop"), Some(ControlFlowOpcode::DoWhileLoop));
        assert_eq!(ControlFlowOpcode::from_mnemonic("jump"), Some(ControlFlowOpcode::Jump));
        assert_eq!(ControlFlowOpcode::from_mnemonic("unknown"), None);
    }

    #[test]
    fn test_to_mnemonic() {
        assert_eq!(ControlFlowOpcode::IfElse.to_mnemonic(), "IFELSE");
        assert_eq!(ControlFlowOpcode::ForLoop.to_mnemonic(), "FORLOOP");
        assert_eq!(ControlFlowOpcode::WhileLoop.to_mnemonic(), "WHILELOOP");
        assert_eq!(ControlFlowOpcode::DoWhileLoop.to_mnemonic(), "DOWHILELOOP");
        assert_eq!(ControlFlowOpcode::Jump.to_mnemonic(), "JUMP");
    }

    #[test]
    fn test_display() {
        assert_eq!(ControlFlowOpcode::IfElse.to_string(), "IFELSE");
        assert_eq!(ControlFlowOpcode::ForLoop.to_string(), "FORLOOP");
        assert_eq!(ControlFlowOpcode::WhileLoop.to_string(), "WHILELOOP");
        assert_eq!(ControlFlowOpcode::DoWhileLoop.to_string(), "DOWHILELOOP");
        assert_eq!(ControlFlowOpcode::Jump.to_string(), "JUMP");
    }

    #[test]
    fn test_opcode_values() {
        assert_eq!(ControlFlowOpcode::IfElse as u8, 0x10);
        assert_eq!(ControlFlowOpcode::ForLoop as u8, 0x11);
        assert_eq!(ControlFlowOpcode::WhileLoop as u8, 0x12);
        assert_eq!(ControlFlowOpcode::DoWhileLoop as u8, 0x13);
        assert_eq!(ControlFlowOpcode::Jump as u8, 0x14);
    }
}
