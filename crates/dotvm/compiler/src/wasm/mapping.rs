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

//! WASM to DotVM opcode mapping
//!
//! This module provides the new refactored opcode mapping system
//! with better separation of concerns and extensibility.

use super::{
    ast::WasmInstruction,
    error::{WasmError, WasmResult},
};
use dotvm_core::bytecode::VmArchitecture;

/// New opcode mapper with improved architecture
pub struct OpcodeMapper {
    /// Target architecture
    target_architecture: VmArchitecture,
}

impl OpcodeMapper {
    /// Create a new opcode mapper
    pub fn new(target_architecture: VmArchitecture) -> Self {
        Self { target_architecture }
    }

    /// Map a WASM instruction to DotVM opcodes
    pub fn map_instruction(&self, instruction: &WasmInstruction) -> WasmResult<Vec<MappedInstruction>> {
        // Placeholder implementation - would delegate to the old mapper for now
        // This allows for gradual migration
        // Default mapping: use the instruction name and operands where applicable
        // This covers basic instructions such as locals, constants, arithmetic, and simple control
        let opcode = instruction.name().to_string();
        let mapped = match instruction {
            // Variable access
            WasmInstruction::LocalGet { local_index } | WasmInstruction::LocalSet { local_index } | WasmInstruction::LocalTee { local_index } => vec![*local_index as u64],

            WasmInstruction::GlobalGet { global_index } | WasmInstruction::GlobalSet { global_index } => vec![*global_index as u64],

            // Constants
            WasmInstruction::I32Const { value } => vec![*value as u64],
            WasmInstruction::I64Const { value } => vec![*value as u64],
            WasmInstruction::F32Const { value } => vec![value.to_bits() as u64],
            WasmInstruction::F64Const { value } => vec![value.to_bits()],

            // Numeric operations without immediates
            inst if matches!(
                inst,
                WasmInstruction::I32Add
                    | WasmInstruction::I32Sub
                    | WasmInstruction::I32Mul
                    | WasmInstruction::I32DivS
                    | WasmInstruction::I32DivU
                    | WasmInstruction::I64Add
                    | WasmInstruction::I64Sub
                    | WasmInstruction::I64Mul
                    | WasmInstruction::I64DivS
                    | WasmInstruction::I64DivU
                    | WasmInstruction::F32Add
                    | WasmInstruction::F32Sub
                    | WasmInstruction::F32Mul
                    | WasmInstruction::F32Div
                    | WasmInstruction::F64Add
                    | WasmInstruction::F64Sub
                    | WasmInstruction::F64Mul
                    | WasmInstruction::F64Div
                    | WasmInstruction::F32ConvertI32S
                    | WasmInstruction::F32ConvertI32U
                    | WasmInstruction::F32ConvertI64S
                    | WasmInstruction::F32ConvertI64U
                    | WasmInstruction::F64ConvertI32S
                    | WasmInstruction::F64ConvertI32U
                    | WasmInstruction::F64ConvertI64S
                    | WasmInstruction::F64ConvertI64U
                    | WasmInstruction::I32WrapI64
                    | WasmInstruction::I64ExtendI32S
                    | WasmInstruction::I64ExtendI32U
            ) =>
            {
                vec![]
            }

            // Simple stack operations
            WasmInstruction::Drop | WasmInstruction::Select => vec![],

            // Control flow instructions
            WasmInstruction::Nop | WasmInstruction::End => vec![],

            // Unsupported or complex features
            _ => return Err(WasmError::unsupported_feature(format!("Instruction: {:?}", instruction))),
        };

        Ok(vec![MappedInstruction { opcode, operands: mapped }])
    }

    /// Get the required architecture for an instruction
    pub fn required_architecture(instruction: &WasmInstruction) -> VmArchitecture {
        match instruction {
            WasmInstruction::V128Load { .. } | WasmInstruction::V128Store { .. } => VmArchitecture::Arch128,
            _ => VmArchitecture::Arch64,
        }
    }
}

/// Mapped instruction result
#[derive(Debug, Clone)]
pub struct MappedInstruction {
    /// The opcode string
    pub opcode: String,
    /// Operands for the instruction
    pub operands: Vec<u64>,
}
