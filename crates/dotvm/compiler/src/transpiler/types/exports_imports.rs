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

//! Export and import type definitions for transpilation

/// Export information
#[derive(Debug, Clone)]
pub struct ExportInfo {
    /// Export name
    pub name: String,
    /// Export kind
    pub kind: ExportKind,
    /// Index of the exported item
    pub index: u32,
    /// Optional description
    pub description: Option<String>,
    /// Whether this export is public
    pub is_public: bool,
}

impl ExportInfo {
    /// Create a new export
    pub fn new(name: String, kind: ExportKind, index: u32) -> Self {
        Self {
            name,
            kind,
            index,
            description: None,
            is_public: true,
        }
    }

    /// Create a function export
    pub fn function(name: String, index: u32) -> Self {
        Self::new(name, ExportKind::Function, index)
    }

    /// Create a memory export
    pub fn memory(name: String, index: u32) -> Self {
        Self::new(name, ExportKind::Memory, index)
    }

    /// Create a global export
    pub fn global(name: String, index: u32) -> Self {
        Self::new(name, ExportKind::Global, index)
    }

    /// Create a table export
    pub fn table(name: String, index: u32) -> Self {
        Self::new(name, ExportKind::Table, index)
    }

    /// Set the description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Mark as private
    pub fn mark_private(mut self) -> Self {
        self.is_public = false;
        self
    }

    /// Check if this is a function export
    pub fn is_function(&self) -> bool {
        matches!(self.kind, ExportKind::Function)
    }

    /// Check if this is a memory export
    pub fn is_memory(&self) -> bool {
        matches!(self.kind, ExportKind::Memory)
    }

    /// Check if this is a global export
    pub fn is_global(&self) -> bool {
        matches!(self.kind, ExportKind::Global)
    }

    /// Check if this is a table export
    pub fn is_table(&self) -> bool {
        matches!(self.kind, ExportKind::Table)
    }
}

/// Export kind enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    /// Function export
    Function,
    /// Memory export
    Memory,
    /// Global variable export
    Global,
    /// Table export
    Table,
}

impl ExportKind {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Memory => "memory",
            Self::Global => "global",
            Self::Table => "table",
        }
    }

    /// Convert to u8 for serialization
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Function => 0,
            Self::Memory => 1,
            Self::Global => 2,
            Self::Table => 3,
        }
    }

    /// Create from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Function),
            1 => Some(Self::Memory),
            2 => Some(Self::Global),
            3 => Some(Self::Table),
            _ => None,
        }
    }
}

/// Import information
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// Import name
    pub name: String,
    /// Module name this import comes from
    pub module_name: String,
    /// Import kind
    pub kind: ImportKind,
    /// Optional description
    pub description: Option<String>,
    /// Whether this import is required
    pub is_required: bool,
}

impl ImportInfo {
    /// Create a new import
    pub fn new(name: String, module_name: String, kind: ImportKind) -> Self {
        Self {
            name,
            module_name,
            kind,
            description: None,
            is_required: true,
        }
    }

    /// Create a function import
    pub fn function(name: String, module_name: String, type_index: u32) -> Self {
        Self::new(name, module_name, ImportKind::Function { type_index })
    }

    /// Create a memory import
    pub fn memory(name: String, module_name: String, min_pages: u32, max_pages: Option<u32>) -> Self {
        Self::new(name, module_name, ImportKind::Memory { min_pages, max_pages })
    }

    /// Create a global import
    pub fn global(name: String, module_name: String, var_type: GlobalType, is_mutable: bool) -> Self {
        Self::new(name, module_name, ImportKind::Global { var_type, is_mutable })
    }

    /// Create a table import
    pub fn table(name: String, module_name: String, element_type: TableElementType, min_size: u32, max_size: Option<u32>) -> Self {
        Self::new(name, module_name, ImportKind::Table { element_type, min_size, max_size })
    }

    /// Set the description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Mark as optional
    pub fn mark_optional(mut self) -> Self {
        self.is_required = false;
        self
    }

    /// Check if this is a function import
    pub fn is_function(&self) -> bool {
        matches!(self.kind, ImportKind::Function { .. })
    }

    /// Check if this is a memory import
    pub fn is_memory(&self) -> bool {
        matches!(self.kind, ImportKind::Memory { .. })
    }

    /// Check if this is a global import
    pub fn is_global(&self) -> bool {
        matches!(self.kind, ImportKind::Global { .. })
    }

    /// Check if this is a table import
    pub fn is_table(&self) -> bool {
        matches!(self.kind, ImportKind::Table { .. })
    }
}

/// Import kind enumeration
#[derive(Debug, Clone, Copy)]
pub enum ImportKind {
    /// Function import with type index
    Function { type_index: u32 },
    /// Memory import with size constraints
    Memory { min_pages: u32, max_pages: Option<u32> },
    /// Global import with type information
    Global { var_type: GlobalType, is_mutable: bool },
    /// Table import with element type and size constraints
    Table { element_type: TableElementType, min_size: u32, max_size: Option<u32> },
}

impl ImportKind {
    /// Convert to u8 for serialization
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Function { .. } => 0,
            Self::Memory { .. } => 1,
            Self::Global { .. } => 2,
            Self::Table { .. } => 3,
        }
    }

    /// Get the type index for function imports
    pub fn function_type_index(&self) -> Option<u32> {
        match self {
            Self::Function { type_index } => Some(*type_index),
            _ => None,
        }
    }

    /// Get memory constraints for memory imports
    pub fn memory_constraints(&self) -> Option<(u32, Option<u32>)> {
        match self {
            Self::Memory { min_pages, max_pages } => Some((*min_pages, *max_pages)),
            _ => None,
        }
    }
}

/// Global variable type for imports/exports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalType {
    I32,
    I64,
    F32,
    F64,
    V128,
    ExternRef,
    FuncRef,
}

impl GlobalType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::V128 => "v128",
            Self::ExternRef => "externref",
            Self::FuncRef => "funcref",
        }
    }

    /// Get the size in bytes
    pub fn size_bytes(&self) -> u32 {
        match self {
            Self::I32 | Self::F32 => 4,
            Self::I64 | Self::F64 | Self::ExternRef | Self::FuncRef => 8,
            Self::V128 => 16,
        }
    }
}

/// Table element type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableElementType {
    /// Function reference
    FuncRef,
    /// External reference
    ExternRef,
}

impl TableElementType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FuncRef => "funcref",
            Self::ExternRef => "externref",
        }
    }

    /// Get the size in bytes
    pub fn size_bytes(&self) -> u32 {
        8 // All references are pointer-sized
    }
}
