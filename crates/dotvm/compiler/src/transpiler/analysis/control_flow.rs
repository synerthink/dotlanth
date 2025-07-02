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

//! Control flow analysis implementation

use super::super::error::{TranspilationError, TranspilationResult};
use crate::wasm::ast::WasmInstruction;

/// Control flow analyzer for WASM functions
pub struct ControlFlowAnalyzer;

impl ControlFlowAnalyzer {
    /// Create a new control flow analyzer
    pub fn new() -> Self {
        Self
    }

    /// Analyze control flow in a sequence of instructions
    pub fn analyze(&self, instructions: &[WasmInstruction]) -> TranspilationResult<ControlFlowInfo> {
        let mut info = ControlFlowInfo::new();

        // Simplified analysis - real implementation would build a proper CFG
        let mut depth: u32 = 0;
        for (index, instruction) in instructions.iter().enumerate() {
            match instruction {
                WasmInstruction::Block { .. } | WasmInstruction::Loop { .. } | WasmInstruction::If { .. } => {
                    depth += 1;
                    if depth > 3 {
                        info.mark_complex();
                    }
                }
                WasmInstruction::End => {
                    depth = depth.saturating_sub(1);
                }
                WasmInstruction::Br { .. } | WasmInstruction::BrIf { .. } => {
                    info.add_branch_target(index);
                }
                _ => {}
            }
        }

        Ok(info)
    }
}

/// Control flow analysis results
#[derive(Debug, Clone)]
pub struct ControlFlowInfo {
    /// Whether the control flow is complex
    is_complex: bool,
    /// Branch targets
    branch_targets: Vec<usize>,
    /// Maximum nesting depth
    max_depth: u32,
}

impl ControlFlowInfo {
    /// Create new control flow info
    pub fn new() -> Self {
        Self {
            is_complex: false,
            branch_targets: Vec::new(),
            max_depth: 0,
        }
    }

    /// Mark as having complex control flow
    pub fn mark_complex(&mut self) {
        self.is_complex = true;
    }

    /// Add a branch target
    pub fn add_branch_target(&mut self, target: usize) {
        self.branch_targets.push(target);
    }

    /// Check if control flow is complex
    pub fn is_complex(&self) -> bool {
        self.is_complex
    }

    /// Get label for instruction if it's a branch target
    pub fn get_label_for_instruction(&self, index: usize) -> Option<String> {
        if self.branch_targets.contains(&index) { Some(format!("label_{}", index)) } else { None }
    }
}

impl Default for ControlFlowInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_flow_analyzer() {
        let analyzer = ControlFlowAnalyzer::new();
        let instructions = vec![WasmInstruction::Block { block_type: None }, WasmInstruction::I32Const { value: 1 }, WasmInstruction::End];

        let result = analyzer.analyze(&instructions);
        assert!(result.is_ok());

        let info = result.unwrap();
        assert!(!info.is_complex());
    }
}
