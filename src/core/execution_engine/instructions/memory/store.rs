use crate::core::execution_engine::{errors::InstructionError, instructions::InstructionProcessor};

/// # Store
///
/// This struct represents the store operation in the instruction set.
pub struct Store;

impl Store {
    /// Executes the store operation.
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
        let value = processor.stack.pop().unwrap();
        let address = processor.stack.pop().unwrap();
        processor.memory.insert(address as usize, value);
        processor.program_counter += 1;
        Ok(())
    }
}
