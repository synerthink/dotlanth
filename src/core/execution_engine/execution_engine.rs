use crate::{
    contracts::SimpleAdditionContract,
    core::{
        dotlang::compiler::compiler::DotLangCompiler,
        execution_engine::{
            errors::InstructionError,
            instructions::{Instruction, InstructionProcessor},
            opcodes::opcode::Opcode,
        },
    },
};
use log::{debug, error, info};

use super::errors::VMError;

pub struct ExecutionEngine;

impl ExecutionEngine {
    pub fn execute_contract(contract: &SimpleAdditionContract) -> Result<i32, VMError> {
        debug!("Compiling and executing contract source code...");
        let compiler = DotLangCompiler;
        let bytecode = compiler
            .compile(&contract.source_code)
            .map_err(|e| VMError::CompilationError(e.to_string()))?;

        info!("Compiled bytecode: {:?}", bytecode);

        let mut processor = InstructionProcessor::new();

        // Load bytecode into memory
        for (i, &byte) in bytecode.iter().enumerate() {
            processor.memory.insert(i, byte as i32);
        }

        while processor.program_counter < bytecode.len() {
            let byte =
                *processor
                    .memory
                    .get(&processor.program_counter)
                    .ok_or(VMError::ExecutionError(
                        "Invalid program counter".to_owned(),
                    ))? as u8;
            info!(
                "Processing byte: {:02x} at position {}",
                byte, processor.program_counter
            );

            let opcode = match Opcode::try_from(byte) {
                Ok(op) => op,
                Err(_) => {
                    error!(
                        "Invalid opcode: {:02x} at position {}",
                        byte, processor.program_counter
                    );
                    return Err(VMError::ExecutionError("Invalid opcode".to_owned()));
                }
            };
            info!("Decoded opcode: {:?}", opcode);

            let instruction = Instruction::from_opcode(opcode)
                .map_err(|e| VMError::ExecutionError(e.to_string()))?;

            info!("Created instruction: {:?}", instruction);

            instruction
                .execute(&mut processor)
                .map_err(|e| VMError::ExecutionError(e.to_string()))?;

            processor.program_counter += 1;
        }

        processor
            .stack
            .pop()
            .ok_or(VMError::ExecutionError("Stack is empty".to_owned()))
    }
}
