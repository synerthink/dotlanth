use crate::core::execution_engine::errors::VMError;
use std::collections::HashMap;

use super::Instruction;

/// # InstructionProcessor
///
/// This struct represents the processor that executes instructions. It maintains the stack, memory,
/// program counter, and a set of available instructions.
pub struct InstructionProcessor {
    pub stack: Vec<i32>,
    pub memory: HashMap<usize, i32>,
    pub program_counter: usize,
    pub instructions: HashMap<String, Instruction>,
}

impl InstructionProcessor {
    /// Creates a new `InstructionProcessor`.
    ///
    /// # Returns
    ///
    /// `InstructionProcessor` - A new instance of `InstructionProcessor` with an empty stack, memory,
    /// program counter set to 0, and an empty instruction set.
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            memory: HashMap::new(),
            program_counter: 0,
            instructions: HashMap::new(),
        }
    }

    /// Executes a given instruction.
    ///
    /// # Arguments
    ///
    /// * `instruction` - A reference to the `Instruction` to be executed.
    ///
    /// # Returns
    ///
    /// `Result<(), VMError>` - The result of the execution, which is Ok or a `VMError`.
    ///
    /// # Errors
    ///
    /// This function propagates errors from the instruction's execution.
    pub fn execute_instruction(&mut self, instruction: &Instruction) -> Result<(), VMError> {
        instruction.execute(self)?;
        Ok(())
    }
}
