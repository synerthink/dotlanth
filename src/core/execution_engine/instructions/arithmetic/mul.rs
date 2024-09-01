use crate::core::execution_engine::{errors::InstructionError, instructions::InstructionProcessor};

/// # Mul
///
/// This struct represents the Multiplication operation in the instruction set.
pub struct Mul;

impl Mul {
    /// Executes the Multiplication operation.
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
    /// It returns `InstructionError::MulisionByZero` if an attempt is made to Mulide by zero.
    pub fn execute(processor: &mut InstructionProcessor) -> Result<(), InstructionError> {
        let val1 = processor
            .stack
            .pop()
            .ok_or(InstructionError::StackUnderflow)?;
        let val2 = processor
            .stack
            .pop()
            .ok_or(InstructionError::StackUnderflow)?;
        let result = val1 * val2;
        processor.stack.push(result);
        processor.program_counter += 1;
        Ok(())
    }
}
