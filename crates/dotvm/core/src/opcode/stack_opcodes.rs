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

//! Stack Operation Opcodes
//!
//! This module defines opcodes for stack manipulation operations like PUSH and POP.

use std::fmt;

/// Stack operation opcodes
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum StackOpcode {
    /// Push a constant onto the stack
    /// Operand: constant_id (u32)
    Push = 0x10,

    /// Pop a value from the stack and discard it
    Pop = 0x11,

    /// Duplicate the top value on the stack
    Dup = 0x12,

    /// Swap the top two values on the stack
    Swap = 0x13,

    /// Push a null value onto the stack
    PushNull = 0x14,

    /// Push a boolean true value onto the stack
    PushTrue = 0x15,

    /// Push a boolean false value onto the stack
    PushFalse = 0x16,

    /// Push a small integer (-128 to 127) onto the stack
    /// Operand: value (i8)
    PushInt8 = 0x17,

    /// Push a 32-bit integer onto the stack
    /// Operand: value (i32)
    PushInt32 = 0x18,

    /// Push a 64-bit integer onto the stack
    /// Operand: value (i64)
    PushInt64 = 0x19,

    /// Push a 64-bit float onto the stack
    /// Operand: value (f64)
    PushFloat64 = 0x1A,

    /// Duplicate the value at depth n (0 = top, 1 = second from top, etc.)
    /// Operand: depth (u8)
    DupN = 0x1B,

    /// Rotate the top n values on the stack
    /// Operand: count (u8)
    Rotate = 0x1C,
}

impl StackOpcode {
    /// Convert from mnemonic string to opcode
    pub fn from_mnemonic(mnemonic: &str) -> Option<Self> {
        match mnemonic.to_uppercase().as_str() {
            "PUSH" => Some(Self::Push),
            "POP" => Some(Self::Pop),
            "DUP" => Some(Self::Dup),
            "SWAP" => Some(Self::Swap),
            "PUSH_NULL" => Some(Self::PushNull),
            "PUSH_TRUE" => Some(Self::PushTrue),
            "PUSH_FALSE" => Some(Self::PushFalse),
            "PUSH_INT8" => Some(Self::PushInt8),
            "PUSH_INT32" => Some(Self::PushInt32),
            "PUSH_INT64" => Some(Self::PushInt64),
            "PUSH_FLOAT64" => Some(Self::PushFloat64),
            "DUP_N" => Some(Self::DupN),
            "ROTATE" => Some(Self::Rotate),
            _ => None,
        }
    }

    /// Convert opcode to mnemonic string
    pub fn to_mnemonic(&self) -> &'static str {
        match self {
            StackOpcode::Push => "PUSH",
            StackOpcode::Pop => "POP",
            StackOpcode::Dup => "DUP",
            StackOpcode::Swap => "SWAP",
            StackOpcode::PushNull => "PUSH_NULL",
            StackOpcode::PushTrue => "PUSH_TRUE",
            StackOpcode::PushFalse => "PUSH_FALSE",
            StackOpcode::PushInt8 => "PUSH_INT8",
            StackOpcode::PushInt32 => "PUSH_INT32",
            StackOpcode::PushInt64 => "PUSH_INT64",
            StackOpcode::PushFloat64 => "PUSH_FLOAT64",
            StackOpcode::DupN => "DUP_N",
            StackOpcode::Rotate => "ROTATE",
        }
    }

    /// Get the opcode's numerical value
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Convert a numerical value back to a StackOpcode
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x10 => Some(Self::Push),
            0x11 => Some(Self::Pop),
            0x12 => Some(Self::Dup),
            0x13 => Some(Self::Swap),
            0x14 => Some(Self::PushNull),
            0x15 => Some(Self::PushTrue),
            0x16 => Some(Self::PushFalse),
            0x17 => Some(Self::PushInt8),
            0x18 => Some(Self::PushInt32),
            0x19 => Some(Self::PushInt64),
            0x1A => Some(Self::PushFloat64),
            0x1B => Some(Self::DupN),
            0x1C => Some(Self::Rotate),
            _ => None,
        }
    }

    /// Get the number of operand bytes this opcode expects
    pub fn operand_size(&self) -> usize {
        match self {
            StackOpcode::Push => 4, // u32 constant_id
            StackOpcode::Pop => 0,
            StackOpcode::Dup => 0,
            StackOpcode::Swap => 0,
            StackOpcode::PushNull => 0,
            StackOpcode::PushTrue => 0,
            StackOpcode::PushFalse => 0,
            StackOpcode::PushInt8 => 1,    // i8
            StackOpcode::PushInt32 => 4,   // i32
            StackOpcode::PushInt64 => 8,   // i64
            StackOpcode::PushFloat64 => 8, // f64
            StackOpcode::DupN => 1,        // u8 depth
            StackOpcode::Rotate => 1,      // u8 count
        }
    }

    /// Check if this opcode modifies the stack size
    pub fn modifies_stack_size(&self) -> bool {
        match self {
            StackOpcode::Push
            | StackOpcode::PushNull
            | StackOpcode::PushTrue
            | StackOpcode::PushFalse
            | StackOpcode::PushInt8
            | StackOpcode::PushInt32
            | StackOpcode::PushInt64
            | StackOpcode::PushFloat64
            | StackOpcode::Dup
            | StackOpcode::DupN => true, // These increase stack size
            StackOpcode::Pop => true,                         // This decreases stack size
            StackOpcode::Swap | StackOpcode::Rotate => false, // These don't change stack size
        }
    }

    /// Get the stack size change for this opcode
    pub fn stack_size_change(&self) -> i32 {
        match self {
            StackOpcode::Push
            | StackOpcode::PushNull
            | StackOpcode::PushTrue
            | StackOpcode::PushFalse
            | StackOpcode::PushInt8
            | StackOpcode::PushInt32
            | StackOpcode::PushInt64
            | StackOpcode::PushFloat64
            | StackOpcode::Dup
            | StackOpcode::DupN => 1, // Push one value
            StackOpcode::Pop => -1,                       // Pop one value
            StackOpcode::Swap | StackOpcode::Rotate => 0, // No net change
        }
    }

    /// Check if this opcode requires operands
    pub fn has_operands(&self) -> bool {
        self.operand_size() > 0
    }

    /// Get a description of what this opcode does
    pub fn description(&self) -> &'static str {
        match self {
            StackOpcode::Push => "Push a constant from the constant pool onto the stack",
            StackOpcode::Pop => "Pop the top value from the stack and discard it",
            StackOpcode::Dup => "Duplicate the top value on the stack",
            StackOpcode::Swap => "Swap the top two values on the stack",
            StackOpcode::PushNull => "Push a null value onto the stack",
            StackOpcode::PushTrue => "Push boolean true onto the stack",
            StackOpcode::PushFalse => "Push boolean false onto the stack",
            StackOpcode::PushInt8 => "Push a small integer (-128 to 127) onto the stack",
            StackOpcode::PushInt32 => "Push a 32-bit integer onto the stack",
            StackOpcode::PushInt64 => "Push a 64-bit integer onto the stack",
            StackOpcode::PushFloat64 => "Push a 64-bit floating point number onto the stack",
            StackOpcode::DupN => "Duplicate the value at the specified depth",
            StackOpcode::Rotate => "Rotate the top n values on the stack",
        }
    }
}

impl fmt::Display for StackOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_mnemonic())
    }
}

/// Stack instruction with operands
#[derive(Debug, Clone, PartialEq)]
pub struct StackInstruction {
    pub opcode: StackOpcode,
    pub operands: Vec<u8>,
}

impl StackInstruction {
    /// Create a new stack instruction
    pub fn new(opcode: StackOpcode, operands: Vec<u8>) -> Self {
        Self { opcode, operands }
    }

    /// Create a PUSH instruction
    pub fn push(constant_id: u32) -> Self {
        Self::new(StackOpcode::Push, constant_id.to_le_bytes().to_vec())
    }

    /// Create a POP instruction
    pub fn pop() -> Self {
        Self::new(StackOpcode::Pop, vec![])
    }

    /// Create a DUP instruction
    pub fn dup() -> Self {
        Self::new(StackOpcode::Dup, vec![])
    }

    /// Create a SWAP instruction
    pub fn swap() -> Self {
        Self::new(StackOpcode::Swap, vec![])
    }

    /// Create a PUSH_NULL instruction
    pub fn push_null() -> Self {
        Self::new(StackOpcode::PushNull, vec![])
    }

    /// Create a PUSH_TRUE instruction
    pub fn push_true() -> Self {
        Self::new(StackOpcode::PushTrue, vec![])
    }

    /// Create a PUSH_FALSE instruction
    pub fn push_false() -> Self {
        Self::new(StackOpcode::PushFalse, vec![])
    }

    /// Create a PUSH_INT8 instruction
    pub fn push_int8(value: i8) -> Self {
        Self::new(StackOpcode::PushInt8, vec![value as u8])
    }

    /// Create a PUSH_INT32 instruction
    pub fn push_int32(value: i32) -> Self {
        Self::new(StackOpcode::PushInt32, value.to_le_bytes().to_vec())
    }

    /// Create a PUSH_INT64 instruction
    pub fn push_int64(value: i64) -> Self {
        Self::new(StackOpcode::PushInt64, value.to_le_bytes().to_vec())
    }

    /// Create a PUSH_FLOAT64 instruction
    pub fn push_float64(value: f64) -> Self {
        Self::new(StackOpcode::PushFloat64, value.to_le_bytes().to_vec())
    }

    /// Create a DUP_N instruction
    pub fn dup_n(depth: u8) -> Self {
        Self::new(StackOpcode::DupN, vec![depth])
    }

    /// Create a ROTATE instruction
    pub fn rotate(count: u8) -> Self {
        Self::new(StackOpcode::Rotate, vec![count])
    }

    /// Encode this instruction to bytes
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = vec![self.opcode.as_u8()];
        bytes.extend_from_slice(&self.operands);
        bytes
    }

    /// Decode an instruction from bytes
    pub fn decode(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.is_empty() {
            return None;
        }

        let opcode = StackOpcode::from_u8(bytes[0])?;
        let operand_size = opcode.operand_size();

        if bytes.len() < 1 + operand_size {
            return None;
        }

        let operands = bytes[1..1 + operand_size].to_vec();
        let instruction = Self::new(opcode, operands);
        Some((instruction, 1 + operand_size))
    }

    /// Get the total size of this instruction in bytes
    pub fn size(&self) -> usize {
        1 + self.operands.len()
    }

    /// Validate that the operands are the correct size for this opcode
    pub fn validate(&self) -> bool {
        self.operands.len() == self.opcode.operand_size()
    }
}

impl fmt::Display for StackInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.operands.is_empty() {
            write!(f, "{}", self.opcode)
        } else {
            write!(f, "{} {:?}", self.opcode, self.operands)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_from_mnemonic() {
        assert_eq!(StackOpcode::from_mnemonic("PUSH"), Some(StackOpcode::Push));
        assert_eq!(StackOpcode::from_mnemonic("pop"), Some(StackOpcode::Pop));
        assert_eq!(StackOpcode::from_mnemonic("DuP"), Some(StackOpcode::Dup));
        assert_eq!(StackOpcode::from_mnemonic("UNKNOWN"), None);
    }

    #[test]
    fn test_opcode_to_mnemonic() {
        assert_eq!(StackOpcode::Push.to_mnemonic(), "PUSH");
        assert_eq!(StackOpcode::Pop.to_mnemonic(), "POP");
        assert_eq!(StackOpcode::Dup.to_mnemonic(), "DUP");
    }

    #[test]
    fn test_opcode_values() {
        assert_eq!(StackOpcode::Push as u8, 0x10);
        assert_eq!(StackOpcode::Pop as u8, 0x11);
        assert_eq!(StackOpcode::Dup as u8, 0x12);
        assert_eq!(StackOpcode::Swap as u8, 0x13);
    }

    #[test]
    fn test_opcode_from_u8() {
        assert_eq!(StackOpcode::from_u8(0x10), Some(StackOpcode::Push));
        assert_eq!(StackOpcode::from_u8(0x11), Some(StackOpcode::Pop));
        assert_eq!(StackOpcode::from_u8(0xFF), None);
    }

    #[test]
    fn test_operand_sizes() {
        assert_eq!(StackOpcode::Push.operand_size(), 4);
        assert_eq!(StackOpcode::Pop.operand_size(), 0);
        assert_eq!(StackOpcode::PushInt8.operand_size(), 1);
        assert_eq!(StackOpcode::PushInt32.operand_size(), 4);
        assert_eq!(StackOpcode::PushInt64.operand_size(), 8);
        assert_eq!(StackOpcode::PushFloat64.operand_size(), 8);
    }

    #[test]
    fn test_stack_size_changes() {
        assert_eq!(StackOpcode::Push.stack_size_change(), 1);
        assert_eq!(StackOpcode::Pop.stack_size_change(), -1);
        assert_eq!(StackOpcode::Dup.stack_size_change(), 1);
        assert_eq!(StackOpcode::Swap.stack_size_change(), 0);
    }

    #[test]
    fn test_instruction_creation() {
        let push_instr = StackInstruction::push(42);
        assert_eq!(push_instr.opcode, StackOpcode::Push);
        assert_eq!(push_instr.operands, 42u32.to_le_bytes().to_vec());

        let pop_instr = StackInstruction::pop();
        assert_eq!(pop_instr.opcode, StackOpcode::Pop);
        assert!(pop_instr.operands.is_empty());

        let int8_instr = StackInstruction::push_int8(-42);
        assert_eq!(int8_instr.opcode, StackOpcode::PushInt8);
        assert_eq!(int8_instr.operands, vec![(-42i8) as u8]);
    }

    #[test]
    fn test_instruction_encoding_decoding() {
        let original = StackInstruction::push(12345);
        let encoded = original.encode();

        let (decoded, size) = StackInstruction::decode(&encoded).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(size, encoded.len());

        // Test instruction without operands
        let pop_instr = StackInstruction::pop();
        let encoded = pop_instr.encode();
        let (decoded, size) = StackInstruction::decode(&encoded).unwrap();
        assert_eq!(decoded, pop_instr);
        assert_eq!(size, 1);
    }

    #[test]
    fn test_instruction_validation() {
        let valid_push = StackInstruction::push(42);
        assert!(valid_push.validate());

        let invalid_push = StackInstruction::new(StackOpcode::Push, vec![1, 2]); // Wrong size
        assert!(!invalid_push.validate());

        let valid_pop = StackInstruction::pop();
        assert!(valid_pop.validate());
    }

    #[test]
    fn test_instruction_display() {
        let push_instr = StackInstruction::push(42);
        let display = format!("{}", push_instr);
        assert!(display.contains("PUSH"));

        let pop_instr = StackInstruction::pop();
        let display = format!("{}", pop_instr);
        assert_eq!(display, "POP");
    }

    #[test]
    fn test_decode_insufficient_bytes() {
        let bytes = vec![0x10]; // PUSH opcode but no operands
        let result = StackInstruction::decode(&bytes);
        assert!(result.is_none());

        let empty_bytes = vec![];
        let result = StackInstruction::decode(&empty_bytes);
        assert!(result.is_none());
    }

    #[test]
    fn test_float64_instruction() {
        let value = 3.14159;
        let instr = StackInstruction::push_float64(value);
        assert_eq!(instr.opcode, StackOpcode::PushFloat64);
        assert_eq!(instr.operands, value.to_le_bytes().to_vec());

        let encoded = instr.encode();
        let (decoded, _) = StackInstruction::decode(&encoded).unwrap();
        assert_eq!(decoded, instr);
    }
}
