use crate::core::execution_engine::instructions::instruction;

use super::instruction_error::InstructionError;
use std::fmt;

/// # VMError
///
/// This enum represents the possible errors that can occur in the virtual machine.
///
/// ## Variants
///
/// - `InstructionError(InstructionError)`: An error related to instruction execution.
/// - `UnknownInstruction(String)`: An unknown instruction was encountered.
/// - `Other(String)`: Any other error, with a message describing the error.
#[derive(Debug)]
pub enum VMError {
    InstructionError(InstructionError),
    UnknownInstruction(String),
    CompilationError(String),
    ExecutionError(String),
    Other(String),
}

impl fmt::Display for VMError {
    /// Formats the `VMError` for display.
    ///
    /// # Arguments
    ///
    /// * `f` - A mutable reference to a formatter.
    ///
    /// # Returns
    ///
    /// `fmt::Result` - The result of formatting.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VMError::InstructionError(err) => write!(f, "Instruction error: {}", err),
            VMError::UnknownInstruction(inst) => write!(f, "Unknown instruction: {}", inst),
            VMError::CompilationError(msg) => write!(f, "Compilation error: {}", msg),
            VMError::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
            VMError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl std::error::Error for VMError {}

impl From<InstructionError> for VMError {
    /// Converts an `InstructionError` into a `VMError`.
    ///
    /// # Arguments
    ///
    /// * `error` - The `InstructionError` to convert.
    ///
    /// # Returns
    ///
    /// `VMError` - The resulting `VMError`.
    fn from(error: InstructionError) -> Self {
        VMError::InstructionError(error)
    }
}
