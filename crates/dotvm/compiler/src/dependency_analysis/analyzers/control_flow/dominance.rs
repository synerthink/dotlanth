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

//! Dominance analysis (builds dominator tree)

use super::{ControlFlowGraph, ControlFlowNode};
use std::collections::{HashMap, HashSet};

/// Computes immediate dominators for each node in the CFG
pub struct DominanceAnalyzer;

impl DominanceAnalyzer {
    /// Returns a map from node ID to its immediate dominator
    pub fn compute_idoms(cfg: &ControlFlowGraph) -> HashMap<usize, usize> {
        let mut idom: HashMap<usize, usize> = HashMap::new();
        let nodes: Vec<usize> = cfg.nodes.keys().copied().collect();
        if nodes.is_empty() {
            return idom;
        }
        let entry = cfg.entry_node;
        // Initialize dom sets: dom(entry) = {entry}, others = all nodes
        let mut doms: HashMap<usize, HashSet<usize>> = nodes.iter().map(|&n| if n == entry { (n, [entry].into()) } else { (n, nodes.iter().copied().collect()) }).collect();

        let mut changed = true;
        while changed {
            changed = false;
            for &n in &nodes {
                if n == entry {
                    continue;
                }
                // intersect preds' dom sets
                let preds: Vec<usize> = cfg.edges.iter().filter_map(|e| if e.to == n { Some(e.from) } else { None }).collect();
                if preds.is_empty() {
                    continue;
                }
                let mut new_dom = preds
                    .iter()
                    .map(|&p| doms.get(&p).unwrap().clone())
                    .fold(None, |acc: Option<HashSet<usize>>, s| Some(if let Some(a) = acc { &a & &s } else { s }))
                    .unwrap();
                new_dom.insert(n);
                if new_dom != doms[&n] {
                    doms.insert(n, new_dom);
                    changed = true;
                }
            }
        }

        // extract idoms: for each n != entry, idom(n) = immediate dominator
        for &n in &nodes {
            if n == entry {
                continue;
            }
            let ds = &doms[&n];
            // pick maximal dom in ds except n
            let idominator = ds.iter().filter(|&&d| d != n).copied().max().unwrap();
            idom.insert(n, idominator);
        }

        idom
    }
}
