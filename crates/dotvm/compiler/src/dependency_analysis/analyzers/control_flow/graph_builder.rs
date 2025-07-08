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

//! Constructs the control flow graph (basic blocks and edges)

use super::{ControlFlowEdge, ControlFlowEdgeType, ControlFlowGraph, ControlFlowNode};
use std::collections::HashMap;

/// Builds a control flow graph from a sequence of basic blocks
pub struct ControlFlowGraphBuilder;

impl ControlFlowGraphBuilder {
    /// Create a new ControlFlowGraphBuilder
    pub fn new() -> Self {
        Self
    }

    /// Create the CFG by linking nodes sequentially & by terminator logic
    pub fn build(mut nodes: Vec<ControlFlowNode>) -> ControlFlowGraph {
        let mut edges = Vec::new();
        let mut node_map: HashMap<usize, ControlFlowNode> = nodes.into_iter().map(|n| (n.id, n)).collect();

        // Sequential edges between consecutive blocks
        let mut ids: Vec<usize> = node_map.keys().copied().collect();
        ids.sort_unstable();
        for window in ids.windows(2) {
            let from = window[0];
            let to = window[1];
            edges.push(ControlFlowEdge {
                from,
                to,
                edge_type: ControlFlowEdgeType::Sequential,
                condition: None,
            });
        }

        // TODO: add conditional and loop edges based on node metadata

        // Build entry/exit markers
        let entry_node = *ids.first().unwrap();
        let exit_nodes: Vec<usize> = ids.iter().copied().rev().take(1).collect();

        ControlFlowGraph {
            nodes: node_map,
            edges,
            entry_node,
            exit_node: exit_nodes.first().copied(),
            exit_nodes,
            loops: Vec::new(),
            unreachable_blocks: Vec::new(),
            complexity: Default::default(),
        }
    }
}
