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

//! Dependency resolution module for dot segments.
//!
//! Provides graph-based dependency management and resolution between dot segments,
//! ensuring valid processing order and cycle detection.

use crate::dots::{DotSegment, ProcessingError};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

pub mod strategies;

/// Represents a directed dependency relationship between two dot segments
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

/// Graph structure modeling dot segment dependencies with topological sorting
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// The underlying directed graph
    graph: DiGraph<String, DependencyType>,

    /// Mapping from dot segment IDs to node indices
    node_indices: HashMap<String, NodeIndex>,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraph {
    /// Initializes an empty dependency graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
        }
    }

    /// Adds a dot segment to the graph if not present
    ///
    /// # Arguments
    /// - `segment_id`: Unique identifier for the dot segment
    ///
    /// # Returns
    /// NodeIndex for the added/existing dot segment
    pub fn add_segment(&mut self, segment_id: &str) -> NodeIndex {
        if let Some(&node_index) = self.node_indices.get(segment_id) {
            return node_index;
        }

        let node_index = self.graph.add_node(segment_id.to_string());
        self.node_indices.insert(segment_id.to_string(), node_index);
        node_index
    }

    /// Establishes a directed dependency between two dot segments
    ///
    /// # Arguments
    /// - `dependent_id`: Requiring dot segment
    /// - `dependency_id`: Required dot segment
    /// - `dependency_type`: Relationship type
    pub fn add_dependency(&mut self, dependent_id: &str, dependency_id: &str, dependency_type: DependencyType) {
        let dependent_index = self.add_segment(dependent_id);
        let dependency_index = self.add_segment(dependency_id);

        self.graph.add_edge(dependency_index, dependent_index, dependency_type);
    }

    /// Generates processing order using topological sort
    ///
    /// # Returns
    /// - Ok(Vec<String>): Valid processing order of dot segment IDs
    /// - Err(ProcessingError): On cyclic dependencies
    pub fn topological_sort(&self) -> Result<Vec<String>, ProcessingError> {
        match toposort(&self.graph, None) {
            Ok(indices) => {
                // Converting NodeIndexes to segment IDs
                let sorted_ids = indices.into_iter().map(|idx| self.graph[idx].clone()).collect();
                Ok(sorted_ids)
            }
            Err(_) => Err(ProcessingError::DependencyResolutionFailed("Circular dependency detected in dot segments".to_string())),
        }
    }

    /// Checks if a dot segment has incoming dependencies
    pub fn has_dependencies(&self, segment_id: &str) -> bool {
        if let Some(&node_index) = self.node_indices.get(segment_id) {
            self.graph.neighbors_directed(node_index, petgraph::Direction::Incoming).count() > 0
        } else {
            false
        }
    }

    /// Get all dependencies for a dot segment
    pub fn get_dependencies(&self, segment_id: &str) -> Vec<String> {
        if let Some(&node_index) = self.node_indices.get(segment_id) {
            self.graph.neighbors_directed(node_index, petgraph::Direction::Incoming).map(|idx| self.graph[idx].clone()).collect()
        } else {
            Vec::new()
        }
    }

    /// Finds all dot segments depending on the specified segment
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

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyResolver {
    /// Creates resolver with default detection strategies:
    /// - Reference detection
    /// - Hierarchical detection
    pub fn new() -> Self {
        let mut resolver = Self { detection_strategies: Vec::new() };

        // Add default strategies
        resolver.detection_strategies.push(Box::new(strategies::reference::ReferenceDetectionStrategy {}));
        resolver.detection_strategies.push(Box::new(strategies::hierarchical::HierarchicalDetectionStrategy {}));

        resolver
    }

    /// Adds custom detection strategy to the resolver
    pub fn add_strategy(&mut self, strategy: Box<dyn DependencyDetectionStrategy>) {
        self.detection_strategies.push(strategy);
    }

    /// Resolves dependencies across all dot segments
    ///
    /// # Workflow
    /// 1. Adds all dot segments to graph
    /// 2. Applies detection strategies
    /// 3. Validates acyclic graph
    ///
    /// # Returns
    /// - Ok(DependencyGraph): Validated dependency graph
    /// - Err(ProcessingError): On resolution failures
    pub fn resolve_dependencies(&self, segments: &[DotSegment]) -> Result<DependencyGraph, ProcessingError> {
        let mut graph = DependencyGraph::new();

        // Add all dot segments to graph
        for segment in segments {
            graph.add_segment(&segment.id);
        }

        // Apply each strategy
        for strategy in &self.detection_strategies {
            strategy.detect_dependencies(segments, &mut graph)?;
        }

        // Cyclic dependency check
        if graph.topological_sort().is_err() {
            return Err(ProcessingError::DependencyResolutionFailed("Circular dependency detected".to_string()));
        }

        Ok(graph)
    }
}

/// Strategy pattern for detecting specific dependency types
pub trait DependencyDetectionStrategy {
    /// Analyzes dot segments and adds dependencies to graph
    ///
    /// # Arguments
    /// - segments: Dot segments to analyze
    /// - graph: Mutable reference to dependency graph
    fn detect_dependencies(&self, segments: &[DotSegment], graph: &mut DependencyGraph) -> Result<(), ProcessingError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dots::dependencies::strategies::reference::ReferenceDetectionStrategy;

    #[test]
    fn test_dependency_resolution() {
        let segments = vec![
            DotSegment::new("segment-1".to_string(), "dot-001".to_string(), "SECTION".to_string(), "This is section 1".to_string(), 0),
            DotSegment::new(
                "segment-2".to_string(),
                "dot-001".to_string(),
                "ARTICLE".to_string(),
                "This is article 2, refer to segment-1".to_string(),
                1,
            ),
            DotSegment::new("segment-3".to_string(), "dot-001".to_string(), "CLAUSE".to_string(), "This is clause 3, see segment-2".to_string(), 2),
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
            DotSegment::new("segment-1".to_string(), "dot-001".to_string(), "SECTION".to_string(), "This is section 1, see segment-3".to_string(), 0),
            DotSegment::new(
                "segment-2".to_string(),
                "dot-001".to_string(),
                "ARTICLE".to_string(),
                "This is article 2, refer to segment-1".to_string(),
                1,
            ),
            DotSegment::new("segment-3".to_string(), "dot-001".to_string(), "CLAUSE".to_string(), "This is clause 3, see segment-2".to_string(), 2),
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
