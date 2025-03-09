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

use super::arithmetic::ArithmeticInstruction;
use super::control_flow::{IfElseInstruction, JumpInstruction, LoopInstruction, LoopType};
use super::instruction::Instruction;
use super::memory::{
    AllocateInstruction, DeallocateInstruction, LoadInstruction, PointerOperationInstruction,
    PointerOperationType, StoreInstruction,
};
use crate::memory::{MemoryHandle, VirtualAddress};
use crate::opcode::arithmetic_opcodes::ArithmeticOpcode;
use crate::opcode::control_flow_opcodes::ControlFlowOpcode;
use crate::opcode::memory_opcodes::MemoryOpcode;
use crate::vm::errors::VMError;
use std::sync::Arc;

/// Enum representing all possible opcodes.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Opcode {
    Arithmetic(ArithmeticOpcode),
    ControlFlow(ControlFlowOpcode),
    Memory(MemoryOpcode),
}

/// Factory responsible for creating instructions.
pub struct InstructionFactory;

impl InstructionFactory {
    pub fn create_instruction(
        opcode: Opcode,
        args: Option<Vec<usize>>,
    ) -> Result<Arc<dyn Instruction>, VMError> {
        match opcode {
            Opcode::Arithmetic(arith_op) => Ok(Arc::new(ArithmeticInstruction::new(arith_op))),
            Opcode::ControlFlow(cf_op) => match cf_op {
                ControlFlowOpcode::IfElse => {
                    // Expecting one argument: false_branch index
                    if let Some(args) = args {
                        if args.len() != 1 {
                            return Err(VMError::InvalidInstructionArguments);
                        }
                        Ok(Arc::new(IfElseInstruction::new(args[0])))
                    } else {
                        Err(VMError::MissingInstructionArguments)
                    }
                }
                ControlFlowOpcode::Jump => {
                    // Expecting one argument: target index
                    if let Some(args) = args {
                        if args.len() != 1 {
                            return Err(VMError::InvalidInstructionArguments);
                        }
                        Ok(Arc::new(JumpInstruction::new(args[0])))
                    } else {
                        Err(VMError::MissingInstructionArguments)
                    }
                }
                ControlFlowOpcode::WhileLoop => {
                    // Expecting two arguments: condition_start and body_start
                    if let Some(args) = args {
                        if args.len() != 2 {
                            return Err(VMError::InvalidInstructionArguments);
                        }
                        Ok(Arc::new(LoopInstruction::new(
                            LoopType::WhileLoop,
                            args[0],
                            args[1],
                        )))
                    } else {
                        Err(VMError::MissingInstructionArguments)
                    }
                }
                ControlFlowOpcode::DoWhileLoop => {
                    // Expecting two arguments: condition_start and body_start
                    if let Some(args) = args {
                        if args.len() != 2 {
                            return Err(VMError::InvalidInstructionArguments);
                        }
                        Ok(Arc::new(LoopInstruction::new(
                            LoopType::DoWhileLoop,
                            args[0],
                            args[1],
                        )))
                    } else {
                        Err(VMError::MissingInstructionArguments)
                    }
                }
                ControlFlowOpcode::ForLoop => {
                    // Expecting three arguments: init, condition, increment
                    // For simplicity, consider condition_start and body_start
                    if let Some(args) = args {
                        if args.len() != 2 {
                            return Err(VMError::InvalidInstructionArguments);
                        }
                        Ok(Arc::new(LoopInstruction::new(
                            LoopType::ForLoop,
                            args[0],
                            args[1],
                        )))
                    } else {
                        Err(VMError::MissingInstructionArguments)
                    }
                }
            },
            Opcode::Memory(mem_op) => match mem_op {
                MemoryOpcode::Load => {
                    // Expecting one argument: memory address
                    if let Some(args) = args {
                        if args.len() != 1 {
                            return Err(VMError::InvalidInstructionArguments);
                        }
                        Ok(Arc::new(LoadInstruction::new(VirtualAddress(args[0]))))
                    } else {
                        Err(VMError::MissingInstructionArguments)
                    }
                }
                MemoryOpcode::Store => {
                    // Expecting one argument: memory address
                    if let Some(args) = args {
                        if args.len() != 1 {
                            return Err(VMError::InvalidInstructionArguments);
                        }
                        Ok(Arc::new(StoreInstruction::new(VirtualAddress(args[0]))))
                    } else {
                        Err(VMError::MissingInstructionArguments)
                    }
                }
                MemoryOpcode::Allocate => {
                    // Expecting one argument: size
                    if let Some(args) = args {
                        if args.len() != 1 {
                            return Err(VMError::InvalidInstructionArguments);
                        }
                        Ok(Arc::new(AllocateInstruction::new(args[0])))
                    } else {
                        Err(VMError::MissingInstructionArguments)
                    }
                }
                MemoryOpcode::Deallocate => {
                    // Expecting one argument: memory handle
                    if let Some(args) = args {
                        if args.len() != 1 {
                            return Err(VMError::InvalidInstructionArguments);
                        }
                        Ok(Arc::new(DeallocateInstruction::new(MemoryHandle(args[0]))))
                    } else {
                        Err(VMError::MissingInstructionArguments)
                    }
                }
                MemoryOpcode::PointerOperation => {
                    // Expecting two arguments: operation type and offset
                    if let Some(args) = args {
                        if args.len() != 2 {
                            return Err(VMError::InvalidInstructionArguments);
                        }
                        let operation = match args[0] {
                            0 => PointerOperationType::Add,
                            1 => PointerOperationType::Subtract,
                            _ => return Err(VMError::UnknownOpcode),
                        };
                        Ok(Arc::new(PointerOperationInstruction::new(
                            operation,
                            args[1] as isize,
                        )))
                    } else {
                        Err(VMError::MissingInstructionArguments)
                    }
                }
            },
            _ => Err(VMError::UnknownOpcode),
        }
    }
}
