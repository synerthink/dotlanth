pub mod jump;
pub mod jump_if;

pub use jump::Jump;
pub use jump_if::JumpIf;

use crate::core::execution_engine::errors::InstructionError;

use super::InstructionProcessor;

/// # ControlFlowInstruction
///
/// This enum represents the various control flow instructions available.
///
/// ## Variants
///
/// - `Jump`: Represents the unconditional jump operation.
/// - `JumpIf`: Represents the conditional jump operation.
#[derive(Debug)]
pub enum ControlFlowInstruction {
    Jump,
    JumpIf,
}

impl ControlFlowInstruction {
    /// Executes the appropriate control flow instruction.
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
    /// This function propagates errors from the specific control flow operations.
    pub fn execute(&self, processor: &mut InstructionProcessor) -> Result<(), InstructionError> {
        match self {
            ControlFlowInstruction::Jump => Jump::execute(processor),
            ControlFlowInstruction::JumpIf => JumpIf::execute(processor),
        }
    }
}

#[cfg(test)]
mod tests {
    mod jump_if_tests;
    mod jump_tests;
}
