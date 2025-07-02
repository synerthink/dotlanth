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

//! Pipeline architecture for transpilation
//!
//! This module provides a flexible pipeline system for processing WASM modules
//! through various stages of transpilation.

pub mod analyzer;
pub mod pipeline_builder;
pub mod postprocessor;
pub mod preprocessor;
pub mod translator;

use super::{
    config::TranspilationConfig,
    error::{TranspilationError, TranspilationResult},
    types::TranspiledModule,
};
use crate::wasm::ast::WasmModule;

/// Trait for pipeline stages
pub trait PipelineStage {
    /// The input type for this stage
    type Input;
    /// The output type for this stage
    type Output;

    /// Execute this pipeline stage
    fn execute(&mut self, input: Self::Input, config: &TranspilationConfig) -> TranspilationResult<Self::Output>;

    /// Get the name of this stage for debugging
    fn name(&self) -> &'static str;

    /// Check if this stage can be skipped based on configuration
    fn can_skip(&self, config: &TranspilationConfig) -> bool {
        false
    }

    /// Get estimated execution time for this stage
    fn estimated_duration(&self, input_size: usize) -> std::time::Duration {
        std::time::Duration::from_millis(input_size as u64 / 1000) // Default heuristic
    }
}

/// Pipeline context for sharing data between stages
#[derive(Debug, Clone, Default)]
pub struct PipelineContext {
    /// Shared metadata between stages
    pub metadata: std::collections::HashMap<String, String>,
    /// Performance metrics
    pub metrics: PipelineMetrics,
    /// Warnings collected during processing
    pub warnings: Vec<String>,
}

impl PipelineContext {
    /// Create a new pipeline context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Record stage execution time
    pub fn record_stage_time(&mut self, stage: &str, duration: std::time::Duration) {
        self.metrics.stage_times.insert(stage.to_string(), duration);
    }

    /// Get total execution time
    pub fn total_time(&self) -> std::time::Duration {
        self.metrics.stage_times.values().sum()
    }
}

/// Performance metrics for the pipeline
#[derive(Debug, Clone, Default)]
pub struct PipelineMetrics {
    /// Execution time for each stage
    pub stage_times: std::collections::HashMap<String, std::time::Duration>,
    /// Memory usage peaks
    pub memory_peaks: std::collections::HashMap<String, usize>,
    /// Number of processed items per stage
    pub processed_items: std::collections::HashMap<String, usize>,
}

impl PipelineMetrics {
    /// Record memory usage for a stage
    pub fn record_memory_usage(&mut self, stage: &str, bytes: usize) {
        self.memory_peaks.insert(stage.to_string(), bytes);
    }

    /// Record number of processed items
    pub fn record_processed_items(&mut self, stage: &str, count: usize) {
        self.processed_items.insert(stage.to_string(), count);
    }

    /// Get processing rate for a stage (items per second)
    pub fn processing_rate(&self, stage: &str) -> Option<f64> {
        let items = self.processed_items.get(stage)?;
        let time = self.stage_times.get(stage)?;

        if time.as_secs_f64() > 0.0 { Some(*items as f64 / time.as_secs_f64()) } else { None }
    }
}

/// Main transpilation pipeline
pub struct TranspilationPipeline {
    /// Pipeline configuration
    config: TranspilationConfig,
    /// Pipeline context
    context: PipelineContext,
    /// Preprocessor stage
    preprocessor: preprocessor::Preprocessor,
    /// Analyzer stage
    analyzer: analyzer::Analyzer,
    /// Translator stage
    translator: translator::Translator,
    /// Postprocessor stage
    postprocessor: postprocessor::Postprocessor,
}

impl TranspilationPipeline {
    /// Create a new pipeline with the given configuration
    pub fn new(config: TranspilationConfig) -> TranspilationResult<Self> {
        config.validate()?;

        Ok(Self {
            preprocessor: preprocessor::Preprocessor::new(&config)?,
            analyzer: analyzer::Analyzer::new(&config)?,
            translator: translator::Translator::new(&config)?,
            postprocessor: postprocessor::Postprocessor::new(&config)?,
            context: PipelineContext::new(),
            config,
        })
    }

    /// Execute the complete pipeline
    pub fn execute(&mut self, wasm_bytes: &[u8]) -> TranspilationResult<TranspiledModule> {
        let start_time = std::time::Instant::now();

        // Stage 1: Preprocessing
        let stage_start = std::time::Instant::now();
        let preprocessed = self
            .preprocessor
            .execute(wasm_bytes.to_vec(), &self.config)
            .map_err(|e| TranspilationError::translation_error("preprocessing", format!("Preprocessing failed: {}", e)))?;
        self.context.record_stage_time("preprocessing", stage_start.elapsed());

        // Stage 2: Analysis
        let stage_start = std::time::Instant::now();
        let analyzed = self
            .analyzer
            .execute(preprocessed, &self.config)
            .map_err(|e| TranspilationError::translation_error("analysis", format!("Analysis failed: {}", e)))?;
        self.context.record_stage_time("analysis", stage_start.elapsed());

        // Stage 3: Translation
        let stage_start = std::time::Instant::now();
        let translated = self
            .translator
            .execute(analyzed, &self.config)
            .map_err(|e| TranspilationError::translation_error("translation", format!("Translation failed: {}", e)))?;
        self.context.record_stage_time("translation", stage_start.elapsed());

        // Stage 4: Postprocessing
        let stage_start = std::time::Instant::now();
        let result = self
            .postprocessor
            .execute(translated, &self.config)
            .map_err(|e| TranspilationError::translation_error("postprocessing", format!("Postprocessing failed: {}", e)))?;
        self.context.record_stage_time("postprocessing", stage_start.elapsed());

        // Record total time
        self.context.metrics.stage_times.insert("total".to_string(), start_time.elapsed());

        Ok(result)
    }

    /// Execute a single pipeline stage with timing and error handling
    fn execute_stage<S: PipelineStage>(&mut self, stage: &mut S, input: S::Input, stage_name: &str) -> TranspilationResult<S::Output> {
        // Check if stage can be skipped
        if stage.can_skip(&self.config) {
            self.context.add_warning(format!("Skipping stage: {}", stage_name));
            // This is a simplified approach - in practice, we'd need a way to pass through input
            // For now, we'll execute the stage anyway
        }

        let start_time = std::time::Instant::now();

        // Execute the stage
        let result = stage
            .execute(input, &self.config)
            .map_err(|e| TranspilationError::translation_error(stage_name, format!("Stage '{}' failed: {}", stage.name(), e)))?;

        // Record timing
        let duration = start_time.elapsed();
        self.context.record_stage_time(stage_name, duration);

        Ok(result)
    }

    /// Get the pipeline context (for metrics and warnings)
    pub fn context(&self) -> &PipelineContext {
        &self.context
    }

    /// Get mutable access to the pipeline context
    pub fn context_mut(&mut self) -> &mut PipelineContext {
        &mut self.context
    }

    /// Get the pipeline configuration
    pub fn config(&self) -> &TranspilationConfig {
        &self.config
    }

    /// Update the pipeline configuration
    pub fn update_config(&mut self, config: TranspilationConfig) -> TranspilationResult<()> {
        config.validate()?;
        self.config = config;

        // Recreate stages with new configuration
        self.preprocessor = preprocessor::Preprocessor::new(&self.config)?;
        self.analyzer = analyzer::Analyzer::new(&self.config)?;
        self.translator = translator::Translator::new(&self.config)?;
        self.postprocessor = postprocessor::Postprocessor::new(&self.config)?;

        Ok(())
    }

    /// Reset the pipeline context (clears metrics and warnings)
    pub fn reset_context(&mut self) {
        self.context = PipelineContext::new();
    }

    /// Get performance report
    pub fn performance_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Transpilation Pipeline Performance Report ===\n\n");

        // Stage timings
        report.push_str("Stage Execution Times:\n");
        for (stage, duration) in &self.context.metrics.stage_times {
            report.push_str(&format!("  {}: {:.2}ms\n", stage, duration.as_millis()));
        }

        // Processing rates
        report.push_str("\nProcessing Rates:\n");
        for stage in ["preprocessing", "analysis", "translation", "postprocessing"] {
            if let Some(rate) = self.context.metrics.processing_rate(stage) {
                report.push_str(&format!("  {}: {:.2} items/sec\n", stage, rate));
            }
        }

        // Memory usage
        if !self.context.metrics.memory_peaks.is_empty() {
            report.push_str("\nPeak Memory Usage:\n");
            for (stage, bytes) in &self.context.metrics.memory_peaks {
                report.push_str(&format!("  {}: {:.2} MB\n", stage, *bytes as f64 / 1024.0 / 1024.0));
            }
        }

        // Warnings
        if !self.context.warnings.is_empty() {
            report.push_str("\nWarnings:\n");
            for warning in &self.context.warnings {
                report.push_str(&format!("  - {}\n", warning));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::config::TranspilationConfig;

    #[test]
    fn test_pipeline_context() {
        let mut context = PipelineContext::new();
        context.add_metadata("test".to_string(), "value".to_string());
        context.add_warning("test warning".to_string());

        assert_eq!(context.get_metadata("test"), Some(&"value".to_string()));
        assert_eq!(context.warnings.len(), 1);
    }

    #[test]
    fn test_pipeline_metrics() {
        let mut metrics = PipelineMetrics::default();
        metrics.record_processed_items("test", 100);
        metrics.stage_times.insert("test".to_string(), std::time::Duration::from_secs(1));

        assert_eq!(metrics.processing_rate("test"), Some(100.0));
    }

    #[test]
    fn test_pipeline_creation() {
        let config = TranspilationConfig::default();
        let pipeline = TranspilationPipeline::new(config);
        assert!(pipeline.is_ok());
    }
}
