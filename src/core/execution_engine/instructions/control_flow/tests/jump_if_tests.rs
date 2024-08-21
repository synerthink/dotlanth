use crate::core::execution_engine::{
    errors::InstructionError,
    instructions::{InstructionProcessor, JumpIf},
};

#[test]
fn test_jump_if_instruction() {
    let mut processor = InstructionProcessor::new();
    processor.stack.push(1);
    processor.stack.push(5);
    JumpIf::execute(&mut processor).unwrap();
    assert_eq!(processor.program_counter, 5);

    processor.stack.push(0);
    processor.stack.push(5);
    JumpIf::execute(&mut processor).unwrap();
    assert_ne!(processor.program_counter, 0);
}

#[test]
fn test_stack_underflow() {
    let mut processor = InstructionProcessor::new();
    let result = JumpIf::execute(&mut processor);
    assert!(matches!(result, Err(InstructionError::StackUnderflow)));
}
