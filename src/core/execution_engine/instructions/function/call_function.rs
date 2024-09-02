use crate::core::execution_engine::{errors::InstructionError, instructions::InstructionProcessor};

#[derive(Debug)]
pub struct CallFunction;

impl CallFunction {
    pub fn execute(processor: &mut InstructionProcessor) -> Result<(), InstructionError> {
        // Retrieve the function index from the stack
        let function_index = processor
            .stack
            .pop()
            .ok_or(InstructionError::StackUnderflow)?;

        // Push the current program counter onto the call stack
        processor.call_stack.push(processor.program_counter);

        // Retrieve the function to be called using the function index
        let function = processor
            .functions
            .get(function_index as usize)
            .ok_or(InstructionError::InvalidFunctionIndex)?;

        // Set the program counter to the start of the function
        processor.program_counter = function.start_address;

        Ok(())
    }
}
