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

//! Module-related type definitions for transpilation

use super::{ExportInfo, GlobalVariable, ImportInfo, MemoryLayout, TranspiledFunction};
use dotvm_core::bytecode::BytecodeHeader;

/// Complete transpiled module
#[derive(Debug, Clone)]
pub struct TranspiledModule {
    /// Module header with architecture information
    pub header: BytecodeHeader,
    /// Transpiled functions
    pub functions: Vec<TranspiledFunction>,
    /// Global variables
    pub globals: Vec<GlobalVariable>,
    /// Memory layout information
    pub memory_layout: MemoryLayout,
    /// Export information
    pub exports: Vec<ExportInfo>,
    /// Import information
    pub imports: Vec<ImportInfo>,
    /// Module metadata
    pub metadata: ModuleMetadata,
}

impl TranspiledModule {
    /// Create a new transpiled module
    pub fn new(header: BytecodeHeader) -> Self {
        Self {
            header,
            functions: Vec::new(),
            globals: Vec::new(),
            memory_layout: MemoryLayout::default(),
            exports: Vec::new(),
            imports: Vec::new(),
            metadata: ModuleMetadata::default(),
        }
    }

    /// Add a function to the module
    pub fn add_function(&mut self, function: TranspiledFunction) {
        self.functions.push(function);
    }

    /// Add a global variable to the module
    pub fn add_global(&mut self, global: GlobalVariable) {
        self.globals.push(global);
    }

    /// Add an export to the module
    pub fn add_export(&mut self, export: ExportInfo) {
        self.exports.push(export);
    }

    /// Add an import to the module
    pub fn add_import(&mut self, import: ImportInfo) {
        self.imports.push(import);
    }

    /// Set the memory layout
    pub fn set_memory_layout(&mut self, layout: MemoryLayout) {
        self.memory_layout = layout;
    }

    /// Get the number of functions in the module
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Get the number of globals in the module
    pub fn global_count(&self) -> usize {
        self.globals.len()
    }

    /// Get the number of exports
    pub fn export_count(&self) -> usize {
        self.exports.len()
    }

    /// Get the number of imports
    pub fn import_count(&self) -> usize {
        self.imports.len()
    }

    /// Find a function by name
    pub fn find_function(&self, name: &str) -> Option<&TranspiledFunction> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Find an export by name
    pub fn find_export(&self, name: &str) -> Option<&ExportInfo> {
        self.exports.iter().find(|e| e.name == name)
    }

    /// Get all exported functions
    pub fn exported_functions(&self) -> Vec<&TranspiledFunction> {
        self.functions.iter().filter(|f| f.is_exported).collect()
    }
}

/// Module metadata for optimization and analysis
#[derive(Debug, Clone, Default)]
pub struct ModuleMetadata {
    /// Total estimated size in bytes
    pub estimated_size: u64,
    /// Complexity score for the entire module
    pub complexity_score: u32,
    /// Whether the module uses advanced features
    pub uses_advanced_features: bool,
    /// Target architecture requirements
    pub architecture_requirements: Vec<String>,
    /// Optimization hints
    pub optimization_hints: Vec<String>,
    /// Performance characteristics
    pub performance_profile: PerformanceProfile,
}

impl ModuleMetadata {
    /// Create new module metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the estimated size
    pub fn set_estimated_size(&mut self, size: u64) {
        self.estimated_size = size;
    }

    /// Set the complexity score
    pub fn set_complexity_score(&mut self, score: u32) {
        self.complexity_score = score;
    }

    /// Mark as using advanced features
    pub fn mark_advanced_features(&mut self) {
        self.uses_advanced_features = true;
    }

    /// Add an architecture requirement
    pub fn add_architecture_requirement(&mut self, requirement: String) {
        self.architecture_requirements.push(requirement);
    }

    /// Add an optimization hint
    pub fn add_optimization_hint(&mut self, hint: String) {
        self.optimization_hints.push(hint);
    }
}

/// Performance profile for the module
#[derive(Debug, Clone, Default)]
pub struct PerformanceProfile {
    /// Estimated execution time in cycles
    pub estimated_cycles: u64,
    /// Memory usage estimate in bytes
    pub memory_usage: u64,
    /// Number of memory allocations
    pub allocation_count: u32,
    /// Whether the module is CPU-intensive
    pub is_cpu_intensive: bool,
    /// Whether the module is memory-intensive
    pub is_memory_intensive: bool,
}

impl PerformanceProfile {
    /// Create a new performance profile
    pub fn new() -> Self {
        Self::default()
    }

    /// Set estimated cycles
    pub fn set_estimated_cycles(&mut self, cycles: u64) {
        self.estimated_cycles = cycles;
    }

    /// Set memory usage
    pub fn set_memory_usage(&mut self, usage: u64) {
        self.memory_usage = usage;
    }

    /// Set allocation count
    pub fn set_allocation_count(&mut self, count: u32) {
        self.allocation_count = count;
    }

    /// Mark as CPU-intensive
    pub fn mark_cpu_intensive(&mut self) {
        self.is_cpu_intensive = true;
    }

    /// Mark as memory-intensive
    pub fn mark_memory_intensive(&mut self) {
        self.is_memory_intensive = true;
    }
}
