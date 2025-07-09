//! Analysis utilities for optimizer

pub mod cfg;
pub mod def_use;
pub mod dominators;
pub mod loops;
pub mod side_effects;

pub use cfg::{BasicBlock, BlockId, ControlFlowGraph};
pub use def_use::{DefUseAnalyzer, DefUseChains, Definition, Use, VariableId};
pub use dominators::{DominatorAnalyzer, DominatorTree};
pub use loops::{LoopAnalyzer, LoopInfo};
pub use side_effects::{SideEffectAnalyzer, SideEffectInfo};
