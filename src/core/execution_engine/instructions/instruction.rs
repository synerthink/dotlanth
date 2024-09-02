use crate::core::execution_engine::{errors::InstructionError, opcodes::opcode::Opcode};

use super::{
    function::FunctionInstruction, immediate_value::ImmediateValueInstruction,
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
    ImmediateValue(ImmediateValueInstruction),
    Function(FunctionInstruction),
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
            Instruction::ImmediateValue(instr) => instr.execute(processor),
            Instruction::Function(instr) => instr.execute(processor),
        }
    }

    pub fn from_opcode(opcode: Opcode) -> Result<Self, InstructionError> {
        match opcode {
            Opcode::Add => Ok(Instruction::Arithmetic(ArithmeticInstruction::Add)),
            Opcode::Sub => Ok(Instruction::Arithmetic(ArithmeticInstruction::Sub)),
            Opcode::Mul => Ok(Instruction::Arithmetic(ArithmeticInstruction::Mul)),
            Opcode::Div => Ok(Instruction::Arithmetic(ArithmeticInstruction::Div)),
            Opcode::LoadFromMemory => Ok(Instruction::Memory(MemoryInstruction::Load)),
            Opcode::StoreToMemory => Ok(Instruction::Memory(MemoryInstruction::Store)),
            Opcode::Jump => Ok(Instruction::ControlFlow(ControlFlowInstruction::Jump)),
            Opcode::JumpIf => Ok(Instruction::ControlFlow(ControlFlowInstruction::JumpIf)),
            Opcode::LoadNumber => Ok(Instruction::ImmediateValue(ImmediateValueInstruction::LoadNumber)),
            Opcode::CallFunction => Ok(Instruction::Function(FunctionInstruction::CallFunction)),
        }
    }
}
