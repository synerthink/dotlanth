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
//! Provides multiple strategies for scheduling segment processing based on
//! dependencies and optimization criteria

use crate::contracts::{ContractSegment, DependencyGraph, DependencyType, ProcessingError};
use std::collections::{HashMap, HashSet, VecDeque};

/// Ordered processing plan with optional parallel batches
#[derive(Debug, Clone)]
pub struct ProcessingOrder {
    /// Ordered list of segment IDs
    segment_ids: Vec<String>,

    /// Optional parallelization information
    parallelization: Option<Vec<Vec<String>>>,
}

impl ProcessingOrder {
    /// Creates linear processing order
    pub fn new(segment_ids: Vec<String>) -> Self {
        Self { segment_ids, parallelization: None }
    }

    /// Creates order with parallel processing groups
    pub fn with_parallelization(segment_ids: Vec<String>, parallelization: Vec<Vec<String>>) -> Self {
        Self {
            segment_ids,
            parallelization: Some(parallelization),
        }
    }

    /// Maps ordered IDs to ContractSegments
    pub fn get_ordered_segments(&self, segments: &[ContractSegment]) -> Vec<ContractSegment> {
        let segment_map: HashMap<&str, &ContractSegment> = segments.iter().map(|s| (s.id.as_str(), s)).collect();

        self.segment_ids
            .iter()
            .filter_map(|id| segment_map.get(id.as_str()).cloned()) // &ContractSegment al
            .cloned() // ContractSegment klonla
            .collect()
    }

    /// Get the parallel processing batches
    pub fn get_parallel_batches(&self) -> Option<&Vec<Vec<String>>> {
        self.parallelization.as_ref()
    }

    /// Check if a given segment ID is in this processing order
    pub fn contains(&self, segment_id: &str) -> bool {
        self.segment_ids.contains(&segment_id.to_string())
    }

    /// Get the position of a segment in the processing order
    pub fn position_of(&self, segment_id: &str) -> Option<usize> {
        self.segment_ids.iter().position(|id| id == segment_id)
    }
}

/// Scheduling algorithm implementations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SchedulingStrategy {
    /// Process segments in topological order based on dependencies
    TopologicalOrder,

    /// Process segments in parallel where possible
    Parallel,

    /// Process segments by type (e.g., all SECTIONs first, then all ARTICLEs)
    ByType,

    /// Process most complex segments first
    ComplexityFirst,
}

/// Algorithm for scheduling segment processing
pub struct SchedulingAlgorithm {
    /// The selected scheduling strategy
    strategy: SchedulingStrategy,

    /// Optional priority function for custom segment ordering
    priority_fn: Option<Box<dyn Fn(&ContractSegment) -> i32>>,
}

impl SchedulingAlgorithm {
    /// Creates scheduler with selected strategy
    pub fn new(strategy: SchedulingStrategy) -> Self {
        Self { strategy, priority_fn: None }
    }

    /// Assigns custom priority function for complexity-based scheduling
    pub fn with_priority_function<F>(mut self, priority_fn: F) -> Self
    where
        F: Fn(&ContractSegment) -> i32 + 'static,
    {
        self.priority_fn = Some(Box::new(priority_fn));
        self
    }

    /// Generates processing order based on strategy:
    ///
    /// # Strategies
    /// - Topological: Strict dependency order
    /// - Parallel: Maximize parallel processing
    /// - ByType: Group by segment type
    /// - ComplexityFirst: Process complex segments first
    pub fn schedule(&self, segments: &[ContractSegment], dependency_graph: &DependencyGraph) -> Result<ProcessingOrder, ProcessingError> {
        match self.strategy {
            SchedulingStrategy::TopologicalOrder => self.schedule_topological(dependency_graph),
            SchedulingStrategy::Parallel => self.schedule_parallel(segments, dependency_graph),
            SchedulingStrategy::ByType => self.schedule_by_type(segments, dependency_graph),
            SchedulingStrategy::ComplexityFirst => self.schedule_by_complexity(segments, dependency_graph),
        }
    }

    /// Topological sorting implementation
    fn schedule_topological(&self, dependency_graph: &DependencyGraph) -> Result<ProcessingOrder, ProcessingError> {
        let ordered_ids = dependency_graph.topological_sort()?;
        Ok(ProcessingOrder::new(ordered_ids))
    }

    /// Parallel batch creation algorithm:
    /// 1. Topological sort as base
    /// 2. Group segments with all dependencies met
    fn schedule_parallel(&self, segments: &[ContractSegment], dependency_graph: &DependencyGraph) -> Result<ProcessingOrder, ProcessingError> {
        // First get the base sort with topological sort
        let ordered_ids = dependency_graph.topological_sort()?;

        // Create parallel processing groups
        let mut parallel_batches: Vec<Vec<String>> = Vec::new();
        let mut processed_ids: HashSet<String> = HashSet::new();

        while processed_ids.len() < ordered_ids.len() {
            let mut current_batch: Vec<String> = Vec::new();

            for segment_id in &ordered_ids {
                if processed_ids.contains(segment_id) {
                    continue;
                }

                // Check the dependencies of the segment
                let dependencies = dependency_graph.get_dependencies(segment_id);

                // If all dependencies are processed, add this segment to this batch
                let all_dependencies_processed = dependencies.iter().all(|dep_id| processed_ids.contains(dep_id));

                if all_dependencies_processed {
                    current_batch.push(segment_id.clone());
                }
            }

            if current_batch.is_empty() {
                return Err(ProcessingError::SchedulingFailed("Failed to create parallel batches".to_string()));
            }

            // Mark all segments in this batch as processed
            for segment_id in &current_batch {
                processed_ids.insert(segment_id.clone());
            }

            parallel_batches.push(current_batch);
        }

        Ok(ProcessingOrder::with_parallelization(ordered_ids, parallel_batches))
    }

    /// Schedule segments by type, respecting dependencies
    fn schedule_by_type(&self, segments: &[ContractSegment], dependency_graph: &DependencyGraph) -> Result<ProcessingOrder, ProcessingError> {
        // First get the base sort with topological sort
        let topological_ids = dependency_graph.topological_sort()?;

        // Ranking of segment types (in order of importance)
        let type_order = ["SECTION", "ARTICLE", "CLAUSE"];

        // Group segments by type
        let mut type_groups: HashMap<&str, Vec<String>> = HashMap::new();

        for segment in segments {
            type_groups.entry(segment.segment_type.as_str()).or_insert_with(Vec::new).push(segment.id.clone());
        }

        // Sort segments by type, but without violating dependencies
        let mut ordered_ids: Vec<String> = Vec::new();

        // Insert in type order first
        for segment_type in type_order.iter() {
            if let Some(segment_ids) = type_groups.get(*segment_type) {
                for segment_id in segment_ids {
                    // Add the segment only if all its dependencies are already in ordered_ids
                    let dependencies = dependency_graph.get_dependencies(segment_id);
                    let all_dependencies_included = dependencies.iter().all(|dep_id| ordered_ids.contains(dep_id));

                    if all_dependencies_included && !ordered_ids.contains(segment_id) {
                        ordered_ids.push(segment_id.clone());
                    }
                }
            }
        }

        // If there are still missing segments, add them in topological order
        for segment_id in topological_ids {
            if !ordered_ids.contains(&segment_id) {
                ordered_ids.push(segment_id);
            }
        }

        Ok(ProcessingOrder::new(ordered_ids))
    }

    /// Schedule segments by complexity, with complex segments first if possible
    fn schedule_by_complexity(&self, segments: &[ContractSegment], dependency_graph: &DependencyGraph) -> Result<ProcessingOrder, ProcessingError> {
        // First get the base sort with topological sort
        let mut topological_ids = dependency_graph.topological_sort()?;
        // If the topological sort is empty, use all segment ids
        if topological_ids.is_empty() {
            topological_ids = segments.iter().map(|s| s.id.clone()).collect();
        }

        // Calculate the complexity of segments
        let mut complexities: HashMap<String, i32> = HashMap::new();
        for segment in segments {
            let complexity = if let Some(priority_fn) = &self.priority_fn {
                priority_fn(segment)
            } else {
                segment.content.len() as i32
            };
            complexities.insert(segment.id.clone(), complexity);
        }

        // Sort with Kahn algorithm-like approach
        let mut ordered_ids: Vec<String> = Vec::new();
        let mut queue: VecDeque<String> = VecDeque::new();

        // Add segments with no dependencies to the queue
        for segment_id in &topological_ids {
            if !dependency_graph.has_dependencies(segment_id) {
                queue.push_back(segment_id.clone());
            }
        }

        while !queue.is_empty() {
            // Find the index of the most complex segment
            let most_complex_idx = (0..queue.len()).max_by_key(|&i| complexities.get(queue.get(i).unwrap()).cloned().unwrap_or(0)).unwrap_or(0);

            let segment_id = queue.remove(most_complex_idx).unwrap();
            ordered_ids.push(segment_id.clone());

            // Add this segment's dependents to the queue
            for dependent_id in dependency_graph.get_dependents(&segment_id) {
                let all_dependencies_processed = dependency_graph.get_dependencies(&dependent_id).iter().all(|dep_id| ordered_ids.contains(dep_id));
                if all_dependencies_processed && !ordered_ids.contains(&dependent_id) {
                    queue.push_back(dependent_id);
                }
            }
        }

        // If some segments are not included in the ranking, add them from topological_ids
        for segment_id in topological_ids {
            if !ordered_ids.contains(&segment_id) {
                ordered_ids.push(segment_id);
            }
        }

        Ok(ProcessingOrder::new(ordered_ids))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{DependencyGraph, DependencyType};

    #[test]
    fn test_topological_scheduling() {
        let segments = vec![
            ContractSegment::new("segment-1".to_string(), "contract-001".to_string(), "SECTION".to_string(), "This is section 1".to_string(), 0),
            ContractSegment::new("segment-2".to_string(), "contract-001".to_string(), "ARTICLE".to_string(), "This is article 2".to_string(), 1),
            ContractSegment::new("segment-3".to_string(), "contract-001".to_string(), "CLAUSE".to_string(), "This is clause 3".to_string(), 2),
        ];

        let mut graph = DependencyGraph::new();

        graph.add_dependency("segment-2", "segment-1", DependencyType::Reference);
        graph.add_dependency("segment-3", "segment-2", DependencyType::Reference);

        let scheduler = SchedulingAlgorithm::new(SchedulingStrategy::TopologicalOrder);
        let order = scheduler.schedule(&segments, &graph).unwrap();

        assert_eq!(order.segment_ids, vec!["segment-1", "segment-2", "segment-3"]);
    }

    #[test]
    fn test_parallel_scheduling() {
        let segments = vec![
            ContractSegment::new("segment-1".to_string(), "contract-001".to_string(), "SECTION".to_string(), "This is section 1".to_string(), 0),
            ContractSegment::new("segment-2".to_string(), "contract-001".to_string(), "ARTICLE".to_string(), "This is article 2".to_string(), 1),
            ContractSegment::new("segment-3".to_string(), "contract-001".to_string(), "CLAUSE".to_string(), "This is clause 3".to_string(), 2),
            ContractSegment::new("segment-4".to_string(), "contract-001".to_string(), "CLAUSE".to_string(), "This is clause 4".to_string(), 3),
        ];

        let mut graph = DependencyGraph::new();

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
        assert!(batches[1].contains(&"segment-2".to_string()));
        assert!(batches[1].contains(&"segment-3".to_string()));
        assert_eq!(batches[2], vec!["segment-4"]);
    }

    #[test]
    fn test_complexity_scheduling() {
        let segments = vec![
            ContractSegment::new(
                "segment-1".to_string(),
                "contract-001".to_string(),
                "SECTION".to_string(),
                "Short content".to_string(), // Daha az karmaşık
                0,
            ),
            ContractSegment::new(
                "segment-2".to_string(),
                "contract-001".to_string(),
                "ARTICLE".to_string(),
                "This is a longer and more complex content".to_string(), // Daha karmaşık
                1,
            ),
            ContractSegment::new(
                "segment-3".to_string(),
                "contract-001".to_string(),
                "CLAUSE".to_string(),
                "Medium complexity content".to_string(), // Orta karmaşıklık
                2,
            ),
        ];

        let graph = DependencyGraph::new();

        let scheduler = SchedulingAlgorithm::new(SchedulingStrategy::ComplexityFirst);
        let order = scheduler.schedule(&segments, &graph).unwrap();

        assert_eq!(order.segment_ids[0], "segment-2");

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

        // Expected order: segment-1 (SECTION - highest priority), segment-2 (ARTICLE), segment-3 (CLAUSE)
        assert_eq!(custom_order.segment_ids[0], "segment-1");
        assert_eq!(custom_order.segment_ids[1], "segment-2");
        assert_eq!(custom_order.segment_ids[2], "segment-3");
    }
}
