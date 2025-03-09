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
use crate::instruction::memory::{
    AllocateInstruction, DeallocateInstruction, LoadInstruction, PointerOperationInstruction,
    PointerOperationType, StoreInstruction,
};
use crate::opcode::{
    arithmetic_opcodes::ArithmeticOpcode, control_flow_opcodes::ControlFlowOpcode,
    memory_opcodes::MemoryOpcode,
};
use crate::vm::errors::VMError;
use std::collections::HashMap;
use std::sync::Arc;

/// Central registry for instruction creation with segregated registries for each opcode category.
pub struct InstructionRegistry {
    /// Closure to create arithmetic instructions.
    pub arithmetic: Box<
        dyn Fn(ArithmeticOpcode, Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError>
            + Send
            + Sync,
    >,
    /// Registry of control flow instruction creators.
    pub control_flow: HashMap<
        ControlFlowOpcode,
        Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
    >,
    /// Registry of memory instruction creators.
    pub memory: HashMap<
        MemoryOpcode,
        Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
    >,
}

impl InstructionRegistry {
    /// Create a new registry populated with default instruction creators.
    pub fn new() -> Self {
        let arithmetic_creator = Box::new(
            |arith_opcode: ArithmeticOpcode, _args: Option<Vec<usize>>| {
                Ok(Arc::new(ArithmeticInstruction::new(arith_opcode)) as Arc<dyn Instruction>)
            },
        );

        let mut control_flow: HashMap<
            ControlFlowOpcode,
            Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        > = HashMap::new();
        control_flow.insert(
            ControlFlowOpcode::IfElse,
            Box::new(|args: Option<Vec<usize>>| {
                if let Some(args) = args {
                    if args.len() != 1 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(IfElseInstruction::new(args[0])) as Arc<dyn Instruction>)
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }),
        );
        control_flow.insert(
            ControlFlowOpcode::Jump,
            Box::new(|args: Option<Vec<usize>>| {
                if let Some(args) = args {
                    if args.len() != 1 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(JumpInstruction::new(args[0])) as Arc<dyn Instruction>)
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }),
        );
        control_flow.insert(
            ControlFlowOpcode::WhileLoop,
            Box::new(|args: Option<Vec<usize>>| {
                if let Some(args) = args {
                    if args.len() != 2 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(
                        Arc::new(LoopInstruction::new(LoopType::WhileLoop, args[0], args[1]))
                            as Arc<dyn Instruction>,
                    )
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }),
        );
        control_flow.insert(
            ControlFlowOpcode::DoWhileLoop,
            Box::new(|args: Option<Vec<usize>>| {
                if let Some(args) = args {
                    if args.len() != 2 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(LoopInstruction::new(
                        LoopType::DoWhileLoop,
                        args[0],
                        args[1],
                    )) as Arc<dyn Instruction>)
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }),
        );
        control_flow.insert(
            ControlFlowOpcode::ForLoop,
            Box::new(|args: Option<Vec<usize>>| {
                // For this example, we consider ForLoop requires two arguments: init and body_start.
                if let Some(args) = args {
                    if args.len() != 2 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(
                        Arc::new(LoopInstruction::new(LoopType::ForLoop, args[0], args[1]))
                            as Arc<dyn Instruction>,
                    )
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }),
        );

        let mut memory: HashMap<
            MemoryOpcode,
            Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        > = HashMap::new();
        memory.insert(
            MemoryOpcode::Load,
            Box::new(|args: Option<Vec<usize>>| {
                if let Some(args) = args {
                    if args.len() != 1 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(LoadInstruction::new(args[0])) as Arc<dyn Instruction>)
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }),
        );
        memory.insert(
            MemoryOpcode::Store,
            Box::new(|args: Option<Vec<usize>>| {
                if let Some(args) = args {
                    if args.len() != 1 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(StoreInstruction::new(args[0])) as Arc<dyn Instruction>)
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }),
        );
        memory.insert(
            MemoryOpcode::Allocate,
            Box::new(|args: Option<Vec<usize>>| {
                if let Some(args) = args {
                    if args.len() != 1 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(AllocateInstruction::new(args[0])) as Arc<dyn Instruction>)
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }),
        );
        memory.insert(
            MemoryOpcode::Deallocate,
            Box::new(|args: Option<Vec<usize>>| {
                if let Some(args) = args {
                    if args.len() != 1 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(DeallocateInstruction::new(args[0])) as Arc<dyn Instruction>)
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }),
        );
        memory.insert(
            MemoryOpcode::PointerOperation,
            Box::new(|args: Option<Vec<usize>>| {
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
                    )) as Arc<dyn Instruction>)
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }),
        );

        InstructionRegistry {
            arithmetic: arithmetic_creator,
            control_flow,
            memory,
        }
    }

    /// Creates an instruction based on the given opcode and arguments.
    /// Delegates to the appropriate registry branch.
    pub fn create_instruction(
        &self,
        opcode: Opcode,
        args: Option<Vec<usize>>,
    ) -> Result<Arc<dyn Instruction>, VMError> {
        match opcode {
            Opcode::Arithmetic(arith_opcode) => (self.arithmetic)(arith_opcode, args),
            Opcode::ControlFlow(cf_opcode) => {
                if let Some(creator) = self.control_flow.get(&cf_opcode) {
                    creator(args)
                } else {
                    Err(VMError::UnknownOpcode)
                }
            }
            Opcode::Memory(mem_opcode) => {
                if let Some(creator) = self.memory.get(&mem_opcode) {
                    creator(args)
                } else {
                    Err(VMError::UnknownOpcode)
                }
            }
        }
    }
}

/// Enum representing all possible opcodes.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Opcode {
    Arithmetic(ArithmeticOpcode),
    ControlFlow(ControlFlowOpcode),
    Memory(MemoryOpcode),
}
