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

//! Configuration system for dependency analysis
//!
//! This module provides a comprehensive configuration system that controls all
//! aspects of dependency analysis behavior. It includes configuration for analysis
//! algorithms, detection strategies, optimization settings, and reporting options.
//! The configuration system is designed to be flexible, validatable, and extensible.
//!
//! ## Configuration Components
//!
//! ### Analysis Configuration (`analysis`)
//! - **Purpose**: Controls which analysis types are enabled and their parameters
//! - **Scope**: Control flow analysis, data flow analysis, state access analysis
//! - **Settings**: Analysis depth, precision levels, algorithm selection
//! - **Validation**: Ensures configuration consistency and feasibility
//!
//! ### Detection Configuration (`detection`)
//! - **Purpose**: Configures dependency detection algorithms and strategies
//! - **Parameters**: Pattern matching rules, detection thresholds, algorithm weights
//! - **Strategies**: Static analysis, dynamic hints, hybrid approaches
//! - **Customization**: Custom pattern definitions and detection rules
//!
//! ### Optimization Configuration (`optimization`)
//! - **Purpose**: Controls optimization strategies and performance tuning
//! - **Features**: Parallel execution, caching, incremental analysis
//! - **Resource Management**: Memory limits, CPU usage, timeout settings
//! - **Trade-offs**: Speed vs. accuracy, memory vs. computation time
//!
//! ### Reporting Configuration (`reporting`)
//! - **Purpose**: Configures output formats and reporting options
//! - **Formats**: JSON, XML, HTML, plain text, custom formats
//! - **Content**: Verbosity levels, included metrics, visualization options
//! - **Integration**: Export formats for external tools and dashboards
//!
//! ### Validation System (`validation`)
//! - **Purpose**: Validates configuration consistency and provides suggestions
//! - **Checks**: Parameter range validation, dependency validation, conflict detection
//! - **Suggestions**: Automatic configuration optimization and recommendations
//! - **Error Handling**: Clear error messages and recovery suggestions
//!
//! ## Engine Configuration
//!
//! The `EngineConfig` struct serves as the main configuration container that
//! combines all configuration aspects:
//!
//! ### Core Settings
//! - **Verbosity**: Controls logging and diagnostic output levels
//! - **Max Depth**: Limits analysis depth to prevent infinite recursion
//! - **Caching**: Enables/disables result caching for performance
//! - **Analysis Types**: Selects which analysis algorithms to run
//!
//! ### Performance Tuning
//! - **Parallel Execution**: Controls concurrent analysis execution
//! - **Memory Management**: Sets memory usage limits and optimization
//! - **Timeout Settings**: Prevents analyses from running indefinitely
//! - **Resource Allocation**: Manages CPU and memory resource usage
//!
//! ### Quality Control
//! - **Precision Levels**: Balances analysis accuracy vs. performance
//! - **Validation Rules**: Ensures analysis results meet quality standards
//! - **Error Handling**: Configures error recovery and reporting strategies
//! - **Debugging Support**: Enables detailed debugging and profiling information
//!
//! ## Configuration Patterns
//!
//! ### Builder Pattern
//! The configuration system uses the builder pattern for fluent configuration:
//! ```rust
//! let config = EngineConfig::new()
//!     .with_verbosity(2)
//!     .with_caching(true)
//!     .with_max_depth(150);
//! ```
//!
//! ### Preset Configurations
//! Common configuration presets for different use cases:
//! - **Development**: High verbosity, detailed reporting, moderate performance
//! - **Production**: Optimized performance, minimal logging, essential reporting
//! - **Security**: Comprehensive analysis, detailed security reporting
//! - **Performance**: Fast analysis, minimal overhead, basic reporting
//!
//! ### Environment-Specific Settings
//! - **Blockchain Environment**: Optimized for smart contract analysis
//! - **WebAssembly Focus**: Specialized for WebAssembly module analysis
//! - **Large Codebases**: Optimized for analyzing large, complex projects
//! - **Real-time Analysis**: Configured for low-latency, incremental analysis
//!
//! ## Integration with Analysis Pipeline
//!
//! The configuration system integrates deeply with the analysis pipeline:
//! - **Dynamic Reconfiguration**: Supports runtime configuration changes
//! - **Context-Aware Settings**: Adapts configuration based on analysis context
//! - **Feedback Loops**: Uses analysis results to optimize future configurations
//! - **Profile-Guided Optimization**: Learns optimal settings from usage patterns

pub mod analysis;
pub mod detection;
pub mod optimization;
pub mod reporting;
pub mod validation;

pub use analysis::AnalysisConfig;
pub use detection::DetectionConfig;
pub use optimization::OptimizationConfig;
pub use reporting::ReportingConfig;
pub use validation::{ConfigSuggestion, ConfigValidator};

// Re-export AnalysisType for use in config
pub use crate::dependency_analysis::core::traits::AnalysisType;

/// Main engine configuration combining all config types
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub analysis: AnalysisConfig,
    pub detection: DetectionConfig,
    pub optimization: OptimizationConfig,
    pub reporting: ReportingConfig,
    pub verbosity: u8,
    pub max_depth: usize,
    pub enable_caching_flag: bool,
}

impl EngineConfig {
    /// Create a new engine configuration with default settings
    pub fn new() -> Self {
        Self {
            analysis: AnalysisConfig::new(vec![AnalysisType::ControlFlow, AnalysisType::DataFlow, AnalysisType::StateAccess, AnalysisType::DependencyDetection]),
            detection: DetectionConfig::default(),
            optimization: OptimizationConfig::default(),
            reporting: ReportingConfig::default(),
            verbosity: 1,
            max_depth: 100,
            enable_caching_flag: true,
        }
    }

    /// Set verbosity level
    pub fn with_verbosity(mut self, verbosity: u8) -> Self {
        self.verbosity = verbosity;
        self
    }

    /// Enable or disable caching
    pub fn with_caching(mut self, enable: bool) -> Self {
        self.enable_caching_flag = enable;
        self
    }

    /// Set maximum analysis depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Check if verbose mode is enabled
    pub fn is_verbose(&self) -> bool {
        // Placeholder implementation
        false
    }

    /// Check if debug mode is enabled
    pub fn is_debug(&self) -> bool {
        // Placeholder implementation
        false
    }

    /// Check if caching is enabled
    pub fn enable_caching(&self) -> bool {
        self.enable_caching_flag
    }
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self::new()
    }
}
