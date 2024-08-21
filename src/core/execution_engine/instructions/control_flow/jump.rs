use crate::core::execution_engine::{errors::InstructionError, instructions::InstructionProcessor};

/// # Jump
///
/// This struct represents the unconditional jump operation in the instruction set.
pub struct Jump;

impl Jump {
    /// Executes the jump operation.
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
    /// This function returns `InstructionError::StackUnderflow` if the stack is empty.
    pub fn execute(processor: &mut InstructionProcessor) -> Result<(), InstructionError> {
        if processor.stack.is_empty() {
            return Err(InstructionError::StackUnderflow);
        }
        let address = processor.stack.pop().unwrap();
        processor.program_counter = address as usize;
        Ok(())
    }
}
