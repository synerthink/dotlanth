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

//! Loop detection in control flow graphs

use super::{ControlFlowEdge, ControlFlowGraph, ControlFlowLoop, LoopType};

/// Detects loops by finding back edges and their bodies
pub struct LoopDetector;

impl LoopDetector {
    /// Returns discovered loops (header, body nodes, back edges, type)
    pub fn detect(cfg: &ControlFlowGraph) -> Vec<ControlFlowLoop> {
        let mut loops = Vec::new();
        // A simple heuristic: any edge from a later node to an earlier is a back edge
        for edge in &cfg.edges {
            if edge.from > edge.to {
                // collect nodes in the loop
                let mut body = Vec::new();
                // BFS from header
                let mut stack = vec![edge.from];
                while let Some(n) = stack.pop() {
                    if !body.contains(&n) {
                        body.push(n);
                        for succ_edge in &cfg.edges {
                            if succ_edge.from == n {
                                stack.push(succ_edge.to);
                            }
                        }
                    }
                }
                loops.push(ControlFlowLoop {
                    header: edge.to,
                    body: body.into_iter().collect(),
                    back_edges: vec![(edge.from, edge.to)],
                    loop_type: LoopType::While,
                });
            }
        }
        loops
    }
}
