pub mod load;
pub mod store;

pub use load::Load;
pub use store::Store;

use crate::core::execution_engine::errors::InstructionError;

use super::InstructionProcessor;

/// # MemoryInstruction
///
/// This enum represents the various memory instructions available.
///
/// ## Variants
///
/// - `Load`: Represents the load operation.
/// - `Store`: Represents the store operation.
#[derive(Debug)]
pub enum MemoryInstruction {
    Load,
    Store,
}

impl MemoryInstruction {
    /// Executes the appropriate memory instruction.
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
    /// This function propagates errors from the specific memory operations.
    pub fn execute(&self, processor: &mut InstructionProcessor) -> Result<(), InstructionError> {
        match self {
            MemoryInstruction::Load => Load::execute(processor),
            MemoryInstruction::Store => Store::execute(processor),
        }
    }
}

#[cfg(test)]
mod tests {
    mod load_tests;
    mod store_tests;
}
