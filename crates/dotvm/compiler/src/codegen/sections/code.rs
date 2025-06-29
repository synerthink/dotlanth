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

//! Code section generator

use crate::{
    codegen::{
        error::{BytecodeGenerationError, BytecodeResult},
        writer::BytecodeWriter,
    },
    transpiler::engine::{Operand, TranspiledFunction, TranspiledInstruction},
};
use std::collections::HashMap;

/// Label information for jump resolution
#[derive(Debug, Clone)]
pub struct LabelInfo {
    pub name: String,
    pub offset: u32,
    pub is_resolved: bool,
}

/// Pending label reference that needs to be resolved
#[derive(Debug, Clone)]
pub struct PendingLabel {
    pub label_name: String,
    pub patch_offset: u32,
    pub instruction_offset: u32,
}

/// Generator for the code section
pub struct CodeGenerator {
    label_table: HashMap<String, LabelInfo>,
    pending_labels: Vec<PendingLabel>,
}

impl CodeGenerator {
    /// Create a new code generator
    pub fn new() -> Self {
        Self {
            label_table: HashMap::new(),
            pending_labels: Vec::new(),
        }
    }

    /// Generate the code section
    pub fn generate(&mut self, writer: &mut BytecodeWriter, functions: &[TranspiledFunction]) -> BytecodeResult<()> {
        // Clear previous state
        self.label_table.clear();
        self.pending_labels.clear();

        // Generate code for each function
        for function in functions {
            self.generate_function(writer, function)?;
        }

        // Resolve all pending labels
        self.resolve_labels(writer)?;

        Ok(())
    }

    /// Generate code for a single function
    fn generate_function(&mut self, writer: &mut BytecodeWriter, function: &TranspiledFunction) -> BytecodeResult<()> {
        // Generate function prologue
        self.generate_function_prologue(writer, function)?;

        // Generate instructions
        for instruction in &function.instructions {
            self.generate_instruction(writer, instruction)?;
        }

        // Generate function epilogue
        self.generate_function_epilogue(writer, function)?;

        Ok(())
    }

    /// Generate function prologue
    fn generate_function_prologue(&mut self, writer: &mut BytecodeWriter, function: &TranspiledFunction) -> BytecodeResult<()> {
        // Function entry marker
        writer.write_u8(0xFF)?; // FUNC_ENTER opcode

        // Local variable allocation
        if function.local_count > 0 {
            writer.write_u8(0xFE)?; // ALLOC_LOCALS opcode
            writer.write_u32(function.local_count as u32)?;
        }

        Ok(())
    }

    /// Generate a single instruction
    fn generate_instruction(&mut self, writer: &mut BytecodeWriter, instruction: &TranspiledInstruction) -> BytecodeResult<()> {
        // Handle labels
        if let Some(label) = &instruction.label {
            self.register_label(label.clone(), writer.position() as u32);
        }

        // Write opcode
        let opcode_hash = self.hash_opcode(&instruction.opcode);
        writer.write_u16(opcode_hash)?;

        // Write operands
        for operand in &instruction.operands {
            match operand {
                Operand::Immediate(value) => {
                    writer.write_u32(*value)?;
                }
                Operand::Register(reg) => {
                    writer.write_u16(*reg)?;
                }
                Operand::Label(label) => {
                    // Record this as a pending label reference
                    let patch_offset = writer.position();
                    self.pending_labels.push(PendingLabel {
                        label_name: label.clone(),
                        patch_offset: patch_offset as u32,
                        instruction_offset: writer.position() as u32,
                    });
                    writer.write_u32(0)?; // Placeholder
                }
                Operand::Memory { base, offset } => {
                    writer.write_u16(*base)?;
                    writer.write_u32(*offset)?;
                }
            }
        }
        Ok(())
    }

    /// Generate function epilogue
    fn generate_function_epilogue(&mut self, writer: &mut BytecodeWriter, _function: &TranspiledFunction) -> BytecodeResult<()> {
        // Function exit marker
        writer.write_u8(0xFD)?; // FUNC_EXIT opcode
        Ok(())
    }

    /// Register a label at the current position
    fn register_label(&mut self, name: String, offset: u32) {
        let label_info = LabelInfo {
            name: name.clone(),
            offset,
            is_resolved: true,
        };
        self.label_table.insert(name, label_info);
    }

    /// Resolve all pending label references
    fn resolve_labels(&mut self, writer: &mut BytecodeWriter) -> BytecodeResult<()> {
        for pending in &self.pending_labels {
            let label_info = self
                .label_table
                .get(&pending.label_name)
                .ok_or_else(|| BytecodeGenerationError::LabelResolutionFailed(format!("Label '{}' not found", pending.label_name)))?;

            if !label_info.is_resolved {
                return Err(BytecodeGenerationError::LabelResolutionFailed(format!("Label '{}' is not resolved", pending.label_name)));
            }

            // Calculate relative offset
            let target_offset = label_info.offset;
            let relative_offset = target_offset.wrapping_sub(pending.instruction_offset);

            // Patch the bytecode
            writer.write_at_offset(pending.patch_offset as usize, &relative_offset.to_le_bytes())?;
        }

        self.pending_labels.clear();
        Ok(())
    }

    /// Hash an opcode string to a 16-bit value
    fn hash_opcode(&self, opcode: &str) -> u16 {
        // Simple hash function for opcodes
        let mut hash = 0u16;
        for byte in opcode.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u16);
        }
        hash
    }

    /// Get the label table
    pub fn label_table(&self) -> &HashMap<String, LabelInfo> {
        &self.label_table
    }

    /// Get pending labels
    pub fn pending_labels(&self) -> &[PendingLabel] {
        &self.pending_labels
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::engine::Operand;

    #[test]
    fn test_opcode_hashing() {
        let generator = CodeGenerator::new();

        let hash1 = generator.hash_opcode("ADD");
        let hash2 = generator.hash_opcode("SUB");
        let hash3 = generator.hash_opcode("ADD"); // Same as hash1

        assert_ne!(hash1, hash2);
        assert_eq!(hash1, hash3);
    }

    #[test]
    fn test_label_registration() {
        let mut generator = CodeGenerator::new();

        generator.register_label("loop_start".to_string(), 100);

        let label_info = generator.label_table.get("loop_start").unwrap();
        assert_eq!(label_info.offset, 100);
        assert!(label_info.is_resolved);
    }

    #[test]
    fn test_instruction_generation() {
        let mut generator = CodeGenerator::new();
        let mut writer = BytecodeWriter::new();

        let instruction = TranspiledInstruction {
            opcode: "ADD".to_string(),
            operands: vec![Operand::Register(1), Operand::Register(2), Operand::Immediate(42)],
            label: None,
        };

        generator.generate_instruction(&mut writer, &instruction).unwrap();

        // Should have written: opcode (2 bytes) + reg (2 bytes) + reg (2 bytes) + immediate (4 bytes)
        assert_eq!(writer.size(), 2 + 2 + 2 + 4);
    }
}
