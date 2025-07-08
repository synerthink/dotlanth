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

//! Export and import table generators

use crate::{
    codegen::{
        error::{BytecodeGenerationError, BytecodeResult},
        sections::function_table::FunctionTable,
        writers::BytecodeWriter,
    },
    transpiler::types::{ExportInfo, ExportKind, ImportInfo, ImportKind},
};

/// Export table entry
#[derive(Debug, Clone)]
pub struct ExportEntry {
    pub name_offset: u32,
    pub kind: ExportKind,
    pub index: u32,
}

/// Export table containing all exports
#[derive(Debug, Clone)]
pub struct ExportTable {
    pub entries: Vec<ExportEntry>,
}

/// Import table entry
#[derive(Debug, Clone)]
pub struct ImportEntry {
    pub name_offset: u32,
    pub module_name_offset: u32,
    pub kind: ImportKind,
    pub index: u32,
}

/// Import table containing all imports
#[derive(Debug, Clone)]
pub struct ImportTable {
    pub entries: Vec<ImportEntry>,
}

/// Generator for export tables
pub struct ExportTableGenerator;

impl ExportTableGenerator {
    /// Generate the export table
    pub fn generate(writer: &mut BytecodeWriter, exports: &[ExportInfo], function_table: &FunctionTable) -> BytecodeResult<ExportTable> {
        let mut entries = Vec::new();

        // Write export count
        writer.write_u32(exports.len() as u32)?;

        // Generate entries for each export
        for export in exports {
            let entry = Self::create_export_entry(export, function_table)?;
            Self::write_export_entry(writer, &entry)?;
            entries.push(entry);
        }

        Ok(ExportTable { entries })
    }

    /// Create an export entry from export info
    fn create_export_entry(export: &ExportInfo, function_table: &FunctionTable) -> BytecodeResult<ExportEntry> {
        // Validate the export index
        match export.kind {
            ExportKind::Function => {
                if export.index as usize >= function_table.entries.len() {
                    return Err(BytecodeGenerationError::ExportResolutionError(format!("Function export index {} out of bounds", export.index)));
                }
            }
            ExportKind::Memory | ExportKind::Global | ExportKind::Table => {
                // TODO: Validate memory, global, and table exports when implemented
            }
        }

        Ok(ExportEntry {
            name_offset: 0, // Will be filled during string table generation
            kind: export.kind,
            index: export.index,
        })
    }

    /// Write a single export entry
    fn write_export_entry(writer: &mut BytecodeWriter, entry: &ExportEntry) -> BytecodeResult<()> {
        writer.write_u32(entry.name_offset)?;
        writer.write_u8(entry.kind as u8)?;
        writer.write_u32(entry.index)?;
        Ok(())
    }

    /// Calculate the size of the export table
    pub fn calculate_size(export_count: usize) -> usize {
        4 + (export_count * Self::entry_size()) // Count + entries
    }

    /// Size of a single export entry
    pub fn entry_size() -> usize {
        4 + 1 + 4 // name_offset + kind + index
    }
}

/// Generator for import tables
pub struct ImportTableGenerator;

impl ImportTableGenerator {
    /// Generate the import table
    pub fn generate(writer: &mut BytecodeWriter, imports: &[ImportInfo]) -> BytecodeResult<ImportTable> {
        let mut entries = Vec::new();

        // Write import count
        writer.write_u32(imports.len() as u32)?;

        // Generate entries for each import
        for import in imports {
            let entry = Self::create_import_entry(import)?;
            Self::write_import_entry(writer, &entry)?;
            entries.push(entry);
        }

        Ok(ImportTable { entries })
    }

    /// Create an import entry from import info
    fn create_import_entry(import: &ImportInfo) -> BytecodeResult<ImportEntry> {
        Ok(ImportEntry {
            name_offset: 0,        // Will be filled during string table generation
            module_name_offset: 0, // Will be filled during string table generation
            kind: import.kind,
            index: 0, // Will be assigned during linking
        })
    }

    /// Write a single import entry
    fn write_import_entry(writer: &mut BytecodeWriter, entry: &ImportEntry) -> BytecodeResult<()> {
        writer.write_u32(entry.name_offset)?;
        writer.write_u32(entry.module_name_offset)?;
        writer.write_u8(entry.kind.to_u8())?;
        writer.write_u32(entry.index)?;
        Ok(())
    }

    /// Calculate the size of the import table
    pub fn calculate_size(import_count: usize) -> usize {
        4 + (import_count * Self::entry_size()) // Count + entries
    }

    /// Size of a single import entry
    pub fn entry_size() -> usize {
        4 + 4 + 1 + 4 // name_offset + module_name_offset + kind + index
    }
}

// TODO: Implement SectionGenerator trait when the framework is ready
// impl SectionGenerator for ExportTableGenerator {
//     fn generate(&self, context: &GenerationContext) -> BytecodeResult<Vec<u8>> {
//         // TODO: hook into writer-based API
//         Err(BytecodeGenerationError::SerializationError(
//             "Export table generator not yet hooked into SectionGenerator trait".into(),
//         ))
//     }
//
//     fn size_estimate(&self, context: &GenerationContext) -> usize {
//         // We cannot estimate without export count, so return 0
//         0
//     }
//
//     fn section_type(&self) -> SectionType {
//         SectionType::ExportTable
//     }
//
//     fn dependencies(&self) -> &'static [SectionType] {
//         &[SectionType::FunctionTable]
//     }
// }

// TODO: Implement SectionGenerator trait when the framework is ready
// impl SectionGenerator for ImportTableGenerator {
//     fn generate(&self, _context: &GenerationContext) -> BytecodeResult<Vec<u8>> {
//         Err(BytecodeGenerationError::SerializationError(
//             "Import table generator not yet hooked into SectionGenerator trait".into(),
//         ))
//     }
//
//     fn size_estimate(&self, _context: &GenerationContext) -> usize {
//         0
//     }
//
//     fn section_type(&self) -> SectionType {
//         SectionType::ImportTable
//     }
//
//     fn dependencies(&self) -> &'static [SectionType] {
//         &[]
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::sections::function_table::{FunctionEntry, MetadataFlags};

    #[test]
    fn test_export_table_generation() {
        let mut writer = BytecodeWriter::new();

        let exports = vec![ExportInfo {
            name: "main".to_string(),
            kind: ExportKind::Function,
            index: 0,
            description: None,
            is_public: true,
        }];

        let function_table = FunctionTable {
            entries: vec![FunctionEntry {
                name_offset: 0,
                code_offset: 0,
                code_size: 0,
                param_count: 0,
                local_count: 0,
                flags: MetadataFlags::default(),
            }],
        };

        let table = ExportTableGenerator::generate(&mut writer, &exports, &function_table).unwrap();

        assert_eq!(table.entries.len(), 1);
        assert_eq!(table.entries[0].kind, ExportKind::Function);
        assert_eq!(table.entries[0].index, 0);

        let expected_size = ExportTableGenerator::calculate_size(1);
        assert_eq!(writer.size(), expected_size);
    }

    #[test]
    fn test_import_table_generation() {
        let mut writer = BytecodeWriter::new();

        let imports = vec![ImportInfo {
            name: "print".to_string(),
            module_name: "env".to_string(),
            kind: ImportKind::Function { type_index: 0 },
            description: None,
            is_required: true,
        }];

        let table = ImportTableGenerator::generate(&mut writer, &imports).unwrap();

        assert_eq!(table.entries.len(), 1);
        assert!(matches!(table.entries[0].kind, ImportKind::Function { .. }));

        let expected_size = ImportTableGenerator::calculate_size(1);
        assert_eq!(writer.size(), expected_size);
    }

    #[test]
    fn test_export_validation() {
        let exports = vec![ExportInfo {
            name: "invalid".to_string(),
            kind: ExportKind::Function,
            index: 999, // Out of bounds
            description: None,
            is_public: true,
        }];

        let function_table = FunctionTable {
            entries: vec![], // Empty function table
        };

        let result = ExportTableGenerator::create_export_entry(&exports[0], &function_table);
        assert!(result.is_err());
    }
}
