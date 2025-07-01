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

//! Core dependency analysis engine

use super::{
    analyzers::{
        AnalysisError, AnalysisResult, AnalysisStats, Analyzer,
        control_flow::{ControlFlowAnalyzer, ControlFlowGraph},
        data_flow::{DataFlowAnalysis, DataFlowAnalyzer},
        state_access::{StateAccessAnalysis, StateAccessAnalyzer},
    },
    config::EngineConfig,
    detection::{DependencyDetector, DependencyInfo, DetectorRegistry},
};
use std::collections::HashMap;
use std::time::Instant;

/// Complete analysis result from the dependency analysis engine
#[derive(Debug, Clone)]
pub struct DependencyAnalysisResult {
    /// State access analysis results
    pub state_access: Option<StateAccessAnalysis>,
    /// Data flow analysis results
    pub data_flow: Option<DataFlowAnalysis>,
    /// Control flow analysis results
    pub control_flow: Option<ControlFlowGraph>,
    /// Detected dependencies
    pub dependencies: Vec<DependencyInfo>,
    /// Analysis statistics
    pub statistics: AnalysisStats,
    /// Analysis metadata
    pub metadata: HashMap<String, String>,
}

/// Main dependency analysis engine
pub struct DependencyAnalysisEngine {
    /// Engine configuration
    config: EngineConfig,
    /// State access analyzer
    state_analyzer: StateAccessAnalyzer,
    /// Data flow analyzer
    data_flow_analyzer: DataFlowAnalyzer,
    /// Control flow analyzer
    control_flow_analyzer: ControlFlowAnalyzer,
    /// Dependency detector registry
    detector_registry: DetectorRegistry,
    /// Cache for analysis results
    result_cache: HashMap<String, DependencyAnalysisResult>,
    /// Analysis statistics
    stats: AnalysisStats,
}

impl DependencyAnalysisEngine {
    /// Create a new dependency analysis engine with the given configuration
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config: config.clone(),
            state_analyzer: StateAccessAnalyzer::new(),
            data_flow_analyzer: DataFlowAnalyzer::new().with_unused_detection(true).with_uninitialized_detection(true),
            control_flow_analyzer: ControlFlowAnalyzer::new().with_unreachable_detection(true).with_loop_analysis(true).with_complexity_calculation(true),
            detector_registry: DetectorRegistry::default(),
            result_cache: HashMap::new(),
            stats: AnalysisStats::new(),
        }
    }

    /// Create a new engine with default configuration
    pub fn with_default_config() -> Self {
        Self::new(EngineConfig::default())
    }

    /// Get the current configuration
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Update the engine configuration
    pub fn update_config(&mut self, config: EngineConfig) {
        self.config = config;
    }

    /// Get analysis statistics
    pub fn statistics(&self) -> &AnalysisStats {
        &self.stats
    }

    /// Clear the result cache
    pub fn clear_cache(&mut self) {
        self.result_cache.clear();
    }

    /// Perform complete dependency analysis on the given input
    pub fn analyze(&mut self, input: &str) -> AnalysisResult<DependencyAnalysisResult> {
        let start_time = Instant::now();

        if input.trim().is_empty() {
            return Err(AnalysisError::EmptyInput);
        }

        // Check cache if enabled
        if self.config.enable_caching {
            let cache_key = self.generate_cache_key(input);
            if let Some(cached_result) = self.result_cache.get(&cache_key) {
                if self.config.is_verbose() {
                    println!("Using cached analysis result");
                }
                return Ok(cached_result.clone());
            }
        }

        if self.config.is_verbose() {
            println!("Starting dependency analysis...");
        }

        let mut result = DependencyAnalysisResult {
            state_access: None,
            data_flow: None,
            control_flow: None,
            dependencies: Vec::new(),
            statistics: AnalysisStats::new(),
            metadata: HashMap::new(),
        };

        // Perform state access analysis
        match self.analyze_state_access(input) {
            Ok(state_analysis) => {
                if self.config.is_debug() {
                    println!("State access analysis completed: {} accesses found", state_analysis.accesses.len());
                }
                result.state_access = Some(state_analysis);
            }
            Err(e) => {
                if self.config.is_verbose() {
                    println!("State access analysis failed: {}", e);
                }
            }
        }

        // Perform data flow analysis
        match self.analyze_data_flow(input) {
            Ok(data_analysis) => {
                if self.config.is_debug() {
                    println!("Data flow analysis completed: {} variables found", data_analysis.variables.len());
                }
                result.data_flow = Some(data_analysis);
            }
            Err(e) => {
                if self.config.is_verbose() {
                    println!("Data flow analysis failed: {}", e);
                }
            }
        }

        // Perform control flow analysis
        match self.generate_control_flow_graph(input) {
            Ok(cfg) => {
                if self.config.is_debug() {
                    println!("Control flow analysis completed: {} nodes, {} edges", cfg.nodes.len(), cfg.edges.len());
                }
                result.control_flow = Some(cfg);
            }
            Err(e) => {
                if self.config.is_verbose() {
                    println!("Control flow analysis failed: {}", e);
                }
            }
        }

        // Detect dependencies
        result.dependencies = self.detect_dependencies(input);
        if self.config.is_debug() {
            println!("Dependency detection completed: {} dependencies found", result.dependencies.len());
        }

        // Update statistics
        let duration = start_time.elapsed();
        result.statistics.set_duration(duration.as_millis() as u64);
        result.statistics.increment_nodes();
        result.statistics.add_dependencies(result.dependencies.len());

        // Add metadata
        result.metadata.insert("analysis_time".to_string(), format!("{}ms", duration.as_millis()));
        result.metadata.insert("input_size".to_string(), input.len().to_string());
        result.metadata.insert("analyzer_version".to_string(), "1.0.0".to_string());

        // Update engine statistics
        self.stats.increment_nodes();
        self.stats.add_dependencies(result.dependencies.len());
        self.stats.set_duration(duration.as_millis() as u64);

        // Cache result if enabled
        if self.config.enable_caching {
            let cache_key = self.generate_cache_key(input);
            self.result_cache.insert(cache_key, result.clone());
        }

        if self.config.is_verbose() {
            println!("Dependency analysis completed in {}ms", duration.as_millis());
        }

        Ok(result)
    }

    /// Analyze state access patterns
    pub fn analyze_state_access(&self, input: &str) -> AnalysisResult<StateAccessAnalysis> {
        if self.config.is_debug() {
            println!("Running state access analysis...");
        }
        self.state_analyzer.analyze(input)
    }

    /// Analyze data flow patterns
    pub fn analyze_data_flow(&self, input: &str) -> AnalysisResult<DataFlowAnalysis> {
        if self.config.is_debug() {
            println!("Running data flow analysis...");
        }
        self.data_flow_analyzer.analyze(input)
    }

    /// Generate control flow graph
    pub fn generate_control_flow_graph(&self, input: &str) -> AnalysisResult<ControlFlowGraph> {
        if self.config.is_debug() {
            println!("Generating control flow graph...");
        }
        self.control_flow_analyzer.analyze(input)
    }

    /// Detect dependencies using all registered detectors
    pub fn detect_dependencies(&self, input: &str) -> Vec<DependencyInfo> {
        if self.config.is_debug() {
            println!("Running dependency detection...");
        }

        let all_results = self.detector_registry.detect_all(input);
        let mut all_dependencies = Vec::new();

        for (detector_name, dependencies) in all_results {
            if self.config.is_debug() {
                println!("Detector '{}' found {} dependencies", detector_name, dependencies.len());
            }
            all_dependencies.extend(dependencies);
        }

        // Remove duplicates based on name and type
        all_dependencies.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| format!("{:?}", a.dependency_type).cmp(&format!("{:?}", b.dependency_type))));
        all_dependencies.dedup_by(|a, b| a.name == b.name && a.dependency_type == b.dependency_type);

        all_dependencies
    }

    /// Get the detector registry for adding custom detectors
    pub fn detector_registry_mut(&mut self) -> &mut DetectorRegistry {
        &mut self.detector_registry
    }

    /// Get the detector registry for read-only access
    pub fn detector_registry(&self) -> &DetectorRegistry {
        &self.detector_registry
    }

    /// Generate a cache key for the input
    fn generate_cache_key(&self, input: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        self.config.verbosity.hash(&mut hasher);
        self.config.max_depth.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Reset all internal state and statistics
    pub fn reset(&mut self) {
        self.result_cache.clear();
        self.stats = AnalysisStats::new();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.result_cache.len(), self.result_cache.capacity())
    }
}

// Maintain backward compatibility with the old API
impl DependencyAnalysisEngine {
    /// Legacy method for backward compatibility
    pub fn last_dependency_list(&self) -> Option<Vec<String>> {
        // This could be implemented by looking at the last analysis result
        // For now, return None to indicate this feature is deprecated
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let config = EngineConfig::new().with_verbosity(1);
        let engine = DependencyAnalysisEngine::new(config);
        assert_eq!(engine.config().verbosity, 1);
    }

    #[test]
    fn test_engine_with_default_config() {
        let engine = DependencyAnalysisEngine::with_default_config();
        assert_eq!(engine.config().verbosity, 1);
    }

    #[test]
    fn test_analyze_empty_input() {
        let mut engine = DependencyAnalysisEngine::with_default_config();
        let result = engine.analyze("");
        assert!(matches!(result, Err(AnalysisError::EmptyInput)));
    }

    #[test]
    fn test_analyze_simple_input() {
        let mut engine = DependencyAnalysisEngine::with_default_config();
        let input = r#"
            dep:module1
            let x = get_state("counter");
            if (x > 0) {
                set_state("counter", x + 1);
            }
        "#;

        let result = engine.analyze(input).unwrap();
        assert!(!result.dependencies.is_empty());
        assert!(result.state_access.is_some());
        assert!(result.data_flow.is_some());
        assert!(result.control_flow.is_some());
    }

    #[test]
    fn test_individual_analyzers() {
        let engine = DependencyAnalysisEngine::with_default_config();

        let input = "let x = y + 1;";

        // Test state access analysis
        let state_result = engine.analyze_state_access(input);
        assert!(state_result.is_ok());

        // Test data flow analysis
        let data_result = engine.analyze_data_flow(input);
        assert!(data_result.is_ok());

        // Test control flow analysis
        let cfg_result = engine.generate_control_flow_graph(input);
        assert!(cfg_result.is_ok());
    }

    #[test]
    fn test_dependency_detection() {
        let engine = DependencyAnalysisEngine::with_default_config();
        let input = r#"
            dep:module1
            import math
            require("fs")
        "#;

        let dependencies = engine.detect_dependencies(input);
        assert!(!dependencies.is_empty());

        let names: Vec<_> = dependencies.iter().map(|d| &d.name).collect();
        assert!(names.iter().any(|name| name.contains("module1")));
    }

    #[test]
    fn test_caching() {
        let mut engine = DependencyAnalysisEngine::new(EngineConfig::new().with_caching(true));

        let input = "dep:test_module";

        // First analysis
        let result1 = engine.analyze(input).unwrap();
        assert_eq!(engine.cache_stats().0, 1); // One item in cache

        // Second analysis (should use cache)
        let result2 = engine.analyze(input).unwrap();
        assert_eq!(result1.dependencies.len(), result2.dependencies.len());
    }

    #[test]
    fn test_statistics() {
        let mut engine = DependencyAnalysisEngine::with_default_config();
        let input = "dep:module1\ndep:module2";

        let _result = engine.analyze(input).unwrap();
        let stats = engine.statistics();

        assert!(stats.nodes_analyzed > 0);
        assert!(stats.dependencies_found > 0);
        assert!(stats.duration_ms >= 0); // Duration might be 0 for very fast operations
    }

    #[test]
    fn test_config_update() {
        let mut engine = DependencyAnalysisEngine::with_default_config();
        assert_eq!(engine.config().verbosity, 1);

        let new_config = EngineConfig::new().with_verbosity(2);
        engine.update_config(new_config);
        assert_eq!(engine.config().verbosity, 2);
    }

    #[test]
    fn test_reset() {
        let mut engine = DependencyAnalysisEngine::new(EngineConfig::new().with_caching(true));

        let _result = engine.analyze("dep:test").unwrap();
        assert!(engine.cache_stats().0 > 0);
        assert!(engine.statistics().nodes_analyzed > 0);

        engine.reset();
        assert_eq!(engine.cache_stats().0, 0);
        assert_eq!(engine.statistics().nodes_analyzed, 0);
    }
}
