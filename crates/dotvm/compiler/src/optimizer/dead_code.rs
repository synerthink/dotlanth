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

//! Dead code elimination optimization pass
//!
//! This module implements dead code elimination that removes unreachable
//! code and unused variables/functions.

use crate::transpiler::engine::{TranspiledFunction, TranspiledInstruction};
use dotvm_core::opcode::control_flow_opcodes::ControlFlowOpcode;
use std::collections::{HashMap, HashSet};

/// Dead code eliminator for DotVM bytecode
pub struct DeadCodeEliminator {
    /// Statistics about eliminated code
    stats: EliminationStats,
}

impl DeadCodeEliminator {
    /// Create a new dead code eliminator
    pub fn new() -> Self {
        Self { stats: EliminationStats::default() }
    }

    /// Eliminate dead code from a list of functions
    pub fn eliminate(&mut self, functions: Vec<TranspiledFunction>) -> Vec<TranspiledFunction> {
        // First pass: eliminate dead functions
        let live_functions = self.eliminate_dead_functions(functions);

        // Second pass: eliminate dead code within functions
        live_functions.into_iter().map(|func| self.eliminate_dead_code_in_function(func)).collect()
    }

    /// Eliminate entire functions that are never called
    fn eliminate_dead_functions(&mut self, functions: Vec<TranspiledFunction>) -> Vec<TranspiledFunction> {
        let mut live_functions = HashSet::new();
        let function_map: HashMap<String, &TranspiledFunction> = functions.iter().map(|f| (f.name.clone(), f)).collect();

        // Start with entry points (functions that might be called externally)
        for function in &functions {
            if self.is_entry_point(&function.name) {
                live_functions.insert(function.name.clone());
            }
        }

        // Iteratively find all reachable functions
        let mut changed = true;
        while changed {
            changed = false;
            let current_live: Vec<String> = live_functions.iter().cloned().collect();

            for func_name in &current_live {
                if let Some(function) = function_map.get(func_name) {
                    for called_func in self.find_called_functions(function) {
                        if live_functions.insert(called_func) {
                            changed = true;
                        }
                    }
                }
            }
        }

        let original_count = functions.len();
        let live_functions_vec: Vec<TranspiledFunction> = functions.into_iter().filter(|f| live_functions.contains(&f.name)).collect();

        self.stats.dead_functions_eliminated = original_count - live_functions_vec.len();
        live_functions_vec
    }

    /// Eliminate dead code within a single function
    fn eliminate_dead_code_in_function(&mut self, mut function: TranspiledFunction) -> TranspiledFunction {
        let original_instruction_count = function.instructions.len();

        // Build control flow graph
        let cfg = self.build_control_flow_graph(&function.instructions);

        // Find reachable instructions
        let reachable = self.find_reachable_instructions(&cfg);

        // Remove unreachable instructions
        function.instructions = function.instructions.into_iter().enumerate().filter(|(i, _)| reachable.contains(i)).map(|(_, inst)| inst).collect();

        // Eliminate unused local variables
        function = self.eliminate_unused_locals(function);

        let eliminated_instructions = original_instruction_count - function.instructions.len();
        self.stats.dead_instructions_eliminated += eliminated_instructions;

        if eliminated_instructions > 0 {
            self.stats.functions_with_dead_code += 1;
        }

        function
    }

    /// Check if a function is an entry point (exported or main)
    fn is_entry_point(&self, function_name: &str) -> bool {
        // Consider functions as entry points if they:
        // 1. Are named "main" or "_start"
        // 2. Are exported (would need export information from Wasm)
        // 3. Are marked with special attributes

        matches!(function_name, "main" | "_start") || 
        function_name.starts_with("export_") ||
        function_name.starts_with("__wasm_") ||
        // Be conservative with function names that look like they might be generated
        function_name.starts_with("func_") // Generated function names from transpiler
    }

    /// Find all functions called by a given function
    fn find_called_functions(&self, function: &TranspiledFunction) -> Vec<String> {
        let mut called_functions = Vec::new();

        for instruction in &function.instructions {
            // Look for function call instructions
            // This would need to be adapted based on the actual instruction format
            if let Some(called_func) = self.extract_function_call(instruction) {
                called_functions.push(called_func);
            }
        }

        called_functions
    }

    /// Extract function name from a call instruction
    fn extract_function_call(&self, instruction: &TranspiledInstruction) -> Option<String> {
        // This is a placeholder - would need to match actual call instruction format
        // For example, if calls are represented as specific opcodes with function names
        None
    }

    /// Build a control flow graph for the function
    fn build_control_flow_graph(&self, instructions: &[TranspiledInstruction]) -> ControlFlowGraph {
        let mut cfg = ControlFlowGraph::new(instructions.len());

        for (i, instruction) in instructions.iter().enumerate() {
            // Add edge to next instruction (fall-through)
            if i + 1 < instructions.len() {
                cfg.add_edge(i, i + 1);
            }

            // Add edges for control flow instructions
            if let Some(targets) = self.get_jump_targets(instruction, i) {
                for target in targets {
                    if target < instructions.len() {
                        cfg.add_edge(i, target);
                    }
                }
            }
        }

        cfg
    }

    /// Get jump targets for control flow instructions
    fn get_jump_targets(&self, instruction: &TranspiledInstruction, current_index: usize) -> Option<Vec<usize>> {
        // This would need to be implemented based on the actual instruction format
        // For now, return None as placeholder
        None
    }

    /// Find all reachable instructions using DFS
    fn find_reachable_instructions(&self, cfg: &ControlFlowGraph) -> HashSet<usize> {
        let mut reachable = HashSet::new();
        let mut stack = vec![0]; // Start from first instruction

        while let Some(node) = stack.pop() {
            if reachable.insert(node) {
                // First time visiting this node
                for &successor in &cfg.edges[node] {
                    stack.push(successor);
                }
            }
        }

        reachable
    }

    /// Eliminate unused local variables
    fn eliminate_unused_locals(&mut self, mut function: TranspiledFunction) -> TranspiledFunction {
        let used_locals = self.find_used_locals(&function.instructions);
        let original_local_count = function.local_count;

        // Count how many locals are actually used
        let max_used_local = used_locals.iter().max().copied().unwrap_or(0);
        function.local_count = (max_used_local + 1).min(function.local_count);

        self.stats.dead_locals_eliminated += original_local_count - function.local_count;

        function
    }

    /// Find all local variables that are used
    fn find_used_locals(&self, instructions: &[TranspiledInstruction]) -> HashSet<usize> {
        let mut used_locals = HashSet::new();

        for instruction in instructions {
            // Extract local variable references from instruction
            if let Some(locals) = self.extract_local_references(instruction) {
                used_locals.extend(locals);
            }
        }

        used_locals
    }

    /// Extract local variable references from an instruction
    fn extract_local_references(&self, instruction: &TranspiledInstruction) -> Option<Vec<usize>> {
        // This would need to be implemented based on the actual instruction format
        // For now, return None as placeholder
        None
    }

    /// Get elimination statistics
    pub fn stats(&self) -> &EliminationStats {
        &self.stats
    }

    /// Reset elimination statistics
    pub fn reset_stats(&mut self) {
        self.stats = EliminationStats::default();
    }
}

impl Default for DeadCodeEliminator {
    fn default() -> Self {
        Self::new()
    }
}

/// Control flow graph representation
#[derive(Debug)]
struct ControlFlowGraph {
    /// Adjacency list representation
    edges: Vec<Vec<usize>>,
}

impl ControlFlowGraph {
    fn new(node_count: usize) -> Self {
        Self { edges: vec![Vec::new(); node_count] }
    }

    fn add_edge(&mut self, from: usize, to: usize) {
        if from < self.edges.len() {
            self.edges[from].push(to);
        }
    }
}

/// Dead code elimination statistics
#[derive(Debug, Default, Clone, Copy)]
pub struct EliminationStats {
    pub dead_functions_eliminated: usize,
    pub dead_instructions_eliminated: usize,
    pub dead_locals_eliminated: usize,
    pub functions_with_dead_code: usize,
}

impl EliminationStats {
    /// Calculate the total amount of dead code eliminated
    pub fn total_eliminated(&self) -> usize {
        self.dead_functions_eliminated + self.dead_instructions_eliminated + self.dead_locals_eliminated
    }

    /// Calculate elimination ratio for instructions
    pub fn instruction_elimination_ratio(&self, original_instructions: usize) -> f64 {
        if original_instructions == 0 {
            0.0
        } else {
            self.dead_instructions_eliminated as f64 / original_instructions as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dead_code_eliminator_creation() {
        let eliminator = DeadCodeEliminator::new();
        assert_eq!(eliminator.stats.dead_functions_eliminated, 0);
    }

    #[test]
    fn test_entry_point_detection() {
        let eliminator = DeadCodeEliminator::new();
        assert!(eliminator.is_entry_point("main"));
        assert!(eliminator.is_entry_point("_start"));
        assert!(eliminator.is_entry_point("export_test"));
        assert!(!eliminator.is_entry_point("internal_function"));
    }

    #[test]
    fn test_control_flow_graph() {
        let mut cfg = ControlFlowGraph::new(3);
        cfg.add_edge(0, 1);
        cfg.add_edge(1, 2);
        cfg.add_edge(0, 2);

        assert_eq!(cfg.edges[0], vec![1, 2]);
        assert_eq!(cfg.edges[1], vec![2]);
        assert_eq!(cfg.edges[2], Vec::<usize>::new());
    }

    #[test]
    fn test_elimination_stats() {
        let mut stats = EliminationStats::default();
        stats.dead_functions_eliminated = 2;
        stats.dead_instructions_eliminated = 10;
        stats.dead_locals_eliminated = 5;

        assert_eq!(stats.total_eliminated(), 17);
        assert_eq!(stats.instruction_elimination_ratio(100), 0.1);
    }

    #[test]
    fn test_empty_function_list() {
        let mut eliminator = DeadCodeEliminator::new();
        let result = eliminator.eliminate(vec![]);
        assert!(result.is_empty());
        assert_eq!(eliminator.stats.dead_functions_eliminated, 0);
    }
}
