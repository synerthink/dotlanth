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

//! Architecture-aware instruction registry for DotVM
//!
//! This module provides instruction registries that are aware of the target
//! architecture and can create appropriate instructions based on the opcode
//! and architecture combination.

use super::{
    arithmetic::ArithmeticInstruction,
    bigint::BigIntInstruction,
    control_flow::{IfElseInstruction, JumpInstruction, LoopInstruction, LoopType},
    crypto::{DecryptInstruction, EncryptInstruction, HashInstruction, SignInstruction, VerifySignatureInstruction},
    instruction::Instruction,
    memory::{AllocateInstruction, DeallocateInstruction, LoadInstruction, PointerOperationInstruction, PointerOperationType, StoreInstruction},
    system_call::{CreateProcessInstruction, ReadSysCallInstruction, ReceiveNetworkPacketInstruction, SendNetworkPacketInstruction, TerminateProcessInstruction, WriteSysCallInstruction},
};
use crate::{
    memory::{Arch64, Arch128, Architecture},
    opcode::{
        architecture_opcodes::{Arch64Opcodes, Arch128Opcodes, ArchitectureOpcodes, BigIntOpcode, Opcode64, Opcode128},
        arithmetic_opcodes::ArithmeticOpcode,
        control_flow_opcodes::ControlFlowOpcode,
        crypto_opcodes::CryptoOpcode,
        memory_opcodes::MemoryOpcode,
        system_call_opcodes::SystemCallOpcode,
    },
    vm::errors::VMError,
};
use std::{marker::PhantomData, sync::Arc};

/// Architecture-aware instruction registry trait
pub trait ArchitectureRegistry<A: Architecture> {
    type Opcode: Clone + Copy + std::fmt::Debug + std::fmt::Display;

    /// Create an instruction from an opcode and optional arguments
    fn create_instruction(&self, opcode: Self::Opcode, args: Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError>;

    /// Check if an opcode is supported by this architecture
    fn supports_opcode(&self, opcode: &Self::Opcode) -> bool;

    /// Get the architecture name
    fn architecture_name(&self) -> &'static str;
}

/// 64-bit architecture instruction registry
pub struct Registry64 {
    _phantom: PhantomData<Arch64>,
}

impl Registry64 {
    pub fn new() -> Self {
        Self { _phantom: PhantomData }
    }

    fn create_arithmetic_instruction(&self, opcode: ArithmeticOpcode, _args: Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> {
        Ok(Arc::new(ArithmeticInstruction::new(opcode)))
    }

    fn create_control_flow_instruction(&self, opcode: ControlFlowOpcode, args: Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> {
        match opcode {
            ControlFlowOpcode::IfElse => {
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
                if let Some(args) = args {
                    if args.len() != 2 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(LoopInstruction::new(LoopType::WhileLoop, args[0], args[1])))
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }
            ControlFlowOpcode::DoWhileLoop => {
                if let Some(args) = args {
                    if args.len() != 2 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(LoopInstruction::new(LoopType::DoWhileLoop, args[0], args[1])))
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }
            ControlFlowOpcode::ForLoop => {
                if let Some(args) = args {
                    if args.len() != 2 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(LoopInstruction::new(LoopType::ForLoop, args[0], args[1])))
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }
        }
    }

    fn create_memory_instruction(&self, opcode: MemoryOpcode, args: Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> {
        match opcode {
            MemoryOpcode::Load => {
                if let Some(args) = args {
                    if args.len() != 1 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(LoadInstruction::new(args[0])))
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }
            MemoryOpcode::Store => {
                if let Some(args) = args {
                    if args.len() != 1 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(StoreInstruction::new(args[0])))
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }
            MemoryOpcode::Allocate => {
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
                if let Some(args) = args {
                    if args.len() != 1 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    Ok(Arc::new(DeallocateInstruction::new(args[0])))
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }
            MemoryOpcode::PointerOperation => {
                if let Some(args) = args {
                    if args.len() != 2 {
                        return Err(VMError::InvalidInstructionArguments);
                    }
                    let operation = match args[0] {
                        0 => PointerOperationType::Add,
                        1 => PointerOperationType::Subtract,
                        _ => return Err(VMError::UnknownOpcode),
                    };
                    Ok(Arc::new(PointerOperationInstruction::new(operation, args[1] as isize)))
                } else {
                    Err(VMError::MissingInstructionArguments)
                }
            }
        }
    }

    fn create_system_call_instruction(&self, opcode: SystemCallOpcode, args: Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> {
        match opcode {
            SystemCallOpcode::Write => Ok(Arc::new(WriteSysCallInstruction::new())),
            SystemCallOpcode::Read => Ok(Arc::new(ReadSysCallInstruction::new())),
            SystemCallOpcode::CreateProcess => Ok(Arc::new(CreateProcessInstruction::new(String::from("echo")))),
            SystemCallOpcode::TerminateProcess => {
                if let Some(vals) = args {
                    let pid = vals.get(0).cloned().unwrap_or(0) as u32;
                    Ok(Arc::new(TerminateProcessInstruction::new(pid)))
                } else {
                    Err(VMError::InvalidInstructionArguments)
                }
            }
            SystemCallOpcode::NetworkSend => Ok(Arc::new(SendNetworkPacketInstruction::new(String::from("127.0.0.1"), 8080))),
            SystemCallOpcode::NetworkReceive => Ok(Arc::new(ReceiveNetworkPacketInstruction::new(8080))),
        }
    }

    fn create_crypto_instruction(&self, opcode: CryptoOpcode, _args: Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> {
        match opcode {
            CryptoOpcode::Hash => Ok(Arc::new(HashInstruction::new())),
            CryptoOpcode::Encrypt => Ok(Arc::new(EncryptInstruction::new())),
            CryptoOpcode::Decrypt => Ok(Arc::new(DecryptInstruction::new())),
            CryptoOpcode::Sign => Ok(Arc::new(SignInstruction::new())),
            CryptoOpcode::VerifySignature => Ok(Arc::new(VerifySignatureInstruction::new())),
        }
    }
}

impl ArchitectureRegistry<Arch64> for Registry64 {
    type Opcode = Opcode64;

    fn create_instruction(&self, opcode: Self::Opcode, args: Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> {
        match opcode {
            Opcode64::Arithmetic(op) => self.create_arithmetic_instruction(op, args),
            Opcode64::ControlFlow(op) => self.create_control_flow_instruction(op, args),
            Opcode64::Memory(op) => self.create_memory_instruction(op, args),
            Opcode64::SystemCall(op) => self.create_system_call_instruction(op, args),
            Opcode64::Crypto(op) => self.create_crypto_instruction(op, args),
        }
    }

    fn supports_opcode(&self, _opcode: &Self::Opcode) -> bool {
        true // All Opcode64 variants are supported in 64-bit architecture
    }

    fn architecture_name(&self) -> &'static str {
        Arch64Opcodes::architecture_name()
    }
}

/// 128-bit architecture instruction registry
pub struct Registry128 {
    base_registry: Registry64,
    _phantom: PhantomData<Arch128>,
}

impl Registry128 {
    pub fn new() -> Self {
        Self {
            base_registry: Registry64::new(),
            _phantom: PhantomData,
        }
    }

    fn create_bigint_instruction(&self, opcode: BigIntOpcode, _args: Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> {
        Ok(Arc::new(BigIntInstruction::new(opcode)))
    }
}

impl ArchitectureRegistry<Arch128> for Registry128 {
    type Opcode = Opcode128;

    fn create_instruction(&self, opcode: Self::Opcode, args: Option<Vec<usize>>) -> Result<Arc<dyn Instruction>, VMError> {
        match opcode {
            Opcode128::Base(base_opcode) => self.base_registry.create_instruction(base_opcode, args),
            Opcode128::BigInt(bigint_opcode) => self.create_bigint_instruction(bigint_opcode, args),
        }
    }

    fn supports_opcode(&self, _opcode: &Self::Opcode) -> bool {
        true // All Opcode128 variants are supported in 128-bit architecture
    }

    fn architecture_name(&self) -> &'static str {
        Arch128Opcodes::architecture_name()
    }
}

/// Registry factory for creating architecture-specific registries
pub struct RegistryFactory;

impl RegistryFactory {
    /// Create a 64-bit registry
    pub fn create_64bit_registry() -> Registry64 {
        Registry64::new()
    }

    /// Create a 128-bit registry
    pub fn create_128bit_registry() -> Registry128 {
        Registry128::new()
    }

    /// Create a registry based on architecture type
    /// Note: This is a placeholder for future generic implementation
    /// For now, use the specific create_*_registry methods
    pub fn create_registry_for_architecture_name(arch_name: &str) -> Result<Box<dyn std::any::Any>, VMError> {
        match arch_name {
            "64-bit" => Ok(Box::new(Self::create_64bit_registry())),
            "128-bit" => Ok(Box::new(Self::create_128bit_registry())),
            _ => Err(VMError::UnknownOpcode),
        }
    }
}

/// Utility functions for working with architecture registries
pub mod utils {
    use super::*;

    /// Detect the required architecture from an opcode value
    pub fn detect_architecture_from_opcode(opcode_value: u16) -> Result<&'static str, VMError> {
        if opcode_value <= Arch64Opcodes::max_opcode_value() {
            Ok(Arch64Opcodes::architecture_name())
        } else if opcode_value <= Arch128Opcodes::max_opcode_value() {
            Ok(Arch128Opcodes::architecture_name())
        } else {
            Err(VMError::UnknownOpcode)
        }
    }

    /// Check if a 64-bit opcode can run on a 128-bit architecture
    pub fn is_backward_compatible(source_arch: &str, target_arch: &str) -> bool {
        match (source_arch, target_arch) {
            ("64-bit", "128-bit") => true,
            (same_source, same_target) if same_source == same_target => true,
            _ => false,
        }
    }

    /// Convert a 64-bit opcode to a 128-bit opcode for backward compatibility
    pub fn convert_64bit_to_128bit(opcode64: Opcode64) -> Opcode128 {
        Opcode128::Base(opcode64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcode::arithmetic_opcodes::ArithmeticOpcode;

    #[test]
    fn test_64bit_registry_creation() {
        let registry = RegistryFactory::create_64bit_registry();
        assert_eq!(registry.architecture_name(), "64-bit");
    }

    #[test]
    fn test_128bit_registry_creation() {
        let registry = RegistryFactory::create_128bit_registry();
        assert_eq!(registry.architecture_name(), "128-bit");
    }

    #[test]
    fn test_64bit_arithmetic_instruction_creation() {
        let registry = RegistryFactory::create_64bit_registry();
        let opcode = Opcode64::Arithmetic(ArithmeticOpcode::Add);

        let instruction = registry.create_instruction(opcode, None);
        assert!(instruction.is_ok());
    }

    #[test]
    fn test_128bit_bigint_instruction_creation() {
        let registry = RegistryFactory::create_128bit_registry();
        let opcode = Opcode128::BigInt(BigIntOpcode::Add);

        let instruction = registry.create_instruction(opcode, None);
        assert!(instruction.is_ok());
    }

    #[test]
    fn test_128bit_base_instruction_creation() {
        let registry = RegistryFactory::create_128bit_registry();
        let opcode = Opcode128::Base(Opcode64::Arithmetic(ArithmeticOpcode::Add));

        let instruction = registry.create_instruction(opcode, None);
        assert!(instruction.is_ok());
    }

    #[test]
    fn test_architecture_detection() {
        // Test 64-bit opcode detection
        let arithmetic_opcode = Opcode64::Arithmetic(ArithmeticOpcode::Add).as_u16();
        let detected_arch = utils::detect_architecture_from_opcode(arithmetic_opcode);
        assert_eq!(detected_arch.unwrap(), "64-bit");

        // Test 128-bit opcode detection
        let bigint_opcode = Opcode128::BigInt(BigIntOpcode::Add).as_u16();
        let detected_arch = utils::detect_architecture_from_opcode(bigint_opcode);
        assert_eq!(detected_arch.unwrap(), "128-bit");
    }

    #[test]
    fn test_backward_compatibility() {
        assert!(utils::is_backward_compatible("64-bit", "128-bit"));
        assert!(utils::is_backward_compatible("64-bit", "64-bit"));
        assert!(utils::is_backward_compatible("128-bit", "128-bit"));
        assert!(!utils::is_backward_compatible("128-bit", "64-bit"));
    }

    #[test]
    fn test_opcode_conversion() {
        let opcode64 = Opcode64::Arithmetic(ArithmeticOpcode::Add);
        let opcode128 = utils::convert_64bit_to_128bit(opcode64);

        match opcode128 {
            Opcode128::Base(converted) => assert_eq!(converted, opcode64),
            _ => panic!("Conversion failed"),
        }
    }
}
