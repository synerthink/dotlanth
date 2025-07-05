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

//! Function table section generator

use crate::{
    codegen::{error::BytecodeResult, writers::BytecodeWriter},
    transpiler::engine::TranspiledFunction,
};

/// Compilation metadata flags for functions
#[derive(Debug, Clone, Default)]
pub struct MetadataFlags {
    pub is_exported: bool,
    pub is_imported: bool,
    pub has_debug_info: bool,
    pub is_optimized: bool,
}

impl MetadataFlags {
    /// Convert flags to a bitfield
    pub fn to_bits(&self) -> u32 {
        let mut bits = 0u32;
        if self.is_exported {
            bits |= 0x01;
        }
        if self.is_imported {
            bits |= 0x02;
        }
        if self.has_debug_info {
            bits |= 0x04;
        }
        if self.is_optimized {
            bits |= 0x08;
        }
        bits
    }

    /// Create flags from a bitfield
    pub fn from_bits(bits: u32) -> Self {
        Self {
            is_exported: (bits & 0x01) != 0,
            is_imported: (bits & 0x02) != 0,
            has_debug_info: (bits & 0x04) != 0,
            is_optimized: (bits & 0x08) != 0,
        }
    }
}

/// Single function entry in the function table
#[derive(Debug, Clone)]
pub struct FunctionEntry {
    pub name_offset: u32,
    pub code_offset: u32,
    pub code_size: u32,
    pub param_count: u16,
    pub local_count: u16,
    pub flags: MetadataFlags,
}

/// Function table containing all function metadata
#[derive(Debug, Clone)]
pub struct FunctionTable {
    pub entries: Vec<FunctionEntry>,
}

/// Generator for the function table section
pub struct FunctionTableGenerator;

impl FunctionTableGenerator {
    /// Generate the function table section
    pub fn generate(writer: &mut BytecodeWriter, functions: &[TranspiledFunction]) -> BytecodeResult<FunctionTable> {
        let mut entries = Vec::new();

        // Write function count
        writer.write_u32(functions.len() as u32)?;

        // Generate entries for each function
        for function in functions {
            let entry = Self::create_function_entry(function)?;
            Self::write_function_entry(writer, &entry)?;
            entries.push(entry);
        }

        Ok(FunctionTable { entries })
    }

    /// Create a function entry from a transpiled function
    fn create_function_entry(function: &TranspiledFunction) -> BytecodeResult<FunctionEntry> {
        let flags = MetadataFlags {
            is_exported: function.is_exported,
            is_imported: false, // Will be set during import resolution
            has_debug_info: function.debug_info.is_some(),
            is_optimized: false, // Will be set during optimization
        };

        Ok(FunctionEntry {
            name_offset: 0, // Will be filled during string table generation
            code_offset: 0, // Will be filled during code generation
            code_size: 0,   // Will be filled during code generation
            param_count: function.param_count as u16,
            local_count: function.local_count as u16,
            flags,
        })
    }

    /// Write a single function entry to the bytecode
    fn write_function_entry(writer: &mut BytecodeWriter, entry: &FunctionEntry) -> BytecodeResult<()> {
        writer.write_u32(entry.name_offset)?;
        writer.write_u32(entry.code_offset)?;
        writer.write_u32(entry.code_size)?;
        writer.write_u16(entry.param_count)?;
        writer.write_u16(entry.local_count)?;
        writer.write_u32(entry.flags.to_bits())?;
        Ok(())
    }

    /// Calculate the size of the function table section
    pub fn calculate_size(function_count: usize) -> usize {
        4 + (function_count * Self::entry_size()) // Count + entries
    }

    /// Size of a single function entry
    pub fn entry_size() -> usize {
        4 + 4 + 4 + 2 + 2 + 4 // name_offset + code_offset + code_size + param_count + local_count + flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::engine::TranspiledInstruction;

    #[test]
    fn test_function_flags() {
        let flags = MetadataFlags {
            is_exported: true,
            is_imported: false,
            has_debug_info: true,
            is_optimized: false,
        };

        let bits = flags.to_bits();
        assert_eq!(bits, 0x05); // 0x01 | 0x04

        let restored_flags = MetadataFlags::from_bits(bits);
        assert_eq!(restored_flags.is_exported, true);
        assert_eq!(restored_flags.is_imported, false);
        assert_eq!(restored_flags.has_debug_info, true);
        assert_eq!(restored_flags.is_optimized, false);
    }

    #[test]
    fn test_function_table_generation() {
        let mut writer = BytecodeWriter::new();

        let functions = vec![TranspiledFunction {
            name: "test_func".to_string(),
            instructions: vec![],
            param_count: 2,
            local_count: 3,
            is_exported: true,
            debug_info: None,
        }];

        let table = FunctionTableGenerator::generate(&mut writer, &functions).unwrap();

        assert_eq!(table.entries.len(), 1);
        assert_eq!(table.entries[0].param_count, 2);
        assert_eq!(table.entries[0].local_count, 3);
        assert!(table.entries[0].flags.is_exported);

        let expected_size = FunctionTableGenerator::calculate_size(1);
        assert_eq!(writer.size(), expected_size);
    }
}
