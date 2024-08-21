use crate::core::execution_engine::{errors::InstructionError, instructions::InstructionProcessor};

/// # JumpIf
///
/// This struct represents the conditional jump operation in the instruction set.
pub struct JumpIf;

impl JumpIf {
    /// Executes the conditional jump operation.
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
        if processor.stack.len() < 2 {
            return Err(InstructionError::StackUnderflow);
        }
        let address = processor.stack.pop().unwrap();
        let condition = processor.stack.pop().unwrap();
        if condition != 0 {
            processor.program_counter = address as usize;
        }
        Ok(())
    }
}
