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

//! New simplified transpilation engine using the pipeline architecture
//!
//! This is the new implementation that replaces the monolithic engine.rs
//! with a clean, modular pipeline-based approach.

use super::{
    config::TranspilationConfig,
    error::{TranspilationError, TranspilationResult},
    pipeline::{TranspilationPipeline, pipeline_builder::PipelineBuilder},
    types::TranspiledModule,
};
use dotvm_core::bytecode::VmArchitecture;

/// New transpilation engine using pipeline architecture
pub struct NewTranspilationEngine {
    /// The transpilation pipeline
    pipeline: TranspilationPipeline,
    /// Engine configuration
    config: TranspilationConfig,
}

impl NewTranspilationEngine {
    /// Create a new transpilation engine with the given configuration
    pub fn new(config: TranspilationConfig) -> TranspilationResult<Self> {
        let pipeline = PipelineBuilder::with_config(config.clone()).build()?;

        Ok(Self { pipeline, config })
    }

    /// Create a new transpilation engine with default configuration for the given architecture
    pub fn with_architecture(target_arch: VmArchitecture) -> TranspilationResult<Self> {
        let config = TranspilationConfig::for_architecture(target_arch);
        Self::new(config)
    }

    /// Create a debug engine (no optimizations, preserve debug info)
    pub fn debug() -> TranspilationResult<Self> {
        let pipeline = PipelineBuilder::new().debug().build()?;
        let config = TranspilationConfig::debug();

        Ok(Self { pipeline, config })
    }

    /// Create a release engine (full optimizations)
    pub fn release() -> TranspilationResult<Self> {
        let pipeline = PipelineBuilder::new().release().build()?;
        let config = TranspilationConfig::release();

        Ok(Self { pipeline, config })
    }

    /// Create a fast engine (minimal processing)
    pub fn fast() -> TranspilationResult<Self> {
        let pipeline = PipelineBuilder::new().fast().build()?;
        let mut config = TranspilationConfig::default();
        config.enable_optimizations = false;

        Ok(Self { pipeline, config })
    }

    /// Transpile a WASM binary to DotVM bytecode
    pub fn transpile(&mut self, wasm_bytes: &[u8]) -> TranspilationResult<TranspiledModule> {
        self.pipeline.execute(wasm_bytes)
    }

    /// Get the engine configuration
    pub fn config(&self) -> &TranspilationConfig {
        &self.config
    }

    /// Update the engine configuration
    pub fn update_config(&mut self, config: TranspilationConfig) -> TranspilationResult<()> {
        self.config = config.clone();
        self.pipeline.update_config(config)?;
        Ok(())
    }

    /// Get performance metrics from the last transpilation
    pub fn performance_report(&self) -> String {
        self.pipeline.performance_report()
    }

    /// Get warnings from the last transpilation
    pub fn warnings(&self) -> &[String] {
        &self.pipeline.context().warnings
    }

    /// Reset performance metrics and warnings
    pub fn reset_metrics(&mut self) {
        self.pipeline.reset_context();
    }

    /// Check if the engine supports a specific feature
    pub fn supports_feature(&self, feature: &str) -> bool {
        self.config.is_feature_enabled(feature)
    }

    /// Get estimated transpilation time for a given input size
    pub fn estimate_transpilation_time(&self, input_size: usize) -> std::time::Duration {
        // Rough estimate based on pipeline stages
        let preprocessing_time = std::time::Duration::from_millis((input_size / 1024).max(1) as u64);
        let analysis_time = std::time::Duration::from_millis((input_size * 5 / 1024).max(1) as u64);
        let translation_time = std::time::Duration::from_millis((input_size * 10 / 1024).max(5) as u64);
        let postprocessing_time = std::time::Duration::from_millis((input_size / 1024).max(1) as u64);

        preprocessing_time + analysis_time + translation_time + postprocessing_time
    }
}

/// Convenience functions for common use cases
impl NewTranspilationEngine {
    /// Quick transpilation with default settings
    pub fn quick_transpile(wasm_bytes: &[u8]) -> TranspilationResult<TranspiledModule> {
        let mut engine = Self::fast()?;
        engine.transpile(wasm_bytes)
    }

    /// Transpile with full optimizations
    pub fn optimized_transpile(wasm_bytes: &[u8]) -> TranspilationResult<TranspiledModule> {
        let mut engine = Self::release()?;
        engine.transpile(wasm_bytes)
    }

    /// Transpile for debugging (preserves debug info)
    pub fn debug_transpile(wasm_bytes: &[u8]) -> TranspilationResult<TranspiledModule> {
        let mut engine = Self::debug()?;
        engine.transpile(wasm_bytes)
    }

    /// Transpile for a specific architecture
    pub fn transpile_for_architecture(wasm_bytes: &[u8], arch: VmArchitecture) -> TranspilationResult<TranspiledModule> {
        let mut engine = Self::with_architecture(arch)?;
        engine.transpile(wasm_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_engine_creation() {
        let config = TranspilationConfig::default();
        let engine = NewTranspilationEngine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_engine_presets() {
        assert!(NewTranspilationEngine::debug().is_ok());
        assert!(NewTranspilationEngine::release().is_ok());
        assert!(NewTranspilationEngine::fast().is_ok());
        assert!(NewTranspilationEngine::with_architecture(VmArchitecture::Arch128).is_ok());
    }

    #[test]
    fn test_feature_support() {
        let engine = NewTranspilationEngine::debug().unwrap();
        // Test with default feature flags
        assert!(engine.supports_feature("simd"));
        assert!(engine.supports_feature("bulk_memory"));
    }

    #[test]
    fn test_time_estimation() {
        let engine = NewTranspilationEngine::fast().unwrap();
        let estimate = engine.estimate_transpilation_time(1024); // 1KB
        assert!(estimate.as_millis() > 0);
    }
}
