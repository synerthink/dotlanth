use crate::core::execution_engine::errors::InstructionError;

use super::{
    ArithmeticInstruction, ControlFlowInstruction, InstructionProcessor, MemoryInstruction,
};

/// # Instruction
///
/// This enum represents the various types of instructions available.
///
/// ## Variants
///
/// - `Arithmetic(ArithmeticInstruction)`: Represents an arithmetic instruction.
/// - `Memory(MemoryInstruction)`: Represents a memory instruction.
/// - `ControlFlow(ControlFlowInstruction)`: Represents a control flow instruction.
#[derive(Debug)]
pub enum Instruction {
    Arithmetic(ArithmeticInstruction),
    Memory(MemoryInstruction),
    ControlFlow(ControlFlowInstruction),
}

impl Instruction {
    /// Executes the appropriate instruction.
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
    pub fn execute(&self, processor: &mut InstructionProcessor) -> Result<(), InstructionError> {
        match self {
            Instruction::Arithmetic(instr) => instr.execute(processor),
            Instruction::Memory(instr) => instr.execute(processor),
            Instruction::ControlFlow(instr) => instr.execute(processor),
        }
    }
}
