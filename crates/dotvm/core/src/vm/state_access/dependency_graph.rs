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

use std::collections::{HashMap, HashSet};

/// Builds dependency relationships between state variables
pub struct DependencyGraph {
    /// Map of read variables to their dependent write variables
    edges: HashMap<String, HashSet<String>>,
}

impl DependencyGraph {
    /// Creates empty dependency graph
    pub fn new() -> Self {
        Self { edges: HashMap::new() }
    }

    /// Adds dependencies between write operations and subsequent reads
    /// # Arguments
    /// - `writes`: Variables being written
    /// - `reads`: Variables being read that depend on the writes
    pub fn add_dependencies(&mut self, writes: &[String], reads: &[String]) {
        for write_var in writes {
            for read_var in reads {
                self.edges.entry(read_var.clone()).or_default().insert(write_var.clone());
            }
        }
    }

    /// Generates Graphviz DOT format representation
    ///
    /// # Returns
    /// String containing valid DOT syntax showing read-after-write dependencies
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph G {\n");
        for (source, targets) in &self.edges {
            for target in targets {
                dot.push_str(&format!("  \"{}\" -> \"{}\";\n", source, target));
            }
        }
        dot.push_str("}\n");
        dot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests dependency graph construction:
    /// 1. Add write dependency from 'a' to ['b', 'c']
    /// 2. Verify DOT output contains correct edges:
    ///    - b -> a (read b depends on write a)
    ///    - c -> a (read c depends on write a)
    #[test]
    fn test_dependency_creation() {
        let mut graph = DependencyGraph::new();
        graph.add_dependencies(&["a".into()], &["b".into(), "c".into()]);

        let dot = graph.to_dot();
        assert!(dot.contains("\"b\" -> \"a\""), "Missing b->a dependency edge");
        assert!(dot.contains("\"c\" -> \"a\""), "Missing c->a dependency edge");
    }
}
