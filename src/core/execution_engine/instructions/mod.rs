pub mod arithmetic;
pub mod control_flow;
pub mod function;
pub mod immediate_value;
pub mod instruction;
pub mod instruction_processor;
pub mod instruction_trait;
pub mod memory;

pub use arithmetic::*;
pub use control_flow::*;
pub use instruction::Instruction;
pub use instruction_processor::InstructionProcessor;
pub use memory::*;
