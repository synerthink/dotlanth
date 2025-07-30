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
