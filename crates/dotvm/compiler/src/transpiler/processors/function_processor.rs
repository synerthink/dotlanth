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

//! Function-level processing for transpilation

use super::{
    super::{
        config::TranspilationConfig,
        error::{TranspilationError, TranspilationResult},
        pipeline::analyzer::FunctionAnalysis,
        types::{FunctionMetadata, TranspiledFunction},
    },
    InstructionProcessor,
};
use crate::wasm::ast::WasmFunction;

/// Processor for converting WASM functions to DotVM functions
pub struct FunctionProcessor {
    /// Instruction processor
    instruction_processor: InstructionProcessor,
}

impl FunctionProcessor {
    /// Create a new function processor
    pub fn new(config: &TranspilationConfig) -> TranspilationResult<Self> {
        Ok(Self {
            instruction_processor: InstructionProcessor::new(config)?,
        })
    }

    /// Process multiple functions
    pub fn process_functions(&mut self, wasm_functions: &[WasmFunction], function_analyses: &[FunctionAnalysis], config: &TranspilationConfig) -> TranspilationResult<Vec<TranspiledFunction>> {
        let mut transpiled_functions = Vec::new();

        for (index, wasm_function) in wasm_functions.iter().enumerate() {
            let analysis = function_analyses.get(index);
            let transpiled = self.process_function(index as u32, wasm_function, analysis, config)?;
            transpiled_functions.push(transpiled);
        }

        Ok(transpiled_functions)
    }

    /// Process a single WASM function
    pub fn process_function(&mut self, index: u32, wasm_function: &WasmFunction, analysis: Option<&FunctionAnalysis>, config: &TranspilationConfig) -> TranspilationResult<TranspiledFunction> {
        // Create basic function structure
        let param_count = wasm_function.signature.params.len();
        let local_count = param_count + wasm_function.locals.len();
        let function_name = self.generate_function_name(index, wasm_function);

        let mut transpiled_function = TranspiledFunction::new(function_name, param_count, local_count);

        // Process instructions
        let instructions = self.instruction_processor.process_instructions(&wasm_function.body, config)?;

        for instruction in instructions {
            transpiled_function.add_instruction(instruction);
        }

        // Set debug information if enabled
        if config.preserve_debug_info {
            transpiled_function.set_debug_info(format!("wasm_function_{}", index));
        }

        // Apply analysis results if available
        if let Some(analysis) = analysis {
            transpiled_function.metadata = self.create_metadata_from_analysis(analysis);
        }

        // Validate function size limits
        if let Some(max_size) = config.max_function_size {
            if transpiled_function.instruction_count() > max_size as usize {
                return Err(TranspilationError::FunctionTooLarge {
                    function: index,
                    size: transpiled_function.instruction_count() as u32,
                    max_size,
                });
            }
        }

        Ok(transpiled_function)
    }

    /// Generate a function name
    fn generate_function_name(&self, index: u32, wasm_function: &WasmFunction) -> String {
        // Try to use a meaningful name if available, otherwise use index
        // In WASM, function names are typically in the name section (not implemented here)
        format!("func_{}", index)
    }

    /// Create function metadata from analysis results
    fn create_metadata_from_analysis(&self, analysis: &FunctionAnalysis) -> FunctionMetadata {
        let mut metadata = FunctionMetadata::new();

        metadata.set_complexity_score(analysis.complexity_score);
        metadata.set_max_stack_depth(analysis.max_stack_depth);

        if analysis.has_complex_control_flow {
            metadata.mark_complex_control_flow();
        }

        if analysis.is_recursive {
            metadata.mark_recursive();
        }

        // Add function calls
        for &call_index in &analysis.function_calls {
            metadata.add_function_call(call_index);
        }

        // Convert memory access patterns
        for access in &analysis.memory_accesses {
            let pattern = super::super::types::MemoryAccessPattern::new(
                access.instruction_index as u64, // Use instruction index as offset for now
                access.size,
                access.access_type == super::super::pipeline::analyzer::MemoryAccessType::Store,
            )
            .with_alignment(if access.is_aligned { access.size } else { 1 });

            metadata.add_memory_access(pattern);
        }

        metadata
    }

    /// Validate function structure
    fn validate_function(&self, function: &TranspiledFunction, config: &TranspilationConfig) -> TranspilationResult<()> {
        // Check parameter count limits
        if function.param_count > 100 {
            return Err(TranspilationError::InvalidFunctionSignature {
                function: 0, // Would need to pass function index
                details: format!("Too many parameters: {}", function.param_count),
            });
        }

        // Check local count limits
        if function.local_count > 1000 {
            return Err(TranspilationError::InvalidFunctionSignature {
                function: 0,
                details: format!("Too many locals: {}", function.local_count),
            });
        }

        // Check instruction count
        if function.instructions.is_empty() {
            return Err(TranspilationError::InvalidFunctionSignature {
                function: 0,
                details: "Function has no instructions".to_string(),
            });
        }

        Ok(())
    }

    /// Optimize function if optimizations are enabled
    fn optimize_function(&mut self, function: &mut TranspiledFunction, config: &TranspilationConfig) -> TranspilationResult<()> {
        if !config.enable_optimizations {
            return Ok(());
        }

        // Apply function-level optimizations based on optimization level
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

    /// Apply basic optimizations
    fn apply_basic_optimizations(&self, function: &mut TranspiledFunction) -> TranspilationResult<()> {
        // Remove redundant moves
        self.remove_redundant_moves(function)?;
        Ok(())
    }

    /// Apply standard optimizations
    fn apply_standard_optimizations(&self, function: &mut TranspiledFunction) -> TranspilationResult<()> {
        // Local variable optimization
        self.optimize_locals(function)?;
        Ok(())
    }

    /// Apply aggressive optimizations
    fn apply_aggressive_optimizations(&self, function: &mut TranspiledFunction) -> TranspilationResult<()> {
        // Advanced optimizations would go here
        Ok(())
    }

    /// Remove redundant move instructions
    fn remove_redundant_moves(&self, function: &mut TranspiledFunction) -> TranspilationResult<()> {
        let mut optimized_instructions = Vec::new();

        for instruction in &function.instructions {
            // Skip redundant moves (simplified check)
            if instruction.opcode.contains("move") && instruction.operands.len() == 2 {
                if let (Some(src), Some(dst)) = (instruction.operands.get(0), instruction.operands.get(1)) {
                    if src == dst {
                        continue; // Skip redundant move
                    }
                }
            }
            optimized_instructions.push(instruction.clone());
        }

        function.instructions = optimized_instructions;
        Ok(())
    }

    /// Optimize local variable usage
    fn optimize_locals(&self, function: &mut TranspiledFunction) -> TranspilationResult<()> {
        // This would involve register allocation and local variable optimization
        // For now, this is a placeholder
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::config::TranspilationConfig;

    #[test]
    fn test_function_processor_creation() {
        let config = TranspilationConfig::default();
        let processor = FunctionProcessor::new(&config);
        assert!(processor.is_ok());
    }

    #[test]
    fn test_function_name_generation() {
        let config = TranspilationConfig::default();
        let processor = FunctionProcessor::new(&config).unwrap();

        // Create a dummy WASM function
        let wasm_function = WasmFunction {
            signature: crate::wasm::ast::WasmFunctionType { params: vec![], results: vec![] },
            locals: vec![],
            body: vec![],
        };

        let name = processor.generate_function_name(42, &wasm_function);
        assert_eq!(name, "func_42");
    }
}
