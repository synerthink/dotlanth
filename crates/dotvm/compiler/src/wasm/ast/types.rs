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

//! WebAssembly type system definitions

use serde::{Deserialize, Serialize};

/// WebAssembly value types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmValueType {
    /// 32-bit integer
    I32,
    /// 64-bit integer
    I64,
    /// 32-bit floating point
    F32,
    /// 64-bit floating point
    F64,
    /// 128-bit vector (SIMD)
    V128,
    /// Function reference
    FuncRef,
    /// External reference
    ExternRef,
}

impl WasmValueType {
    /// Get the size of this type in bytes
    pub fn size_bytes(&self) -> u32 {
        match self {
            Self::I32 | Self::F32 => 4,
            Self::I64 | Self::F64 | Self::FuncRef | Self::ExternRef => 8,
            Self::V128 => 16,
        }
    }

    /// Check if this is a numeric type
    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::I32 | Self::I64 | Self::F32 | Self::F64)
    }

    /// Check if this is an integer type
    pub fn is_integer(&self) -> bool {
        matches!(self, Self::I32 | Self::I64)
    }

    /// Check if this is a floating-point type
    pub fn is_float(&self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }

    /// Check if this is a reference type
    pub fn is_reference(&self) -> bool {
        matches!(self, Self::FuncRef | Self::ExternRef)
    }

    /// Check if this is a vector type
    pub fn is_vector(&self) -> bool {
        matches!(self, Self::V128)
    }

    /// Get the default value for this type
    pub fn default_value(&self) -> WasmValue {
        match self {
            Self::I32 => WasmValue::I32(0),
            Self::I64 => WasmValue::I64(0),
            Self::F32 => WasmValue::F32(0.0),
            Self::F64 => WasmValue::F64(0.0),
            Self::V128 => WasmValue::V128([0; 16]),
            Self::FuncRef => WasmValue::FuncRef(None),
            Self::ExternRef => WasmValue::ExternRef(None),
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::V128 => "v128",
            Self::FuncRef => "funcref",
            Self::ExternRef => "externref",
        }
    }
}

impl std::fmt::Display for WasmValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// WebAssembly runtime values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WasmValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    V128([u8; 16]),
    FuncRef(Option<u32>),
    ExternRef(Option<u32>),
}

impl WasmValue {
    /// Get the type of this value
    pub fn value_type(&self) -> WasmValueType {
        match self {
            Self::I32(_) => WasmValueType::I32,
            Self::I64(_) => WasmValueType::I64,
            Self::F32(_) => WasmValueType::F32,
            Self::F64(_) => WasmValueType::F64,
            Self::V128(_) => WasmValueType::V128,
            Self::FuncRef(_) => WasmValueType::FuncRef,
            Self::ExternRef(_) => WasmValueType::ExternRef,
        }
    }

    /// Check if this value is zero/null
    pub fn is_zero(&self) -> bool {
        match self {
            Self::I32(v) => *v == 0,
            Self::I64(v) => *v == 0,
            Self::F32(v) => *v == 0.0,
            Self::F64(v) => *v == 0.0,
            Self::V128(v) => v.iter().all(|&b| b == 0),
            Self::FuncRef(v) => v.is_none(),
            Self::ExternRef(v) => v.is_none(),
        }
    }
}

/// WebAssembly function type (signature)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WasmFunctionType {
    /// Parameter types
    pub params: Vec<WasmValueType>,
    /// Result types
    pub results: Vec<WasmValueType>,
}

impl WasmFunctionType {
    /// Create a new function type
    pub fn new(params: Vec<WasmValueType>, results: Vec<WasmValueType>) -> Self {
        Self { params, results }
    }

    /// Create a function type with no parameters or results
    pub fn empty() -> Self {
        Self::new(Vec::new(), Vec::new())
    }

    /// Get the number of parameters
    pub fn param_count(&self) -> usize {
        self.params.len()
    }

    /// Get the number of results
    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    /// Check if this function returns a value
    pub fn has_result(&self) -> bool {
        !self.results.is_empty()
    }

    /// Check if this function takes parameters
    pub fn has_params(&self) -> bool {
        !self.params.is_empty()
    }

    /// Get the signature as a string
    pub fn signature_string(&self) -> String {
        let params = self.params.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(", ");

        let results = self.results.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(", ");

        if self.results.is_empty() {
            format!("({}) -> ()", params)
        } else {
            format!("({}) -> ({})", params, results)
        }
    }
}

impl std::fmt::Display for WasmFunctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.signature_string())
    }
}

/// WebAssembly table type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmTableType {
    /// Element type
    pub element_type: WasmValueType,
    /// Initial size
    pub initial: u32,
    /// Maximum size (if limited)
    pub maximum: Option<u32>,
}

impl WasmTableType {
    /// Create a new table type
    pub fn new(element_type: WasmValueType, initial: u32, maximum: Option<u32>) -> Self {
        Self { element_type, initial, maximum }
    }

    /// Check if the table can grow
    pub fn can_grow(&self) -> bool {
        self.maximum.map_or(true, |max| max > self.initial)
    }

    /// Get the maximum possible size
    pub fn max_size(&self) -> u32 {
        self.maximum.unwrap_or(u32::MAX)
    }
}

/// WebAssembly memory type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmMemoryType {
    /// Initial size in pages
    pub initial: u32,
    /// Maximum size in pages (if limited)
    pub maximum: Option<u32>,
    /// Whether memory is shared
    pub shared: bool,
}

impl WasmMemoryType {
    /// Create a new memory type
    pub fn new(initial: u32, maximum: Option<u32>, shared: bool) -> Self {
        Self { initial, maximum, shared }
    }

    /// Get the initial size in bytes
    pub fn initial_bytes(&self) -> u64 {
        self.initial as u64 * 65536 // 64KB pages
    }

    /// Get the maximum size in bytes
    pub fn max_bytes(&self) -> Option<u64> {
        self.maximum.map(|pages| pages as u64 * 65536)
    }

    /// Check if the memory can grow
    pub fn can_grow(&self) -> bool {
        self.maximum.map_or(true, |max| max > self.initial)
    }
}

/// WebAssembly global type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmGlobalType {
    /// Value type
    pub value_type: WasmValueType,
    /// Whether the global is mutable
    pub mutable: bool,
}

impl WasmGlobalType {
    /// Create a new global type
    pub fn new(value_type: WasmValueType, mutable: bool) -> Self {
        Self { value_type, mutable }
    }

    /// Create an immutable global type
    pub fn immutable(value_type: WasmValueType) -> Self {
        Self::new(value_type, false)
    }

    /// Create a mutable global type
    pub fn mutable(value_type: WasmValueType) -> Self {
        Self::new(value_type, true)
    }
}

/// Memory argument for memory instructions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemArg {
    /// Memory offset
    pub offset: u64,
    /// Alignment (power of 2)
    pub align: u32,
}

impl MemArg {
    /// Create a new memory argument
    pub fn new(offset: u64, align: u32) -> Self {
        Self { offset, align }
    }

    /// Create a memory argument with zero offset
    pub fn with_align(align: u32) -> Self {
        Self::new(0, align)
    }

    /// Check if the alignment is valid (power of 2)
    pub fn is_valid_alignment(&self) -> bool {
        self.align > 0 && (self.align & (self.align - 1)) == 0
    }

    /// Get the actual alignment in bytes
    pub fn alignment_bytes(&self) -> u32 {
        1 << self.align
    }
}

impl Default for MemArg {
    fn default() -> Self {
        Self::new(0, 0) // Natural alignment
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_type_properties() {
        assert_eq!(WasmValueType::I32.size_bytes(), 4);
        assert_eq!(WasmValueType::I64.size_bytes(), 8);
        assert_eq!(WasmValueType::V128.size_bytes(), 16);

        assert!(WasmValueType::I32.is_numeric());
        assert!(WasmValueType::I32.is_integer());
        assert!(!WasmValueType::I32.is_float());
        assert!(!WasmValueType::I32.is_reference());

        assert!(WasmValueType::F32.is_numeric());
        assert!(!WasmValueType::F32.is_integer());
        assert!(WasmValueType::F32.is_float());

        assert!(WasmValueType::FuncRef.is_reference());
        assert!(!WasmValueType::FuncRef.is_numeric());
    }

    #[test]
    fn test_function_type() {
        let func_type = WasmFunctionType::new(vec![WasmValueType::I32, WasmValueType::I32], vec![WasmValueType::I32]);

        assert_eq!(func_type.param_count(), 2);
        assert_eq!(func_type.result_count(), 1);
        assert!(func_type.has_params());
        assert!(func_type.has_result());

        let signature = func_type.signature_string();
        assert_eq!(signature, "(i32, i32) -> (i32)");
    }

    #[test]
    fn test_wasm_value() {
        let val = WasmValue::I32(42);
        assert_eq!(val.value_type(), WasmValueType::I32);
        assert!(!val.is_zero());

        let zero_val = WasmValue::I32(0);
        assert!(zero_val.is_zero());
    }

    #[test]
    fn test_memarg() {
        let memarg = MemArg::new(8, 2);
        assert_eq!(memarg.offset, 8);
        assert_eq!(memarg.align, 2);
        assert_eq!(memarg.alignment_bytes(), 4);
        assert!(memarg.is_valid_alignment());

        let invalid_memarg = MemArg::new(0, 3); // 3 is not a power of 2
        assert!(!invalid_memarg.is_valid_alignment());
    }
}
