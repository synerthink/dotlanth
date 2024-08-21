use crate::core::execution_engine::{
    errors::InstructionError,
    instructions::{InstructionProcessor, Load},
};

#[test]
fn test_load_instruction() {
    let mut processor = InstructionProcessor::new();
    processor.memory.insert(0, 42);
    processor.stack.push(0);
    Load::execute(&mut processor).unwrap();
    assert_eq!(processor.stack.pop(), Some(42));
}

#[test]
fn test_stack_underflow() {
    let mut processor = InstructionProcessor::new();
    let result = Load::execute(&mut processor);
    assert!(matches!(result, Err(InstructionError::StackUnderflow)));
}

#[test]
fn test_invalid_memory_address() {
    let mut processor = InstructionProcessor::new();
    processor.stack.push(999); // Invalid memory address
    let result = Load::execute(&mut processor);
    assert!(matches!(result, Err(InstructionError::InvalidMemoryAddress)));
}
