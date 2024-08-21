use crate::core::execution_engine::instructions::instruction_processor::InstructionProcessor;
use crate::core::execution_engine::instructions::arithmetic::Add;
use crate::core::execution_engine::errors::InstructionError;

#[test]
fn test_add_instruction() {
    let mut processor = InstructionProcessor::new();
    processor.stack.push(1);
    processor.stack.push(2);
    Add::execute(&mut processor).unwrap();
    assert_eq!(processor.stack.pop(), Some(3));
}

#[test]
fn test_stack_underflow() {
    let mut processor = InstructionProcessor::new();
    let result = Add::execute(&mut processor);
    assert!(matches!(result, Err(InstructionError::StackUnderflow)));
}