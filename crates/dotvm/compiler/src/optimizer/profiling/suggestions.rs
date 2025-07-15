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

//! Optimization suggestions based on profiling data

use std::collections::HashMap;
use std::time::Duration;

/// An optimization suggestion
#[derive(Debug, Clone)]
pub struct OptimizationSuggestion {
    /// Priority level (1 = highest, 5 = lowest)
    pub priority: u8,
    /// Component or area this suggestion applies to
    pub component: String,
    /// Description of the suggestion
    pub description: String,
    /// Expected impact (percentage improvement)
    pub expected_impact: f32,
    /// Difficulty of implementation (1 = easy, 5 = very hard)
    pub difficulty: u8,
    /// Estimated time to implement
    pub estimated_time: Duration,
}

/// Optimization suggestion generator
pub struct OptimizationSuggester {
    /// Performance data for analysis
    performance_data: HashMap<String, PerformanceMetrics>,
    /// Suggestion rules
    rules: Vec<SuggestionRule>,
}

/// Performance metrics for a component
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub execution_time: Duration,
    pub memory_usage: usize,
    pub cpu_usage: f32,
    pub cache_hit_rate: f32,
    pub error_rate: f32,
}

/// A rule for generating suggestions
struct SuggestionRule {
    name: String,
    condition: Box<dyn Fn(&PerformanceMetrics) -> bool>,
    suggestion_generator: Box<dyn Fn(&PerformanceMetrics) -> OptimizationSuggestion>,
}

impl OptimizationSuggester {
    /// Create a new optimization suggester
    pub fn new() -> Self {
        let mut suggester = Self {
            performance_data: HashMap::new(),
            rules: Vec::new(),
        };

        suggester.add_default_rules();
        suggester
    }

    /// Add performance data for a component
    pub fn add_performance_data(&mut self, component: String, metrics: PerformanceMetrics) {
        self.performance_data.insert(component, metrics);
    }

    /// Generate optimization suggestions
    pub fn generate_suggestions(&self) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        for (component, metrics) in &self.performance_data {
            for rule in &self.rules {
                if (rule.condition)(metrics) {
                    let mut suggestion = (rule.suggestion_generator)(metrics);
                    suggestion.component = component.clone();
                    suggestions.push(suggestion);
                }
            }
        }

        // Sort by priority and expected impact
        suggestions.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| b.expected_impact.partial_cmp(&a.expected_impact).unwrap()));

        suggestions
    }

    /// Add default suggestion rules
    fn add_default_rules(&mut self) {
        // High execution time rule
        self.rules.push(SuggestionRule {
            name: "high_execution_time".to_string(),
            condition: Box::new(|metrics| metrics.execution_time.as_millis() > 100),
            suggestion_generator: Box::new(|metrics| OptimizationSuggestion {
                priority: 1,
                component: String::new(), // Will be filled by caller
                description: "Consider optimizing algorithms or adding parallelization".to_string(),
                expected_impact: 20.0 + (metrics.execution_time.as_millis() as f32 / 10.0).min(50.0),
                difficulty: 3,
                estimated_time: <Duration as DurationExt>::from_hours(8),
            }),
        });

        // High memory usage rule
        self.rules.push(SuggestionRule {
            name: "high_memory_usage".to_string(),
            condition: Box::new(|metrics| metrics.memory_usage > 1024 * 1024), // > 1MB
            suggestion_generator: Box::new(|metrics| OptimizationSuggestion {
                priority: 2,
                component: String::new(),
                description: "Reduce memory allocations and implement object pooling".to_string(),
                expected_impact: 15.0 + (metrics.memory_usage as f32 / 1024.0 / 1024.0 * 5.0).min(30.0),
                difficulty: 2,
                estimated_time: <Duration as DurationExt>::from_hours(4),
            }),
        });

        // Low cache hit rate rule
        self.rules.push(SuggestionRule {
            name: "low_cache_hit_rate".to_string(),
            condition: Box::new(|metrics| metrics.cache_hit_rate < 0.8),
            suggestion_generator: Box::new(|metrics| OptimizationSuggestion {
                priority: 2,
                component: String::new(),
                description: "Improve caching strategy and data locality".to_string(),
                expected_impact: (0.9 - metrics.cache_hit_rate) * 40.0,
                difficulty: 3,
                estimated_time: <Duration as DurationExt>::from_hours(6),
            }),
        });

        // High CPU usage rule
        self.rules.push(SuggestionRule {
            name: "high_cpu_usage".to_string(),
            condition: Box::new(|metrics| metrics.cpu_usage > 0.8),
            suggestion_generator: Box::new(|metrics| OptimizationSuggestion {
                priority: 1,
                component: String::new(),
                description: "Optimize CPU-intensive operations and consider parallelization".to_string(),
                expected_impact: (metrics.cpu_usage - 0.5) * 30.0,
                difficulty: 4,
                estimated_time: <Duration as DurationExt>::from_hours(12),
            }),
        });

        // High error rate rule
        self.rules.push(SuggestionRule {
            name: "high_error_rate".to_string(),
            condition: Box::new(|metrics| metrics.error_rate > 0.01), // > 1%
            suggestion_generator: Box::new(|metrics| OptimizationSuggestion {
                priority: 1,
                component: String::new(),
                description: "Improve error handling and add input validation".to_string(),
                expected_impact: metrics.error_rate * 100.0 * 2.0, // 2x error rate as impact
                difficulty: 2,
                estimated_time: <Duration as DurationExt>::from_hours(3),
            }),
        });
    }

    /// Add a custom suggestion rule
    pub fn add_rule<F, G>(&mut self, name: String, condition: F, suggestion_generator: G)
    where
        F: Fn(&PerformanceMetrics) -> bool + 'static,
        G: Fn(&PerformanceMetrics) -> OptimizationSuggestion + 'static,
    {
        self.rules.push(SuggestionRule {
            name,
            condition: Box::new(condition),
            suggestion_generator: Box::new(suggestion_generator),
        });
    }

    /// Get suggestions for a specific component
    pub fn get_suggestions_for_component(&self, component: &str) -> Vec<OptimizationSuggestion> {
        if let Some(metrics) = self.performance_data.get(component) {
            let mut suggestions = Vec::new();

            for rule in &self.rules {
                if (rule.condition)(metrics) {
                    let mut suggestion = (rule.suggestion_generator)(metrics);
                    suggestion.component = component.to_string();
                    suggestions.push(suggestion);
                }
            }

            suggestions.sort_by_key(|s| s.priority);
            suggestions
        } else {
            Vec::new()
        }
    }

    /// Get top N suggestions across all components
    pub fn get_top_suggestions(&self, n: usize) -> Vec<OptimizationSuggestion> {
        let mut all_suggestions = self.generate_suggestions();
        all_suggestions.truncate(n);
        all_suggestions
    }
}

impl Default for OptimizationSuggester {
    fn default() -> Self {
        Self::new()
    }
}

// Helper trait for Duration
trait DurationExt {
    fn from_hours(hours: u64) -> Duration;
}

impl DurationExt for Duration {
    fn from_hours(hours: u64) -> Duration {
        Duration::from_secs(hours * 3600)
    }
}
