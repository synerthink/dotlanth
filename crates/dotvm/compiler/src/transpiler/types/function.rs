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

//! Function-related type definitions for transpilation

use super::instruction::TranspiledInstruction;

/// Represents a transpiled DotVM function
#[derive(Debug, Clone)]
pub struct TranspiledFunction {
    /// Function name
    pub name: String,
    /// DotVM bytecode instructions
    pub instructions: Vec<TranspiledInstruction>,
    /// Parameter count
    pub param_count: usize,
    /// Local variable count
    pub local_count: usize,
    /// Whether this function is exported
    pub is_exported: bool,
    /// Debug information (source file, line numbers, etc.)
    pub debug_info: Option<String>,
    /// Function metadata for optimization
    pub metadata: FunctionMetadata,
}

impl TranspiledFunction {
    /// Create a new transpiled function with basic information
    pub fn new(name: String, param_count: usize, local_count: usize) -> Self {
        Self {
            name,
            instructions: Vec::new(),
            param_count,
            local_count,
            is_exported: false,
            debug_info: None,
            metadata: FunctionMetadata::default(),
        }
    }

    /// Add an instruction to this function
    pub fn add_instruction(&mut self, instruction: TranspiledInstruction) {
        self.instructions.push(instruction);
    }

    /// Set the export status of this function
    pub fn set_exported(&mut self, exported: bool) {
        self.is_exported = exported;
    }

    /// Set debug information for this function
    pub fn set_debug_info(&mut self, debug_info: String) {
        self.debug_info = Some(debug_info);
    }

    /// Get the total number of instructions in this function
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Check if this function has complex control flow
    pub fn has_complex_control_flow(&self) -> bool {
        self.metadata.has_complex_control_flow
    }
}

/// Function metadata for optimization and debugging
#[derive(Debug, Clone, Default)]
pub struct FunctionMetadata {
    /// Whether the function uses complex control flow
    pub has_complex_control_flow: bool,
    /// Maximum stack depth required
    pub max_stack_depth: u32,
    /// Memory access patterns
    pub memory_accesses: Vec<MemoryAccessPattern>,
    /// Function calls made by this function
    pub function_calls: Vec<u32>,
    /// Whether the function is recursive
    pub is_recursive: bool,
    /// Estimated execution complexity
    pub complexity_score: u32,
}

impl FunctionMetadata {
    /// Create new metadata with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a memory access pattern
    pub fn add_memory_access(&mut self, pattern: MemoryAccessPattern) {
        self.memory_accesses.push(pattern);
    }

    /// Add a function call reference
    pub fn add_function_call(&mut self, function_index: u32) {
        self.function_calls.push(function_index);
    }

    /// Set the complexity score
    pub fn set_complexity_score(&mut self, score: u32) {
        self.complexity_score = score;
    }

    /// Mark as having complex control flow
    pub fn mark_complex_control_flow(&mut self) {
        self.has_complex_control_flow = true;
    }

    /// Set maximum stack depth
    pub fn set_max_stack_depth(&mut self, depth: u32) {
        self.max_stack_depth = depth;
    }

    /// Mark as recursive
    pub fn mark_recursive(&mut self) {
        self.is_recursive = true;
    }
}

/// Memory access pattern for optimization
#[derive(Debug, Clone)]
pub struct MemoryAccessPattern {
    /// Memory offset being accessed
    pub offset: u64,
    /// Size of the access in bytes
    pub size: u32,
    /// Whether this is a write operation
    pub is_write: bool,
    /// Frequency of this access pattern
    pub frequency: u32,
    /// Alignment requirements
    pub alignment: u32,
}

impl MemoryAccessPattern {
    /// Create a new memory access pattern
    pub fn new(offset: u64, size: u32, is_write: bool) -> Self {
        Self {
            offset,
            size,
            is_write,
            frequency: 1,
            alignment: size, // Default alignment to access size
        }
    }

    /// Create a read access pattern
    pub fn read(offset: u64, size: u32) -> Self {
        Self::new(offset, size, false)
    }

    /// Create a write access pattern
    pub fn write(offset: u64, size: u32) -> Self {
        Self::new(offset, size, true)
    }

    /// Increment the frequency counter
    pub fn increment_frequency(&mut self) {
        self.frequency += 1;
    }

    /// Set custom alignment requirements
    pub fn with_alignment(mut self, alignment: u32) -> Self {
        self.alignment = alignment;
        self
    }

    /// Check if this access is aligned
    pub fn is_aligned(&self) -> bool {
        self.offset % self.alignment as u64 == 0
    }
}
