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

//! Dependency resolution module for contract segments.
//!
//! Provides graph-based dependency management and resolution between contract segments,
//! ensuring valid processing order and cycle detection.

use crate::contracts::{ContractSegment, ProcessingError};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// Represents a directed dependency relationship between two contract segments
#[derive(Debug, Clone)]
pub struct SegmentDependency {
    /// ID of the segment requiring another to function
    pub dependent_id: String,

    /// ID of the required segment
    pub dependency_id: String,

    /// Nature of the dependency relationship
    pub dependency_type: DependencyType,

    /// Weight indicating importance (0.0-1.0)
    pub strength: f32,
}

/// Categorizes different types of dependencies between segments
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyType {
    /// Dependent references the target in its content
    Reference,

    /// Dependent extends functionality of the target
    Extension,

    /// Dependent overrides behavior of the target
    Override,

    /// Dependent requires target to be processed first
    Prerequisite,
}

/// Graph structure modeling segment dependencies with topological sorting
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// The underlying directed graph
    graph: DiGraph<String, DependencyType>,

    /// Mapping from segment IDs to node indices
    node_indices: HashMap<String, NodeIndex>,
}

impl DependencyGraph {
    /// Initializes an empty dependency graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
        }
    }

    /// Adds a segment to the graph if not present
    ///
    /// # Arguments
    /// - `segment_id`: Unique identifier for the segment
    ///
    /// # Returns
    /// NodeIndex for the added/existing segment
    pub fn add_segment(&mut self, segment_id: &str) -> NodeIndex {
        if let Some(&node_index) = self.node_indices.get(segment_id) {
            return node_index;
        }

        let node_index = self.graph.add_node(segment_id.to_string());
        self.node_indices.insert(segment_id.to_string(), node_index);
        node_index
    }

    /// Establishes a directed dependency between two segments
    ///
    /// # Arguments
    /// - `dependent_id`: Requiring segment
    /// - `dependency_id`: Required segment  
    /// - `dependency_type`: Relationship type
    pub fn add_dependency(&mut self, dependent_id: &str, dependency_id: &str, dependency_type: DependencyType) {
        let dependent_index = self.add_segment(dependent_id);
        let dependency_index = self.add_segment(dependency_id);

        self.graph.add_edge(dependency_index, dependent_index, dependency_type);
    }

    /// Generates processing order using topological sort
    ///
    /// # Returns
    /// - Ok(Vec<String>): Valid processing order
    /// - Err(ProcessingError): On cyclic dependencies
    pub fn topological_sort(&self) -> Result<Vec<String>, ProcessingError> {
        match toposort(&self.graph, None) {
            Ok(indices) => {
                // Converting NodeIndexes to segment IDs
                let sorted_ids = indices.into_iter().map(|idx| self.graph[idx].clone()).collect();
                Ok(sorted_ids)
            }
            Err(_) => Err(ProcessingError::DependencyResolutionFailed("Circular dependency detected in segments".to_string())),
        }
    }

    /// Checks if a segment has incoming dependencies
    pub fn has_dependencies(&self, segment_id: &str) -> bool {
        if let Some(&node_index) = self.node_indices.get(segment_id) {
            self.graph.neighbors_directed(node_index, petgraph::Direction::Incoming).count() > 0
        } else {
            false
        }
    }

    /// Get all dependencies for a segment
    pub fn get_dependencies(&self, segment_id: &str) -> Vec<String> {
        if let Some(&node_index) = self.node_indices.get(segment_id) {
            self.graph.neighbors_directed(node_index, petgraph::Direction::Incoming).map(|idx| self.graph[idx].clone()).collect()
        } else {
            Vec::new()
        }
    }

    /// Finds all segments depending on the specified segment
    pub fn get_dependents(&self, segment_id: &str) -> Vec<String> {
        if let Some(&node_index) = self.node_indices.get(segment_id) {
            self.graph.neighbors_directed(node_index, petgraph::Direction::Outgoing).map(|idx| self.graph[idx].clone()).collect()
        } else {
            Vec::new()
        }
    }
}

/// Orchestrates dependency detection using multiple strategies
pub struct DependencyResolver {
    /// Strategies for detecting dependencies
    detection_strategies: Vec<Box<dyn DependencyDetectionStrategy>>,
}

impl DependencyResolver {
    /// Creates resolver with default detection strategies:
    /// - Reference detection
    /// - Hierarchical detection
    pub fn new() -> Self {
        let mut resolver = Self { detection_strategies: Vec::new() };

        // Varsayılan stratejileri ekle
        resolver.detection_strategies.push(Box::new(ReferenceDetectionStrategy {}));
        resolver.detection_strategies.push(Box::new(HierarchicalDetectionStrategy {}));

        resolver
    }

    /// Adds custom detection strategy to the resolver
    pub fn add_strategy(&mut self, strategy: Box<dyn DependencyDetectionStrategy>) {
        self.detection_strategies.push(strategy);
    }

    /// Resolves dependencies across all segments
    ///
    /// # Workflow
    /// 1. Adds all segments to graph
    /// 2. Applies detection strategies
    /// 3. Validates acyclic graph
    ///
    /// # Returns
    /// - Ok(DependencyGraph): Validated dependency graph
    /// - Err(ProcessingError): On resolution failures
    pub fn resolve_dependencies(&self, segments: &[ContractSegment]) -> Result<DependencyGraph, ProcessingError> {
        let mut graph = DependencyGraph::new();

        // Add all segments to graph
        for segment in segments {
            graph.add_segment(&segment.id);
        }

        // Apply each strategy
        for strategy in &self.detection_strategies {
            strategy.detect_dependencies(segments, &mut graph)?;
        }

        // Cyclic dependency check
        if let Err(_) = graph.topological_sort() {
            return Err(ProcessingError::DependencyResolutionFailed("Circular dependency detected".to_string()));
        }

        Ok(graph)
    }
}

/// Strategy pattern for detecting specific dependency types
pub trait DependencyDetectionStrategy {
    /// Analyzes segments and adds dependencies to graph
    ///
    /// # Arguments
    /// - segments: Contract segments to analyze
    /// - graph: Mutable reference to dependency graph
    fn detect_dependencies(&self, segments: &[ContractSegment], graph: &mut DependencyGraph) -> Result<(), ProcessingError>;
}

/// Detects references through content analysis
pub struct ReferenceDetectionStrategy {}

impl DependencyDetectionStrategy for ReferenceDetectionStrategy {
    /// Identifies dependencies by searching for:
    /// - "see {id}" patterns
    /// - "refer to {id}" patterns  
    /// - "as per {id}" patterns
    fn detect_dependencies(&self, segments: &[ContractSegment], graph: &mut DependencyGraph) -> Result<(), ProcessingError> {
        let segment_map: HashMap<&str, &ContractSegment> = segments.iter().map(|s| (s.id.as_str(), s)).collect();

        for dependent in segments {
            for (potential_dependency_id, dependency) in &segment_map {
                // Kendi kendine bağımlılık olmamalı
                if dependent.id == dependency.id {
                    continue;
                }

                if dependent.content.contains(&format!("see {}", dependency.id))
                    || dependent.content.contains(&format!("refer to {}", dependency.id))
                    || dependent.content.contains(&format!("as per {}", dependency.id))
                {
                    graph.add_dependency(&dependent.id, &dependency.id, DependencyType::Reference);
                }
            }
        }

        Ok(())
    }
}

/// Detects hierarchical relationships based on segment types
pub struct HierarchicalDetectionStrategy {}

impl DependencyDetectionStrategy for HierarchicalDetectionStrategy {
    /// Establishes dependencies based on:
    /// - Segment type hierarchy (SECTION > ARTICLE > CLAUSE)
    /// - Position within same contract
    fn detect_dependencies(&self, segments: &[ContractSegment], graph: &mut DependencyGraph) -> Result<(), ProcessingError> {
        let segment_types = ["SECTION", "ARTICLE", "CLAUSE"];

        // Map the segment types in priority order.
        let type_priority: HashMap<&str, usize> = segment_types.iter().enumerate().map(|(i, &s_type)| (s_type, i)).collect();

        // Group segments by type
        let mut grouped_segments: HashMap<&str, Vec<&ContractSegment>> = HashMap::new();
        for segment in segments {
            grouped_segments.entry(segment.segment_type.as_str()).or_insert_with(Vec::new).push(segment);
        }

        // For each segment type, add dependency on higher priority types
        for (segment_type, segments_of_type) in &grouped_segments {
            if let Some(&type_prio) = type_priority.get(segment_type) {
                for segment in segments_of_type {
                    for higher_type in segment_types.iter().take(type_prio) {
                        if let Some(higher_segments) = grouped_segments.get(higher_type) {
                            for higher_segment in higher_segments {
                                if segment.contract_id == higher_segment.contract_id && segment.position > higher_segment.position {
                                    graph.add_dependency(&segment.id, &higher_segment.id, DependencyType::Prerequisite);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_resolution() {
        let segments = vec![
            ContractSegment::new("segment-1".to_string(), "contract-001".to_string(), "SECTION".to_string(), "This is section 1".to_string(), 0),
            ContractSegment::new(
                "segment-2".to_string(),
                "contract-001".to_string(),
                "ARTICLE".to_string(),
                "This is article 2, refer to segment-1".to_string(),
                1,
            ),
            ContractSegment::new(
                "segment-3".to_string(),
                "contract-001".to_string(),
                "CLAUSE".to_string(),
                "This is clause 3, see segment-2".to_string(),
                2,
            ),
        ];

        let resolver = DependencyResolver::new();
        let graph = resolver.resolve_dependencies(&segments).unwrap();

        let sorted = graph.topological_sort().unwrap();

        assert!(sorted.iter().position(|id| id == "segment-1").unwrap() < sorted.iter().position(|id| id == "segment-2").unwrap());
        assert!(sorted.iter().position(|id| id == "segment-2").unwrap() < sorted.iter().position(|id| id == "segment-3").unwrap());
    }

    #[test]
    fn test_circular_dependency() {
        let segments = vec![
            ContractSegment::new(
                "segment-1".to_string(),
                "contract-001".to_string(),
                "SECTION".to_string(),
                "This is section 1, see segment-3".to_string(),
                0,
            ),
            ContractSegment::new(
                "segment-2".to_string(),
                "contract-001".to_string(),
                "ARTICLE".to_string(),
                "This is article 2, refer to segment-1".to_string(),
                1,
            ),
            ContractSegment::new(
                "segment-3".to_string(),
                "contract-001".to_string(),
                "CLAUSE".to_string(),
                "This is clause 3, see segment-2".to_string(),
                2,
            ),
        ];

        let mut resolver = DependencyResolver::new();

        resolver.detection_strategies.clear();
        resolver.detection_strategies.push(Box::new(ReferenceDetectionStrategy {}));

        let mut graph = DependencyGraph::new();
        graph.add_segment("segment-1");
        graph.add_segment("segment-2");
        graph.add_segment("segment-3");

        graph.add_dependency("segment-2", "segment-1", DependencyType::Reference);
        graph.add_dependency("segment-3", "segment-2", DependencyType::Reference);
        graph.add_dependency("segment-1", "segment-3", DependencyType::Reference);

        assert!(graph.topological_sort().is_err());
    }
}
