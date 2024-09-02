use call_function::CallFunction;

use crate::core::execution_engine::errors::InstructionError;

use super::InstructionProcessor;

pub mod call_function;
pub mod function;

#[derive(Debug)]
pub enum FunctionInstruction {
    CallFunction,
}

impl FunctionInstruction {
    pub fn execute(&self, processor: &mut InstructionProcessor) -> Result<(), InstructionError> {
        match self {
            FunctionInstruction::CallFunction => CallFunction::execute(processor),
        }
    }
}
