use crate::core::execution_engine::errors::InstructionError;

use super::InstructionProcessor;

/// # InstructionTrait
///
/// This trait represents the behavior of an instruction. Any instruction that implements this trait
/// can be executed by the `InstructionProcessor`.
pub trait InstructionTrait {
    /// Executes the instruction.
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
    /// This function propagates errors from the specific instruction's execution.
    fn execute(&self, processor: &mut InstructionProcessor) -> Result<(), InstructionError>;
}
