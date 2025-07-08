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

//! Data section generator

use crate::{
    codegen::{error::BytecodeResult, writers::BytecodeWriter},
    transpiler::types::TranspiledModule,
};

/// Generator for the data section
pub struct DataGenerator;

impl DataGenerator {
    /// Generate the data section
    pub fn generate(writer: &mut BytecodeWriter, module: &TranspiledModule) -> BytecodeResult<()> {
        // Write string table
        Self::generate_string_table(writer, module)?;

        // Write constant pool
        Self::generate_constant_pool(writer, module)?;

        // Write global variables
        Self::generate_global_variables(writer, module)?;

        Ok(())
    }

    /// Generate the string table
    fn generate_string_table(writer: &mut BytecodeWriter, module: &TranspiledModule) -> BytecodeResult<()> {
        // Collect all strings from the module
        let mut strings = Vec::new();

        // Add function names
        for function in &module.functions {
            strings.push(function.name.clone());
        }

        // Add export names
        for export in &module.exports {
            strings.push(export.name.clone());
        }

        // Add import names
        for import in &module.imports {
            strings.push(import.name.clone());
            strings.push(import.module_name.clone());
        }

        // Remove duplicates and sort for deterministic output
        strings.sort();
        strings.dedup();

        // Write string count
        writer.write_u32(strings.len() as u32)?;

        // Write each string
        for string in &strings {
            writer.write_string(string)?;
        }

        Ok(())
    }

    /// Generate the constant pool
    fn generate_constant_pool(writer: &mut BytecodeWriter, _module: &TranspiledModule) -> BytecodeResult<()> {
        // TODO: Implement constant pool generation
        // For now, write empty constant pool
        writer.write_u32(0)?; // Constant count
        Ok(())
    }

    /// Generate global variables section
    fn generate_global_variables(writer: &mut BytecodeWriter, _module: &TranspiledModule) -> BytecodeResult<()> {
        // TODO: Implement global variables generation
        // For now, write empty globals section
        writer.write_u32(0)?; // Global count
        Ok(())
    }

    /// Calculate the minimum size of the data section
    pub fn calculate_min_size() -> usize {
        4 + 4 + 4 // string_count + constant_count + global_count
    }
}

// TODO: Implement SectionGenerator trait when the framework is ready
// impl SectionGenerator for DataGenerator {
//     fn generate(&self, context: &GenerationContext) -> BytecodeResult<Vec<u8>> {
//         // TODO: hook into writer-based API
//         Err(BytecodeGenerationError::SerializationError(
//             "Data section generator not yet hooked into SectionGenerator trait".into(),
//         ))
//     }
//
//     fn size_estimate(&self, _context: &GenerationContext) -> usize {
//         Self::calculate_min_size()
//     }
//
//     fn section_type(&self) -> SectionType {
//         SectionType::Data
//     }
//
//     fn dependencies(&self) -> &'static [SectionType] {
//         &[]
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::types::{ExportInfo, ExportKind, ImportInfo, ImportKind, TranspiledFunction};
    use dotvm_core::bytecode::{BytecodeHeader, VmArchitecture};

    #[test]
    fn test_data_section_generation() {
        let mut writer = BytecodeWriter::new();

        let module = TranspiledModule {
            header: BytecodeHeader::new(VmArchitecture::Arch64),
            functions: vec![TranspiledFunction {
                name: "main".to_string(),
                instructions: vec![],
                param_count: 0,
                local_count: 0,
                is_exported: true,
                debug_info: None,
                metadata: crate::transpiler::types::function::FunctionMetadata::default(),
            }],
            globals: vec![],
            memory_layout: crate::transpiler::types::variables::MemoryLayout {
                initial_pages: 1,
                maximum_pages: None,
                page_size: 65536,
                segments: Vec::new(),
                protection: crate::transpiler::types::variables::MemoryProtection::default(),
            },
            exports: vec![ExportInfo {
                name: "main".to_string(),
                kind: ExportKind::Function,
                index: 0,
                description: None,
                is_public: true,
            }],
            imports: vec![ImportInfo {
                name: "print".to_string(),
                module_name: "env".to_string(),
                kind: ImportKind::Function { type_index: 0 },
                description: None,
                is_required: true,
            }],
            metadata: crate::transpiler::types::module::ModuleMetadata::default(),
        };

        DataGenerator::generate(&mut writer, &module).unwrap();

        // Should have at least the minimum size
        assert!(writer.size() >= DataGenerator::calculate_min_size());
    }
}
