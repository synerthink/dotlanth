// Dotlanth
// Copyright (C) 2025 Synerthink

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Analyzer framework and common types for dependency analysis
//!
//! This module provides the core analyzer implementations that perform different types
//! of analysis on WebAssembly modules and DotVM bytecode. Each analyzer specializes in
//! a specific aspect of program analysis and contributes to the overall understanding
//! of the code's behavior, dependencies, and structure.
//!
//! ## Available Analyzers
//!
//! ### Control Flow Analysis (`control_flow`)
//! - **Purpose**: Constructs and analyzes control flow graphs (CFGs)
//! - **Capabilities**: Loop detection, dominance analysis, reachability analysis
//! - **Use Cases**: Optimization, dead code elimination, complexity analysis
//! - **Output**: Control flow graphs, loop structures, dominance trees
//!
//! ### Data Flow Analysis (`data_flow`)
//! - **Purpose**: Tracks how data flows through the program
//! - **Capabilities**: Definition-use chains, liveness analysis, reaching definitions
//! - **Use Cases**: Variable optimization, constant propagation, dead code elimination
//! - **Output**: Data flow information, variable lifetime data
//!
//! ### State Access Analysis (`state_access`)
//! - **Purpose**: Analyzes state access patterns
//! - **Capabilities**: Read/write tracking, conflict detection, optimization hints
//! - **Use Cases**: Security analysis, reentrancy detection
//! - **Output**: State access patterns, conflict reports, optimization suggestions
//!
//! ## Common Analysis Framework
//!
//! All analyzers share common interfaces and data structures:
//! - **AnalysisResult**: Standardized result type for all analyses
//! - **AnalysisStats**: Performance and statistical information
//! - **Error Handling**: Consistent error reporting across analyzers
//! - **Configuration**: Unified configuration system for analysis parameters
//!
//! ## Integration with Compiler Pipeline
//!
//! These analyzers integrate seamlessly with the DotVM compiler pipeline:
//! 1. **Input**: Receive parsed WebAssembly AST or DotVM bytecode
//! 2. **Analysis**: Perform specialized analysis using domain-specific algorithms
//! 3. **Output**: Provide structured results for optimization and code generation
//! 4. **Caching**: Support result caching for improved performance
//!
//! ## Performance Considerations
//!
//! - **Incremental Analysis**: Support for analyzing only changed parts of code
//! - **Parallel Execution**: Some analyzers can run concurrently
//! - **Memory Efficiency**: Optimized data structures for large programs
//! - **Configurable Depth**: Adjustable analysis depth for performance tuning

pub mod control_flow;
pub mod data_flow;
pub mod state_access;

// Re-export common types
pub use crate::dependency_analysis::core::traits::{AnalysisType, DependencyAnalyzer};

// Common error and result types
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Empty input provided to analyzer")]
    EmptyInput,
    #[error("Analysis depth limit exceeded: {0}")]
    DepthLimitExceeded(usize),
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
}

pub type AnalysisResult<T> = Result<T, AnalysisError>;

use std::collections::HashMap;

/// Common trait for all analyzers
pub trait Analyzer {
    /// The type of result this analyzer produces
    type Result;

    /// Analyze the given input and return the result
    fn analyze(&self, input: &str) -> AnalysisResult<Self::Result>;

    /// Get the name of this analyzer
    fn name(&self) -> &'static str;

    /// Check if this analyzer can handle the given input
    fn can_analyze(&self, input: &str) -> bool {
        !input.trim().is_empty()
    }

    /// Reset any internal state (for stateful analyzers)
    fn reset(&mut self) {}
}

/// Analysis context shared between analyzers
#[derive(Debug, Clone)]
pub struct AnalysisContext {
    /// Current analysis depth
    pub depth: usize,
    /// Maximum allowed depth
    pub max_depth: usize,
    /// Visited nodes (for cycle detection)
    pub visited: std::collections::HashSet<String>,
    /// Analysis metadata
    pub metadata: HashMap<String, String>,
}

impl AnalysisContext {
    /// Create a new analysis context
    pub fn new(max_depth: usize) -> Self {
        Self {
            depth: 0,
            max_depth,
            visited: std::collections::HashSet::new(),
            metadata: HashMap::new(),
        }
    }

    /// Enter a new analysis level
    pub fn enter(&mut self, node: String) -> AnalysisResult<()> {
        if self.depth >= self.max_depth {
            return Err(AnalysisError::DepthLimitExceeded(self.max_depth));
        }

        if self.visited.contains(&node) {
            return Err(AnalysisError::CircularDependency(node));
        }

        self.depth += 1;
        self.visited.insert(node);
        Ok(())
    }

    /// Exit the current analysis level
    pub fn exit(&mut self, node: &str) {
        self.depth = self.depth.saturating_sub(1);
        self.visited.remove(node);
    }

    /// Add metadata to the context
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get metadata from the context
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Analysis statistics
#[derive(Debug, Clone, Default)]
pub struct AnalysisStats {
    /// Number of nodes analyzed
    pub nodes_analyzed: usize,
    /// Number of dependencies found
    pub dependencies_found: usize,
    /// Analysis duration in milliseconds
    pub duration_ms: u64,
    /// Maximum depth reached
    pub max_depth_reached: usize,
}

impl AnalysisStats {
    /// Create new empty statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment nodes analyzed counter
    pub fn increment_nodes(&mut self) {
        self.nodes_analyzed += 1;
    }

    /// Add dependencies found
    pub fn add_dependencies(&mut self, count: usize) {
        self.dependencies_found += count;
    }

    /// Set analysis duration
    pub fn set_duration(&mut self, duration_ms: u64) {
        self.duration_ms = duration_ms;
    }

    /// Update maximum depth reached
    pub fn update_max_depth(&mut self, depth: usize) {
        if depth > self.max_depth_reached {
            self.max_depth_reached = depth;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_context_creation() {
        let context = AnalysisContext::new(10);
        assert_eq!(context.depth, 0);
        assert_eq!(context.max_depth, 10);
        assert!(context.visited.is_empty());
        assert!(context.metadata.is_empty());
    }

    #[test]
    fn test_analysis_context_enter_exit() {
        let mut context = AnalysisContext::new(10);

        assert!(context.enter("node1".to_string()).is_ok());
        assert_eq!(context.depth, 1);
        assert!(context.visited.contains("node1"));

        context.exit("node1");
        assert_eq!(context.depth, 0);
        assert!(!context.visited.contains("node1"));
    }

    #[test]
    fn test_analysis_context_depth_limit() {
        let mut context = AnalysisContext::new(2);

        assert!(context.enter("node1".to_string()).is_ok());
        assert!(context.enter("node2".to_string()).is_ok());

        let result = context.enter("node3".to_string());
        assert!(matches!(result, Err(AnalysisError::DepthLimitExceeded(2))));
    }

    #[test]
    fn test_analysis_context_circular_dependency() {
        let mut context = AnalysisContext::new(10);

        assert!(context.enter("node1".to_string()).is_ok());
        let result = context.enter("node1".to_string());
        assert!(matches!(result, Err(AnalysisError::CircularDependency(_))));
    }

    #[test]
    fn test_analysis_stats() {
        let mut stats = AnalysisStats::new();

        stats.increment_nodes();
        stats.add_dependencies(5);
        stats.set_duration(100);
        stats.update_max_depth(3);

        assert_eq!(stats.nodes_analyzed, 1);
        assert_eq!(stats.dependencies_found, 5);
        assert_eq!(stats.duration_ms, 100);
        assert_eq!(stats.max_depth_reached, 3);
    }
}
