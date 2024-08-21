use crate::core::execution_engine::{errors::InstructionError, instructions::{InstructionProcessor, Jump}};

#[test]
fn test_jump_instruction() {
    let mut processor = InstructionProcessor::new();
    processor.stack.push(5);
    Jump::execute(&mut processor).unwrap();
    assert_eq!(processor.program_counter, 5);
}

#[test]
fn test_stack_underflow() {
    let mut processor = InstructionProcessor::new();
    let result = Jump::execute(&mut processor);
    assert!(matches!(result, Err(InstructionError::StackUnderflow)));
}