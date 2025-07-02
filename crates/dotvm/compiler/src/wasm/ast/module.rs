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

//! WebAssembly module definition

use super::{
    instructions::WasmInstruction,
    types::{WasmFunctionType, WasmGlobalType, WasmMemoryType, WasmTableType, WasmValueType},
};
use serde::{Deserialize, Serialize};

/// Complete WebAssembly module
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmModule {
    /// Type section - function signatures
    pub types: Vec<WasmFunctionType>,
    /// Import section
    pub imports: Vec<WasmImport>,
    /// Function section - type indices for functions
    pub function_types: Vec<u32>,
    /// Table section
    pub tables: Vec<WasmTable>,
    /// Memory section
    pub memories: Vec<WasmMemory>,
    /// Global section
    pub globals: Vec<WasmGlobal>,
    /// Export section
    pub exports: Vec<WasmExport>,
    /// Start function index (if any)
    pub start_function: Option<u32>,
    /// Element section
    pub elements: Vec<WasmElement>,
    /// Code section - function bodies
    pub functions: Vec<WasmFunction>,
    /// Data section
    pub data_segments: Vec<WasmDataSegment>,
    /// Custom sections
    pub custom_sections: Vec<WasmCustomSection>,
}

impl WasmModule {
    /// Create a new empty module
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            imports: Vec::new(),
            function_types: Vec::new(),
            tables: Vec::new(),
            memories: Vec::new(),
            globals: Vec::new(),
            exports: Vec::new(),
            start_function: None,
            elements: Vec::new(),
            functions: Vec::new(),
            data_segments: Vec::new(),
            custom_sections: Vec::new(),
        }
    }

    /// Get the total number of functions (imported + defined)
    pub fn total_function_count(&self) -> usize {
        self.import_function_count() + self.functions.len()
    }

    /// Get the number of imported functions
    pub fn import_function_count(&self) -> usize {
        self.imports.iter().filter(|imp| matches!(imp.kind, WasmImportKind::Function { .. })).count()
    }

    /// Get the total number of globals (imported + defined)
    pub fn total_global_count(&self) -> usize {
        self.import_global_count() + self.globals.len()
    }

    /// Get the number of imported globals
    pub fn import_global_count(&self) -> usize {
        self.imports.iter().filter(|imp| matches!(imp.kind, WasmImportKind::Global { .. })).count()
    }

    /// Get the total number of tables (imported + defined)
    pub fn total_table_count(&self) -> usize {
        self.import_table_count() + self.tables.len()
    }

    /// Get the number of imported tables
    pub fn import_table_count(&self) -> usize {
        self.imports.iter().filter(|imp| matches!(imp.kind, WasmImportKind::Table(_))).count()
    }

    /// Get the total number of memories (imported + defined)
    pub fn total_memory_count(&self) -> usize {
        self.import_memory_count() + self.memories.len()
    }

    /// Get the number of imported memories
    pub fn import_memory_count(&self) -> usize {
        self.imports.iter().filter(|imp| matches!(imp.kind, WasmImportKind::Memory(_))).count()
    }

    /// Find an export by name
    pub fn find_export(&self, name: &str) -> Option<&WasmExport> {
        self.exports.iter().find(|exp| exp.name == name)
    }

    /// Find an import by module and name
    pub fn find_import(&self, module: &str, name: &str) -> Option<&WasmImport> {
        self.imports.iter().find(|imp| imp.module == module && imp.name == name)
    }

    /// Get all exported functions
    pub fn exported_functions(&self) -> Vec<&WasmExport> {
        self.exports.iter().filter(|exp| matches!(exp.kind, WasmExportKind::Function)).collect()
    }

    /// Get all exported globals
    pub fn exported_globals(&self) -> Vec<&WasmExport> {
        self.exports.iter().filter(|exp| matches!(exp.kind, WasmExportKind::Global)).collect()
    }

    /// Check if the module has a start function
    pub fn has_start_function(&self) -> bool {
        self.start_function.is_some()
    }

    /// Validate module structure
    pub fn validate(&self) -> Result<(), String> {
        // Check function type indices
        for (i, &type_index) in self.function_types.iter().enumerate() {
            if type_index as usize >= self.types.len() {
                return Err(format!("Function {} references invalid type index {}", i, type_index));
            }
        }

        // Check export indices
        for export in &self.exports {
            match export.kind {
                WasmExportKind::Function => {
                    if export.index as usize >= self.total_function_count() {
                        return Err(format!("Export '{}' references invalid function index {}", export.name, export.index));
                    }
                }
                WasmExportKind::Global => {
                    if export.index as usize >= self.total_global_count() {
                        return Err(format!("Export '{}' references invalid global index {}", export.name, export.index));
                    }
                }
                WasmExportKind::Table => {
                    if export.index as usize >= self.total_table_count() {
                        return Err(format!("Export '{}' references invalid table index {}", export.name, export.index));
                    }
                }
                WasmExportKind::Memory => {
                    if export.index as usize >= self.total_memory_count() {
                        return Err(format!("Export '{}' references invalid memory index {}", export.name, export.index));
                    }
                }
            }
        }

        // Check start function
        if let Some(start_func) = self.start_function {
            if start_func as usize >= self.total_function_count() {
                return Err(format!("Start function references invalid function index {}", start_func));
            }
        }

        Ok(())
    }
}

impl Default for WasmModule {
    fn default() -> Self {
        Self::new()
    }
}

/// WebAssembly function definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmFunction {
    /// Function signature (type)
    pub signature: WasmFunctionType,
    /// Local variables (excluding parameters)
    pub locals: Vec<WasmValueType>,
    /// Function body (instructions)
    pub body: Vec<WasmInstruction>,
}

impl WasmFunction {
    /// Create a new function
    pub fn new(signature: WasmFunctionType, locals: Vec<WasmValueType>, body: Vec<WasmInstruction>) -> Self {
        Self { signature, locals, body }
    }

    /// Get the total number of locals (parameters + locals)
    pub fn total_locals(&self) -> usize {
        self.signature.params.len() + self.locals.len()
    }

    /// Get the number of parameters
    pub fn param_count(&self) -> usize {
        self.signature.params.len()
    }

    /// Get the number of local variables (excluding parameters)
    pub fn local_count(&self) -> usize {
        self.locals.len()
    }

    /// Get the number of instructions
    pub fn instruction_count(&self) -> usize {
        self.body.len()
    }

    /// Check if the function is empty
    pub fn is_empty(&self) -> bool {
        self.body.is_empty()
    }
}

/// WebAssembly global variable
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmGlobal {
    /// Global type
    pub global_type: WasmGlobalType,
    /// Initialization expression
    pub init_expr: Vec<WasmInstruction>,
}

impl WasmGlobal {
    /// Create a new global
    pub fn new(global_type: WasmGlobalType, init_expr: Vec<WasmInstruction>) -> Self {
        Self { global_type, init_expr }
    }

    /// Check if the global is mutable
    pub fn is_mutable(&self) -> bool {
        self.global_type.mutable
    }

    /// Get the value type
    pub fn value_type(&self) -> WasmValueType {
        self.global_type.value_type
    }
}

/// WebAssembly table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmTable {
    /// Table type
    pub table_type: WasmTableType,
}

impl WasmTable {
    /// Create a new table
    pub fn new(table_type: WasmTableType) -> Self {
        Self { table_type }
    }

    /// Get the element type
    pub fn element_type(&self) -> WasmValueType {
        self.table_type.element_type
    }

    /// Get the initial size
    pub fn initial_size(&self) -> u32 {
        self.table_type.initial
    }

    /// Get the maximum size
    pub fn max_size(&self) -> Option<u32> {
        self.table_type.maximum
    }
}

/// WebAssembly memory
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmMemory {
    /// Memory type
    pub memory_type: WasmMemoryType,
}

impl WasmMemory {
    /// Create a new memory
    pub fn new(memory_type: WasmMemoryType) -> Self {
        Self { memory_type }
    }

    /// Get the initial size in pages
    pub fn initial_pages(&self) -> u32 {
        self.memory_type.initial
    }

    /// Get the maximum size in pages
    pub fn max_pages(&self) -> Option<u32> {
        self.memory_type.maximum
    }

    /// Check if memory is shared
    pub fn is_shared(&self) -> bool {
        self.memory_type.shared
    }

    /// Get the initial size in bytes
    pub fn initial_bytes(&self) -> u64 {
        self.memory_type.initial_bytes()
    }
}

/// WebAssembly import
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmImport {
    /// Module name
    pub module: String,
    /// Import name
    pub name: String,
    /// Import kind
    pub kind: WasmImportKind,
}

impl WasmImport {
    /// Create a new import
    pub fn new(module: String, name: String, kind: WasmImportKind) -> Self {
        Self { module, name, kind }
    }

    /// Create a function import
    pub fn function(module: String, name: String, type_index: u32) -> Self {
        Self::new(module, name, WasmImportKind::Function { type_index })
    }

    /// Create a global import
    pub fn global(module: String, name: String, value_type: WasmValueType, mutable: bool) -> Self {
        Self::new(module, name, WasmImportKind::Global { value_type, mutable })
    }

    /// Get the import key (module::name)
    pub fn key(&self) -> String {
        format!("{}::{}", self.module, self.name)
    }
}

/// WebAssembly import kinds
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WasmImportKind {
    Function { type_index: u32 },
    Table(WasmTable),
    Memory(WasmMemory),
    Global { value_type: WasmValueType, mutable: bool },
}

/// WebAssembly export
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmExport {
    /// Export name
    pub name: String,
    /// Export kind
    pub kind: WasmExportKind,
    /// Index of the exported item
    pub index: u32,
}

impl WasmExport {
    /// Create a new export
    pub fn new(name: String, kind: WasmExportKind, index: u32) -> Self {
        Self { name, kind, index }
    }

    /// Create a function export
    pub fn function(name: String, index: u32) -> Self {
        Self::new(name, WasmExportKind::Function, index)
    }

    /// Create a global export
    pub fn global(name: String, index: u32) -> Self {
        Self::new(name, WasmExportKind::Global, index)
    }

    /// Create a memory export
    pub fn memory(name: String, index: u32) -> Self {
        Self::new(name, WasmExportKind::Memory, index)
    }

    /// Create a table export
    pub fn table(name: String, index: u32) -> Self {
        Self::new(name, WasmExportKind::Table, index)
    }
}

/// WebAssembly export kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WasmExportKind {
    Function,
    Table,
    Memory,
    Global,
}

impl WasmExportKind {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Table => "table",
            Self::Memory => "memory",
            Self::Global => "global",
        }
    }
}

/// WebAssembly element segment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmElement {
    /// Table index
    pub table_index: u32,
    /// Offset expression
    pub offset: Vec<WasmInstruction>,
    /// Function indices
    pub functions: Vec<u32>,
}

/// WebAssembly data segment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmDataSegment {
    /// Memory index
    pub memory_index: u32,
    /// Offset expression
    pub offset: Vec<WasmInstruction>,
    /// Data bytes
    pub data: Vec<u8>,
}

impl WasmDataSegment {
    /// Create a new data segment
    pub fn new(memory_index: u32, offset: Vec<WasmInstruction>, data: Vec<u8>) -> Self {
        Self { memory_index, offset, data }
    }

    /// Get the size of the data
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Check if the segment is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// WebAssembly custom section
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmCustomSection {
    /// Section name
    pub name: String,
    /// Section data
    pub data: Vec<u8>,
}

impl WasmCustomSection {
    /// Create a new custom section
    pub fn new(name: String, data: Vec<u8>) -> Self {
        Self { name, data }
    }

    /// Get the size of the section
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Check if this is a name section
    pub fn is_name_section(&self) -> bool {
        self.name == "name"
    }

    /// Check if this is a producers section
    pub fn is_producers_section(&self) -> bool {
        self.name == "producers"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creation() {
        let module = WasmModule::new();
        assert_eq!(module.total_function_count(), 0);
        assert_eq!(module.total_global_count(), 0);
        assert!(!module.has_start_function());
    }

    #[test]
    fn test_function_counts() {
        let mut module = WasmModule::new();

        // Add an imported function
        module.imports.push(WasmImport::function("env".to_string(), "print".to_string(), 0));

        // Add a defined function
        module.functions.push(WasmFunction::new(WasmFunctionType::empty(), vec![], vec![]));

        assert_eq!(module.import_function_count(), 1);
        assert_eq!(module.total_function_count(), 2);
    }

    #[test]
    fn test_export_finding() {
        let mut module = WasmModule::new();
        module.exports.push(WasmExport::function("main".to_string(), 0));

        let export = module.find_export("main");
        assert!(export.is_some());
        assert_eq!(export.unwrap().index, 0);

        let missing = module.find_export("missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_function_properties() {
        let func = WasmFunction::new(
            WasmFunctionType::new(vec![WasmValueType::I32], vec![WasmValueType::I32]),
            vec![WasmValueType::I64],
            vec![WasmInstruction::LocalGet { local_index: 0 }],
        );

        assert_eq!(func.param_count(), 1);
        assert_eq!(func.local_count(), 1);
        assert_eq!(func.total_locals(), 2);
        assert_eq!(func.instruction_count(), 1);
        assert!(!func.is_empty());
    }

    #[test]
    fn test_data_segment() {
        let segment = WasmDataSegment::new(0, vec![WasmInstruction::I32Const { value: 0 }], vec![1, 2, 3, 4]);

        assert_eq!(segment.memory_index, 0);
        assert_eq!(segment.size(), 4);
        assert!(!segment.is_empty());
    }
}
