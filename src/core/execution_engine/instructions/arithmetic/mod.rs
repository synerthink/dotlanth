pub mod add;
pub mod div;
pub mod sub;

pub use add::Add;
pub use div::Div;
pub use sub::Sub;

use crate::core::execution_engine::errors::InstructionError;

use super::InstructionProcessor;

/// # ArithmeticInstruction
///
/// This enum represents the various arithmetic instructions available.
/// 
/// ## Variants
/// 
/// - `Add`: Represents the addition operation.
/// - `Sub`: Represents the subtraction operation.
/// - `Div`: Represents the division operation.
#[derive(Debug)]
pub enum ArithmeticInstruction {
    Add,
    Sub,
    Div,
}

impl ArithmeticInstruction {
    /// Executes the appropriate arithmetic instruction.
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
    /// This function propagates errors from the specific arithmetic operations.
    pub fn execute(&self, processor: &mut InstructionProcessor) -> Result<(), InstructionError> {
        match self {
            ArithmeticInstruction::Add => Add::execute(processor).map_err(Into::into),
            ArithmeticInstruction::Sub => Sub::execute(processor).map_err(Into::into),
            ArithmeticInstruction::Div => Div::execute(processor).map_err(Into::into),
        }
    }
}

#[cfg(test)]
mod tests {
    mod add_tests;
    mod sub_tests;
    mod div_tests;
}
