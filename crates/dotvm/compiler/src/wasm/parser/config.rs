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

//! Parser configuration and feature flags

use super::super::{ast::SectionLimits, error::WasmResult};
use wasmparser::WasmFeatures;

/// Configuration for the WASM parser
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// WASM features to enable
    pub features: WasmFeatures,
    /// Section size limits
    pub limits: SectionLimits,
    /// Whether to validate the module structure
    pub validate_structure: bool,
    /// Whether to preserve custom sections
    pub preserve_custom_sections: bool,
    /// Whether to parse debug information
    pub parse_debug_info: bool,
    /// Whether to perform strict validation
    pub strict_validation: bool,
    /// Maximum nesting depth for control structures
    pub max_nesting_depth: u32,
    /// Whether to allow multi-value returns
    pub allow_multi_value: bool,
    /// Whether to allow bulk memory operations
    pub allow_bulk_memory: bool,
    /// Whether to allow SIMD instructions
    pub allow_simd: bool,
    /// Whether to allow reference types
    pub allow_reference_types: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            features: WasmFeatures::default(),
            limits: SectionLimits::default(),
            validate_structure: true,
            preserve_custom_sections: true,
            parse_debug_info: true,
            strict_validation: false,
            max_nesting_depth: 1024,
            allow_multi_value: true,
            allow_bulk_memory: true,
            allow_simd: true,
            allow_reference_types: true,
        }
    }
}

impl ParserConfig {
    /// Create a new parser configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a strict configuration for security-sensitive environments
    pub fn strict() -> Self {
        Self {
            features: WasmFeatures::default(),
            limits: SectionLimits::strict(),
            validate_structure: true,
            preserve_custom_sections: false,
            parse_debug_info: false,
            strict_validation: true,
            max_nesting_depth: 100,
            allow_multi_value: false,
            allow_bulk_memory: false,
            allow_simd: false,
            allow_reference_types: false,
        }
    }

    /// Create a permissive configuration for development
    pub fn permissive() -> Self {
        Self {
            features: WasmFeatures::all(),
            limits: SectionLimits::unlimited(),
            validate_structure: false,
            preserve_custom_sections: true,
            parse_debug_info: true,
            strict_validation: false,
            max_nesting_depth: 10000,
            allow_multi_value: true,
            allow_bulk_memory: true,
            allow_simd: true,
            allow_reference_types: true,
        }
    }

    /// Enable specific WASM features
    pub fn with_features(mut self, features: WasmFeatures) -> Self {
        self.features = features;
        self
    }

    /// Set section limits
    pub fn with_limits(mut self, limits: SectionLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Enable or disable structure validation
    pub fn with_validation(mut self, validate: bool) -> Self {
        self.validate_structure = validate;
        self
    }

    /// Enable or disable custom section preservation
    pub fn with_custom_sections(mut self, preserve: bool) -> Self {
        self.preserve_custom_sections = preserve;
        self
    }

    /// Enable or disable debug information parsing
    pub fn with_debug_info(mut self, parse: bool) -> Self {
        self.parse_debug_info = parse;
        self
    }

    /// Enable or disable strict validation
    pub fn with_strict_validation(mut self, strict: bool) -> Self {
        self.strict_validation = strict;
        self
    }

    /// Set maximum nesting depth
    pub fn with_max_nesting_depth(mut self, depth: u32) -> Self {
        self.max_nesting_depth = depth;
        self
    }

    /// Enable or disable multi-value returns
    pub fn with_multi_value(mut self, allow: bool) -> Self {
        self.allow_multi_value = allow;
        if allow {
            self.features.multi_value = true;
        }
        self
    }

    /// Enable or disable bulk memory operations
    pub fn with_bulk_memory(mut self, allow: bool) -> Self {
        self.allow_bulk_memory = allow;
        if allow {
            self.features.bulk_memory = true;
        }
        self
    }

    /// Enable or disable SIMD instructions
    pub fn with_simd(mut self, allow: bool) -> Self {
        self.allow_simd = allow;
        if allow {
            self.features.simd = true;
        }
        self
    }

    /// Enable or disable reference types
    pub fn with_reference_types(mut self, allow: bool) -> Self {
        self.allow_reference_types = allow;
        if allow {
            self.features.reference_types = true;
        }
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> WasmResult<()> {
        if self.max_nesting_depth == 0 {
            return Err(super::super::error::WasmError::invalid_binary("Maximum nesting depth cannot be zero"));
        }

        if self.max_nesting_depth > 100000 {
            return Err(super::super::error::WasmError::invalid_binary("Maximum nesting depth is too large (max 100000)"));
        }

        Ok(())
    }

    /// Check if a feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "multi_value" => self.allow_multi_value && self.features.multi_value,
            "bulk_memory" => self.allow_bulk_memory && self.features.bulk_memory,
            "simd" => self.allow_simd && self.features.simd,
            "reference_types" => self.allow_reference_types && self.features.reference_types,
            "threads" => self.features.threads,
            "tail_call" => self.features.tail_call,
            "function_references" => self.features.function_references,
            "gc" => self.features.gc,
            "memory64" => self.features.memory64,
            "exceptions" => self.features.exceptions,
            "component_model" => self.features.component_model,
            _ => false,
        }
    }

    /// Get a summary of enabled features
    pub fn feature_summary(&self) -> Vec<String> {
        let mut features = Vec::new();

        if self.is_feature_enabled("multi_value") {
            features.push("multi_value".to_string());
        }
        if self.is_feature_enabled("bulk_memory") {
            features.push("bulk_memory".to_string());
        }
        if self.is_feature_enabled("simd") {
            features.push("simd".to_string());
        }
        if self.is_feature_enabled("reference_types") {
            features.push("reference_types".to_string());
        }
        if self.is_feature_enabled("threads") {
            features.push("threads".to_string());
        }
        if self.is_feature_enabled("tail_call") {
            features.push("tail_call".to_string());
        }
        if self.is_feature_enabled("function_references") {
            features.push("function_references".to_string());
        }
        if self.is_feature_enabled("gc") {
            features.push("gc".to_string());
        }
        if self.is_feature_enabled("memory64") {
            features.push("memory64".to_string());
        }
        if self.is_feature_enabled("exceptions") {
            features.push("exceptions".to_string());
        }
        if self.is_feature_enabled("component_model") {
            features.push("component_model".to_string());
        }

        features
    }
}

/// Builder for parser configuration
pub struct ParserConfigBuilder {
    config: ParserConfig,
}

impl ParserConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self { config: ParserConfig::default() }
    }

    /// Start with strict configuration
    pub fn strict() -> Self {
        Self { config: ParserConfig::strict() }
    }

    /// Start with permissive configuration
    pub fn permissive() -> Self {
        Self { config: ParserConfig::permissive() }
    }

    /// Set WASM features
    pub fn features(mut self, features: WasmFeatures) -> Self {
        self.config.features = features;
        self
    }

    /// Set section limits
    pub fn limits(mut self, limits: SectionLimits) -> Self {
        self.config.limits = limits;
        self
    }

    /// Enable validation
    pub fn validation(mut self, enable: bool) -> Self {
        self.config.validate_structure = enable;
        self
    }

    /// Enable custom sections
    pub fn custom_sections(mut self, enable: bool) -> Self {
        self.config.preserve_custom_sections = enable;
        self
    }

    /// Enable debug info
    pub fn debug_info(mut self, enable: bool) -> Self {
        self.config.parse_debug_info = enable;
        self
    }

    /// Enable strict validation
    pub fn strict_validation(mut self, enable: bool) -> Self {
        self.config.strict_validation = enable;
        self
    }

    /// Set max nesting depth
    pub fn max_nesting_depth(mut self, depth: u32) -> Self {
        self.config.max_nesting_depth = depth;
        self
    }

    /// Enable multi-value
    pub fn multi_value(mut self, enable: bool) -> Self {
        self.config.allow_multi_value = enable;
        if enable {
            self.config.features.multi_value = true;
        }
        self
    }

    /// Enable bulk memory
    pub fn bulk_memory(mut self, enable: bool) -> Self {
        self.config.allow_bulk_memory = enable;
        if enable {
            self.config.features.bulk_memory = true;
        }
        self
    }

    /// Enable SIMD
    pub fn simd(mut self, enable: bool) -> Self {
        self.config.allow_simd = enable;
        if enable {
            self.config.features.simd = true;
        }
        self
    }

    /// Enable reference types
    pub fn reference_types(mut self, enable: bool) -> Self {
        self.config.allow_reference_types = enable;
        if enable {
            self.config.features.reference_types = true;
        }
        self
    }

    /// Build the configuration
    pub fn build(self) -> WasmResult<ParserConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl Default for ParserConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ParserConfig::default();
        assert!(config.validate_structure);
        assert!(config.preserve_custom_sections);
        assert_eq!(config.max_nesting_depth, 1024);
    }

    #[test]
    fn test_strict_config() {
        let config = ParserConfig::strict();
        assert!(config.strict_validation);
        assert!(!config.preserve_custom_sections);
        assert_eq!(config.max_nesting_depth, 100);
    }

    #[test]
    fn test_config_builder() {
        let config = ParserConfigBuilder::new().validation(false).max_nesting_depth(500).simd(true).build().unwrap();

        assert!(!config.validate_structure);
        assert_eq!(config.max_nesting_depth, 500);
        assert!(config.allow_simd);
    }

    #[test]
    fn test_feature_checking() {
        let config = ParserConfig::default();
        assert!(config.is_feature_enabled("multi_value"));
        assert!(config.is_feature_enabled("bulk_memory"));

        let strict_config = ParserConfig::strict();
        assert!(!strict_config.is_feature_enabled("simd"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = ParserConfig::default();
        config.max_nesting_depth = 0;
        assert!(config.validate().is_err());

        config.max_nesting_depth = 1000;
        assert!(config.validate().is_ok());
    }
}
