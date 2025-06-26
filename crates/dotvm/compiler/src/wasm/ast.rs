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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a WebAssembly value type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WasmValueType {
    I32,
    I64,
    F32,
    F64,
    V128,
    FuncRef,
    ExternRef,
}

/// Represents a WebAssembly function signature
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmFunctionType {
    pub params: Vec<WasmValueType>,
    pub results: Vec<WasmValueType>,
}

/// Represents a WebAssembly instruction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WasmInstruction {
    // Control flow
    Unreachable,
    Nop,
    Block { block_type: Option<WasmValueType> },
    Loop { block_type: Option<WasmValueType> },
    If { block_type: Option<WasmValueType> },
    Else,
    End,
    Br { label_index: u32 },
    BrIf { label_index: u32 },
    BrTable { labels: Vec<u32>, default: u32 },
    Return,
    Call { function_index: u32 },
    CallIndirect { type_index: u32, table_index: u32 },

    // Parametric
    Drop,
    Select,
    SelectWithType { types: Vec<WasmValueType> },

    // Variable access
    LocalGet { local_index: u32 },
    LocalSet { local_index: u32 },
    LocalTee { local_index: u32 },
    GlobalGet { global_index: u32 },
    GlobalSet { global_index: u32 },

    // Memory operations
    I32Load { memarg: MemArg },
    I64Load { memarg: MemArg },
    F32Load { memarg: MemArg },
    F64Load { memarg: MemArg },
    I32Load8S { memarg: MemArg },
    I32Load8U { memarg: MemArg },
    I32Load16S { memarg: MemArg },
    I32Load16U { memarg: MemArg },
    I64Load8S { memarg: MemArg },
    I64Load8U { memarg: MemArg },
    I64Load16S { memarg: MemArg },
    I64Load16U { memarg: MemArg },
    I64Load32S { memarg: MemArg },
    I64Load32U { memarg: MemArg },
    I32Store { memarg: MemArg },
    I64Store { memarg: MemArg },
    F32Store { memarg: MemArg },
    F64Store { memarg: MemArg },
    I32Store8 { memarg: MemArg },
    I32Store16 { memarg: MemArg },
    I64Store8 { memarg: MemArg },
    I64Store16 { memarg: MemArg },
    I64Store32 { memarg: MemArg },
    MemorySize,
    MemoryGrow,

    // Numeric operations - I32
    I32Const { value: i32 },
    I32Eqz,
    I32Eq,
    I32Ne,
    I32LtS,
    I32LtU,
    I32GtS,
    I32GtU,
    I32LeS,
    I32LeU,
    I32GeS,
    I32GeU,
    I32Clz,
    I32Ctz,
    I32Popcnt,
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32ShrU,
    I32Rotl,
    I32Rotr,

    // Numeric operations - I64
    I64Const { value: i64 },
    I64Eqz,
    I64Eq,
    I64Ne,
    I64LtS,
    I64LtU,
    I64GtS,
    I64GtU,
    I64LeS,
    I64LeU,
    I64GeS,
    I64GeU,
    I64Clz,
    I64Ctz,
    I64Popcnt,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64DivU,
    I64RemS,
    I64RemU,
    I64And,
    I64Or,
    I64Xor,
    I64Shl,
    I64ShrS,
    I64ShrU,
    I64Rotl,
    I64Rotr,

    // Numeric operations - F32
    F32Const { value: f32 },
    F32Eq,
    F32Ne,
    F32Lt,
    F32Gt,
    F32Le,
    F32Ge,
    F32Abs,
    F32Neg,
    F32Ceil,
    F32Floor,
    F32Trunc,
    F32Nearest,
    F32Sqrt,
    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    F32Min,
    F32Max,
    F32Copysign,

    // Numeric operations - F64
    F64Const { value: f64 },
    F64Eq,
    F64Ne,
    F64Lt,
    F64Gt,
    F64Le,
    F64Ge,
    F64Abs,
    F64Neg,
    F64Ceil,
    F64Floor,
    F64Trunc,
    F64Nearest,
    F64Sqrt,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Min,
    F64Max,
    F64Copysign,

    // Conversion operations
    I32WrapI64,
    I32TruncF32S,
    I32TruncF32U,
    I32TruncF64S,
    I32TruncF64U,
    I64ExtendI32S,
    I64ExtendI32U,
    I64TruncF32S,
    I64TruncF32U,
    I64TruncF64S,
    I64TruncF64U,
    F32ConvertI32S,
    F32ConvertI32U,
    F32ConvertI64S,
    F32ConvertI64U,
    F32DemoteF64,
    F64ConvertI32S,
    F64ConvertI32U,
    F64ConvertI64S,
    F64ConvertI64U,
    F64PromoteF32,
    I32ReinterpretF32,
    I64ReinterpretF64,
    F32ReinterpretI32,
    F64ReinterpretI64,
}

/// Memory argument for memory operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemArg {
    pub offset: u64,
    pub align: u32,
}

/// Represents a WebAssembly function body
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmFunction {
    pub signature: WasmFunctionType,
    pub locals: Vec<WasmValueType>,
    pub body: Vec<WasmInstruction>,
}

/// Represents a WebAssembly global
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmGlobal {
    pub value_type: WasmValueType,
    pub mutable: bool,
    pub init_expr: Vec<WasmInstruction>,
}

/// Represents a WebAssembly memory
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmMemory {
    pub min_pages: u32,
    pub max_pages: Option<u32>,
    pub shared: bool,
}

/// Represents a WebAssembly table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmTable {
    pub element_type: WasmValueType,
    pub initial: u32,
    pub maximum: Option<u32>,
}

/// Represents a WebAssembly export
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmExport {
    pub name: String,
    pub kind: WasmExportKind,
    pub index: u32,
}

/// WebAssembly export kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WasmExportKind {
    Function,
    Table,
    Memory,
    Global,
}

/// Represents a WebAssembly import
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmImport {
    pub module: String,
    pub name: String,
    pub kind: WasmImportKind,
}

/// WebAssembly import kinds
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WasmImportKind {
    Function { type_index: u32 },
    Table(WasmTable),
    Memory(WasmMemory),
    Global { value_type: WasmValueType, mutable: bool },
}

/// Represents a complete WebAssembly module AST
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmModule {
    pub types: Vec<WasmFunctionType>,
    pub imports: Vec<WasmImport>,
    pub functions: Vec<WasmFunction>,
    pub tables: Vec<WasmTable>,
    pub memories: Vec<WasmMemory>,
    pub globals: Vec<WasmGlobal>,
    pub exports: Vec<WasmExport>,
    pub start_function: Option<u32>,
    pub element_segments: Vec<WasmElementSegment>,
    pub data_segments: Vec<WasmDataSegment>,
}

/// Represents a WebAssembly element segment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmElementSegment {
    pub table_index: u32,
    pub offset: Vec<WasmInstruction>,
    pub elements: Vec<u32>,
}

/// Represents a WebAssembly data segment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmDataSegment {
    pub memory_index: u32,
    pub offset: Vec<WasmInstruction>,
    pub data: Vec<u8>,
}

impl WasmModule {
    /// Create a new empty WebAssembly module
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            imports: Vec::new(),
            functions: Vec::new(),
            tables: Vec::new(),
            memories: Vec::new(),
            globals: Vec::new(),
            exports: Vec::new(),
            start_function: None,
            element_segments: Vec::new(),
            data_segments: Vec::new(),
        }
    }

    /// Get the total number of functions (imported + defined)
    pub fn function_count(&self) -> usize {
        let imported_functions = self.imports.iter().filter(|import| matches!(import.kind, WasmImportKind::Function { .. })).count();
        imported_functions + self.functions.len()
    }

    /// Get the total number of globals (imported + defined)
    pub fn global_count(&self) -> usize {
        let imported_globals = self.imports.iter().filter(|import| matches!(import.kind, WasmImportKind::Global { .. })).count();
        imported_globals + self.globals.len()
    }

    /// Get the total number of tables (imported + defined)
    pub fn table_count(&self) -> usize {
        let imported_tables = self.imports.iter().filter(|import| matches!(import.kind, WasmImportKind::Table(_))).count();
        imported_tables + self.tables.len()
    }

    /// Get the total number of memories (imported + defined)
    pub fn memory_count(&self) -> usize {
        let imported_memories = self.imports.iter().filter(|import| matches!(import.kind, WasmImportKind::Memory(_))).count();
        imported_memories + self.memories.len()
    }
}

impl Default for WasmModule {
    fn default() -> Self {
        Self::new()
    }
}
