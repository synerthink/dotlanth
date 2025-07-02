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

//! Postprocessing stage for optimization and validation

use super::{
    super::{
        config::TranspilationConfig,
        error::{TranspilationError, TranspilationResult},
        types::TranspiledModule,
    },
    PipelineStage,
};

/// Postprocessor stage for final optimizations and validation
pub struct Postprocessor {
    /// Whether to perform optimizations
    enable_optimizations: bool,
    /// Whether to validate output
    validate_output: bool,
}

impl Postprocessor {
    /// Create a new postprocessor
    pub fn new(config: &TranspilationConfig) -> TranspilationResult<Self> {
        Ok(Self {
            enable_optimizations: config.enable_optimizations,
            validate_output: true, // Always validate by default
        })
    }

    /// Optimize the transpiled module
    fn optimize_module(&self, module: &mut TranspiledModule, config: &TranspilationConfig) -> TranspilationResult<()> {
        if !self.enable_optimizations {
            return Ok(());
        }

        // Function-level optimizations
        for function in &mut module.functions {
            self.optimize_function(function, config)?;
        }

        // Module-level optimizations
        self.optimize_function_ordering(module, config)?;
        self.optimize_memory_layout(module, config)?;

        Ok(())
    }

    /// Optimize a single function
    fn optimize_function(&self, function: &mut crate::transpiler::types::TranspiledFunction, config: &TranspilationConfig) -> TranspilationResult<()> {
        match config.effective_optimization_level() {
            super::super::config::OptimizationLevel::O0 => {
                // No optimizations
            }
            super::super::config::OptimizationLevel::O1 => {
                self.apply_basic_optimizations(function)?;
            }
            super::super::config::OptimizationLevel::O2 => {
                self.apply_basic_optimizations(function)?;
                self.apply_standard_optimizations(function)?;
            }
            super::super::config::OptimizationLevel::O3 => {
                self.apply_basic_optimizations(function)?;
                self.apply_standard_optimizations(function)?;
                self.apply_aggressive_optimizations(function)?;
            }
        }

        Ok(())
    }

    /// Apply basic optimizations (O1)
    fn apply_basic_optimizations(&self, function: &mut crate::transpiler::types::TranspiledFunction) -> TranspilationResult<()> {
        // Remove redundant instructions
        self.remove_redundant_instructions(function)?;

        // Basic constant folding
        self.apply_constant_folding(function)?;

        Ok(())
    }

    /// Apply standard optimizations (O2)
    fn apply_standard_optimizations(&self, function: &mut crate::transpiler::types::TranspiledFunction) -> TranspilationResult<()> {
        // Dead code elimination
        self.eliminate_dead_code(function)?;

        // Peephole optimizations
        self.apply_peephole_optimizations(function)?;

        Ok(())
    }

    /// Apply aggressive optimizations (O3)
    fn apply_aggressive_optimizations(&self, function: &mut crate::transpiler::types::TranspiledFunction) -> TranspilationResult<()> {
        // Advanced optimizations would go here
        // For now, this is a placeholder
        Ok(())
    }

    /// Remove redundant instructions
    fn remove_redundant_instructions(&self, function: &mut crate::transpiler::types::TranspiledFunction) -> TranspilationResult<()> {
        let mut optimized_instructions = Vec::new();
        let mut i = 0;

        while i < function.instructions.len() {
            let current = &function.instructions[i];

            // Look for patterns like: load X, store X (redundant store)
            if i + 1 < function.instructions.len() {
                let next = &function.instructions[i + 1];

                // Check for load followed by immediate store to same location
                if self.is_redundant_load_store_pair(current, next) {
                    // Skip the redundant store
                    optimized_instructions.push(current.clone());
                    i += 2; // Skip both instructions for now (simplified)
                    continue;
                }
            }

            optimized_instructions.push(current.clone());
            i += 1;
        }

        function.instructions = optimized_instructions;
        Ok(())
    }

    /// Check if two instructions form a redundant load-store pair
    fn is_redundant_load_store_pair(&self, first: &crate::transpiler::types::TranspiledInstruction, second: &crate::transpiler::types::TranspiledInstruction) -> bool {
        // Simplified check - in practice this would be more sophisticated
        first.opcode.contains("load") && second.opcode.contains("store")
    }

    /// Apply constant folding
    fn apply_constant_folding(&self, function: &mut crate::transpiler::types::TranspiledFunction) -> TranspilationResult<()> {
        let mut optimized_instructions = Vec::new();
        let mut i = 0;

        while i < function.instructions.len() {
            // Look for patterns like: const A, const B, add -> const (A+B)
            if i + 2 < function.instructions.len() {
                if let Some(folded) = self.try_fold_constants(&function.instructions[i..i + 3]) {
                    optimized_instructions.push(folded);
                    i += 3;
                    continue;
                }
            }

            optimized_instructions.push(function.instructions[i].clone());
            i += 1;
        }

        function.instructions = optimized_instructions;
        Ok(())
    }

    /// Try to fold a sequence of instructions into a constant
    fn try_fold_constants(&self, instructions: &[crate::transpiler::types::TranspiledInstruction]) -> Option<crate::transpiler::types::TranspiledInstruction> {
        if instructions.len() < 3 {
            return None;
        }

        // Look for: immediate, immediate, add pattern
        let first = &instructions[0];
        let second = &instructions[1];
        let third = &instructions[2];

        if first.opcode.contains("const") && second.opcode.contains("const") && third.opcode.contains("add") {
            // Extract values and fold (simplified)
            if let (Some(val1), Some(val2)) = (self.extract_immediate_value(first), self.extract_immediate_value(second)) {
                let result = val1.wrapping_add(val2);
                return Some(crate::transpiler::types::TranspiledInstruction::new(
                    "i32.const".to_string(),
                    vec![crate::transpiler::types::Operand::immediate(result)],
                ));
            }
        }

        None
    }

    /// Extract immediate value from a constant instruction
    fn extract_immediate_value(&self, instruction: &crate::transpiler::types::TranspiledInstruction) -> Option<u32> {
        if instruction.operands.len() == 1 {
            if let crate::transpiler::types::Operand::Immediate(value) = &instruction.operands[0] {
                return Some(*value);
            }
        }
        None
    }

    /// Eliminate dead code
    fn eliminate_dead_code(&self, function: &mut crate::transpiler::types::TranspiledFunction) -> TranspilationResult<()> {
        // Mark reachable instructions
        let mut reachable = vec![false; function.instructions.len()];
        let mut worklist = vec![0]; // Start from first instruction

        while let Some(index) = worklist.pop() {
            if index >= reachable.len() || reachable[index] {
                continue;
            }

            reachable[index] = true;

            // Add successors to worklist
            let instruction = &function.instructions[index];

            // Sequential successor
            if index + 1 < function.instructions.len() {
                worklist.push(index + 1);
            }

            // Branch targets (simplified - would need proper control flow analysis)
            if instruction.opcode.contains("br") || instruction.opcode.contains("jump") {
                // Would need to resolve branch targets
            }
        }

        // Remove unreachable instructions
        let mut optimized_instructions = Vec::new();
        for (i, instruction) in function.instructions.iter().enumerate() {
            if reachable[i] {
                optimized_instructions.push(instruction.clone());
            }
        }

        function.instructions = optimized_instructions;
        Ok(())
    }

    /// Apply peephole optimizations
    fn apply_peephole_optimizations(&self, function: &mut crate::transpiler::types::TranspiledFunction) -> TranspilationResult<()> {
        let mut optimized_instructions = Vec::new();
        let mut i = 0;

        while i < function.instructions.len() {
            let current = &function.instructions[i];

            // Look for optimization patterns
            if i + 1 < function.instructions.len() {
                let next = &function.instructions[i + 1];

                // Pattern: push X, pop -> (nothing)
                if current.opcode.contains("push") && next.opcode.contains("pop") {
                    i += 2; // Skip both instructions
                    continue;
                }

                // Pattern: load X, load X -> load X, dup
                if current.opcode == next.opcode && current.opcode.contains("load") {
                    optimized_instructions.push(current.clone());
                    optimized_instructions.push(crate::transpiler::types::TranspiledInstruction::new("dup".to_string(), vec![]));
                    i += 2;
                    continue;
                }
            }

            optimized_instructions.push(current.clone());
            i += 1;
        }

        function.instructions = optimized_instructions;
        Ok(())
    }

    /// Optimize function ordering for better cache locality
    fn optimize_function_ordering(&self, module: &mut TranspiledModule, _config: &TranspilationConfig) -> TranspilationResult<()> {
        // Sort functions by estimated call frequency (hottest first)
        // This is a simplified heuristic - real implementation would use call graph analysis
        module.functions.sort_by(|a, b| {
            let a_score = if a.is_exported { 100 } else { a.instruction_count() };
            let b_score = if b.is_exported { 100 } else { b.instruction_count() };
            b_score.cmp(&a_score) // Descending order
        });

        Ok(())
    }

    /// Optimize memory layout
    fn optimize_memory_layout(&self, module: &mut TranspiledModule, _config: &TranspilationConfig) -> TranspilationResult<()> {
        // Sort globals by size and access frequency
        module.globals.sort_by(|a, b| {
            // Prioritize by size (larger first for alignment)
            b.size_bytes().cmp(&a.size_bytes())
        });

        Ok(())
    }

    /// Validate the transpiled module
    fn validate_module(&self, module: &TranspiledModule, config: &TranspilationConfig) -> TranspilationResult<()> {
        // Validate function count limits
        if let Some(max_size) = config.max_function_size {
            for function in &module.functions {
                if function.instruction_count() > max_size as usize {
                    return Err(TranspilationError::postprocessing_error(
                        "validation",
                        format!("Function '{}' exceeds maximum size: {} > {}", function.name, function.instruction_count(), max_size),
                    ));
                }
            }
        }

        // Validate exports reference valid indices
        for export in &module.exports {
            match export.kind {
                crate::transpiler::types::ExportKind::Function => {
                    if export.index as usize >= module.functions.len() {
                        return Err(TranspilationError::postprocessing_error(
                            "validation",
                            format!("Export '{}' references non-existent function {}", export.name, export.index),
                        ));
                    }
                }
                crate::transpiler::types::ExportKind::Global => {
                    if export.index as usize >= module.globals.len() {
                        return Err(TranspilationError::postprocessing_error(
                            "validation",
                            format!("Export '{}' references non-existent global {}", export.name, export.index),
                        ));
                    }
                }
                _ => {} // Other validations would go here
            }
        }

        // Validate architecture compatibility
        if module.header.architecture != config.target_architecture {
            return Err(TranspilationError::postprocessing_error(
                "validation",
                format!("Module architecture {:?} doesn't match target {:?}", module.header.architecture, config.target_architecture),
            ));
        }

        Ok(())
    }
}

impl PipelineStage for Postprocessor {
    type Input = TranspiledModule;
    type Output = TranspiledModule;

    fn execute(&mut self, mut input: Self::Input, config: &TranspilationConfig) -> TranspilationResult<Self::Output> {
        // Apply optimizations
        self.optimize_module(&mut input, config)?;

        // Validate the result
        if self.validate_output {
            self.validate_module(&input, config)?;
        }

        Ok(input)
    }

    fn name(&self) -> &'static str {
        "postprocessor"
    }

    fn can_skip(&self, config: &TranspilationConfig) -> bool {
        // Can skip optimizations but not validation
        !config.enable_optimizations && !self.validate_output
    }

    fn estimated_duration(&self, input_size: usize) -> std::time::Duration {
        // Postprocessing time depends on optimization level
        let base_time = input_size / 1024; // 1ms per KB base
        std::time::Duration::from_millis(base_time.max(1) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::config::TranspilationConfig;

    #[test]
    fn test_postprocessor_creation() {
        let config = TranspilationConfig::default();
        let postprocessor = Postprocessor::new(&config);
        assert!(postprocessor.is_ok());
    }

    #[test]
    fn test_constant_folding() {
        let config = TranspilationConfig::default();
        let postprocessor = Postprocessor::new(&config).unwrap();

        let instructions = vec![
            crate::transpiler::types::TranspiledInstruction::new("i32.const".to_string(), vec![crate::transpiler::types::Operand::immediate(5)]),
            crate::transpiler::types::TranspiledInstruction::new("i32.const".to_string(), vec![crate::transpiler::types::Operand::immediate(3)]),
            crate::transpiler::types::TranspiledInstruction::new("i32.add".to_string(), vec![]),
        ];

        let folded = postprocessor.try_fold_constants(&instructions);
        assert!(folded.is_some());

        if let Some(instruction) = folded {
            assert_eq!(instruction.opcode, "i32.const");
            if let crate::transpiler::types::Operand::Immediate(value) = &instruction.operands[0] {
                assert_eq!(*value, 8);
            }
        }
    }
}
