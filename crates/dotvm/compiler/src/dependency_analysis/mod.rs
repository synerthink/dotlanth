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

//! Dependency analysis module for the DotVM compiler
//!
//! This module provides comprehensive dependency analysis capabilities including:
//! - State access analysis
//! - Data flow analysis  
//! - Control flow graph generation
//! - Dependency detection with customizable patterns
//!
//! # Example Usage
//!
//! ```rust
//! use dotvm_compiler::dependency_analysis::{DependencyAnalysisEngine, EngineConfig};
//!
//! let config = EngineConfig::new()
//!     .with_verbosity(1)
//!     .with_caching(true);
//!     
//! let mut engine = DependencyAnalysisEngine::new(config);
//!
//! let code = r#"
//!     dep:module1
//!     let x = get_state("counter");
//!     if (x > 0) {
//!         set_state("counter", x + 1);
//!     }
//! "#;
//!
//! let result = engine.analyze(code).unwrap();
//! println!("Found {} dependencies", result.dependencies.len());
//! ```

pub mod analyzers;
pub mod config;
pub mod detection;
pub mod engine;

// Re-export main types for convenience
pub use config::EngineConfig;
pub use engine::{DependencyAnalysisEngine, DependencyAnalysisResult};

// Re-export analyzer types
pub use analyzers::{
    AnalysisError, AnalysisResult, AnalysisStats, Analyzer,
    control_flow::{ControlFlowAnalyzer, ControlFlowGraph, ControlFlowNodeType},
    data_flow::{DataFlowAnalysis, DataFlowAnalyzer, DataFlowIssue, DataFlowIssueType},
    state_access::{StateAccessAnalysis, StateAccessAnalyzer, StateAccessType, StateConflict},
};

// Re-export detection types
pub use detection::pattern_matcher::PatternType;
pub use detection::{DependencyDetector, DependencyInfo, DependencyType, DetectorRegistry, MatchResult, Pattern, PatternMatcher};

// Legacy compatibility - re-export the old types with deprecation warnings
#[deprecated(since = "2.0.0", note = "Use EngineConfig instead")]
pub type EngineConfig_Legacy = config::EngineConfig;

#[deprecated(since = "2.0.0", note = "Use DependencyAnalysisEngine instead")]
pub type DependencyAnalysisEngine_Legacy = engine::DependencyAnalysisEngine;

/// Create a dependency analysis engine with default configuration
///
/// This is a convenience function for quick setup.
pub fn create_default_engine() -> DependencyAnalysisEngine {
    DependencyAnalysisEngine::with_default_config()
}

/// Create a dependency analysis engine with custom configuration
///
/// # Arguments
/// * `verbosity` - Verbosity level (0 = quiet, 1 = normal, 2 = verbose)
/// * `enable_caching` - Whether to enable result caching
/// * `max_depth` - Maximum depth for dependency traversal
///
/// # Example
/// ```rust
/// use dotvm_compiler::dependency_analysis::create_engine;
///
/// let engine = create_engine(2, true, 50);
/// ```
pub fn create_engine(verbosity: u8, enable_caching: bool, max_depth: usize) -> DependencyAnalysisEngine {
    let config = EngineConfig::new().with_verbosity(verbosity).with_caching(enable_caching).with_max_depth(max_depth);

    DependencyAnalysisEngine::new(config)
}

/// Quick analysis function for simple use cases
///
/// This function performs a complete dependency analysis with default settings.
///
/// # Arguments
/// * `input` - The code to analyze
///
/// # Returns
/// * `Ok(DependencyAnalysisResult)` - Analysis results
/// * `Err(AnalysisError)` - Analysis error
///
/// # Example
/// ```rust
/// use dotvm_compiler::dependency_analysis::quick_analyze;
///
/// let code = "dep:module1\nlet x = 1;";
/// let result = quick_analyze(code).unwrap();
/// println!("Found {} dependencies", result.dependencies.len());
/// ```
pub fn quick_analyze(input: &str) -> AnalysisResult<DependencyAnalysisResult> {
    let mut engine = create_default_engine();
    engine.analyze(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_engine() {
        let engine = create_default_engine();
        assert_eq!(engine.config().verbosity, 1);
        assert!(engine.config().enable_caching);
    }

    #[test]
    fn test_create_engine() {
        let engine = create_engine(2, false, 200);
        assert_eq!(engine.config().verbosity, 2);
        assert!(!engine.config().enable_caching);
        assert_eq!(engine.config().max_depth, 200);
    }

    #[test]
    fn test_quick_analyze() {
        let input = r#"
            dep:module1
            let x = get_state("test");
            if (x > 0) {
                return x;
            }
        "#;

        let result = quick_analyze(input).unwrap();
        assert!(!result.dependencies.is_empty());
        assert!(result.state_access.is_some());
        assert!(result.data_flow.is_some());
        assert!(result.control_flow.is_some());
    }

    #[test]
    fn test_module_exports() {
        // Test that all main types are accessible
        let _config: EngineConfig = EngineConfig::new();
        let _engine: DependencyAnalysisEngine = create_default_engine();

        // Test analyzer types
        let _analyzer: StateAccessAnalyzer = StateAccessAnalyzer::new();
        let _data_analyzer: DataFlowAnalyzer = DataFlowAnalyzer::new();
        let _cfg_analyzer: ControlFlowAnalyzer = ControlFlowAnalyzer::new();

        // Test detection types
        let _registry: DetectorRegistry = DetectorRegistry::new();
        let _pattern: Pattern = Pattern::exact("test".to_string());
    }

    #[test]
    fn test_error_handling() {
        let result = quick_analyze("");
        assert!(matches!(result, Err(AnalysisError::EmptyInput)));
    }

    #[test]
    fn test_comprehensive_analysis() {
        let input = r#"
            // Module dependencies
            dep:crypto
            import math
            require("fs")
            
            // State operations
            let balance = get_state("balance");
            set_state("balance", balance + 100);
            
            // Control flow
            if (balance > 1000) {
                while (balance > 0) {
                    balance = balance - 10;
                    update_state("balance", balance);
                }
            } else {
                return balance;
            }
            
            // Function calls
            let result = calculate(balance, 0.1);
            return result;
        "#;

        let result = quick_analyze(input).unwrap();

        // Should find multiple dependencies
        assert!(result.dependencies.len() >= 3);

        // Should have state access analysis
        let state_analysis = result.state_access.unwrap();
        assert!(!state_analysis.accesses.is_empty());
        assert!(state_analysis.locations.contains("balance"));

        // Should have data flow analysis
        let data_analysis = result.data_flow.unwrap();
        assert!(!data_analysis.variables.is_empty());

        // Should have control flow analysis
        let cfg = result.control_flow.unwrap();
        assert!(!cfg.nodes.is_empty());
        assert!(!cfg.edges.is_empty());
        assert!(cfg.complexity.cyclomatic > 1); // Has conditional and loop

        // Should have analysis metadata
        assert!(result.metadata.contains_key("analysis_time"));
        assert!(result.metadata.contains_key("input_size"));
    }
}
