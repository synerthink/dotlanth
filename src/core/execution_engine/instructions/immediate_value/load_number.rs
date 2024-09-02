use crate::core::execution_engine::{errors::InstructionError, instructions::InstructionProcessor};

#[derive(Debug)]
pub struct LoadNumber;

impl LoadNumber {
    pub fn execute(processor: &mut InstructionProcessor) -> Result<(), InstructionError> {
        // Assuming the value to load is the next instruction in memory
        processor.program_counter += 1;
        let value = processor
            .memory
            .get(&processor.program_counter)
            .ok_or(InstructionError::InvalidMemoryAddress)?;
        processor.stack.push(*value);
        processor.program_counter += 1;
        Ok(())
    }
}
