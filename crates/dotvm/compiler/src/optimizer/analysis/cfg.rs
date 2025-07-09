// Dotlanth
// Copyright (C) 2025 Synerthink

//! Control Flow Graph analysis for optimization

use crate::transpiler::types::instruction::Operand;
use crate::transpiler::types::{TranspiledFunction, TranspiledInstruction};
use std::collections::{HashMap, HashSet, VecDeque};

/// Basic block in the control flow graph
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Block identifier
    pub id: BlockId,
    /// Instructions in this block
    pub instructions: Vec<TranspiledInstruction>,
    /// Predecessor blocks
    pub predecessors: HashSet<BlockId>,
    /// Successor blocks
    pub successors: HashSet<BlockId>,
    /// Whether this block is reachable
    pub reachable: bool,
}

/// Block identifier
pub type BlockId = usize;

/// Control Flow Graph representation
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    /// All basic blocks in the function
    pub blocks: HashMap<BlockId, BasicBlock>,
    /// Entry block ID
    pub entry_block: BlockId,
    /// Exit blocks (blocks that end with return)
    pub exit_blocks: HashSet<BlockId>,
    /// Next available block ID
    next_id: BlockId,
}

impl ControlFlowGraph {
    /// Build a CFG from a transpiled function
    pub fn from_function(function: &TranspiledFunction) -> Self {
        let mut cfg = Self {
            blocks: HashMap::new(),
            entry_block: 0,
            exit_blocks: HashSet::new(),
            next_id: 0,
        };

        cfg.build_blocks(function);
        cfg.compute_edges();
        cfg.mark_reachable_blocks();
        cfg
    }

    /// Build basic blocks from instructions
    fn build_blocks(&mut self, function: &TranspiledFunction) {
        let mut current_block = self.new_block();
        let entry_id = current_block.id;
        self.entry_block = entry_id;

        for instruction in &function.instructions {
            // Check if this instruction starts a new block
            if self.is_block_leader(instruction) && !current_block.instructions.is_empty() {
                self.blocks.insert(current_block.id, current_block);
                current_block = self.new_block();
            }

            current_block.instructions.push(instruction.clone());

            // Check if this instruction ends a block
            if self.is_block_terminator(instruction) {
                if self.is_return_instruction(instruction) {
                    self.exit_blocks.insert(current_block.id);
                }
                self.blocks.insert(current_block.id, current_block);
                current_block = self.new_block();
            }
        }

        // Add the last block if it has instructions
        if !current_block.instructions.is_empty() {
            self.blocks.insert(current_block.id, current_block);
        }
    }

    /// Compute edges between blocks
    fn compute_edges(&mut self) {
        let block_ids: Vec<BlockId> = self.blocks.keys().cloned().collect();

        for &block_id in &block_ids {
            if let Some(block) = self.blocks.get(&block_id) {
                let successors = self.compute_block_successors(block);

                // Update current block's successors
                if let Some(block_mut) = self.blocks.get_mut(&block_id) {
                    block_mut.successors = successors.clone();
                }

                // Update successor blocks' predecessors
                for &successor_id in &successors {
                    if let Some(successor_block) = self.blocks.get_mut(&successor_id) {
                        successor_block.predecessors.insert(block_id);
                    }
                }
            }
        }
    }

    /// Mark reachable blocks starting from entry
    fn mark_reachable_blocks(&mut self) {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(self.entry_block);
        visited.insert(self.entry_block);

        while let Some(block_id) = queue.pop_front() {
            if let Some(block) = self.blocks.get_mut(&block_id) {
                block.reachable = true;

                for &successor_id in &block.successors.clone() {
                    if !visited.contains(&successor_id) {
                        visited.insert(successor_id);
                        queue.push_back(successor_id);
                    }
                }
            }
        }
    }

    /// Create a new basic block
    fn new_block(&mut self) -> BasicBlock {
        let id = self.next_id;
        self.next_id += 1;
        BasicBlock {
            id,
            instructions: Vec::new(),
            predecessors: HashSet::new(),
            successors: HashSet::new(),
            reachable: false,
        }
    }

    /// Check if instruction is a block leader
    fn is_block_leader(&self, instruction: &TranspiledInstruction) -> bool {
        // Simplified - in real implementation, check for jump targets, etc.
        instruction.label.is_some() || instruction.opcode.starts_with("LABEL")
    }

    /// Check if instruction terminates a block
    fn is_block_terminator(&self, instruction: &TranspiledInstruction) -> bool {
        matches!(instruction.opcode.as_str(), "JUMP" | "JUMP_IF" | "BR" | "BR_IF" | "RETURN" | "BR_TABLE")
    }

    /// Check if instruction is a return
    fn is_return_instruction(&self, instruction: &TranspiledInstruction) -> bool {
        instruction.opcode == "RETURN"
    }

    /// Compute successors for a block
    fn compute_block_successors(&self, block: &BasicBlock) -> HashSet<BlockId> {
        let mut successors = HashSet::new();

        if let Some(last_instruction) = block.instructions.last() {
            match last_instruction.opcode.as_str() {
                "JUMP" | "BR" => {
                    if let Some(target) = self.extract_jump_target(last_instruction) {
                        if let Some(target_id) = self.find_block_by_label(&target) {
                            successors.insert(target_id);
                        }
                    }
                }
                "JUMP_IF" | "BR_IF" => {
                    if let Some(target) = self.extract_jump_target(last_instruction) {
                        if let Some(target_id) = self.find_block_by_label(&target) {
                            successors.insert(target_id);
                        }
                    }
                    // Also add fall-through successor
                    if let Some(next_id) = self.find_next_block(block.id) {
                        successors.insert(next_id);
                    }
                }
                "RETURN" => {
                    // No successors for return
                }
                _ => {
                    // Fall-through to next block
                    if let Some(next_id) = self.find_next_block(block.id) {
                        successors.insert(next_id);
                    }
                }
            }
        }

        successors
    }

    /// Find block that contains the given label
    fn find_block_by_label(&self, _label: &str) -> Option<BlockId> {
        // Simplified implementation
        None
    }

    /// Extract jump target from instruction
    fn extract_jump_target(&self, instruction: &TranspiledInstruction) -> Option<String> {
        // Extract target from operands - simplified implementation
        instruction.operands.first().and_then(|op| match op {
            Operand::Label(label) => Some(label.clone()),
            _ => None,
        })
    }

    /// Find the next block in sequence
    fn find_next_block(&self, current_id: BlockId) -> Option<BlockId> {
        // Simplified implementation
        self.blocks.keys().find(|&&id| id == current_id + 1).copied()
    }

    /// Get unreachable blocks
    pub fn unreachable_blocks(&self) -> Vec<BlockId> {
        self.blocks.values().filter(|block| !block.reachable).map(|block| block.id).collect()
    }

    /// Get blocks with no predecessors (other than entry)
    pub fn orphaned_blocks(&self) -> Vec<BlockId> {
        self.blocks
            .values()
            .filter(|block| block.id != self.entry_block && block.predecessors.is_empty())
            .map(|block| block.id)
            .collect()
    }
}
