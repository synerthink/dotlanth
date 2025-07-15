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

//! Definition-Use analysis for optimization

use crate::optimizer::analysis::cfg::{BlockId, ControlFlowGraph};
use crate::transpiler::types::instruction::Operand;
use crate::transpiler::types::{TranspiledFunction, TranspiledInstruction};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Variable identifier
pub type VariableId = usize;

/// Definition site information
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Definition {
    /// Block where the definition occurs
    pub block_id: BlockId,
    /// Instruction index within the block
    pub instruction_index: usize,
    /// Variable being defined
    pub variable: VariableId,
}

/// Use site information
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Use {
    /// Block where the use occurs
    pub block_id: BlockId,
    /// Instruction index within the block
    pub instruction_index: usize,
    /// Variable being used
    pub variable: VariableId,
}

/// Definition-Use chains
#[derive(Debug, Clone)]
pub struct DefUseChains {
    /// Map from definitions to their uses
    pub def_to_uses: HashMap<Definition, HashSet<Use>>,
    /// Map from uses to their reaching definitions
    pub use_to_defs: HashMap<Use, HashSet<Definition>>,
    /// All definitions in the function
    pub all_definitions: HashSet<Definition>,
    /// All uses in the function
    pub all_uses: HashSet<Use>,
}

/// Def-Use analyzer
pub struct DefUseAnalyzer {
    cfg: ControlFlowGraph,
    chains: DefUseChains,
}

impl DefUseAnalyzer {
    /// Create a new def-use analyzer
    pub fn new(cfg: ControlFlowGraph) -> Self {
        Self {
            cfg,
            chains: DefUseChains {
                def_to_uses: HashMap::new(),
                use_to_defs: HashMap::new(),
                all_definitions: HashSet::new(),
                all_uses: HashSet::new(),
            },
        }
    }

    /// Analyze a function and build def-use chains
    pub fn analyze(&mut self, function: &TranspiledFunction) -> &DefUseChains {
        self.collect_definitions_and_uses();
        self.compute_reaching_definitions();
        self.build_def_use_chains();
        &self.chains
    }

    /// Collect all definitions and uses in the function
    fn collect_definitions_and_uses(&mut self) {
        for (block_id, block) in &self.cfg.blocks {
            for (inst_index, instruction) in block.instructions.iter().enumerate() {
                // Collect definitions
                if let Some(defined_var) = self.extract_definition(instruction) {
                    let def = Definition {
                        block_id: *block_id,
                        instruction_index: inst_index,
                        variable: defined_var,
                    };
                    self.chains.all_definitions.insert(def);
                }

                // Collect uses
                for used_var in self.extract_uses(instruction) {
                    let use_site = Use {
                        block_id: *block_id,
                        instruction_index: inst_index,
                        variable: used_var,
                    };
                    self.chains.all_uses.insert(use_site);
                }
            }
        }
    }

    /// Compute reaching definitions using dataflow analysis
    fn compute_reaching_definitions(&mut self) {
        let mut reaching_in: HashMap<BlockId, BTreeSet<Definition>> = HashMap::new();
        let mut reaching_out: HashMap<BlockId, BTreeSet<Definition>> = HashMap::new();
        let mut changed = true;

        // Initialize
        for &block_id in self.cfg.blocks.keys() {
            reaching_in.insert(block_id, BTreeSet::new());
            reaching_out.insert(block_id, BTreeSet::new());
        }

        // Iterative dataflow analysis
        while changed {
            changed = false;

            for (&block_id, block) in &self.cfg.blocks {
                // Compute reaching_in[block] = union of reaching_out[pred] for all predecessors
                let mut new_reaching_in = BTreeSet::new();
                for &pred_id in &block.predecessors {
                    if let Some(pred_out) = reaching_out.get(&pred_id) {
                        new_reaching_in.extend(pred_out.iter().cloned());
                    }
                }

                if new_reaching_in != *reaching_in.get(&block_id).unwrap() {
                    reaching_in.insert(block_id, new_reaching_in.clone());
                    changed = true;
                }

                // Compute reaching_out[block] = (reaching_in[block] - killed) + generated
                let mut new_reaching_out = new_reaching_in;

                // Remove killed definitions and add generated definitions
                for (inst_index, instruction) in block.instructions.iter().enumerate() {
                    if let Some(defined_var) = self.extract_definition(instruction) {
                        // Kill previous definitions of the same variable
                        new_reaching_out.retain(|def| def.variable != defined_var);

                        // Add new definition
                        let new_def = Definition {
                            block_id,
                            instruction_index: inst_index,
                            variable: defined_var,
                        };
                        new_reaching_out.insert(new_def);
                    }
                }

                if new_reaching_out != *reaching_out.get(&block_id).unwrap() {
                    reaching_out.insert(block_id, new_reaching_out);
                    changed = true;
                }
            }
        }

        // Store reaching definitions for use in building chains
        self.store_reaching_definitions(reaching_in, reaching_out);
    }

    /// Build def-use chains from reaching definitions
    fn build_def_use_chains(&mut self) {
        for use_site in &self.chains.all_uses.clone() {
            let reaching_defs = self.get_reaching_definitions_at_use(use_site);

            for def in &reaching_defs {
                // Add use to definition's use set
                self.chains.def_to_uses.entry(def.clone()).or_insert_with(HashSet::new).insert(use_site.clone());
            }

            // Add reaching definitions to use's definition set
            self.chains.use_to_defs.insert(use_site.clone(), reaching_defs);
        }
    }

    /// Extract the variable being defined by an instruction
    fn extract_definition(&self, instruction: &TranspiledInstruction) -> Option<VariableId> {
        match instruction.opcode.as_str() {
            "SET_LOCAL" | "TEE_LOCAL" => {
                // Extract variable ID from first operand
                instruction.operands.first().and_then(|op| match op {
                    Operand::Stack { offset } => Some((*offset).abs() as VariableId),
                    _ => None,
                })
            }
            "SET_GLOBAL" => instruction.operands.first().and_then(|op| match op {
                Operand::Global { index } => Some(*index as VariableId),
                _ => None,
            }),
            _ => None,
        }
    }

    /// Extract variables being used by an instruction
    fn extract_uses(&self, instruction: &TranspiledInstruction) -> Vec<VariableId> {
        let mut uses = Vec::new();

        match instruction.opcode.as_str() {
            "GET_LOCAL" => {
                // Local variables are typically accessed via stack operations
                // For now, use a simplified approach
                if let Some(Operand::Stack { offset }) = instruction.operands.first() {
                    uses.push((*offset).abs() as VariableId);
                }
            }
            "GET_GLOBAL" => {
                if let Some(Operand::Global { index }) = instruction.operands.first() {
                    uses.push(*index as VariableId);
                }
            }
            "ADD" | "SUB" | "MUL" | "DIV" => {
                // Extract variables from all operands
                for operand in &instruction.operands {
                    if let Some(var_id) = self.extract_variable_from_operand(operand) {
                        uses.push(var_id);
                    }
                }
            }
            _ => {
                // For other instructions, check all operands for variable references
                for operand in &instruction.operands {
                    if let Some(var_id) = self.extract_variable_from_operand(operand) {
                        uses.push(var_id);
                    }
                }
            }
        }

        uses
    }

    /// Extract variable ID from operand
    fn extract_variable_from_operand(&self, operand: &crate::transpiler::types::instruction::Operand) -> Option<VariableId> {
        match operand {
            Operand::Global { index } => Some(*index as VariableId),
            Operand::Stack { offset } => Some((*offset).abs() as VariableId),
            _ => None,
        }
    }

    /// Store reaching definitions (simplified implementation)
    fn store_reaching_definitions(&mut self, _reaching_in: HashMap<BlockId, BTreeSet<Definition>>, _reaching_out: HashMap<BlockId, BTreeSet<Definition>>) {
        // Store for later use in building chains
    }

    /// Get reaching definitions at a use site
    fn get_reaching_definitions_at_use(&self, use_site: &Use) -> HashSet<Definition> {
        // Simplified implementation - in practice, would use stored reaching definitions
        self.chains.all_definitions.iter().filter(|def| def.variable == use_site.variable).cloned().collect()
    }

    /// Get the def-use chains
    pub fn chains(&self) -> &DefUseChains {
        &self.chains
    }

    /// Check if a variable is live at a given point
    pub fn is_variable_live(&self, variable: VariableId, block_id: BlockId, instruction_index: usize) -> bool {
        // Check if there are any uses of this variable that can be reached from this point
        self.chains
            .all_uses
            .iter()
            .any(|use_site| use_site.variable == variable && self.can_reach(block_id, instruction_index, use_site.block_id, use_site.instruction_index))
    }

    /// Check if one program point can reach another
    fn can_reach(&self, from_block: BlockId, from_inst: usize, to_block: BlockId, to_inst: usize) -> bool {
        // Simplified implementation - would use CFG traversal
        from_block == to_block && from_inst < to_inst
    }
}
