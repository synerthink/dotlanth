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

//! Loop analysis for optimization

use crate::optimizer::analysis::cfg::{BlockId, ControlFlowGraph};
use crate::optimizer::analysis::dominators::DominatorTree;
use std::collections::{HashMap, HashSet};

/// Information about a loop
#[derive(Debug, Clone)]
pub struct LoopInfo {
    /// Loop header (entry point)
    pub header: BlockId,
    /// All blocks in the loop
    pub blocks: HashSet<BlockId>,
    /// Loop back edges (tail -> header)
    pub back_edges: HashSet<(BlockId, BlockId)>,
    /// Nested loops within this loop
    pub nested_loops: Vec<LoopInfo>,
    /// Loop depth (0 for outermost loops)
    pub depth: usize,
    /// Whether this is a natural loop
    pub is_natural: bool,
}

/// Loop analyzer
pub struct LoopAnalyzer {
    cfg: ControlFlowGraph,
    dominator_tree: DominatorTree,
    loops: Vec<LoopInfo>,
}

impl LoopAnalyzer {
    /// Create a new loop analyzer
    pub fn new(cfg: ControlFlowGraph, dominator_tree: DominatorTree) -> Self {
        Self {
            cfg,
            dominator_tree,
            loops: Vec::new(),
        }
    }

    /// Analyze loops in the function
    pub fn analyze(&mut self) -> &Vec<LoopInfo> {
        self.find_back_edges();
        self.identify_natural_loops();
        self.compute_loop_nesting();
        &self.loops
    }

    /// Find back edges in the CFG
    fn find_back_edges(&mut self) {
        let mut back_edges = HashSet::new();

        for (&block_id, block) in &self.cfg.blocks {
            for &successor_id in &block.successors {
                // A back edge exists if successor dominates current block
                if self.dominates(successor_id, block_id) {
                    back_edges.insert((block_id, successor_id));
                }
            }
        }

        // Create loops from back edges
        for &(tail, header) in &back_edges {
            let mut loop_info = LoopInfo {
                header,
                blocks: HashSet::new(),
                back_edges: HashSet::new(),
                nested_loops: Vec::new(),
                depth: 0,
                is_natural: true,
            };

            loop_info.back_edges.insert((tail, header));
            loop_info.blocks.insert(header);

            // Find all blocks in the natural loop
            self.find_loop_blocks(&mut loop_info, tail);

            self.loops.push(loop_info);
        }
    }

    /// Find all blocks in a natural loop
    fn find_loop_blocks(&self, loop_info: &mut LoopInfo, tail: BlockId) {
        let mut stack = vec![tail];
        let mut visited = HashSet::new();

        loop_info.blocks.insert(tail);
        visited.insert(tail);

        while let Some(current) = stack.pop() {
            if current == loop_info.header {
                continue;
            }

            let block = &self.cfg.blocks[&current];
            for &pred_id in &block.predecessors {
                if !visited.contains(&pred_id) {
                    visited.insert(pred_id);
                    loop_info.blocks.insert(pred_id);
                    stack.push(pred_id);
                }
            }
        }
    }

    /// Identify natural loops and merge loops with same header
    fn identify_natural_loops(&mut self) {
        // Group loops by header
        let mut header_to_loops: HashMap<BlockId, Vec<usize>> = HashMap::new();

        for (i, loop_info) in self.loops.iter().enumerate() {
            header_to_loops.entry(loop_info.header).or_insert_with(Vec::new).push(i);
        }

        // Merge loops with same header
        let mut merged_loops = Vec::new();
        let mut processed = HashSet::new();

        for (&header, loop_indices) in &header_to_loops {
            if loop_indices.len() == 1 {
                let idx = loop_indices[0];
                if !processed.contains(&idx) {
                    merged_loops.push(self.loops[idx].clone());
                    processed.insert(idx);
                }
            } else {
                // Merge multiple loops with same header
                let mut merged_loop = LoopInfo {
                    header,
                    blocks: HashSet::new(),
                    back_edges: HashSet::new(),
                    nested_loops: Vec::new(),
                    depth: 0,
                    is_natural: true,
                };

                for &idx in loop_indices {
                    let loop_info = &self.loops[idx];
                    merged_loop.blocks.extend(&loop_info.blocks);
                    merged_loop.back_edges.extend(&loop_info.back_edges);
                    processed.insert(idx);
                }

                merged_loops.push(merged_loop);
            }
        }

        self.loops = merged_loops;
    }

    /// Compute loop nesting relationships
    fn compute_loop_nesting(&mut self) {
        // Sort loops by size (smaller loops are more deeply nested)
        self.loops.sort_by_key(|loop_info| loop_info.blocks.len());

        // Compute nesting relationships
        for i in 0..self.loops.len() {
            for j in (i + 1)..self.loops.len() {
                let (inner_blocks, outer_blocks) = {
                    let inner = &self.loops[i].blocks;
                    let outer = &self.loops[j].blocks;
                    (inner.clone(), outer.clone())
                };

                // If inner loop is subset of outer loop, it's nested
                if inner_blocks.is_subset(&outer_blocks) {
                    let inner_loop = self.loops[i].clone();
                    self.loops[j].nested_loops.push(inner_loop);
                    self.loops[i].depth = self.loops[j].depth + 1;
                }
            }
        }

        // Remove nested loops from top-level list
        let mut top_level_loops = Vec::new();
        for loop_info in &self.loops {
            if loop_info.depth == 0 {
                top_level_loops.push(loop_info.clone());
            }
        }
        self.loops = top_level_loops;
    }

    /// Check if a block dominates another
    fn dominates(&self, dominator: BlockId, dominated: BlockId) -> bool {
        if dominator == dominated {
            return true;
        }

        let mut current = dominated;
        while let Some(idom) = self.dominator_tree.idom.get(&current).and_then(|&x| x) {
            if idom == dominator {
                return true;
            }
            current = idom;
        }
        false
    }

    /// Get all loops
    pub fn loops(&self) -> &Vec<LoopInfo> {
        &self.loops
    }

    /// Check if a block is in a loop
    pub fn is_in_loop(&self, block_id: BlockId) -> bool {
        self.find_containing_loop(block_id).is_some()
    }

    /// Find the innermost loop containing a block
    pub fn find_containing_loop(&self, block_id: BlockId) -> Option<&LoopInfo> {
        self.find_containing_loop_recursive(&self.loops, block_id)
    }

    /// Recursive helper for finding containing loop
    fn find_containing_loop_recursive<'a>(&self, loops: &'a [LoopInfo], block_id: BlockId) -> Option<&'a LoopInfo> {
        for loop_info in loops {
            if loop_info.blocks.contains(&block_id) {
                // Check nested loops first (more specific)
                if let Some(nested) = self.find_containing_loop_recursive(&loop_info.nested_loops, block_id) {
                    return Some(nested);
                }
                return Some(loop_info);
            }
        }
        None
    }

    /// Get loop depth for a block
    pub fn loop_depth(&self, block_id: BlockId) -> usize {
        if let Some(loop_info) = self.find_containing_loop(block_id) { loop_info.depth + 1 } else { 0 }
    }
}
