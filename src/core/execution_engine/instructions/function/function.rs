use crate::core::execution_engine::instructions::Instruction;

pub struct Function {
    pub start_address: usize,
    pub body: Vec<Box<Instruction>>,
}
