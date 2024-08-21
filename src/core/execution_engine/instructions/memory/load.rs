use crate::core::execution_engine::{errors::InstructionError, instructions::InstructionProcessor};

/// # Load
///
/// This struct represents the load operation in the instruction set.
pub struct Load;

impl Load {
    /// Executes the load operation.
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
    /// It returns `InstructionError::InvalidMemoryAddress` if the memory address is invalid.
    pub fn execute(processor: &mut InstructionProcessor) -> Result<(), InstructionError> {
        if processor.stack.is_empty() {
            return Err(InstructionError::StackUnderflow);
        }
        let address = processor.stack.pop().unwrap();
        if let Some(&value) = processor.memory.get(&(address as usize)) {
            processor.stack.push(value)
        } else {
            return Err(InstructionError::InvalidMemoryAddress);
        }
        processor.program_counter += 1;
        Ok(())
    }
}
