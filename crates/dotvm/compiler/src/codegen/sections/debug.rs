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

//! Debug information generator

use crate::{
    codegen::{error::BytecodeResult, writers::BytecodeWriter},
    transpiler::types::TranspiledFunction,
};

/// Debug information for a single function
#[derive(Debug, Clone)]
pub struct FunctionDebugInfo {
    pub function_index: u32,
    pub source_file_offset: u32,
    pub line_number_table_offset: u32,
    pub local_variable_table_offset: u32,
}

/// Line number mapping entry
#[derive(Debug, Clone)]
pub struct LineNumberEntry {
    pub bytecode_offset: u32,
    pub line_number: u32,
}

/// Local variable information
#[derive(Debug, Clone)]
pub struct LocalVariableInfo {
    pub name_offset: u32,
    pub type_offset: u32,
    pub start_offset: u32,
    pub end_offset: u32,
    pub local_index: u16,
}

/// Complete debug information
#[derive(Debug, Clone)]
pub struct DebugInfo {
    pub functions: Vec<FunctionDebugInfo>,
    pub line_numbers: Vec<LineNumberEntry>,
    pub local_variables: Vec<LocalVariableInfo>,
}

/// Generator for debug information
pub struct DebugInfoGenerator;

impl DebugInfoGenerator {
    /// Generate debug information section
    pub fn generate(writer: &mut BytecodeWriter, functions: &[TranspiledFunction], include_debug_info: bool) -> BytecodeResult<DebugInfo> {
        if !include_debug_info {
            // Write empty debug section
            writer.write_u32(0)?; // Function count
            writer.write_u32(0)?; // Line number count
            writer.write_u32(0)?; // Local variable count

            return Ok(DebugInfo {
                functions: Vec::new(),
                line_numbers: Vec::new(),
                local_variables: Vec::new(),
            });
        }

        let mut debug_info = DebugInfo {
            functions: Vec::new(),
            line_numbers: Vec::new(),
            local_variables: Vec::new(),
        };

        // Generate debug info for each function
        for (index, function) in functions.iter().enumerate() {
            if let Some(ref debug_data) = function.debug_info {
                let function_debug = Self::generate_function_debug_info(writer, index as u32, function, debug_data, &mut debug_info)?;
                debug_info.functions.push(function_debug);
            }
        }

        // Write debug tables
        Self::write_debug_tables(writer, &debug_info)?;

        Ok(debug_info)
    }

    /// Generate debug info for a single function
    fn generate_function_debug_info(
        _writer: &mut BytecodeWriter,
        function_index: u32,
        _function: &TranspiledFunction,
        _debug_data: &str, // TODO: Define proper debug data structure
        debug_info: &mut DebugInfo,
    ) -> BytecodeResult<FunctionDebugInfo> {
        // TODO: Parse debug data and generate proper debug information
        // For now, create minimal debug info

        let function_debug = FunctionDebugInfo {
            function_index,
            source_file_offset: 0, // Will be filled during string table generation
            line_number_table_offset: debug_info.line_numbers.len() as u32,
            local_variable_table_offset: debug_info.local_variables.len() as u32,
        };

        // Add dummy line number entry
        debug_info.line_numbers.push(LineNumberEntry { bytecode_offset: 0, line_number: 1 });

        Ok(function_debug)
    }

    /// Write debug tables to bytecode
    fn write_debug_tables(writer: &mut BytecodeWriter, debug_info: &DebugInfo) -> BytecodeResult<()> {
        // Write function debug info count
        writer.write_u32(debug_info.functions.len() as u32)?;

        // Write function debug info entries
        for function_debug in &debug_info.functions {
            Self::write_function_debug_info(writer, function_debug)?;
        }

        // Write line number table
        writer.write_u32(debug_info.line_numbers.len() as u32)?;
        for line_entry in &debug_info.line_numbers {
            Self::write_line_number_entry(writer, line_entry)?;
        }

        // Write local variable table
        writer.write_u32(debug_info.local_variables.len() as u32)?;
        for local_var in &debug_info.local_variables {
            Self::write_local_variable_info(writer, local_var)?;
        }

        Ok(())
    }

    /// Write function debug info entry
    fn write_function_debug_info(writer: &mut BytecodeWriter, info: &FunctionDebugInfo) -> BytecodeResult<()> {
        writer.write_u32(info.function_index)?;
        writer.write_u32(info.source_file_offset)?;
        writer.write_u32(info.line_number_table_offset)?;
        writer.write_u32(info.local_variable_table_offset)?;
        Ok(())
    }

    /// Write line number entry
    fn write_line_number_entry(writer: &mut BytecodeWriter, entry: &LineNumberEntry) -> BytecodeResult<()> {
        writer.write_u32(entry.bytecode_offset)?;
        writer.write_u32(entry.line_number)?;
        Ok(())
    }

    /// Write local variable info
    fn write_local_variable_info(writer: &mut BytecodeWriter, info: &LocalVariableInfo) -> BytecodeResult<()> {
        writer.write_u32(info.name_offset)?;
        writer.write_u32(info.type_offset)?;
        writer.write_u32(info.start_offset)?;
        writer.write_u32(info.end_offset)?;
        writer.write_u16(info.local_index)?;
        Ok(())
    }

    /// Calculate the minimum size of the debug section
    pub fn calculate_min_size() -> usize {
        4 + 4 + 4 // function_count + line_number_count + local_variable_count
    }
}

// TODO: Implement SectionGenerator trait when the framework is ready
// impl SectionGenerator for DebugInfoGenerator {
//     fn generate(&self, _context: &GenerationContext) -> BytecodeResult<Vec<u8>> {
//         Err(BytecodeGenerationError::SerializationError(
//             "Debug info generator not yet hooked into SectionGenerator trait".into(),
//         ))
//     }
//
//     fn size_estimate(&self, _context: &GenerationContext) -> usize {
//         DebugInfoGenerator::calculate_min_size()
//     }
//
//     fn section_type(&self) -> SectionType {
//         SectionType::DebugInfo
//     }
//
//     fn dependencies(&self) -> &'static [SectionType] {
//         &[]
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::types::TranspiledFunction;

    #[test]
    fn test_empty_debug_info() {
        let mut writer = BytecodeWriter::new();
        let functions = vec![];

        let debug_info = DebugInfoGenerator::generate(&mut writer, &functions, false).unwrap();

        assert!(debug_info.functions.is_empty());
        assert!(debug_info.line_numbers.is_empty());
        assert!(debug_info.local_variables.is_empty());

        assert_eq!(writer.size(), DebugInfoGenerator::calculate_min_size());
    }

    #[test]
    fn test_debug_info_with_function() {
        let mut writer = BytecodeWriter::new();

        let functions = vec![TranspiledFunction {
            name: "test".to_string(),
            instructions: vec![],
            param_count: 0,
            local_count: 0,
            is_exported: false,
            debug_info: Some("test.rs:1".to_string()),
            metadata: crate::transpiler::types::function::FunctionMetadata::default(),
        }];

        let debug_info = DebugInfoGenerator::generate(&mut writer, &functions, true).unwrap();

        assert_eq!(debug_info.functions.len(), 1);
        assert_eq!(debug_info.line_numbers.len(), 1);
        assert!(writer.size() > DebugInfoGenerator::calculate_min_size());
    }
}
