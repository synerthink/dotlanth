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

//! Hotspot detection for optimization

use std::collections::HashMap;
use std::time::Duration;

/// Types of performance hotspots
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HotspotType {
    /// Slow optimization pass
    SlowPass,
    /// Memory-intensive operation
    MemoryIntensive,
    /// CPU-intensive operation
    CpuIntensive,
    /// I/O bottleneck
    IoBottleneck,
}

/// A detected performance hotspot
#[derive(Debug, Clone)]
pub struct Hotspot {
    /// Type of hotspot
    pub hotspot_type: HotspotType,
    /// Location identifier (e.g., pass name, function name)
    pub location: String,
    /// Severity score (0.0 to 1.0)
    pub severity: f32,
    /// Time spent in this hotspot
    pub time_spent: Duration,
    /// Percentage of total execution time
    pub percentage: f32,
    /// Suggested improvements
    pub suggestions: Vec<String>,
}

/// Hotspot detector
pub struct HotspotDetector {
    /// Threshold for considering something a hotspot (percentage)
    threshold: f32,
    /// Collected timing data
    timing_data: HashMap<String, Duration>,
    /// Total execution time
    total_time: Duration,
}

impl HotspotDetector {
    /// Create a new hotspot detector
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            timing_data: HashMap::new(),
            total_time: Duration::default(),
        }
    }

    /// Record timing data for a component
    pub fn record_timing(&mut self, component: String, duration: Duration) {
        self.timing_data.insert(component, duration);
        self.total_time += duration;
    }

    /// Detect hotspots in the recorded data
    pub fn detect_hotspots(&self) -> Vec<Hotspot> {
        let mut hotspots = Vec::new();

        for (component, &duration) in &self.timing_data {
            let percentage = if self.total_time.as_nanos() > 0 {
                duration.as_nanos() as f32 / self.total_time.as_nanos() as f32 * 100.0
            } else {
                0.0
            };

            if percentage >= self.threshold {
                let hotspot_type = self.classify_hotspot(component);
                let severity = (percentage / 100.0).min(1.0);
                let suggestions = self.generate_suggestions(component, &hotspot_type);

                hotspots.push(Hotspot {
                    hotspot_type,
                    location: component.clone(),
                    severity,
                    time_spent: duration,
                    percentage,
                    suggestions,
                });
            }
        }

        // Sort by severity (highest first)
        hotspots.sort_by(|a, b| b.severity.partial_cmp(&a.severity).unwrap());
        hotspots
    }

    /// Classify the type of hotspot
    fn classify_hotspot(&self, component: &str) -> HotspotType {
        if component.contains("memory") || component.contains("allocation") {
            HotspotType::MemoryIntensive
        } else if component.contains("io") || component.contains("file") {
            HotspotType::IoBottleneck
        } else if component.contains("pass") || component.contains("optimization") {
            HotspotType::SlowPass
        } else {
            HotspotType::CpuIntensive
        }
    }

    /// Generate suggestions for improving a hotspot
    fn generate_suggestions(&self, component: &str, hotspot_type: &HotspotType) -> Vec<String> {
        let mut suggestions = Vec::new();

        match hotspot_type {
            HotspotType::SlowPass => {
                suggestions.push("Consider optimizing the algorithm used in this pass".to_string());
                suggestions.push("Try parallelizing the pass if possible".to_string());
                suggestions.push("Add caching to avoid redundant computations".to_string());
            }
            HotspotType::MemoryIntensive => {
                suggestions.push("Reduce memory allocations".to_string());
                suggestions.push("Use object pooling or memory reuse".to_string());
                suggestions.push("Consider streaming processing for large data".to_string());
            }
            HotspotType::CpuIntensive => {
                suggestions.push("Profile at instruction level for more specific optimizations".to_string());
                suggestions.push("Consider using more efficient algorithms".to_string());
                suggestions.push("Look for opportunities to parallelize".to_string());
            }
            HotspotType::IoBottleneck => {
                suggestions.push("Use asynchronous I/O operations".to_string());
                suggestions.push("Implement buffering or batching".to_string());
                suggestions.push("Consider caching frequently accessed data".to_string());
            }
        }

        // Add component-specific suggestions
        if component.contains("constant_folding") {
            suggestions.push("Cache constant values to avoid recomputation".to_string());
        } else if component.contains("dead_code") {
            suggestions.push("Use incremental analysis to avoid full rescans".to_string());
        } else if component.contains("peephole") {
            suggestions.push("Optimize pattern matching with lookup tables".to_string());
        }

        suggestions
    }

    /// Get summary statistics
    pub fn get_statistics(&self) -> HotspotStatistics {
        let hotspots = self.detect_hotspots();

        HotspotStatistics {
            total_components: self.timing_data.len(),
            hotspot_count: hotspots.len(),
            total_time: self.total_time,
            hotspot_time: hotspots.iter().map(|h| h.time_spent).sum(),
            average_severity: if hotspots.is_empty() {
                0.0
            } else {
                hotspots.iter().map(|h| h.severity).sum::<f32>() / hotspots.len() as f32
            },
        }
    }
}

/// Statistics about detected hotspots
#[derive(Debug, Clone)]
pub struct HotspotStatistics {
    pub total_components: usize,
    pub hotspot_count: usize,
    pub total_time: Duration,
    pub hotspot_time: Duration,
    pub average_severity: f32,
}
