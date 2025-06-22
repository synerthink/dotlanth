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

//! Processing order determination algorithms
//!
//! Provides multiple strategies for scheduling dot segment processing based on
//! dependencies and optimization criteria

use crate::dots::{DependencyGraph, DotSegment, ProcessingError};
use std::collections::HashMap;

pub mod strategies;

/// Ordered processing plan with optional parallel batches for dot segments
#[derive(Debug, Clone)]
pub struct ProcessingOrder {
    /// Ordered list of dot segment IDs
    segment_ids: Vec<String>,

    /// Optional parallelization information
    parallelization: Option<Vec<Vec<String>>>,
}

impl ProcessingOrder {
    /// Creates linear processing order for dot segments
    pub fn new(segment_ids: Vec<String>) -> Self {
        Self { segment_ids, parallelization: None }
    }

    /// Creates order with parallel processing groups for dot segments
    pub fn with_parallelization(segment_ids: Vec<String>, parallelization: Vec<Vec<String>>) -> Self {
        Self {
            segment_ids,
            parallelization: Some(parallelization),
        }
    }

    /// Maps ordered IDs to DotSegments
    pub fn get_ordered_segments(&self, segments: &[DotSegment]) -> Vec<DotSegment> {
        let segment_map: HashMap<&str, &DotSegment> = segments.iter().map(|s| (s.id.as_str(), s)).collect();

        self.segment_ids
            .iter()
            .filter_map(|id| segment_map.get(id.as_str()).cloned()) // &DotSegment
            .cloned()
            .collect()
    }

    /// Get the parallel processing batches
    pub fn get_parallel_batches(&self) -> Option<&Vec<Vec<String>>> {
        self.parallelization.as_ref()
    }

    /// Check if a given dot segment ID is in this processing order
    pub fn contains(&self, segment_id: &str) -> bool {
        self.segment_ids.contains(&segment_id.to_string())
    }

    /// Get the position of a dot segment in the processing order
    pub fn position_of(&self, segment_id: &str) -> Option<usize> {
        self.segment_ids.iter().position(|id| id == segment_id)
    }
}

/// Scheduling algorithm implementations for dot segments
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SchedulingStrategy {
    /// Process dot segments in topological order based on dependencies
    TopologicalOrder,

    /// Process dot segments in parallel where possible
    Parallel,

    /// Process dot segments by type (e.g., all SECTIONs first, then all ARTICLEs)
    ByType,

    /// Process most complex dot segments first
    ComplexityFirst,
}

/// Algorithm for scheduling dot segment processing
pub struct SchedulingAlgorithm {
    /// The selected scheduling strategy
    strategy: SchedulingStrategy,

    /// Optional priority function for custom dot segment ordering
    priority_fn: Option<Box<dyn Fn(&DotSegment) -> i32>>,
}

impl SchedulingAlgorithm {
    /// Creates scheduler with selected strategy
    pub fn new(strategy: SchedulingStrategy) -> Self {
        Self { strategy, priority_fn: None }
    }

    /// Assigns custom priority function for complexity-based scheduling
    pub fn with_priority_function<F>(mut self, priority_fn: F) -> Self
    where
        F: Fn(&DotSegment) -> i32 + 'static,
    {
        self.priority_fn = Some(Box::new(priority_fn));
        self
    }

    /// Generates processing order based on strategy:
    ///
    /// # Strategies
    /// - Topological: Strict dependency order
    /// - Parallel: Maximize parallel processing
    /// - ByType: Group by dot segment type
    /// - ComplexityFirst: Process complex dot segments first
    pub fn schedule(&self, segments: &[DotSegment], dependency_graph: &DependencyGraph) -> Result<ProcessingOrder, ProcessingError> {
        match self.strategy {
            SchedulingStrategy::TopologicalOrder => strategies::topological::schedule_topological(dependency_graph),
            SchedulingStrategy::Parallel => strategies::parallel::schedule_parallel(segments, dependency_graph),
            SchedulingStrategy::ByType => strategies::by_type::schedule_by_type(segments, dependency_graph),
            SchedulingStrategy::ComplexityFirst => strategies::complexity_first::schedule_by_complexity(segments, dependency_graph, &self.priority_fn),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dots::DependencyType;

    #[test]
    fn test_topological_scheduling() {
        let segments = vec![
            DotSegment::new("segment-1".to_string(), "contract-001".to_string(), "SECTION".to_string(), "This is section 1".to_string(), 0),
            DotSegment::new("segment-2".to_string(), "contract-001".to_string(), "ARTICLE".to_string(), "This is article 2".to_string(), 1),
            DotSegment::new("segment-3".to_string(), "contract-001".to_string(), "CLAUSE".to_string(), "This is clause 3".to_string(), 2),
        ];

        let mut graph = DependencyGraph::new();
        graph.add_segment("segment-1");
        graph.add_segment("segment-2");
        graph.add_segment("segment-3");
        // Dependency: segment-1 -> segment-2 (segment-2 depends on segment-1)
        graph.add_dependency("segment-2", "segment-1", DependencyType::Reference);
        // Dependency: segment-2 -> segment-3 (segment-3 depends on segment-2)
        graph.add_dependency("segment-3", "segment-2", DependencyType::Reference);

        let scheduler = SchedulingAlgorithm::new(SchedulingStrategy::TopologicalOrder);
        let order = scheduler.schedule(&segments, &graph).unwrap();

        assert_eq!(order.segment_ids, vec!["segment-1", "segment-2", "segment-3"]);
    }

    #[test]
    fn test_parallel_scheduling() {
        let segments = vec![
            DotSegment::new("segment-1".to_string(), "dot-001".to_string(), "SECTION".to_string(), "This is section 1".to_string(), 0),
            DotSegment::new("segment-2".to_string(), "dot-001".to_string(), "ARTICLE".to_string(), "This is article 2".to_string(), 1),
            DotSegment::new("segment-3".to_string(), "dot-001".to_string(), "CLAUSE".to_string(), "This is clause 3".to_string(), 2),
            DotSegment::new("segment-4".to_string(), "dot-001".to_string(), "CLAUSE".to_string(), "This is clause 4".to_string(), 3),
        ];

        let mut graph = DependencyGraph::new();
        graph.add_segment("segment-1");
        graph.add_segment("segment-2");
        graph.add_segment("segment-3");
        graph.add_segment("segment-4");

        graph.add_dependency("segment-2", "segment-1", DependencyType::Reference);
        graph.add_dependency("segment-3", "segment-1", DependencyType::Reference);
        graph.add_dependency("segment-4", "segment-2", DependencyType::Reference);
        graph.add_dependency("segment-4", "segment-3", DependencyType::Reference);

        let scheduler = SchedulingAlgorithm::new(SchedulingStrategy::Parallel);
        let order = scheduler.schedule(&segments, &graph).unwrap();

        let batches = order.get_parallel_batches().unwrap();

        // Expected parallel batches:
        // Batch 1: segment-1
        // Batch 2: segment-2, segment-3 (dependent on segment-1, can be processed in parallel)
        // Batch 3: segment-4 (dependent on segment-2 and segment-3)
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0], vec!["segment-1"]);
        // Order within a batch is not guaranteed, so check for presence and length.
        assert_eq!(batches[1].len(), 2);
        assert!(batches[1].contains(&"segment-2".to_string()));
        assert!(batches[1].contains(&"segment-3".to_string()));
        assert_eq!(batches[2], vec!["segment-4"]);
    }

    #[test]
    fn test_complexity_scheduling() {
        let segments = vec![
            DotSegment::new(
                "segment-1".to_string(),
                "dot-001".to_string(),
                "SECTION".to_string(),
                "Short content".to_string(), // Less complex
                0,
            ),
            DotSegment::new(
                "segment-2".to_string(),
                "dot-001".to_string(),
                "ARTICLE".to_string(),
                "This is a longer and more complex content".to_string(), // More complex
                1,
            ),
            DotSegment::new(
                "segment-3".to_string(),
                "dot-001".to_string(),
                "CLAUSE".to_string(),
                "Medium complexity content".to_string(), // Medium complexity
                2,
            ),
        ];

        let mut graph = DependencyGraph::new();
        // Add segments to the graph so they are known, even if there are no explicit dependencies for this test case.
        graph.add_segment("segment-1");
        graph.add_segment("segment-2");
        graph.add_segment("segment-3");

        let scheduler = SchedulingAlgorithm::new(SchedulingStrategy::ComplexityFirst);
        let order = scheduler.schedule(&segments, &graph).unwrap();

        // Default complexity is content length.
        // Lengths: seg1=13, seg2=42, seg3=26
        // Expected order by complexity (desc): segment-2, segment-3, segment-1
        assert_eq!(order.segment_ids, vec!["segment-2", "segment-3", "segment-1"]);

        // Test with special complexity function
        let custom_scheduler = SchedulingAlgorithm::new(SchedulingStrategy::ComplexityFirst).with_priority_function(|segment| {
            // Custom complexity function - based on segment type
            match segment.segment_type.as_str() {
                "SECTION" => 3, // Highest priority
                "ARTICLE" => 2,
                "CLAUSE" => 1, // Lowest priority
                _ => 0,
            }
        });

        let custom_order = custom_scheduler.schedule(&segments, &graph).unwrap();

        // Expected order by custom priority (desc): segment-1 (SECTION), segment-2 (ARTICLE), segment-3 (CLAUSE)
        assert_eq!(custom_order.segment_ids, vec!["segment-1", "segment-2", "segment-3"]);
    }
}
