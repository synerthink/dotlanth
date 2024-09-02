use load_number::LoadNumber;

use crate::core::execution_engine::errors::InstructionError;

use super::InstructionProcessor;

pub mod load_number;

#[derive(Debug)]
pub enum ImmediateValueInstruction {
    LoadNumber,
}

impl ImmediateValueInstruction {
    pub fn execute(&self, processor: &mut InstructionProcessor) -> Result<(), InstructionError> {
        match self {
            ImmediateValueInstruction::LoadNumber => LoadNumber::execute(processor),
        }
    }
}
