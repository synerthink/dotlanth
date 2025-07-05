// Dotlanth
// Writer framework for bytecode output

pub mod buffer;
pub mod bytecode;
pub mod formatter;
pub mod patch;
pub mod traits;

// Re-export primary writer types
pub use bytecode::BytecodeWriter;
pub use traits::PatchPoint;
