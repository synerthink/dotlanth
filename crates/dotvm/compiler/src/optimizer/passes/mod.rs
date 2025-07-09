//! Optimization passes for DotVM bytecode

pub mod constant_folding;
pub mod dead_code;
pub mod peephole;

// Re-export main types for convenience
pub use constant_folding::ConstantFolder;
pub use dead_code::DeadCodeEliminator;
pub use peephole::PeepholeOptimizer;
