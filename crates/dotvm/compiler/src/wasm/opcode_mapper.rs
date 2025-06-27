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

//! WebAssembly to DotVM opcode mapping
//!
//! This module defines the mapping between WebAssembly instructions and DotVM opcodes.
//! It supports architecture-aware mapping where higher architectures can handle
//! more complex operations.

use crate::wasm::ast::WasmInstruction;
use dotvm_core::bytecode::VmArchitecture;
use dotvm_core::opcode::{
    architecture_opcodes::{Opcode64, Opcode128, Opcode256, Opcode512},
    arithmetic_opcodes::ArithmeticOpcode,
    control_flow_opcodes::ControlFlowOpcode,
    memory_opcodes::MemoryOpcode,
    system_call_opcodes::SystemCallOpcode,
};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during opcode mapping
#[derive(Error, Debug)]
pub enum OpcodeMappingError {
    #[error("Unsupported WASM instruction for architecture {arch:?}: {instruction:?}")]
    UnsupportedInstruction { instruction: String, arch: VmArchitecture },
    #[error("Invalid operand for instruction {instruction}: {reason}")]
    InvalidOperand { instruction: String, reason: String },
    #[error("Architecture {arch:?} cannot handle instruction {instruction}")]
    IncompatibleArchitecture { instruction: String, arch: VmArchitecture },
}

/// Represents a mapped DotVM instruction with operands
#[derive(Debug, Clone)]
pub struct MappedInstruction {
    /// The DotVM opcode
    pub opcode: MappedOpcode,
    /// Operands for the instruction
    pub operands: Vec<u64>,
    /// Additional metadata
    pub metadata: InstructionMetadata,
}

/// Architecture-aware opcode representation
#[derive(Debug, Clone)]
pub enum MappedOpcode {
    Arch64(Opcode64),
    Arch128(Opcode128),
    Arch256(Opcode256),
    Arch512(Opcode512),
}

impl MappedOpcode {
    /// Get the numerical value of the opcode for bytecode generation
    pub fn as_u16(&self) -> u16 {
        match self {
            MappedOpcode::Arch64(op) => op.as_u16(),
            MappedOpcode::Arch128(op) => op.as_u16(),
            MappedOpcode::Arch256(op) => op.as_u16(),
            MappedOpcode::Arch512(op) => op.as_u16(),
        }
    }

    /// Get the target architecture for this opcode
    pub fn target_architecture(&self) -> VmArchitecture {
        match self {
            MappedOpcode::Arch64(_) => VmArchitecture::Arch64,
            MappedOpcode::Arch128(_) => VmArchitecture::Arch128,
            MappedOpcode::Arch256(_) => VmArchitecture::Arch256,
            MappedOpcode::Arch512(_) => VmArchitecture::Arch512,
        }
    }
}

/// Additional metadata for mapped instructions
#[derive(Debug, Clone, Default)]
pub struct InstructionMetadata {
    /// Whether this instruction requires special handling
    pub requires_special_handling: bool,
    /// Stack effect (items consumed, items produced)
    pub stack_effect: (u32, u32),
    /// Memory access pattern if applicable
    pub memory_access: Option<MemoryAccess>,
    /// Control flow information
    pub control_flow: Option<ControlFlowInfo>,
}

/// Memory access pattern information
#[derive(Debug, Clone)]
pub struct MemoryAccess {
    pub offset: u64,
    pub align: u32,
    pub size: u32, // Size in bytes
}

/// Control flow information
#[derive(Debug, Clone)]
pub enum ControlFlowInfo {
    Branch { target: u32 },
    ConditionalBranch { target: u32 },
    Call { function_index: u32 },
    Return,
    Loop,
    Block,
}

/// WebAssembly to DotVM opcode mapper
pub struct OpcodeMapper {
    /// Target architecture for mapping
    target_architecture: VmArchitecture,
    /// Mapping cache for performance
    mapping_cache: HashMap<String, MappedInstruction>,
}

impl OpcodeMapper {
    /// Create a new opcode mapper for the specified architecture
    pub fn new(target_architecture: VmArchitecture) -> Self {
        Self {
            target_architecture,
            mapping_cache: HashMap::new(),
        }
    }

    /// Map a WASM instruction to DotVM opcode(s)
    pub fn map_instruction(&mut self, instruction: &WasmInstruction) -> Result<Vec<MappedInstruction>, OpcodeMappingError> {
        match instruction {
            // Control flow instructions
            WasmInstruction::Unreachable => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::SystemCall(SystemCallOpcode::TerminateProcess))?,
                operands: vec![],
                metadata: InstructionMetadata {
                    control_flow: Some(ControlFlowInfo::Return),
                    ..Default::default()
                },
            }]),

            WasmInstruction::Nop => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::ControlFlow(ControlFlowOpcode::Jump))?, // Use Jump with 0 offset as NOP
                operands: vec![0],                                                         // Jump to next instruction (effectively NOP)
                metadata: InstructionMetadata::default(),
            }]),

            WasmInstruction::Block { block_type: _ } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::ControlFlow(ControlFlowOpcode::IfElse))?, // Use IfElse for block structure
                operands: vec![],
                metadata: InstructionMetadata {
                    control_flow: Some(ControlFlowInfo::Block),
                    ..Default::default()
                },
            }]),

            WasmInstruction::Loop { block_type: _ } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::ControlFlow(ControlFlowOpcode::WhileLoop))?,
                operands: vec![],
                metadata: InstructionMetadata {
                    control_flow: Some(ControlFlowInfo::Loop),
                    ..Default::default()
                },
            }]),

            WasmInstruction::If { block_type: _ } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::ControlFlow(ControlFlowOpcode::IfElse))?,
                operands: vec![],
                metadata: InstructionMetadata {
                    stack_effect: (1, 0), // Consumes condition
                    control_flow: Some(ControlFlowInfo::ConditionalBranch { target: 0 }),
                    ..Default::default()
                },
            }]),

            WasmInstruction::Else => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::ControlFlow(ControlFlowOpcode::IfElse))?, // Part of IfElse structure
                operands: vec![],
                metadata: InstructionMetadata::default(),
            }]),

            WasmInstruction::End => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::ControlFlow(ControlFlowOpcode::Jump))?, // End block with jump
                operands: vec![0],                                                         // Jump to end of block
                metadata: InstructionMetadata::default(),
            }]),

            WasmInstruction::Br { label_index } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::ControlFlow(ControlFlowOpcode::Jump))?,
                operands: vec![*label_index as u64],
                metadata: InstructionMetadata {
                    control_flow: Some(ControlFlowInfo::Branch { target: *label_index }),
                    ..Default::default()
                },
            }]),

            WasmInstruction::BrIf { label_index } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::ControlFlow(ControlFlowOpcode::IfElse))?, // Use IfElse for conditional jump
                operands: vec![*label_index as u64],
                metadata: InstructionMetadata {
                    stack_effect: (1, 0), // Consumes condition
                    control_flow: Some(ControlFlowInfo::ConditionalBranch { target: *label_index }),
                    ..Default::default()
                },
            }]),

            WasmInstruction::Return => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::SystemCall(SystemCallOpcode::TerminateProcess))?, // Use terminate for return
                operands: vec![],
                metadata: InstructionMetadata {
                    control_flow: Some(ControlFlowInfo::Return),
                    ..Default::default()
                },
            }]),

            WasmInstruction::Call { function_index } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::SystemCall(SystemCallOpcode::CreateProcess))?, // Use create process for call
                operands: vec![*function_index as u64],
                metadata: InstructionMetadata {
                    control_flow: Some(ControlFlowInfo::Call { function_index: *function_index }),
                    ..Default::default()
                },
            }]),

            // Memory instructions
            WasmInstruction::I32Load { memarg } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Memory(MemoryOpcode::Load))?,
                operands: vec![memarg.offset, memarg.align as u64, 4], // size = 4 bytes
                metadata: InstructionMetadata {
                    stack_effect: (1, 1), // Consumes address, produces value
                    memory_access: Some(MemoryAccess {
                        offset: memarg.offset,
                        align: memarg.align,
                        size: 4,
                    }),
                    ..Default::default()
                },
            }]),

            WasmInstruction::I64Load { memarg } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Memory(MemoryOpcode::Load))?,
                operands: vec![memarg.offset, memarg.align as u64, 8], // size = 8 bytes
                metadata: InstructionMetadata {
                    stack_effect: (1, 1),
                    memory_access: Some(MemoryAccess {
                        offset: memarg.offset,
                        align: memarg.align,
                        size: 8,
                    }),
                    ..Default::default()
                },
            }]),

            WasmInstruction::I32Store { memarg } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Memory(MemoryOpcode::Store))?,
                operands: vec![memarg.offset, memarg.align as u64, 4], // size = 4 bytes
                metadata: InstructionMetadata {
                    stack_effect: (2, 0), // Consumes address and value
                    memory_access: Some(MemoryAccess {
                        offset: memarg.offset,
                        align: memarg.align,
                        size: 4,
                    }),
                    ..Default::default()
                },
            }]),

            WasmInstruction::I64Store { memarg } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Memory(MemoryOpcode::Store))?,
                operands: vec![memarg.offset, memarg.align as u64, 8], // size = 8 bytes
                metadata: InstructionMetadata {
                    stack_effect: (2, 0),
                    memory_access: Some(MemoryAccess {
                        offset: memarg.offset,
                        align: memarg.align,
                        size: 8,
                    }),
                    ..Default::default()
                },
            }]),

            // Arithmetic instructions
            WasmInstruction::I32Add => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Arithmetic(ArithmeticOpcode::Add))?,
                operands: vec![32], // 32-bit operation
                metadata: InstructionMetadata {
                    stack_effect: (2, 1), // Consumes two values, produces one
                    ..Default::default()
                },
            }]),

            WasmInstruction::I32Sub => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Arithmetic(ArithmeticOpcode::Subtract))?,
                operands: vec![32], // 32-bit operation
                metadata: InstructionMetadata {
                    stack_effect: (2, 1),
                    ..Default::default()
                },
            }]),

            WasmInstruction::I32Mul => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Arithmetic(ArithmeticOpcode::Multiply))?,
                operands: vec![32], // 32-bit operation
                metadata: InstructionMetadata {
                    stack_effect: (2, 1),
                    ..Default::default()
                },
            }]),

            WasmInstruction::I32DivS => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Arithmetic(ArithmeticOpcode::Divide))?,
                operands: vec![32, 1], // 32-bit signed operation
                metadata: InstructionMetadata {
                    stack_effect: (2, 1),
                    ..Default::default()
                },
            }]),

            WasmInstruction::I32DivU => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Arithmetic(ArithmeticOpcode::Divide))?,
                operands: vec![32, 0], // 32-bit unsigned operation
                metadata: InstructionMetadata {
                    stack_effect: (2, 1),
                    ..Default::default()
                },
            }]),

            WasmInstruction::I64Add => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Arithmetic(ArithmeticOpcode::Add))?,
                operands: vec![64], // 64-bit operation
                metadata: InstructionMetadata {
                    stack_effect: (2, 1),
                    ..Default::default()
                },
            }]),

            WasmInstruction::I64Sub => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Arithmetic(ArithmeticOpcode::Subtract))?,
                operands: vec![64], // 64-bit operation
                metadata: InstructionMetadata {
                    stack_effect: (2, 1),
                    ..Default::default()
                },
            }]),

            WasmInstruction::I64Mul => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Arithmetic(ArithmeticOpcode::Multiply))?,
                operands: vec![64], // 64-bit operation
                metadata: InstructionMetadata {
                    stack_effect: (2, 1),
                    ..Default::default()
                },
            }]),

            // Constants
            WasmInstruction::I32Const { value } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Memory(MemoryOpcode::Allocate))?, // Use allocate for const
                operands: vec![*value as u32 as u64, 32],                            // value and size
                metadata: InstructionMetadata {
                    stack_effect: (0, 1), // Produces one value
                    ..Default::default()
                },
            }]),

            WasmInstruction::I64Const { value } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Memory(MemoryOpcode::Allocate))?, // Use allocate for const
                operands: vec![*value as u64, 64],                                   // value and size
                metadata: InstructionMetadata {
                    stack_effect: (0, 1),
                    ..Default::default()
                },
            }]),

            // Variable access
            WasmInstruction::LocalGet { local_index } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Memory(MemoryOpcode::Load))?,
                operands: vec![*local_index as u64],
                metadata: InstructionMetadata {
                    stack_effect: (0, 1),
                    ..Default::default()
                },
            }]),

            WasmInstruction::LocalSet { local_index } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Memory(MemoryOpcode::Store))?,
                operands: vec![*local_index as u64],
                metadata: InstructionMetadata {
                    stack_effect: (1, 0),
                    ..Default::default()
                },
            }]),

            WasmInstruction::LocalTee { local_index } => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Memory(MemoryOpcode::Store))?,
                operands: vec![*local_index as u64],
                metadata: InstructionMetadata {
                    stack_effect: (1, 1), // Consumes and produces the same value
                    ..Default::default()
                },
            }]),

            // Stack manipulation
            WasmInstruction::Drop => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Memory(MemoryOpcode::Deallocate))?, // Use deallocate for drop
                operands: vec![],
                metadata: InstructionMetadata {
                    stack_effect: (1, 0),
                    ..Default::default()
                },
            }]),

            WasmInstruction::Select => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::ControlFlow(ControlFlowOpcode::IfElse))?, // Use IfElse for select
                operands: vec![],
                metadata: InstructionMetadata {
                    stack_effect: (3, 1), // Consumes condition and two values, produces one
                    ..Default::default()
                },
            }]),

            // Conversion instructions
            WasmInstruction::F32ConvertI32S => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Arithmetic(ArithmeticOpcode::Add))?, // Use Add as placeholder for conversion
                operands: vec![32, 32],                                                 // Convert from i32 to f32
                metadata: InstructionMetadata {
                    stack_effect: (1, 1),
                    ..Default::default()
                },
            }]),

            WasmInstruction::F32Add => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Arithmetic(ArithmeticOpcode::Add))?,
                operands: vec![32], // 32-bit float operation
                metadata: InstructionMetadata {
                    stack_effect: (2, 1),
                    ..Default::default()
                },
            }]),

            WasmInstruction::MemorySize => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Memory(MemoryOpcode::Load))?, // Use Load as placeholder
                operands: vec![0],                                               // Memory index
                metadata: InstructionMetadata {
                    stack_effect: (0, 1),
                    ..Default::default()
                },
            }]),

            WasmInstruction::MemoryGrow => Ok(vec![MappedInstruction {
                opcode: self.map_to_arch(Opcode64::Memory(MemoryOpcode::Allocate))?, // Use Allocate for grow
                operands: vec![0],                                                   // Memory index
                metadata: InstructionMetadata {
                    stack_effect: (1, 1),
                    ..Default::default()
                },
            }]),

            // Add more instruction mappings as needed...
            _ => Err(OpcodeMappingError::UnsupportedInstruction {
                instruction: format!("{:?}", instruction),
                arch: self.target_architecture,
            }),
        }
    }

    /// Map a base opcode to the target architecture
    fn map_to_arch(&self, base_opcode: Opcode64) -> Result<MappedOpcode, OpcodeMappingError> {
        match self.target_architecture {
            VmArchitecture::Arch64 => Ok(MappedOpcode::Arch64(base_opcode)),
            VmArchitecture::Arch128 => Ok(MappedOpcode::Arch128(Opcode128::Base(base_opcode))),
            VmArchitecture::Arch256 => Ok(MappedOpcode::Arch256(Opcode256::Base(Opcode128::Base(base_opcode)))),
            VmArchitecture::Arch512 => Ok(MappedOpcode::Arch512(Opcode512::Base(Opcode256::Base(Opcode128::Base(base_opcode))))),
            VmArchitecture::Arch32 => Err(OpcodeMappingError::IncompatibleArchitecture {
                instruction: format!("{:?}", base_opcode),
                arch: self.target_architecture,
            }),
        }
    }

    /// Determine the minimum required architecture for a WASM instruction
    pub fn required_architecture(instruction: &WasmInstruction) -> VmArchitecture {
        match instruction {
            // Most basic WASM instructions can run on 64-bit architecture
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
            | WasmInstruction::I32Load { .. }
            | WasmInstruction::I64Load { .. }
            | WasmInstruction::I32Store { .. }
            | WasmInstruction::I64Store { .. }
            | WasmInstruction::LocalGet { .. }
            | WasmInstruction::LocalSet { .. }
            | WasmInstruction::LocalTee { .. }
            | WasmInstruction::Call { .. }
            | WasmInstruction::Return
            | WasmInstruction::Br { .. }
            | WasmInstruction::BrIf { .. }
            | WasmInstruction::Block { .. }
            | WasmInstruction::Loop { .. }
            | WasmInstruction::If { .. }
            | WasmInstruction::Else
            | WasmInstruction::End
            | WasmInstruction::Drop
            | WasmInstruction::Select
            | WasmInstruction::Nop
            | WasmInstruction::Unreachable => VmArchitecture::Arch64,

            // SIMD instructions might require higher architectures
            _ => VmArchitecture::Arch64, // Default to 64-bit for now
        }
    }

    /// Get comprehensive mapping statistics
    pub fn get_mapping_stats(&self) -> MappingStats {
        MappingStats {
            target_architecture: self.target_architecture,
            cache_size: self.mapping_cache.len(),
        }
    }

    /// Clear the mapping cache
    pub fn clear_cache(&mut self) {
        self.mapping_cache.clear();
    }
}

/// Statistics about the opcode mapping process
#[derive(Debug)]
pub struct MappingStats {
    pub target_architecture: VmArchitecture,
    pub cache_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mapper_creation() {
        let mapper = OpcodeMapper::new(VmArchitecture::Arch64);
        assert_eq!(mapper.target_architecture, VmArchitecture::Arch64);
        assert_eq!(mapper.mapping_cache.len(), 0);
    }

    #[test]
    fn test_basic_arithmetic_mapping() {
        let mut mapper = OpcodeMapper::new(VmArchitecture::Arch64);
        let instruction = WasmInstruction::I32Add;
        let result = mapper.map_instruction(&instruction).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].metadata.stack_effect, (2, 1));

        match &result[0].opcode {
            MappedOpcode::Arch64(Opcode64::Arithmetic(ArithmeticOpcode::Add)) => {}
            _ => panic!("Expected Add opcode"),
        }
    }

    #[test]
    fn test_memory_instruction_mapping() {
        let mut mapper = OpcodeMapper::new(VmArchitecture::Arch64);
        let instruction = WasmInstruction::I32Load {
            memarg: crate::wasm::ast::MemArg { offset: 4, align: 2 },
        };
        let result = mapper.map_instruction(&instruction).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].operands, vec![4, 2, 4]); // offset, align, size
        assert_eq!(result[0].metadata.stack_effect, (1, 1));
        assert!(result[0].metadata.memory_access.is_some());
    }

    #[test]
    fn test_required_architecture() {
        assert_eq!(OpcodeMapper::required_architecture(&WasmInstruction::I32Add), VmArchitecture::Arch64);
        assert_eq!(OpcodeMapper::required_architecture(&WasmInstruction::I64Mul), VmArchitecture::Arch64);
    }

    #[test]
    fn test_architecture_mapping() {
        let mut mapper64 = OpcodeMapper::new(VmArchitecture::Arch64);
        let mut mapper128 = OpcodeMapper::new(VmArchitecture::Arch128);

        let instruction = WasmInstruction::I32Add;

        let result64 = mapper64.map_instruction(&instruction).unwrap();
        let result128 = mapper128.map_instruction(&instruction).unwrap();

        assert_eq!(result64[0].opcode.target_architecture(), VmArchitecture::Arch64);
        assert_eq!(result128[0].opcode.target_architecture(), VmArchitecture::Arch128);
    }
}
