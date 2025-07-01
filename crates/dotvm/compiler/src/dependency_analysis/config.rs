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

//! Configuration types for dependency analysis

/// Configuration for the dependency analysis engine
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Verbosity level: 0 = quiet, 1 = normal, 2 = verbose
    pub verbosity: u8,
    /// Enable caching of analysis results
    pub enable_caching: bool,
    /// Maximum depth for dependency traversal
    pub max_depth: usize,
    /// Enable parallel analysis when possible
    pub enable_parallel: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            verbosity: 1,
            enable_caching: true,
            max_depth: 100,
            enable_parallel: false,
        }
    }
}

impl EngineConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set verbosity level
    pub fn with_verbosity(mut self, verbosity: u8) -> Self {
        self.verbosity = verbosity;
        self
    }

    /// Enable or disable caching
    pub fn with_caching(mut self, enable: bool) -> Self {
        self.enable_caching = enable;
        self
    }

    /// Set maximum dependency traversal depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Enable or disable parallel analysis
    pub fn with_parallel(mut self, enable: bool) -> Self {
        self.enable_parallel = enable;
        self
    }

    /// Check if verbose logging is enabled
    pub fn is_verbose(&self) -> bool {
        self.verbosity > 0
    }

    /// Check if debug logging is enabled
    pub fn is_debug(&self) -> bool {
        self.verbosity > 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EngineConfig::default();
        assert_eq!(config.verbosity, 1);
        assert!(config.enable_caching);
        assert_eq!(config.max_depth, 100);
        assert!(!config.enable_parallel);
    }

    #[test]
    fn test_config_builder() {
        let config = EngineConfig::new().with_verbosity(2).with_caching(false).with_max_depth(50).with_parallel(true);

        assert_eq!(config.verbosity, 2);
        assert!(!config.enable_caching);
        assert_eq!(config.max_depth, 50);
        assert!(config.enable_parallel);
    }

    #[test]
    fn test_verbosity_checks() {
        let quiet_config = EngineConfig::new().with_verbosity(0);
        assert!(!quiet_config.is_verbose());
        assert!(!quiet_config.is_debug());

        let normal_config = EngineConfig::new().with_verbosity(1);
        assert!(normal_config.is_verbose());
        assert!(!normal_config.is_debug());

        let debug_config = EngineConfig::new().with_verbosity(2);
        assert!(debug_config.is_verbose());
        assert!(debug_config.is_debug());
    }
}
