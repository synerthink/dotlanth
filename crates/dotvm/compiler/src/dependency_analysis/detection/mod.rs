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

//! Dependency detection algorithms and pattern matching
//!
//! This module provides sophisticated algorithms for detecting dependencies in
//! WebAssembly modules and DotVM bytecode. It uses pattern matching, heuristic
//! analysis, and configurable detection strategies to identify various types
//! of dependencies including module imports, function calls, and state access.
//!
//! ## Core Detection Capabilities
//!
//! ### Dependency Detection (`dependency_detector`)
//! - **Purpose**: Main interface for dependency detection operations
//! - **Scope**: Detects module dependencies, function calls, external references
//! - **Algorithms**: Pattern-based detection, AST traversal, bytecode analysis
//! - **Output**: Structured dependency information with metadata and locations
//!
//! ### Pattern Matching (`pattern_matcher`)
//! - **Purpose**: Flexible pattern matching engine for dependency identification
//! - **Patterns**: Regular expressions, syntax patterns, semantic patterns
//! - **Extensibility**: Supports custom pattern definitions and matching rules
//! - **Performance**: Optimized matching algorithms for large codebases
//!
//! ### Detection Strategies (`strategies`)
//! - **Static Analysis**: Compile-time dependency detection without execution
//! - **Dynamic Hints**: Runtime information to improve detection accuracy
//! - **Hybrid Approach**: Combines static and dynamic information for best results
//! - **Configurable**: Adjustable strategies based on analysis requirements
//!
//! ### Pattern Matchers (`matchers`)
//! - **Graph Matchers**: Detect dependencies through graph structure analysis
//! - **Heuristic Matchers**: Use heuristics for probabilistic dependency detection
//! - **Instruction Matchers**: Analyze individual instructions for dependencies
//! - **Sequence Matchers**: Detect patterns in instruction sequences
//!
//! ### Pattern Definitions (`patterns`)
//! - **Control Patterns**: Control flow related dependencies
//! - **Data Patterns**: Data flow and variable dependencies
//! - **Dependency Patterns**: Module and external dependencies
//! - **State Patterns**: Blockchain state access patterns
//!
//! ## Dependency Types
//!
//! ### Module Dependencies
//! - **Import Statements**: Direct module imports and requires
//! - **Dynamic Imports**: Runtime module loading and dependency injection
//! - **Transitive Dependencies**: Dependencies of imported modules
//! - **Circular Dependencies**: Detection and reporting of circular references
//!
//! ### Function Dependencies
//! - **Direct Calls**: Explicit function calls and invocations
//! - **Indirect Calls**: Function pointer calls and dynamic dispatch
//! - **Callback Dependencies**: Functions passed as callbacks or handlers
//! - **Virtual Calls**: Interface and virtual method calls
//!
//! ### Data Dependencies
//! - **Variable References**: Dependencies through shared variables
//! - **Memory Dependencies**: Shared memory access patterns
//! - **State Dependencies**: Blockchain state variable dependencies
//! - **Resource Dependencies**: File, network, and system resource access
//!
//! ## Detection Algorithms
//!
//! ### Static Analysis
//! - **AST Traversal**: Systematic traversal of abstract syntax trees
//! - **Control Flow Analysis**: Dependencies through control flow paths
//! - **Data Flow Analysis**: Dependencies through data flow chains
//! - **Symbol Resolution**: Resolving symbols to their definitions
//!
//! ### Pattern Recognition
//! - **Syntax Patterns**: Recognizing dependency patterns in source code
//! - **Semantic Patterns**: Understanding meaning behind code constructs
//! - **Behavioral Patterns**: Identifying patterns in program behavior
//! - **Anti-Patterns**: Detecting problematic dependency patterns
//!
//! ### Heuristic Methods
//! - **Probabilistic Detection**: Using probability for uncertain dependencies
//! - **Machine Learning**: Learning patterns from training data
//! - **Fuzzy Matching**: Approximate matching for similar patterns
//! - **Confidence Scoring**: Assigning confidence levels to detected dependencies
//!
//! ## Registry System
//!
//! The `DetectorRegistry` provides a flexible system for managing multiple detectors:
//! - **Registration**: Easy registration of new detector implementations
//! - **Discovery**: Automatic discovery and loading of available detectors
//! - **Coordination**: Coordinated execution of multiple detectors
//! - **Result Aggregation**: Combining results from different detectors
//!
//! ## Performance Optimization
//!
//! - **Parallel Detection**: Running multiple detectors concurrently
//! - **Incremental Analysis**: Only analyzing changed parts of code
//! - **Caching**: Caching detection results for repeated analysis
//! - **Early Termination**: Stopping analysis when sufficient information is found
//!
//! ## Integration with Security Analysis
//!
//! - **Vulnerability Detection**: Identifying potentially dangerous dependencies
//! - **Supply Chain Analysis**: Analyzing dependency chains for security risks
//! - **Malware Detection**: Detecting suspicious dependency patterns
//! - **Compliance Checking**: Ensuring dependencies meet security requirements

pub mod dependency_detector;
pub mod matchers;
pub mod pattern_matcher;
pub mod patterns;
pub mod strategies;

pub use dependency_detector::{DependencyDetector, DependencyInfo, DependencyType};
pub use pattern_matcher::{MatchResult, Pattern};

use std::collections::HashMap;

/// Common trait for all dependency detection algorithms
pub trait Detector {
    /// The type of dependencies this detector finds
    type Dependency;

    /// Detect dependencies in the given input
    fn detect(&self, input: &str) -> Vec<Self::Dependency>;

    /// Get the name of this detector
    fn name(&self) -> &'static str;

    /// Check if this detector can handle the given input
    fn can_detect(&self, input: &str) -> bool {
        !input.trim().is_empty()
    }
}

/// Registry for managing multiple detectors
pub struct DetectorRegistry {
    /// Registered detectors
    detectors: HashMap<String, Box<dyn Detector<Dependency = DependencyInfo>>>,
}

impl DetectorRegistry {
    /// Create a new detector registry
    pub fn new() -> Self {
        Self { detectors: HashMap::new() }
    }

    /// Register a new detector
    pub fn register<D>(&mut self, name: String, detector: D)
    where
        D: Detector<Dependency = DependencyInfo> + 'static,
    {
        self.detectors.insert(name, Box::new(detector));
    }

    /// Get a detector by name
    pub fn get(&self, name: &str) -> Option<&dyn Detector<Dependency = DependencyInfo>> {
        self.detectors.get(name).map(|d| d.as_ref())
    }

    /// Run all registered detectors on the input
    pub fn detect_all(&self, input: &str) -> HashMap<String, Vec<DependencyInfo>> {
        let mut results = HashMap::new();

        for (name, detector) in &self.detectors {
            if detector.can_detect(input) {
                let dependencies = detector.detect(input);
                results.insert(name.clone(), dependencies);
            }
        }

        results
    }

    /// List all registered detector names
    pub fn list_detectors(&self) -> Vec<&String> {
        self.detectors.keys().collect()
    }
}

impl Default for DetectorRegistry {
    fn default() -> Self {
        let mut registry = Self::new();

        // Register default detectors
        registry.register("basic".to_string(), dependency_detector::BasicDependencyDetector::new());

        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDetector;

    impl Detector for TestDetector {
        type Dependency = DependencyInfo;

        fn detect(&self, _input: &str) -> Vec<Self::Dependency> {
            vec![DependencyInfo {
                name: "test_dep".to_string(),
                dependency_type: DependencyType::Module,
                source_location: None,
                metadata: HashMap::new(),
            }]
        }

        fn name(&self) -> &'static str {
            "test"
        }
    }

    #[test]
    fn test_detector_registry_creation() {
        let registry = DetectorRegistry::new();
        assert!(registry.detectors.is_empty());
    }

    #[test]
    fn test_detector_registry_register() {
        let mut registry = DetectorRegistry::new();
        registry.register("test".to_string(), TestDetector);

        assert_eq!(registry.detectors.len(), 1);
        assert!(registry.get("test").is_some());
    }

    #[test]
    fn test_detector_registry_detect_all() {
        let mut registry = DetectorRegistry::new();
        registry.register("test".to_string(), TestDetector);

        let results = registry.detect_all("some input");
        assert_eq!(results.len(), 1);
        assert!(results.contains_key("test"));
        assert_eq!(results["test"].len(), 1);
    }

    #[test]
    fn test_default_registry() {
        let registry = DetectorRegistry::default();
        assert!(!registry.detectors.is_empty());
        assert!(registry.get("basic").is_some());
    }
}
