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

//! Pipeline builder for creating custom transpilation pipelines

use super::{
    super::{
        config::TranspilationConfig,
        error::{TranspilationError, TranspilationResult},
    },
    TranspilationPipeline,
};

/// Builder for creating custom transpilation pipelines
pub struct PipelineBuilder {
    /// Configuration for the pipeline
    config: TranspilationConfig,
    /// Whether to enable preprocessing
    enable_preprocessing: bool,
    /// Whether to enable analysis
    enable_analysis: bool,
    /// Whether to enable translation
    enable_translation: bool,
    /// Whether to enable postprocessing
    enable_postprocessing: bool,
}

impl PipelineBuilder {
    /// Create a new pipeline builder
    pub fn new() -> Self {
        Self {
            config: TranspilationConfig::default(),
            enable_preprocessing: true,
            enable_analysis: true,
            enable_translation: true,
            enable_postprocessing: true,
        }
    }

    /// Create a builder with a specific configuration
    pub fn with_config(config: TranspilationConfig) -> Self {
        Self {
            config,
            enable_preprocessing: true,
            enable_analysis: true,
            enable_translation: true,
            enable_postprocessing: true,
        }
    }

    /// Set the transpilation configuration
    pub fn config(mut self, config: TranspilationConfig) -> Self {
        self.config = config;
        self
    }

    /// Enable or disable preprocessing stage
    pub fn preprocessing(mut self, enable: bool) -> Self {
        self.enable_preprocessing = enable;
        self
    }

    /// Enable or disable analysis stage
    pub fn analysis(mut self, enable: bool) -> Self {
        self.enable_analysis = enable;
        self
    }

    /// Enable or disable translation stage
    pub fn translation(mut self, enable: bool) -> Self {
        self.enable_translation = enable;
        self
    }

    /// Enable or disable postprocessing stage
    pub fn postprocessing(mut self, enable: bool) -> Self {
        self.enable_postprocessing = enable;
        self
    }

    /// Create a minimal pipeline (preprocessing and translation only)
    pub fn minimal(mut self) -> Self {
        self.enable_preprocessing = true;
        self.enable_analysis = false;
        self.enable_translation = true;
        self.enable_postprocessing = false;
        self
    }

    /// Create a fast pipeline (skip analysis and optimizations)
    pub fn fast(mut self) -> Self {
        self.enable_preprocessing = true;
        self.enable_analysis = false;
        self.enable_translation = true;
        self.enable_postprocessing = false;
        self.config.enable_optimizations = false;
        self
    }

    /// Create a debug pipeline (full analysis, no optimizations)
    pub fn debug(mut self) -> Self {
        self.enable_preprocessing = true;
        self.enable_analysis = true;
        self.enable_translation = true;
        self.enable_postprocessing = true;
        self.config.enable_optimizations = false;
        self.config.preserve_debug_info = true;
        self
    }

    /// Create a release pipeline (full optimizations)
    pub fn release(mut self) -> Self {
        self.enable_preprocessing = true;
        self.enable_analysis = true;
        self.enable_translation = true;
        self.enable_postprocessing = true;
        self.config.enable_optimizations = true;
        self.config.preserve_debug_info = false;
        self.config.optimization_level = super::super::config::OptimizationLevel::O3;
        self
    }

    /// Build the pipeline
    pub fn build(self) -> TranspilationResult<TranspilationPipeline> {
        // Validate configuration
        self.config.validate()?;

        // Ensure essential stages are enabled
        if !self.enable_preprocessing {
            return Err(TranspilationError::InvalidConfiguration(
                "Preprocessing stage cannot be disabled - it's required for safety".to_string(),
            ));
        }

        if !self.enable_translation {
            return Err(TranspilationError::InvalidConfiguration(
                "Translation stage cannot be disabled - it's the core functionality".to_string(),
            ));
        }

        // Create the pipeline
        TranspilationPipeline::new(self.config)
    }

    /// Build a custom pipeline with specific stage configuration
    pub fn build_custom(self) -> TranspilationResult<CustomPipeline> {
        self.config.validate()?;

        Ok(CustomPipeline {
            config: self.config,
            enable_preprocessing: self.enable_preprocessing,
            enable_analysis: self.enable_analysis,
            enable_translation: self.enable_translation,
            enable_postprocessing: self.enable_postprocessing,
        })
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Custom pipeline with configurable stages
pub struct CustomPipeline {
    /// Pipeline configuration
    config: TranspilationConfig,
    /// Stage enablement flags
    enable_preprocessing: bool,
    enable_analysis: bool,
    enable_translation: bool,
    enable_postprocessing: bool,
}

impl CustomPipeline {
    /// Execute the custom pipeline
    pub fn execute(&mut self, wasm_bytes: &[u8]) -> TranspilationResult<crate::transpiler::types::TranspiledModule> {
        use super::{PipelineStage, analyzer::Analyzer, postprocessor::Postprocessor, preprocessor::Preprocessor, translator::Translator};

        // Stage 1: Preprocessing (required)
        let mut preprocessor = Preprocessor::new(&self.config)?;
        let preprocessed = preprocessor.execute(wasm_bytes.to_vec(), &self.config)?;

        // Stage 2: Analysis (optional)
        let analyzed = if self.enable_analysis {
            let mut analyzer = Analyzer::new(&self.config)?;
            analyzer.execute(preprocessed, &self.config)?
        } else {
            // Create minimal analysis result
            super::analyzer::AnalysisResult {
                module: preprocessed,
                required_architecture: self.config.target_architecture,
                architecture_info: super::analyzer::ArchitectureInfo {
                    minimum_architecture: self.config.target_architecture,
                    recommended_architecture: self.config.target_architecture,
                    required_features: Vec::new(),
                    optional_features: Vec::new(),
                    warnings: Vec::new(),
                },
                function_analyses: Vec::new(),
                optimization_hints: Vec::new(),
                performance_profile: super::analyzer::PerformanceProfile::default(),
            }
        };

        // Stage 3: Translation (required)
        let mut translator = Translator::new(&self.config)?;
        let translated = translator.execute(analyzed, &self.config)?;

        // Stage 4: Postprocessing (optional)
        let result = if self.enable_postprocessing {
            let mut postprocessor = Postprocessor::new(&self.config)?;
            postprocessor.execute(translated, &self.config)?
        } else {
            translated
        };

        Ok(result)
    }

    /// Get the pipeline configuration
    pub fn config(&self) -> &TranspilationConfig {
        &self.config
    }

    /// Check if a stage is enabled
    pub fn is_stage_enabled(&self, stage: &str) -> bool {
        match stage {
            "preprocessing" => self.enable_preprocessing,
            "analysis" => self.enable_analysis,
            "translation" => self.enable_translation,
            "postprocessing" => self.enable_postprocessing,
            _ => false,
        }
    }
}

/// Predefined pipeline configurations
pub struct PipelinePresets;

impl PipelinePresets {
    /// Create a minimal pipeline for basic transpilation
    pub fn minimal() -> TranspilationResult<TranspilationPipeline> {
        PipelineBuilder::new().minimal().build()
    }

    /// Create a fast pipeline for quick transpilation
    pub fn fast() -> TranspilationResult<TranspilationPipeline> {
        PipelineBuilder::new().fast().build()
    }

    /// Create a debug pipeline with full analysis
    pub fn debug() -> TranspilationResult<TranspilationPipeline> {
        PipelineBuilder::new().debug().build()
    }

    /// Create a release pipeline with full optimizations
    pub fn release() -> TranspilationResult<TranspilationPipeline> {
        PipelineBuilder::new().release().build()
    }

    /// Create a pipeline for a specific architecture
    pub fn for_architecture(arch: dotvm_core::bytecode::VmArchitecture) -> TranspilationResult<TranspilationPipeline> {
        let config = TranspilationConfig::for_architecture(arch);
        PipelineBuilder::with_config(config).build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotvm_core::bytecode::VmArchitecture;

    #[test]
    fn test_pipeline_builder() {
        let builder = PipelineBuilder::new();
        let pipeline = builder.build();
        assert!(pipeline.is_ok());
    }

    #[test]
    fn test_minimal_pipeline() {
        let pipeline = PipelineBuilder::new().minimal().build();
        assert!(pipeline.is_ok());
    }

    #[test]
    fn test_custom_pipeline() {
        let custom = PipelineBuilder::new().analysis(false).postprocessing(false).build_custom();
        assert!(custom.is_ok());

        let custom = custom.unwrap();
        assert!(!custom.is_stage_enabled("analysis"));
        assert!(!custom.is_stage_enabled("postprocessing"));
        assert!(custom.is_stage_enabled("preprocessing"));
        assert!(custom.is_stage_enabled("translation"));
    }

    #[test]
    fn test_pipeline_presets() {
        assert!(PipelinePresets::minimal().is_ok());
        assert!(PipelinePresets::fast().is_ok());
        assert!(PipelinePresets::debug().is_ok());
        assert!(PipelinePresets::release().is_ok());
        assert!(PipelinePresets::for_architecture(VmArchitecture::Arch128).is_ok());
    }

    #[test]
    fn test_invalid_pipeline() {
        // Should fail if trying to disable required stages
        let result = PipelineBuilder::new().preprocessing(false).build();
        assert!(result.is_err());

        let result = PipelineBuilder::new().translation(false).build();
        assert!(result.is_err());
    }
}
