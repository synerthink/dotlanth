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

//! Reachability analysis to find unreachable blocks

use super::ControlFlowGraph;

/// Finds nodes that are unreachable from the entry node
pub struct ReachabilityAnalyzer;

impl ReachabilityAnalyzer {
    /// Returns IDs of unreachable nodes
    pub fn find_unreachable(cfg: &ControlFlowGraph) -> Vec<usize> {
        let mut visited = Vec::new();
        let mut stack = vec![cfg.entry_node];
        while let Some(n) = stack.pop() {
            if !visited.contains(&n) {
                visited.push(n);
                for edge in &cfg.edges {
                    if edge.from == n {
                        stack.push(edge.to);
                    }
                }
            }
        }
        cfg.nodes.keys().filter(|&&n| !visited.contains(&n)).copied().collect()
    }
}
