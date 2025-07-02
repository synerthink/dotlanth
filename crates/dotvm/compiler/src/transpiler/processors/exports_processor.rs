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

//! Export and import processing for transpilation

use super::super::{
    config::TranspilationConfig,
    error::{TranspilationError, TranspilationResult},
    types::{ExportInfo, ExportKind, GlobalType, ImportInfo, ImportKind, TableElementType},
};
use crate::wasm::ast::{WasmExport, WasmExportKind, WasmImport, WasmImportKind};

/// Processor for exports and imports
pub struct ExportsProcessor;

impl ExportsProcessor {
    /// Create a new exports processor
    pub fn new(_config: &TranspilationConfig) -> TranspilationResult<Self> {
        Ok(Self)
    }

    /// Process exports and imports
    pub fn process_exports_imports(&mut self, wasm_exports: &[WasmExport], wasm_imports: &[WasmImport], _config: &TranspilationConfig) -> TranspilationResult<(Vec<ExportInfo>, Vec<ImportInfo>)> {
        let exports = self.process_exports(wasm_exports)?;
        let imports = self.process_imports(wasm_imports)?;
        Ok((exports, imports))
    }

    /// Process exports
    fn process_exports(&self, wasm_exports: &[WasmExport]) -> TranspilationResult<Vec<ExportInfo>> {
        let mut exports = Vec::new();

        for export in wasm_exports {
            let kind = match export.kind {
                WasmExportKind::Function => ExportKind::Function,
                WasmExportKind::Memory => ExportKind::Memory,
                WasmExportKind::Global => ExportKind::Global,
                WasmExportKind::Table => ExportKind::Table,
            };

            exports.push(ExportInfo::new(export.name.clone(), kind, export.index));
        }

        Ok(exports)
    }

    /// Process imports
    fn process_imports(&self, wasm_imports: &[WasmImport]) -> TranspilationResult<Vec<ImportInfo>> {
        let mut imports = Vec::new();

        for import in wasm_imports {
            let kind = match &import.kind {
                WasmImportKind::Function { type_index } => ImportKind::Function { type_index: *type_index },
                WasmImportKind::Memory(memory) => ImportKind::Memory {
                    min_pages: memory.initial_pages(),
                    max_pages: memory.max_pages(),
                },
                WasmImportKind::Global { .. } => ImportKind::Global {
                    var_type: GlobalType::I32, // Simplified
                    is_mutable: false,
                },
                WasmImportKind::Table(_) => ImportKind::Table {
                    element_type: TableElementType::FuncRef,
                    min_size: 0,
                    max_size: None,
                },
            };

            imports.push(ImportInfo::new(import.name.clone(), import.module.clone(), kind));
        }

        Ok(imports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::config::TranspilationConfig;

    #[test]
    fn test_exports_processor_creation() {
        let config = TranspilationConfig::default();
        let processor = ExportsProcessor::new(&config);
        assert!(processor.is_ok());
    }
}
