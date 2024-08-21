use crate::core::execution_engine::errors::InstructionError;
use crate::core::execution_engine::instructions::arithmetic::Div;
use crate::core::execution_engine::instructions::instruction_processor::InstructionProcessor;

#[test]
fn test_div_instruction() {
    let mut processor = InstructionProcessor::new();
    processor.stack.push(2);
    processor.stack.push(6);
    Div::execute(&mut processor).unwrap();
    assert_eq!(processor.stack.pop(), Some(3));
}

#[test]
fn test_stack_underflow() {
    let mut processor = InstructionProcessor::new();
    let result = Div::execute(&mut processor);
    assert!(matches!(result, Err(InstructionError::StackUnderflow)));
}
