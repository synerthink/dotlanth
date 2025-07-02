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

//! Instruction-related type definitions for transpilation

/// A single transpiled instruction with additional information
#[derive(Debug, Clone)]
pub struct TranspiledInstruction {
    /// Opcode string
    pub opcode: String,
    /// Operands
    pub operands: Vec<Operand>,
    /// Optional label for this instruction
    pub label: Option<String>,
    /// Source location information for debugging
    pub source_location: Option<SourceLocation>,
    /// Instruction metadata
    pub metadata: InstructionMetadata,
}

impl TranspiledInstruction {
    /// Create a new transpiled instruction
    pub fn new(opcode: String, operands: Vec<Operand>) -> Self {
        Self {
            opcode,
            operands,
            label: None,
            source_location: None,
            metadata: InstructionMetadata::default(),
        }
    }

    /// Create an instruction with a label
    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    /// Create an instruction with source location
    pub fn with_source_location(mut self, location: SourceLocation) -> Self {
        self.source_location = Some(location);
        self
    }

    /// Add an operand to this instruction
    pub fn add_operand(&mut self, operand: Operand) {
        self.operands.push(operand);
    }

    /// Set the label for this instruction
    pub fn set_label(&mut self, label: String) {
        self.label = Some(label);
    }

    /// Check if this instruction has a label
    pub fn has_label(&self) -> bool {
        self.label.is_some()
    }

    /// Get the number of operands
    pub fn operand_count(&self) -> usize {
        self.operands.len()
    }
}

/// Operand types for instructions
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// Immediate value
    Immediate(u32),
    /// Large immediate value (64-bit)
    LargeImmediate(u64),
    /// Register reference
    Register(u16),
    /// Label reference for jumps
    Label(String),
    /// Memory reference with base register and offset
    Memory { base: u16, offset: u32 },
    /// Stack reference with offset
    Stack { offset: i32 },
    /// Global variable reference
    Global { index: u32 },
}

impl Operand {
    /// Create an immediate operand
    pub fn immediate(value: u32) -> Self {
        Self::Immediate(value)
    }

    /// Create a large immediate operand
    pub fn large_immediate(value: u64) -> Self {
        Self::LargeImmediate(value)
    }

    /// Create a register operand
    pub fn register(reg: u16) -> Self {
        Self::Register(reg)
    }

    /// Create a label operand
    pub fn label(label: String) -> Self {
        Self::Label(label)
    }

    /// Create a memory operand
    pub fn memory(base: u16, offset: u32) -> Self {
        Self::Memory { base, offset }
    }

    /// Create a stack operand
    pub fn stack(offset: i32) -> Self {
        Self::Stack { offset }
    }

    /// Create a global operand
    pub fn global(index: u32) -> Self {
        Self::Global { index }
    }

    /// Check if this operand is an immediate value
    pub fn is_immediate(&self) -> bool {
        matches!(self, Self::Immediate(_) | Self::LargeImmediate(_))
    }

    /// Check if this operand is a register
    pub fn is_register(&self) -> bool {
        matches!(self, Self::Register(_))
    }

    /// Check if this operand is a memory reference
    pub fn is_memory(&self) -> bool {
        matches!(self, Self::Memory { .. })
    }

    /// Check if this operand is a label
    pub fn is_label(&self) -> bool {
        matches!(self, Self::Label(_))
    }
}

/// Source location information for debugging
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// Source file name
    pub file: String,
    /// Line number (1-based)
    pub line: u32,
    /// Column number (1-based)
    pub column: u32,
    /// Byte offset in the source
    pub offset: u32,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(file: String, line: u32, column: u32, offset: u32) -> Self {
        Self { file, line, column, offset }
    }
}

/// Metadata for individual instructions
#[derive(Debug, Clone, Default)]
pub struct InstructionMetadata {
    /// Estimated execution cycles
    pub estimated_cycles: u32,
    /// Whether this instruction can be optimized
    pub can_optimize: bool,
    /// Whether this instruction affects control flow
    pub affects_control_flow: bool,
    /// Whether this instruction accesses memory
    pub accesses_memory: bool,
    /// Architecture-specific hints
    pub arch_hints: Vec<String>,
}

impl InstructionMetadata {
    /// Create new instruction metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Set estimated execution cycles
    pub fn with_cycles(mut self, cycles: u32) -> Self {
        self.estimated_cycles = cycles;
        self
    }

    /// Mark as optimizable
    pub fn mark_optimizable(mut self) -> Self {
        self.can_optimize = true;
        self
    }

    /// Mark as affecting control flow
    pub fn mark_control_flow(mut self) -> Self {
        self.affects_control_flow = true;
        self
    }

    /// Mark as accessing memory
    pub fn mark_memory_access(mut self) -> Self {
        self.accesses_memory = true;
        self
    }

    /// Add an architecture hint
    pub fn add_arch_hint(mut self, hint: String) -> Self {
        self.arch_hints.push(hint);
        self
    }
}
