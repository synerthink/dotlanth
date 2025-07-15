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

//! Dominator analysis for optimization

use crate::optimizer::analysis::cfg::{BlockId, ControlFlowGraph};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Dominator tree representation
#[derive(Debug, Clone)]
pub struct DominatorTree {
    /// Immediate dominator for each block
    pub idom: HashMap<BlockId, Option<BlockId>>,
    /// Dominance frontier for each block
    pub dominance_frontier: HashMap<BlockId, HashSet<BlockId>>,
    /// Children in the dominator tree
    pub children: HashMap<BlockId, HashSet<BlockId>>,
}

/// Dominator analyzer
pub struct DominatorAnalyzer {
    cfg: ControlFlowGraph,
    tree: DominatorTree,
}

impl DominatorAnalyzer {
    /// Create a new dominator analyzer
    pub fn new(cfg: ControlFlowGraph) -> Self {
        Self {
            cfg,
            tree: DominatorTree {
                idom: HashMap::new(),
                dominance_frontier: HashMap::new(),
                children: HashMap::new(),
            },
        }
    }

    /// Compute dominator tree using Lengauer-Tarjan algorithm
    pub fn analyze(&mut self) -> &DominatorTree {
        self.compute_dominators();
        self.compute_dominance_frontier();
        self.build_dominator_tree();
        &self.tree
    }

    /// Compute immediate dominators
    fn compute_dominators(&mut self) {
        let blocks: Vec<BlockId> = self.cfg.blocks.keys().cloned().collect();
        let mut dominators: HashMap<BlockId, BTreeSet<BlockId>> = HashMap::new();

        // Initialize: entry block dominates only itself
        for &block_id in &blocks {
            if block_id == self.cfg.entry_block {
                let mut dom_set = BTreeSet::new();
                dom_set.insert(block_id);
                dominators.insert(block_id, dom_set);
            } else {
                // All other blocks initially dominated by all blocks
                dominators.insert(block_id, blocks.iter().cloned().collect());
            }
        }

        // Iterative algorithm
        let mut changed = true;
        while changed {
            changed = false;

            for &block_id in &blocks {
                if block_id == self.cfg.entry_block {
                    continue;
                }

                let block = &self.cfg.blocks[&block_id];
                let mut new_dominators = BTreeSet::new();
                new_dominators.insert(block_id); // Block dominates itself

                // Intersection of dominators of all predecessors
                let mut first = true;
                for &pred_id in &block.predecessors {
                    if first {
                        new_dominators.extend(dominators[&pred_id].iter().cloned());
                        first = false;
                    } else {
                        let pred_dominators = &dominators[&pred_id];
                        new_dominators = new_dominators.intersection(pred_dominators).cloned().collect();
                        new_dominators.insert(block_id); // Always include self
                    }
                }

                if new_dominators != dominators[&block_id] {
                    dominators.insert(block_id, new_dominators);
                    changed = true;
                }
            }
        }

        // Compute immediate dominators
        for &block_id in &blocks {
            if block_id == self.cfg.entry_block {
                self.tree.idom.insert(block_id, None);
                continue;
            }

            let block_dominators = &dominators[&block_id];
            let mut candidates: BTreeSet<BlockId> = block_dominators.clone();
            candidates.remove(&block_id); // Remove self

            // Find immediate dominator (dominator that is not dominated by any other dominator)
            let mut idom = None;
            for &candidate in &candidates {
                let mut is_immediate = true;
                for &other in &candidates {
                    if other != candidate && dominators[&other].contains(&candidate) {
                        is_immediate = false;
                        break;
                    }
                }
                if is_immediate {
                    idom = Some(candidate);
                    break;
                }
            }

            self.tree.idom.insert(block_id, idom);
        }
    }

    /// Compute dominance frontier
    fn compute_dominance_frontier(&mut self) {
        for &block_id in self.cfg.blocks.keys() {
            self.tree.dominance_frontier.insert(block_id, HashSet::new());
        }

        for (&block_id, block) in &self.cfg.blocks {
            if block.predecessors.len() >= 2 {
                for &pred_id in &block.predecessors {
                    let mut runner = pred_id;

                    // Walk up dominator tree until we reach block's immediate dominator
                    while Some(runner) != self.tree.idom[&block_id] {
                        self.tree.dominance_frontier.get_mut(&runner).unwrap().insert(block_id);

                        if let Some(idom) = self.tree.idom[&runner] {
                            runner = idom;
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Build dominator tree structure
    fn build_dominator_tree(&mut self) {
        for &block_id in self.cfg.blocks.keys() {
            self.tree.children.insert(block_id, HashSet::new());
        }

        for (&child, &parent_opt) in &self.tree.idom {
            if let Some(parent) = parent_opt {
                self.tree.children.get_mut(&parent).unwrap().insert(child);
            }
        }
    }

    /// Check if block_a dominates block_b
    pub fn dominates(&self, block_a: BlockId, block_b: BlockId) -> bool {
        if block_a == block_b {
            return true;
        }

        let mut current = block_b;
        while let Some(idom) = self.tree.idom[&current] {
            if idom == block_a {
                return true;
            }
            current = idom;
        }
        false
    }

    /// Get dominator tree
    pub fn tree(&self) -> &DominatorTree {
        &self.tree
    }
}
