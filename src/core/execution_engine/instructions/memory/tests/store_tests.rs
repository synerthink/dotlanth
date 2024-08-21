use crate::core::execution_engine::{errors::InstructionError, instructions::{InstructionProcessor, Store}};

#[test]
fn test_store_instruction() {
    let mut processor = InstructionProcessor::new();
    processor.stack.push(0);
    processor.stack.push(42);
    Store::execute(&mut processor).unwrap();
    assert_eq!(processor.memory.get(&0), Some(&42));
}

#[test]
fn test_stack_underflow() {
    let mut processor = InstructionProcessor::new();
    let result = Store::execute(&mut processor);
    assert!(matches!(result, Err(InstructionError::StackUnderflow)));
}