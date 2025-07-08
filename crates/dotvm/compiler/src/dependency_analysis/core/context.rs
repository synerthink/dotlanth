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

//! Analysis context management

use std::collections::HashMap;

/// Shared analysis context for depth tracking and metadata
#[derive(Debug, Clone)]
pub struct AnalysisContext {
    /// Current depth
    pub depth: usize,
    /// Maximum allowed depth
    pub max_depth: usize,
    /// Visited nodes for cycle detection
    pub visited: std::collections::HashSet<String>,
    /// Arbitrary metadata
    pub metadata: HashMap<String, String>,
}

impl AnalysisContext {
    /// Create a new context with the given max depth
    pub fn new(max_depth: usize) -> Self {
        Self {
            depth: 0,
            max_depth,
            visited: Default::default(),
            metadata: Default::default(),
        }
    }

    /// Enter a new node in the analysis
    pub fn enter(&mut self, node: String) -> Result<(), String> {
        if self.depth >= self.max_depth {
            return Err(format!("Depth limit {} reached", self.max_depth));
        }
        if !self.visited.insert(node.clone()) {
            return Err(format!("Circular dependency on {}", node));
        }
        self.depth += 1;
        Ok(())
    }

    /// Exit a node
    pub fn exit(&mut self, node: &str) {
        self.depth = self.depth.saturating_sub(1);
        self.visited.remove(node);
    }

    /// Add metadata key/value
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Retrieve metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}
