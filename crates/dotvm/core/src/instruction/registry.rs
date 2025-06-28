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
use super::crypto::{DecryptInstruction, EncryptInstruction, HashInstruction, SignInstruction, VerifySignatureInstruction};
use super::instruction::Instruction;
use super::system_call::{CreateProcessInstruction, ReadSysCallInstruction, ReceiveNetworkPacketInstruction, SendNetworkPacketInstruction, TerminateProcessInstruction, WriteSysCallInstruction};
use crate::instruction::memory::{AllocateInstruction, DeallocateInstruction, LoadInstruction, PointerOperationInstruction, PointerOperationType, StoreInstruction};
use crate::opcode::{arithmetic_opcodes::ArithmeticOpcode, control_flow_opcodes::ControlFlowOpcode, crypto_opcodes::CryptoOpcode, memory_opcodes::MemoryOpcode, system_call_opcodes::SystemCallOpcode};
use crate::vm::errors::VMError;
use std::collections::HashMap;
use std::sync::Arc;

/// Enum representing all possible opcodes.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Opcode {
    Arithmetic(ArithmeticOpcode),
    ControlFlow(ControlFlowOpcode),
    Memory(MemoryOpcode),
    SystemCall(SystemCallOpcode),
    Crypto(CryptoOpcode),
}

/// Central registry for instruction creation segregated by opcode category.
pub struct InstructionRegistry {
    /// Creator for arithmetic instructions.
    pub arithmetic: Box<dyn Fn(ArithmeticOpcode, Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
    /// Registry for control flow instruction creators.
    pub control_flow: HashMap<ControlFlowOpcode, Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>>,
    /// Registry for memory instruction creators.
    pub memory: HashMap<MemoryOpcode, Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>>,
    /// Registry for system call instruction creators.
    pub system_calls: HashMap<SystemCallOpcode, Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>>,
    /// Registry for cryptographic instruction creators.
    pub crypto: HashMap<CryptoOpcode, Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>>,
}

impl Default for InstructionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl InstructionRegistry {
    /// Creates a new InstructionRegistry populated with default instruction creators.
    pub fn new() -> Self {
        InstructionRegistry {
            arithmetic: Self::build_arithmetic_registry(),
            control_flow: Self::build_control_flow_registry(),
            memory: Self::build_memory_registry(),
            system_calls: Self::build_system_calls_registry(),
            crypto: Self::build_crypto_registry(),
        }
    }

    fn build_arithmetic_registry() -> Box<dyn Fn(ArithmeticOpcode, Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync> {
        Box::new(|opcode: ArithmeticOpcode, _args: Option<Vec<usize>>| Ok(Arc::new(ArithmeticInstruction::new(opcode)) as Arc<dyn Instruction>))
    }

    fn build_control_flow_registry() -> HashMap<ControlFlowOpcode, Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>> {
        let mut registry = HashMap::new();
        registry.insert(
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
            }) as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
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
            }) as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
            ControlFlowOpcode::WhileLoop,
            Box::new(|args: Option<Vec<usize>>| {
                if let Some(args) = args {
                    if args.len() != 2 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(LoopInstruction::new(LoopType::WhileLoop, args[0], args[1])) as Arc<dyn Instruction>)
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }) as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
            ControlFlowOpcode::DoWhileLoop,
            Box::new(|args: Option<Vec<usize>>| {
                if let Some(args) = args {
                    if args.len() != 2 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(LoopInstruction::new(LoopType::DoWhileLoop, args[0], args[1])) as Arc<dyn Instruction>)
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }) as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
            ControlFlowOpcode::ForLoop,
            Box::new(|args: Option<Vec<usize>>| {
                if let Some(args) = args {
                    if args.len() != 2 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(LoopInstruction::new(LoopType::ForLoop, args[0], args[1])) as Arc<dyn Instruction>)
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }) as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry
    }

    fn build_memory_registry() -> HashMap<MemoryOpcode, Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>> {
        let mut registry = HashMap::new();
        registry.insert(
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
            }) as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
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
            }) as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
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
            }) as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
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
            }) as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
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
                    Ok(Arc::new(PointerOperationInstruction::new(operation, args[1] as isize)) as Arc<dyn Instruction>)
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }) as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry
    }

    fn build_system_calls_registry() -> HashMap<SystemCallOpcode, Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>> {
        let mut registry = HashMap::new();
        registry.insert(
            SystemCallOpcode::Write,
            Box::new(|_args: Option<Vec<usize>>| Ok(Arc::new(WriteSysCallInstruction::new()) as Arc<dyn Instruction>))
                as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
            SystemCallOpcode::Read,
            Box::new(|_args: Option<Vec<usize>>| Ok(Arc::new(ReadSysCallInstruction::new()) as Arc<dyn Instruction>))
                as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
            SystemCallOpcode::CreateProcess,
            Box::new(|_args: Option<Vec<usize>>| {
                // Use a fixed command ("echo") for demonstration.
                Ok(Arc::new(CreateProcessInstruction::new(String::from("echo"))) as Arc<dyn Instruction>)
            }) as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
            SystemCallOpcode::TerminateProcess,
            Box::new(|args: Option<Vec<usize>>| {
                if let Some(vals) = args {
                    let pid = vals.first().cloned().unwrap_or(0) as u32;
                    Ok(Arc::new(TerminateProcessInstruction::new(pid)) as Arc<dyn Instruction>)
                } else {
                    Err(VMError::InvalidInstructionArguments)
                }
            }) as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
            SystemCallOpcode::NetworkSend,
            Box::new(|_args: Option<Vec<usize>>| Ok(Arc::new(SendNetworkPacketInstruction::new(String::from("127.0.0.1"), 8080)) as Arc<dyn Instruction>))
                as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
            SystemCallOpcode::NetworkReceive,
            Box::new(|_args: Option<Vec<usize>>| Ok(Arc::new(ReceiveNetworkPacketInstruction::new(8080)) as Arc<dyn Instruction>))
                as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry
    }

    fn build_crypto_registry() -> HashMap<CryptoOpcode, Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>> {
        let mut registry = HashMap::new();
        registry.insert(
            CryptoOpcode::Hash,
            Box::new(|_args: Option<Vec<usize>>| Ok(Arc::new(HashInstruction::new()) as Arc<dyn Instruction>))
                as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
            CryptoOpcode::Encrypt,
            Box::new(|_args: Option<Vec<usize>>| Ok(Arc::new(EncryptInstruction::new()) as Arc<dyn Instruction>))
                as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
            CryptoOpcode::Decrypt,
            Box::new(|_args: Option<Vec<usize>>| Ok(Arc::new(DecryptInstruction::new()) as Arc<dyn Instruction>))
                as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
            CryptoOpcode::Sign,
            Box::new(|_args: Option<Vec<usize>>| Ok(Arc::new(SignInstruction::new()) as Arc<dyn Instruction>))
                as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry.insert(
            CryptoOpcode::VerifySignature,
            Box::new(|_args: Option<Vec<usize>>| Ok(Arc::new(VerifySignatureInstruction::new()) as Arc<dyn Instruction>))
                as Box<dyn Fn(Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> + Send + Sync>,
        );
        registry
    }

    /// Creates an instruction based on the given opcode and arguments by delegating to the corresponding registry.
    pub fn create_instruction(&self, opcode: Opcode, args: Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> {
        match opcode {
            Opcode::Arithmetic(op) => (self.arithmetic)(op, args),
            Opcode::ControlFlow(op) => {
                if let Some(creator) = self.control_flow.get(&op) {
                    creator(args)
                } else {
                    Err(VMError::UnknownOpcode)
                }
            }
            Opcode::Memory(op) => {
                if let Some(creator) = self.memory.get(&op) {
                    creator(args)
                } else {
                    Err(VMError::UnknownOpcode)
                }
            }
            Opcode::SystemCall(op) => {
                if let Some(creator) = self.system_calls.get(&op) {
                    creator(args)
                } else {
                    Err(VMError::UnknownOpcode)
                }
            }
            Opcode::Crypto(op) => {
                if let Some(creator) = self.crypto.get(&op) {
                    creator(args)
                } else {
                    Err(VMError::UnknownOpcode)
                }
            }
        }
    }
}
