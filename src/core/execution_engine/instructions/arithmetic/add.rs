use crate::core::execution_engine::{errors::InstructionError, instructions::InstructionProcessor};

/// # Add
///
/// This struct represents the addition operation in the instruction set.
pub struct Add;

impl Add {
    /// Executes the addition operation.
    ///
    /// # Arguments
    ///
    /// * `processor` - A mutable reference to the `InstructionProcessor`.
    ///
    /// # Returns
    ///
    /// `Result<(), InstructionError>` - The result of the execution, which is Ok or an `InstructionError`.
    ///
    /// # Errors
    ///
    /// This function returns `InstructionError::StackUnderflow` if the stack does not have enough values.
    pub fn execute(processor: &mut InstructionProcessor) -> Result<(), InstructionError> {
        let val1 = processor
            .stack
            .pop()
            .ok_or(InstructionError::StackUnderflow)?;
        let val2 = processor
            .stack
            .pop()
            .ok_or(InstructionError::StackUnderflow)?;
        let result = val1 + val2;
        processor.stack.push(result);
        processor.program_counter += 1;
        Ok(())
    }
}
