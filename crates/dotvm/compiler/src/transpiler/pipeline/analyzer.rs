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

//! Analysis stage for architecture requirements and optimization opportunities

use super::{
    super::{
        config::TranspilationConfig,
        error::{TranspilationError, TranspilationResult},
    },
    PipelineStage,
};
use crate::wasm::{
    ast::{WasmFunction, WasmInstruction, WasmModule},
    opcode_mapper::OpcodeMapper,
};
use dotvm_core::bytecode::VmArchitecture;

/// Analysis results containing architecture requirements and optimization hints
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// Original WASM module
    pub module: WasmModule,
    /// Required architecture for this module
    pub required_architecture: VmArchitecture,
    /// Architecture compatibility information
    pub architecture_info: ArchitectureInfo,
    /// Function analysis results
    pub function_analyses: Vec<FunctionAnalysis>,
    /// Module-level optimization hints
    pub optimization_hints: Vec<OptimizationHint>,
    /// Performance characteristics
    pub performance_profile: PerformanceProfile,
}

/// Architecture compatibility information
#[derive(Debug, Clone)]
pub struct ArchitectureInfo {
    /// Minimum required architecture
    pub minimum_architecture: VmArchitecture,
    /// Recommended architecture for optimal performance
    pub recommended_architecture: VmArchitecture,
    /// Required features
    pub required_features: Vec<String>,
    /// Optional features that would improve performance
    pub optional_features: Vec<String>,
    /// Architecture-specific warnings
    pub warnings: Vec<String>,
}

/// Analysis results for a single function
#[derive(Debug, Clone)]
pub struct FunctionAnalysis {
    /// Function index
    pub index: u32,
    /// Complexity score (0-100)
    pub complexity_score: u32,
    /// Whether the function has complex control flow
    pub has_complex_control_flow: bool,
    /// Maximum stack depth required
    pub max_stack_depth: u32,
    /// Memory access patterns
    pub memory_accesses: Vec<MemoryAccessInfo>,
    /// Function calls made by this function
    pub function_calls: Vec<u32>,
    /// Whether the function is recursive
    pub is_recursive: bool,
    /// Estimated execution cycles
    pub estimated_cycles: u64,
    /// Optimization opportunities
    pub optimization_opportunities: Vec<OptimizationOpportunity>,
}

/// Memory access information
#[derive(Debug, Clone)]
pub struct MemoryAccessInfo {
    /// Instruction index within the function
    pub instruction_index: usize,
    /// Type of memory access
    pub access_type: MemoryAccessType,
    /// Size of the access in bytes
    pub size: u32,
    /// Whether the access is aligned
    pub is_aligned: bool,
    /// Access frequency (estimated)
    pub frequency: u32,
}

/// Types of memory access
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAccessType {
    Load,
    Store,
    Atomic,
    Bulk,
}

/// Optimization opportunities
#[derive(Debug, Clone)]
pub struct OptimizationOpportunity {
    /// Type of optimization
    pub optimization_type: OptimizationType,
    /// Description of the opportunity
    pub description: String,
    /// Estimated performance benefit (0-100)
    pub benefit_score: u32,
    /// Implementation difficulty (0-100)
    pub difficulty_score: u32,
}

/// Types of optimizations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationType {
    ConstantFolding,
    DeadCodeElimination,
    LoopOptimization,
    Inlining,
    Vectorization,
    MemoryOptimization,
    ControlFlowOptimization,
}

/// Module-level optimization hints
#[derive(Debug, Clone)]
pub struct OptimizationHint {
    /// Type of hint
    pub hint_type: OptimizationHintType,
    /// Description
    pub description: String,
    /// Priority (0-100, higher is more important)
    pub priority: u32,
}

/// Types of optimization hints
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationHintType {
    ArchitectureSpecific,
    MemoryLayout,
    FunctionOrdering,
    ParallelizationOpportunity,
    CacheOptimization,
}

/// Performance profile for the module
#[derive(Debug, Clone, Default)]
pub struct PerformanceProfile {
    /// Estimated total execution cycles
    pub estimated_total_cycles: u64,
    /// Estimated memory usage in bytes
    pub estimated_memory_usage: u64,
    /// Number of memory allocations
    pub allocation_count: u32,
    /// Whether the module is CPU-intensive
    pub is_cpu_intensive: bool,
    /// Whether the module is memory-intensive
    pub is_memory_intensive: bool,
    /// Hotspot functions (most frequently called)
    pub hotspot_functions: Vec<u32>,
}

/// Analyzer stage for architecture and optimization analysis
pub struct Analyzer {
    /// Opcode mapper for architecture analysis
    opcode_mapper: OpcodeMapper,
    /// Analysis configuration
    analysis_config: AnalysisConfig,
}

impl Analyzer {
    /// Create a new analyzer
    pub fn new(config: &TranspilationConfig) -> TranspilationResult<Self> {
        Ok(Self {
            opcode_mapper: OpcodeMapper::new(config.target_architecture),
            analysis_config: AnalysisConfig::from_transpilation_config(config),
        })
    }

    /// Analyze architecture requirements for the module
    fn analyze_architecture(&self, module: &WasmModule) -> TranspilationResult<ArchitectureInfo> {
        let mut minimum_arch = VmArchitecture::Arch64;
        let mut required_features = Vec::new();
        let mut optional_features = Vec::new();
        let mut warnings = Vec::new();

        // Analyze all functions for architecture requirements
        for function in &module.functions {
            for instruction in &function.body {
                let inst_arch = OpcodeMapper::required_architecture(instruction);
                if (inst_arch as u8) > (minimum_arch as u8) {
                    minimum_arch = inst_arch;
                }

                // Check for specific feature requirements
                // Note: These are placeholder checks - actual WASM instruction variants may differ
                match instruction {
                    // SIMD instructions would be detected here
                    // WasmInstruction::V128Load { .. } |
                    // WasmInstruction::V128Store { .. } => {
                    //     if !required_features.contains(&"simd".to_string()) {
                    //         required_features.push("simd".to_string());
                    //     }
                    // }

                    // Thread instructions would be detected here
                    // WasmInstruction::MemoryAtomicNotify { .. } |
                    // WasmInstruction::MemoryAtomicWait32 { .. } => {
                    //     if !required_features.contains(&"threads".to_string()) {
                    //         required_features.push("threads".to_string());
                    //     }
                    // }

                    // Bulk memory instructions would be detected here
                    // WasmInstruction::MemoryCopy { .. } |
                    // WasmInstruction::MemoryFill { .. } => {
                    //     if !required_features.contains(&"bulk_memory".to_string()) {
                    //         required_features.push("bulk_memory".to_string());
                    //     }
                    // }
                    _ => {}
                }
            }
        }

        // Determine recommended architecture
        let recommended_arch = if required_features.contains(&"simd".to_string()) { VmArchitecture::Arch128 } else { minimum_arch };

        // Add optional features that could improve performance
        if module.functions.len() > 100 {
            optional_features.push("function_caching".to_string());
        }

        if self.has_intensive_memory_operations(module) {
            optional_features.push("memory_prefetching".to_string());
        }

        // Generate warnings
        if minimum_arch as u8 > self.analysis_config.target_architecture as u8 {
            warnings.push(format!("Module requires {:?} but target is {:?}", minimum_arch, self.analysis_config.target_architecture));
        }

        Ok(ArchitectureInfo {
            minimum_architecture: minimum_arch,
            recommended_architecture: recommended_arch,
            required_features,
            optional_features,
            warnings,
        })
    }

    /// Check if the module has intensive memory operations
    fn has_intensive_memory_operations(&self, module: &WasmModule) -> bool {
        let mut memory_ops = 0;
        let mut total_instructions = 0;

        for function in &module.functions {
            for instruction in &function.body {
                total_instructions += 1;
                match instruction {
                    WasmInstruction::I32Load { .. }
                    | WasmInstruction::I64Load { .. }
                    | WasmInstruction::F32Load { .. }
                    | WasmInstruction::F64Load { .. }
                    | WasmInstruction::I32Store { .. }
                    | WasmInstruction::I64Store { .. }
                    | WasmInstruction::F32Store { .. }
                    | WasmInstruction::F64Store { .. } => {
                        memory_ops += 1;
                    }
                    _ => {}
                }
            }
        }

        // Consider intensive if more than 30% of instructions are memory operations
        total_instructions > 0 && (memory_ops * 100 / total_instructions) > 30
    }

    /// Analyze a single function
    fn analyze_function(&self, index: u32, function: &WasmFunction) -> TranspilationResult<FunctionAnalysis> {
        let mut analysis = FunctionAnalysis {
            index,
            complexity_score: 0,
            has_complex_control_flow: false,
            max_stack_depth: 0,
            memory_accesses: Vec::new(),
            function_calls: Vec::new(),
            is_recursive: false,
            estimated_cycles: 0,
            optimization_opportunities: Vec::new(),
        };

        // Analyze instructions
        let mut stack_depth = 0u32;
        let mut max_depth = 0u32;
        let mut control_flow_depth = 0u32;

        for (inst_index, instruction) in function.body.iter().enumerate() {
            // Update stack depth estimation
            let (pop_count, push_count) = self.estimate_stack_effect(instruction);
            stack_depth = stack_depth.saturating_sub(pop_count).saturating_add(push_count);
            max_depth = max_depth.max(stack_depth);

            // Analyze control flow
            match instruction {
                WasmInstruction::Block { .. } | WasmInstruction::Loop { .. } | WasmInstruction::If { .. } => {
                    control_flow_depth += 1;
                    if control_flow_depth > 3 {
                        analysis.has_complex_control_flow = true;
                    }
                }
                WasmInstruction::End => {
                    control_flow_depth = control_flow_depth.saturating_sub(1);
                }
                WasmInstruction::Call { function_index } => {
                    analysis.function_calls.push(*function_index);
                    if *function_index == index {
                        analysis.is_recursive = true;
                    }
                }
                _ => {}
            }

            // Analyze memory accesses
            if let Some(access_info) = self.analyze_memory_access(inst_index, instruction) {
                analysis.memory_accesses.push(access_info);
            }

            // Estimate cycles for this instruction
            analysis.estimated_cycles += self.estimate_instruction_cycles(instruction);
        }

        analysis.max_stack_depth = max_depth;

        // Calculate complexity score
        analysis.complexity_score = self.calculate_complexity_score(&analysis, function);

        // Find optimization opportunities
        analysis.optimization_opportunities = self.find_optimization_opportunities(&analysis, function);

        Ok(analysis)
    }

    /// Estimate stack effect of an instruction (pop_count, push_count)
    fn estimate_stack_effect(&self, instruction: &WasmInstruction) -> (u32, u32) {
        match instruction {
            WasmInstruction::I32Const { .. } | WasmInstruction::I64Const { .. } | WasmInstruction::F32Const { .. } | WasmInstruction::F64Const { .. } => (0, 1),

            WasmInstruction::I32Add | WasmInstruction::I64Add | WasmInstruction::F32Add | WasmInstruction::F64Add => (2, 1),

            WasmInstruction::I32Load { .. } | WasmInstruction::I64Load { .. } | WasmInstruction::F32Load { .. } | WasmInstruction::F64Load { .. } => (1, 1),

            WasmInstruction::I32Store { .. } | WasmInstruction::I64Store { .. } | WasmInstruction::F32Store { .. } | WasmInstruction::F64Store { .. } => (2, 0),

            WasmInstruction::Drop => (1, 0),
            WasmInstruction::Select => (3, 1),

            _ => (0, 0), // Simplified - real implementation would be more comprehensive
        }
    }

    /// Analyze memory access for an instruction
    fn analyze_memory_access(&self, inst_index: usize, instruction: &WasmInstruction) -> Option<MemoryAccessInfo> {
        match instruction {
            WasmInstruction::I32Load { memarg } => Some(MemoryAccessInfo {
                instruction_index: inst_index,
                access_type: MemoryAccessType::Load,
                size: 4,
                is_aligned: memarg.align >= 2, // 4-byte alignment
                frequency: 1,
            }),
            WasmInstruction::I64Load { memarg } => Some(MemoryAccessInfo {
                instruction_index: inst_index,
                access_type: MemoryAccessType::Load,
                size: 8,
                is_aligned: memarg.align >= 3, // 8-byte alignment
                frequency: 1,
            }),
            WasmInstruction::I32Store { memarg } => Some(MemoryAccessInfo {
                instruction_index: inst_index,
                access_type: MemoryAccessType::Store,
                size: 4,
                is_aligned: memarg.align >= 2,
                frequency: 1,
            }),
            WasmInstruction::I64Store { memarg } => Some(MemoryAccessInfo {
                instruction_index: inst_index,
                access_type: MemoryAccessType::Store,
                size: 8,
                is_aligned: memarg.align >= 3,
                frequency: 1,
            }),
            // Bulk memory operations would be handled here
            // WasmInstruction::MemoryCopy { .. } |
            // WasmInstruction::MemoryFill { .. } => Some(MemoryAccessInfo {
            //     instruction_index: inst_index,
            //     access_type: MemoryAccessType::Bulk,
            //     size: 0, // Variable size
            //     is_aligned: false, // Conservative assumption
            //     frequency: 1,
            // }),
            _ => None,
        }
    }

    /// Estimate execution cycles for an instruction
    fn estimate_instruction_cycles(&self, instruction: &WasmInstruction) -> u64 {
        match instruction {
            // Arithmetic operations
            WasmInstruction::I32Add | WasmInstruction::I64Add => 1,
            WasmInstruction::I32Mul | WasmInstruction::I64Mul => 3,
            WasmInstruction::I32DivS | WasmInstruction::I32DivU | WasmInstruction::I64DivS | WasmInstruction::I64DivU => 10,
            WasmInstruction::F32Add | WasmInstruction::F64Add => 2,
            WasmInstruction::F32Mul | WasmInstruction::F64Mul => 4,
            WasmInstruction::F32Div | WasmInstruction::F64Div => 15,

            // Memory operations
            WasmInstruction::I32Load { .. } | WasmInstruction::I64Load { .. } | WasmInstruction::F32Load { .. } | WasmInstruction::F64Load { .. } => 3,
            WasmInstruction::I32Store { .. } | WasmInstruction::I64Store { .. } | WasmInstruction::F32Store { .. } | WasmInstruction::F64Store { .. } => 2,

            // Control flow
            WasmInstruction::Call { .. } => 5,
            WasmInstruction::CallIndirect { .. } => 8,
            WasmInstruction::Br { .. } | WasmInstruction::BrIf { .. } => 2,

            // Constants and simple operations
            WasmInstruction::I32Const { .. } | WasmInstruction::I64Const { .. } | WasmInstruction::F32Const { .. } | WasmInstruction::F64Const { .. } => 1,

            _ => 1, // Default estimate
        }
    }

    /// Calculate complexity score for a function
    fn calculate_complexity_score(&self, analysis: &FunctionAnalysis, function: &WasmFunction) -> u32 {
        let mut score = 0u32;

        // Base score from instruction count
        score += (function.body.len() / 10) as u32;

        // Add score for control flow complexity
        if analysis.has_complex_control_flow {
            score += 20;
        }

        // Add score for recursion
        if analysis.is_recursive {
            score += 15;
        }

        // Add score for memory operations
        score += (analysis.memory_accesses.len() / 5) as u32;

        // Add score for function calls
        score += (analysis.function_calls.len() * 2) as u32;

        // Add score for stack depth
        score += analysis.max_stack_depth / 4;

        // Cap at 100
        score.min(100)
    }

    /// Find optimization opportunities in a function
    fn find_optimization_opportunities(&self, analysis: &FunctionAnalysis, function: &WasmFunction) -> Vec<OptimizationOpportunity> {
        let mut opportunities = Vec::new();

        // Check for constant folding opportunities
        if self.has_constant_folding_opportunities(function) {
            opportunities.push(OptimizationOpportunity {
                optimization_type: OptimizationType::ConstantFolding,
                description: "Function contains constant expressions that can be folded".to_string(),
                benefit_score: 30,
                difficulty_score: 20,
            });
        }

        // Check for dead code
        if self.has_dead_code(function) {
            opportunities.push(OptimizationOpportunity {
                optimization_type: OptimizationType::DeadCodeElimination,
                description: "Function contains unreachable code".to_string(),
                benefit_score: 25,
                difficulty_score: 15,
            });
        }

        // Check for inlining opportunities
        if function.body.len() < 10 && !analysis.is_recursive {
            opportunities.push(OptimizationOpportunity {
                optimization_type: OptimizationType::Inlining,
                description: "Small function suitable for inlining".to_string(),
                benefit_score: 40,
                difficulty_score: 30,
            });
        }

        // Check for vectorization opportunities
        if self.has_vectorization_opportunities(function) {
            opportunities.push(OptimizationOpportunity {
                optimization_type: OptimizationType::Vectorization,
                description: "Function contains loops suitable for vectorization".to_string(),
                benefit_score: 60,
                difficulty_score: 70,
            });
        }

        opportunities
    }

    /// Check for constant folding opportunities
    fn has_constant_folding_opportunities(&self, function: &WasmFunction) -> bool {
        // Look for patterns like: const, const, add
        for window in function.body.windows(3) {
            if let [WasmInstruction::I32Const { .. }, WasmInstruction::I32Const { .. }, WasmInstruction::I32Add] = window {
                return true;
            }
        }
        false
    }

    /// Check for dead code
    fn has_dead_code(&self, function: &WasmFunction) -> bool {
        // Simplified check - look for unreachable instructions after return
        let mut found_return = false;
        for instruction in &function.body {
            if found_return {
                match instruction {
                    WasmInstruction::End => break,
                    _ => return true, // Found instruction after return
                }
            }
            if matches!(instruction, WasmInstruction::Return) {
                found_return = true;
            }
        }
        false
    }

    /// Check for vectorization opportunities
    fn has_vectorization_opportunities(&self, function: &WasmFunction) -> bool {
        // Look for loop patterns with arithmetic operations
        let mut in_loop = false;
        let mut has_arithmetic = false;

        for instruction in &function.body {
            match instruction {
                WasmInstruction::Loop { .. } => in_loop = true,
                WasmInstruction::End => in_loop = false,
                WasmInstruction::I32Add
                | WasmInstruction::I64Add
                | WasmInstruction::F32Add
                | WasmInstruction::F64Add
                | WasmInstruction::I32Mul
                | WasmInstruction::I64Mul
                | WasmInstruction::F32Mul
                | WasmInstruction::F64Mul => {
                    if in_loop {
                        has_arithmetic = true;
                    }
                }
                _ => {}
            }
        }

        has_arithmetic
    }

    /// Generate module-level optimization hints
    fn generate_optimization_hints(&self, module: &WasmModule, function_analyses: &[FunctionAnalysis]) -> Vec<OptimizationHint> {
        let mut hints = Vec::new();

        // Function ordering hint
        if function_analyses.len() > 10 {
            hints.push(OptimizationHint {
                hint_type: OptimizationHintType::FunctionOrdering,
                description: "Consider reordering functions based on call frequency".to_string(),
                priority: 40,
            });
        }

        // Memory layout hint
        if module.globals.len() > 20 {
            hints.push(OptimizationHint {
                hint_type: OptimizationHintType::MemoryLayout,
                description: "Consider optimizing global variable layout for cache efficiency".to_string(),
                priority: 50,
            });
        }

        // Parallelization hint
        let independent_functions = function_analyses.iter().filter(|f| f.function_calls.is_empty() && !f.is_recursive).count();

        if independent_functions > 5 {
            hints.push(OptimizationHint {
                hint_type: OptimizationHintType::ParallelizationOpportunity,
                description: format!("{} functions could potentially be parallelized", independent_functions),
                priority: 60,
            });
        }

        hints
    }

    /// Generate performance profile
    fn generate_performance_profile(&self, module: &WasmModule, function_analyses: &[FunctionAnalysis]) -> PerformanceProfile {
        let mut profile = PerformanceProfile::default();

        // Calculate total estimated cycles
        profile.estimated_total_cycles = function_analyses.iter().map(|f| f.estimated_cycles).sum();

        // Estimate memory usage
        profile.estimated_memory_usage = (module.globals.len() * 8) as u64; // Rough estimate
        for memory in &module.memories {
            profile.estimated_memory_usage += memory.min_pages as u64 * 65536;
        }

        // Count allocations (simplified)
        profile.allocation_count = module.globals.len() as u32;

        // Determine if CPU or memory intensive
        profile.is_cpu_intensive = profile.estimated_total_cycles > 10000;
        profile.is_memory_intensive = profile.estimated_memory_usage > 1024 * 1024; // 1MB

        // Find hotspot functions (top 20% by estimated cycles)
        let mut indexed_analyses: Vec<_> = function_analyses.iter().map(|f| (f.index, f.estimated_cycles)).collect();
        indexed_analyses.sort_by(|a, b| b.1.cmp(&a.1));

        let hotspot_count = (indexed_analyses.len() / 5).max(1);
        profile.hotspot_functions = indexed_analyses.into_iter().take(hotspot_count).map(|(index, _)| index).collect();

        profile
    }
}

impl PipelineStage for Analyzer {
    type Input = WasmModule;
    type Output = AnalysisResult;

    fn execute(&mut self, input: Self::Input, _config: &TranspilationConfig) -> TranspilationResult<Self::Output> {
        // Analyze architecture requirements
        let architecture_info = self.analyze_architecture(&input)?;

        // Analyze each function
        let mut function_analyses = Vec::new();
        for (index, function) in input.functions.iter().enumerate() {
            let analysis = self.analyze_function(index as u32, function)?;
            function_analyses.push(analysis);
        }

        // Generate optimization hints
        let optimization_hints = self.generate_optimization_hints(&input, &function_analyses);

        // Generate performance profile
        let performance_profile = self.generate_performance_profile(&input, &function_analyses);

        Ok(AnalysisResult {
            required_architecture: architecture_info.minimum_architecture,
            architecture_info,
            function_analyses,
            optimization_hints,
            performance_profile,
            module: input,
        })
    }

    fn name(&self) -> &'static str {
        "analyzer"
    }

    fn can_skip(&self, config: &TranspilationConfig) -> bool {
        // Skip analysis if optimizations are disabled and we're not doing architecture checking
        !config.enable_optimizations && !config.enable_arch_features
    }

    fn estimated_duration(&self, input_size: usize) -> std::time::Duration {
        // Analysis is more intensive, roughly 5ms per KB
        std::time::Duration::from_millis((input_size * 5 / 1024).max(1) as u64)
    }
}

/// Configuration for analysis
#[derive(Debug, Clone)]
struct AnalysisConfig {
    /// Target architecture
    target_architecture: VmArchitecture,
    /// Whether to perform deep analysis
    deep_analysis: bool,
    /// Whether to generate optimization hints
    generate_hints: bool,
}

impl AnalysisConfig {
    /// Create analysis config from transpilation config
    fn from_transpilation_config(config: &TranspilationConfig) -> Self {
        Self {
            target_architecture: config.target_architecture,
            deep_analysis: config.enable_optimizations,
            generate_hints: config.enable_optimizations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::config::TranspilationConfig;

    #[test]
    fn test_analyzer_creation() {
        let config = TranspilationConfig::default();
        let analyzer = Analyzer::new(&config);
        assert!(analyzer.is_ok());
    }

    #[test]
    fn test_stack_effect_estimation() {
        let config = TranspilationConfig::default();
        let analyzer = Analyzer::new(&config).unwrap();

        let (pop, push) = analyzer.estimate_stack_effect(&WasmInstruction::I32Const { value: 42 });
        assert_eq!(pop, 0);
        assert_eq!(push, 1);

        let (pop, push) = analyzer.estimate_stack_effect(&WasmInstruction::I32Add);
        assert_eq!(pop, 2);
        assert_eq!(push, 1);
    }

    #[test]
    fn test_instruction_cycles_estimation() {
        let config = TranspilationConfig::default();
        let analyzer = Analyzer::new(&config).unwrap();

        assert_eq!(analyzer.estimate_instruction_cycles(&WasmInstruction::I32Add), 1);
        assert_eq!(analyzer.estimate_instruction_cycles(&WasmInstruction::I32DivS), 10);
        assert_eq!(analyzer.estimate_instruction_cycles(&WasmInstruction::Call { function_index: 0 }), 5);
    }
}
