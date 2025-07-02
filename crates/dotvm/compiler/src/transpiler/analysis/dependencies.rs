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

//! Function dependency analysis

use crate::wasm::ast::WasmModule;

/// Function dependency analyzer
pub struct DependencyAnalyzer;

impl DependencyAnalyzer {
    /// Create a new dependency analyzer
    pub fn new() -> Self {
        Self
    }

    /// Analyze function dependencies
    pub fn analyze_dependencies(&self, module: &WasmModule) -> DependencyGraph {
        let mut graph = DependencyGraph::new();

        // Build dependency graph from function calls
        for (caller_index, function) in module.functions.iter().enumerate() {
            for instruction in &function.body {
                if let crate::wasm::ast::WasmInstruction::Call { function_index } = instruction {
                    graph.add_dependency(caller_index as u32, *function_index);
                }
            }
        }

        graph
    }
}

/// Function dependency graph
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Dependencies: caller -> list of callees
    dependencies: std::collections::HashMap<u32, Vec<u32>>,
}

impl DependencyGraph {
    /// Create a new dependency graph
    pub fn new() -> Self {
        Self {
            dependencies: std::collections::HashMap::new(),
        }
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, caller: u32, callee: u32) {
        self.dependencies.entry(caller).or_insert_with(Vec::new).push(callee);
    }

    /// Get dependencies for a function
    pub fn get_dependencies(&self, function: u32) -> Option<&Vec<u32>> {
        self.dependencies.get(&function)
    }

    /// Check if a function is recursive
    pub fn is_recursive(&self, function: u32) -> bool {
        if let Some(deps) = self.dependencies.get(&function) { deps.contains(&function) } else { false }
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency(0, 1);
        graph.add_dependency(0, 0); // Recursive call

        assert!(graph.is_recursive(0));
        assert!(!graph.is_recursive(1));
        assert_eq!(graph.get_dependencies(0), Some(&vec![1, 0]));
    }
}
