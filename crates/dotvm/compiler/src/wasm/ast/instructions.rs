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

//! WebAssembly instruction definitions

use super::types::{MemArg, WasmValueType};
use serde::{Deserialize, Serialize};

/// WebAssembly instruction set
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WasmInstruction {
    // Control flow instructions
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

    // Parametric instructions
    Drop,
    Select,
    SelectWithType { types: Vec<WasmValueType> },

    // Variable instructions
    LocalGet { local_index: u32 },
    LocalSet { local_index: u32 },
    LocalTee { local_index: u32 },
    GlobalGet { global_index: u32 },
    GlobalSet { global_index: u32 },

    // Table instructions
    TableGet { table_index: u32 },
    TableSet { table_index: u32 },
    TableInit { table_index: u32, elem_index: u32 },
    ElemDrop { elem_index: u32 },
    TableCopy { dst_table: u32, src_table: u32 },
    TableGrow { table_index: u32 },
    TableSize { table_index: u32 },
    TableFill { table_index: u32 },

    // Memory instructions
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
    MemoryInit { data_index: u32 },
    DataDrop { data_index: u32 },
    MemoryCopy,
    MemoryFill,

    // Numeric instructions - Constants
    I32Const { value: i32 },
    I64Const { value: i64 },
    F32Const { value: f32 },
    F64Const { value: f64 },

    // Numeric instructions - I32 operations
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

    // Numeric instructions - I64 operations
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

    // Numeric instructions - F32 operations
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

    // Numeric instructions - F64 operations
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

    // Conversion instructions
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

    // Sign extension instructions
    I32Extend8S,
    I32Extend16S,
    I64Extend8S,
    I64Extend16S,
    I64Extend32S,

    // Saturating truncation instructions
    I32TruncSatF32S,
    I32TruncSatF32U,
    I32TruncSatF64S,
    I32TruncSatF64U,
    I64TruncSatF32S,
    I64TruncSatF32U,
    I64TruncSatF64S,
    I64TruncSatF64U,

    // Reference instructions
    RefNull { ref_type: WasmValueType },
    RefIsNull,
    RefFunc { function_index: u32 },

    // SIMD instructions (V128)
    V128Load { memarg: MemArg },
    V128Load8x8S { memarg: MemArg },
    V128Load8x8U { memarg: MemArg },
    V128Load16x4S { memarg: MemArg },
    V128Load16x4U { memarg: MemArg },
    V128Load32x2S { memarg: MemArg },
    V128Load32x2U { memarg: MemArg },
    V128Load8Splat { memarg: MemArg },
    V128Load16Splat { memarg: MemArg },
    V128Load32Splat { memarg: MemArg },
    V128Load64Splat { memarg: MemArg },
    V128Store { memarg: MemArg },
    V128Const { value: [u8; 16] },
    I8x16Shuffle { lanes: [u8; 16] },
    I8x16Swizzle,
    I8x16Splat,
    I16x8Splat,
    I32x4Splat,
    I64x2Splat,
    F32x4Splat,
    F64x2Splat,
    I8x16ExtractLaneS { lane: u8 },
    I8x16ExtractLaneU { lane: u8 },
    I8x16ReplaceLane { lane: u8 },
    I16x8ExtractLaneS { lane: u8 },
    I16x8ExtractLaneU { lane: u8 },
    I16x8ReplaceLane { lane: u8 },
    I32x4ExtractLane { lane: u8 },
    I32x4ReplaceLane { lane: u8 },
    I64x2ExtractLane { lane: u8 },
    I64x2ReplaceLane { lane: u8 },
    F32x4ExtractLane { lane: u8 },
    F32x4ReplaceLane { lane: u8 },
    F64x2ExtractLane { lane: u8 },
    F64x2ReplaceLane { lane: u8 },
    I8x16Eq,
    I8x16Ne,
    I8x16LtS,
    I8x16LtU,
    I8x16GtS,
    I8x16GtU,
    I8x16LeS,
    I8x16LeU,
    I8x16GeS,
    I8x16GeU,
    I16x8Eq,
    I16x8Ne,
    I16x8LtS,
    I16x8LtU,
    I16x8GtS,
    I16x8GtU,
    I16x8LeS,
    I16x8LeU,
    I16x8GeS,
    I16x8GeU,
    I32x4Eq,
    I32x4Ne,
    I32x4LtS,
    I32x4LtU,
    I32x4GtS,
    I32x4GtU,
    I32x4LeS,
    I32x4LeU,
    I32x4GeS,
    I32x4GeU,
    F32x4Eq,
    F32x4Ne,
    F32x4Lt,
    F32x4Gt,
    F32x4Le,
    F32x4Ge,
    F64x2Eq,
    F64x2Ne,
    F64x2Lt,
    F64x2Gt,
    F64x2Le,
    F64x2Ge,
    V128Not,
    V128And,
    V128AndNot,
    V128Or,
    V128Xor,
    V128Bitselect,
    V128AnyTrue,
    I8x16AllTrue,
    I8x16Bitmask,
    I8x16NarrowI16x8S,
    I8x16NarrowI16x8U,
    I8x16Shl,
    I8x16ShrS,
    I8x16ShrU,
    I8x16Add,
    I8x16AddSatS,
    I8x16AddSatU,
    I8x16Sub,
    I8x16SubSatS,
    I8x16SubSatU,
    I8x16MinS,
    I8x16MinU,
    I8x16MaxS,
    I8x16MaxU,
    I8x16AvgrU,
    I16x8ExtAddPairwiseI8x16S,
    I16x8ExtAddPairwiseI8x16U,
    I16x8AllTrue,
    I16x8Bitmask,
    I16x8NarrowI32x4S,
    I16x8NarrowI32x4U,
    I16x8ExtendLowI8x16S,
    I16x8ExtendHighI8x16S,
    I16x8ExtendLowI8x16U,
    I16x8ExtendHighI8x16U,
    I16x8Shl,
    I16x8ShrS,
    I16x8ShrU,
    I16x8Add,
    I16x8AddSatS,
    I16x8AddSatU,
    I16x8Sub,
    I16x8SubSatS,
    I16x8SubSatU,
    I16x8Mul,
    I16x8MinS,
    I16x8MinU,
    I16x8MaxS,
    I16x8MaxU,
    I16x8AvgrU,
    I16x8ExtMulLowI8x16S,
    I16x8ExtMulHighI8x16S,
    I16x8ExtMulLowI8x16U,
    I16x8ExtMulHighI8x16U,
    I32x4ExtAddPairwiseI16x8S,
    I32x4ExtAddPairwiseI16x8U,
    I32x4AllTrue,
    I32x4Bitmask,
    I32x4ExtendLowI16x8S,
    I32x4ExtendHighI16x8S,
    I32x4ExtendLowI16x8U,
    I32x4ExtendHighI16x8U,
    I32x4Shl,
    I32x4ShrS,
    I32x4ShrU,
    I32x4Add,
    I32x4Sub,
    I32x4Mul,
    I32x4MinS,
    I32x4MinU,
    I32x4MaxS,
    I32x4MaxU,
    I32x4DotI16x8S,
    I32x4ExtMulLowI16x8S,
    I32x4ExtMulHighI16x8S,
    I32x4ExtMulLowI16x8U,
    I32x4ExtMulHighI16x8U,
    I64x2AllTrue,
    I64x2Bitmask,
    I64x2ExtendLowI32x4S,
    I64x2ExtendHighI32x4S,
    I64x2ExtendLowI32x4U,
    I64x2ExtendHighI32x4U,
    I64x2Shl,
    I64x2ShrS,
    I64x2ShrU,
    I64x2Add,
    I64x2Sub,
    I64x2Mul,
    I64x2ExtMulLowI32x4S,
    I64x2ExtMulHighI32x4S,
    I64x2ExtMulLowI32x4U,
    I64x2ExtMulHighI32x4U,
    F32x4Ceil,
    F32x4Floor,
    F32x4Trunc,
    F32x4Nearest,
    F32x4Abs,
    F32x4Neg,
    F32x4Sqrt,
    F32x4Add,
    F32x4Sub,
    F32x4Mul,
    F32x4Div,
    F32x4Min,
    F32x4Max,
    F32x4PMin,
    F32x4PMax,
    F64x2Ceil,
    F64x2Floor,
    F64x2Trunc,
    F64x2Nearest,
    F64x2Abs,
    F64x2Neg,
    F64x2Sqrt,
    F64x2Add,
    F64x2Sub,
    F64x2Mul,
    F64x2Div,
    F64x2Min,
    F64x2Max,
    F64x2PMin,
    F64x2PMax,
    I32x4TruncSatF32x4S,
    I32x4TruncSatF32x4U,
    F32x4ConvertI32x4S,
    F32x4ConvertI32x4U,
    I32x4TruncSatF64x2SZero,
    I32x4TruncSatF64x2UZero,
    F64x2ConvertLowI32x4S,
    F64x2ConvertLowI32x4U,
    F32x4DemoteF64x2Zero,
    F64x2PromoteLowF32x4,
}

impl WasmInstruction {
    /// Get the instruction name as a string
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unreachable => "unreachable",
            Self::Nop => "nop",
            Self::Block { .. } => "block",
            Self::Loop { .. } => "loop",
            Self::If { .. } => "if",
            Self::Else => "else",
            Self::End => "end",
            Self::Br { .. } => "br",
            Self::BrIf { .. } => "br_if",
            Self::BrTable { .. } => "br_table",
            Self::Return => "return",
            Self::Call { .. } => "call",
            Self::CallIndirect { .. } => "call_indirect",
            Self::Drop => "drop",
            Self::Select => "select",
            Self::SelectWithType { .. } => "select",
            Self::LocalGet { .. } => "local.get",
            Self::LocalSet { .. } => "local.set",
            Self::LocalTee { .. } => "local.tee",
            Self::GlobalGet { .. } => "global.get",
            Self::GlobalSet { .. } => "global.set",
            Self::I32Load { .. } => "i32.load",
            Self::I64Load { .. } => "i64.load",
            Self::F32Load { .. } => "f32.load",
            Self::F64Load { .. } => "f64.load",
            Self::I32Store { .. } => "i32.store",
            Self::I64Store { .. } => "i64.store",
            Self::F32Store { .. } => "f32.store",
            Self::F64Store { .. } => "f64.store",
            Self::MemorySize => "memory.size",
            Self::MemoryGrow => "memory.grow",
            Self::I32Const { .. } => "i32.const",
            Self::I64Const { .. } => "i64.const",
            Self::F32Const { .. } => "f32.const",
            Self::F64Const { .. } => "f64.const",
            Self::I32Add => "i32.add",
            Self::I32Sub => "i32.sub",
            Self::I32Mul => "i32.mul",
            Self::I32DivS => "i32.div_s",
            Self::I32DivU => "i32.div_u",
            Self::I64Add => "i64.add",
            Self::I64Sub => "i64.sub",
            Self::I64Mul => "i64.mul",
            Self::I64DivS => "i64.div_s",
            Self::I64DivU => "i64.div_u",
            Self::F32Add => "f32.add",
            Self::F32Sub => "f32.sub",
            Self::F32Mul => "f32.mul",
            Self::F32Div => "f32.div",
            Self::F64Add => "f64.add",
            Self::F64Sub => "f64.sub",
            Self::F64Mul => "f64.mul",
            Self::F64Div => "f64.div",
            // Add more as needed...
            _ => "unknown",
        }
    }

    /// Check if this instruction affects control flow
    pub fn affects_control_flow(&self) -> bool {
        matches!(
            self,
            Self::Unreachable
                | Self::Block { .. }
                | Self::Loop { .. }
                | Self::If { .. }
                | Self::Else
                | Self::End
                | Self::Br { .. }
                | Self::BrIf { .. }
                | Self::BrTable { .. }
                | Self::Return
                | Self::Call { .. }
                | Self::CallIndirect { .. }
        )
    }

    /// Check if this instruction accesses memory
    pub fn accesses_memory(&self) -> bool {
        matches!(
            self,
            Self::I32Load { .. }
                | Self::I64Load { .. }
                | Self::F32Load { .. }
                | Self::F64Load { .. }
                | Self::I32Load8S { .. }
                | Self::I32Load8U { .. }
                | Self::I32Load16S { .. }
                | Self::I32Load16U { .. }
                | Self::I64Load8S { .. }
                | Self::I64Load8U { .. }
                | Self::I64Load16S { .. }
                | Self::I64Load16U { .. }
                | Self::I64Load32S { .. }
                | Self::I64Load32U { .. }
                | Self::I32Store { .. }
                | Self::I64Store { .. }
                | Self::F32Store { .. }
                | Self::F64Store { .. }
                | Self::I32Store8 { .. }
                | Self::I32Store16 { .. }
                | Self::I64Store8 { .. }
                | Self::I64Store16 { .. }
                | Self::I64Store32 { .. }
                | Self::MemorySize
                | Self::MemoryGrow
                | Self::MemoryInit { .. }
                | Self::DataDrop { .. }
                | Self::MemoryCopy
                | Self::MemoryFill
                | Self::V128Load { .. }
                | Self::V128Store { .. }
        )
    }

    /// Check if this instruction is a constant
    pub fn is_constant(&self) -> bool {
        matches!(
            self,
            Self::I32Const { .. } | Self::I64Const { .. } | Self::F32Const { .. } | Self::F64Const { .. } | Self::V128Const { .. } | Self::RefNull { .. }
        )
    }

    /// Check if this instruction is arithmetic
    pub fn is_arithmetic(&self) -> bool {
        matches!(
            self,
            Self::I32Add
                | Self::I32Sub
                | Self::I32Mul
                | Self::I32DivS
                | Self::I32DivU
                | Self::I32RemS
                | Self::I32RemU
                | Self::I32And
                | Self::I32Or
                | Self::I32Xor
                | Self::I32Shl
                | Self::I32ShrS
                | Self::I32ShrU
                | Self::I32Rotl
                | Self::I32Rotr
                | Self::I64Add
                | Self::I64Sub
                | Self::I64Mul
                | Self::I64DivS
                | Self::I64DivU
                | Self::I64RemS
                | Self::I64RemU
                | Self::I64And
                | Self::I64Or
                | Self::I64Xor
                | Self::I64Shl
                | Self::I64ShrS
                | Self::I64ShrU
                | Self::I64Rotl
                | Self::I64Rotr
                | Self::F32Add
                | Self::F32Sub
                | Self::F32Mul
                | Self::F32Div
                | Self::F32Min
                | Self::F32Max
                | Self::F64Add
                | Self::F64Sub
                | Self::F64Mul
                | Self::F64Div
                | Self::F64Min
                | Self::F64Max
        )
    }

    /// Check if this instruction is a comparison
    pub fn is_comparison(&self) -> bool {
        matches!(
            self,
            Self::I32Eqz
                | Self::I32Eq
                | Self::I32Ne
                | Self::I32LtS
                | Self::I32LtU
                | Self::I32GtS
                | Self::I32GtU
                | Self::I32LeS
                | Self::I32LeU
                | Self::I32GeS
                | Self::I32GeU
                | Self::I64Eqz
                | Self::I64Eq
                | Self::I64Ne
                | Self::I64LtS
                | Self::I64LtU
                | Self::I64GtS
                | Self::I64GtU
                | Self::I64LeS
                | Self::I64LeU
                | Self::I64GeS
                | Self::I64GeU
                | Self::F32Eq
                | Self::F32Ne
                | Self::F32Lt
                | Self::F32Gt
                | Self::F32Le
                | Self::F32Ge
                | Self::F64Eq
                | Self::F64Ne
                | Self::F64Lt
                | Self::F64Gt
                | Self::F64Le
                | Self::F64Ge
        )
    }

    /// Check if this instruction uses SIMD
    pub fn is_simd(&self) -> bool {
        matches!(
            self,
            Self::V128Load { .. }
                | Self::V128Store { .. }
                | Self::V128Const { .. }
                | Self::I8x16Shuffle { .. }
                | Self::I8x16Swizzle
                | Self::I8x16Splat
                | Self::I16x8Splat
                | Self::I32x4Splat
                | Self::I64x2Splat
                | Self::F32x4Splat
                | Self::F64x2Splat
        ) || self.name().contains("x")
    }

    /// Get the result type of this instruction (if any)
    pub fn result_type(&self) -> Option<WasmValueType> {
        match self {
            Self::I32Const { .. } | Self::I32Add | Self::I32Sub | Self::I32Mul | Self::I32DivS | Self::I32DivU | Self::I32Load { .. } | Self::LocalGet { .. } => Some(WasmValueType::I32),
            Self::I64Const { .. } | Self::I64Add | Self::I64Sub | Self::I64Mul | Self::I64DivS | Self::I64DivU | Self::I64Load { .. } => Some(WasmValueType::I64),
            Self::F32Const { .. } | Self::F32Add | Self::F32Sub | Self::F32Mul | Self::F32Div | Self::F32Load { .. } => Some(WasmValueType::F32),
            Self::F64Const { .. } | Self::F64Add | Self::F64Sub | Self::F64Mul | Self::F64Div | Self::F64Load { .. } => Some(WasmValueType::F64),
            Self::V128Const { .. } | Self::V128Load { .. } => Some(WasmValueType::V128),
            Self::RefNull { ref_type } => Some(*ref_type),
            Self::RefFunc { .. } => Some(WasmValueType::FuncRef),
            _ => None,
        }
    }
}

impl std::fmt::Display for WasmInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_properties() {
        let add_inst = WasmInstruction::I32Add;
        assert!(add_inst.is_arithmetic());
        assert!(!add_inst.affects_control_flow());
        assert!(!add_inst.accesses_memory());
        assert_eq!(add_inst.result_type(), Some(WasmValueType::I32));

        let load_inst = WasmInstruction::I32Load { memarg: MemArg::default() };
        assert!(load_inst.accesses_memory());
        assert!(!load_inst.is_arithmetic());
        assert_eq!(load_inst.result_type(), Some(WasmValueType::I32));

        let br_inst = WasmInstruction::Br { label_index: 0 };
        assert!(br_inst.affects_control_flow());
        assert!(!br_inst.is_arithmetic());
        assert_eq!(br_inst.result_type(), None);

        let const_inst = WasmInstruction::I32Const { value: 42 };
        assert!(const_inst.is_constant());
        assert_eq!(const_inst.result_type(), Some(WasmValueType::I32));

        let simd_inst = WasmInstruction::V128Load { memarg: MemArg::default() };
        assert!(simd_inst.is_simd());
        assert!(simd_inst.accesses_memory());
        assert_eq!(simd_inst.result_type(), Some(WasmValueType::V128));
    }

    #[test]
    fn test_instruction_names() {
        assert_eq!(WasmInstruction::I32Add.name(), "i32.add");
        assert_eq!(WasmInstruction::F64Mul.name(), "f64.mul");
        assert_eq!(WasmInstruction::LocalGet { local_index: 0 }.name(), "local.get");
        assert_eq!(WasmInstruction::Call { function_index: 0 }.name(), "call");
    }
}
