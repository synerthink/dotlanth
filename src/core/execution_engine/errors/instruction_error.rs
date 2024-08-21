use std::fmt;

/// # InstructionError
///
/// This enum represents the possible errors that can occur during the execution of an instruction.
///
/// ## Variants
///
/// - `InvalidOpcode`: The opcode provided is not recognized.
/// - `DivisionByZero`: An attempt was made to divide by zero.
/// - `StackUnderflow`: An operation was attempted on an empty stack.
/// - `InvalidMemoryAddress`: An invalid memory address was accessed.
/// - `Other(String)`: Any other error, with a message describing the error.
#[derive(Debug)]
pub enum InstructionError {
    InvalidOpcode,
    DivisionByZero,
    StackUnderflow,
    InvalidMemoryAddress,
    Other(String),
}

impl fmt::Display for InstructionError {
    /// Formats the `InstructionError` for display.
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
            InstructionError::InvalidOpcode => write!(f, "Invalid opcode"),
            InstructionError::DivisionByZero => write!(f, "Division by zero"),
            InstructionError::StackUnderflow => write!(f, "Stack underflow"),
            InstructionError::InvalidMemoryAddress => write!(f, "Invalid memory address"),
            InstructionError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl std::error::Error for InstructionError {}
