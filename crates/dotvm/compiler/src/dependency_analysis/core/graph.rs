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

//! Dependency graph representation

use petgraph::Directed;
use petgraph::graph::{Graph, NodeIndex};

/// Node in a dependency graph
#[derive(Debug, Clone)]
pub struct DepNode {
    /// Unique identifier
    pub id: String,
}

/// Edge in a dependency graph
#[derive(Debug, Clone)]
pub struct DepEdge;

/// Dependency graph type alias
pub type DependencyGraph = Graph<DepNode, DepEdge, Directed>;

/// Utility for building dependency graphs
pub struct GraphBuilder {
    graph: DependencyGraph,
    indices: std::collections::HashMap<String, NodeIndex>,
}

impl GraphBuilder {
    /// Create a new graph builder
    pub fn new() -> Self {
        Self {
            graph: DependencyGraph::new(),
            indices: Default::default(),
        }
    }

    /// Add a node if not exists, returns its index
    pub fn add_node(&mut self, id: String) -> NodeIndex {
        if let Some(&idx) = self.indices.get(&id) {
            return idx;
        }
        let idx = self.graph.add_node(DepNode { id: id.clone() });
        self.indices.insert(id, idx);
        idx
    }

    /// Add an edge between two node IDs
    pub fn add_edge(&mut self, from: &str, to: &str) {
        let u = self.add_node(from.to_string());
        let v = self.add_node(to.to_string());
        self.graph.add_edge(u, v, DepEdge);
    }

    /// Finalize and return the graph
    pub fn build(self) -> DependencyGraph {
        self.graph
    }
}
