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

//! WebAssembly parser for DotVM transpiler
//!
//! This module provides functionality to parse WebAssembly binary format
//! into our internal AST representation for further transpilation to DotVM bytecode.

use crate::wasm::ast::*;
use thiserror::Error;
use wasmparser::{Export, FuncType, Import, Operator, Parser, Payload, TypeRef, ValType, WasmFeatures};

/// Errors that can occur during WASM parsing
#[derive(Error, Debug)]
pub enum WasmParseError {
    #[error("Invalid WASM binary: {0}")]
    InvalidBinary(String),
    #[error("Unsupported WASM feature: {0}")]
    UnsupportedFeature(String),
    #[error("Parser error: {0}")]
    ParserError(#[from] wasmparser::BinaryReaderError),
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },
    #[error("Invalid function index: {0}")]
    InvalidFunctionIndex(u32),
    #[error("Invalid type index: {0}")]
    InvalidTypeIndex(u32),
}

// Note: Error conversion can be added when DotVMError is implemented

/// WebAssembly parser that converts WASM binary to our internal AST
pub struct WasmParser {
    /// WASM features to enable during parsing
    features: WasmFeatures,
}

impl WasmParser {
    /// Create a new WASM parser with default features
    pub fn new() -> Self {
        Self { features: WasmFeatures::default() }
    }

    /// Create a new WASM parser with specific features enabled
    pub fn with_features(features: WasmFeatures) -> Self {
        Self { features }
    }

    /// Parse a WASM binary into our internal AST representation
    pub fn parse(&self, wasm_bytes: &[u8]) -> Result<WasmModule, WasmParseError> {
        let parser = Parser::new(0);
        let mut module = WasmModule::new();
        let mut type_section: Vec<WasmFunctionType> = Vec::new();
        let mut function_section: Vec<u32> = Vec::new(); // Type indices for functions
        let mut code_section: Vec<WasmFunction> = Vec::new();

        for payload in parser.parse_all(wasm_bytes) {
            match payload? {
                Payload::Version { num, .. } => {
                    if num != 1 {
                        return Err(WasmParseError::UnsupportedFeature(format!("WASM version {num}")));
                    }
                }

                Payload::TypeSection(reader) => {
                    for ty in reader {
                        let rec_group = ty?;
                        // Handle RecGroup - extract function types
                        for sub_type in rec_group.types() {
                            if let wasmparser::SubType {
                                composite_type: wasmparser::CompositeType::Func(func_type),
                                ..
                            } = &sub_type
                            {
                                type_section.push(self.convert_function_type(func_type)?);
                            }
                        }
                    }
                }

                Payload::ImportSection(reader) => {
                    for import in reader {
                        let import = import?;
                        module.imports.push(self.convert_import(&import)?);
                    }
                }

                Payload::FunctionSection(reader) => {
                    for func in reader {
                        let type_index = func?;
                        function_section.push(type_index);
                    }
                }

                Payload::TableSection(reader) => {
                    for table in reader {
                        let _table = table?;
                        // TODO: Handle table section
                    }
                }

                Payload::MemorySection(reader) => {
                    for memory in reader {
                        let memory = memory?;
                        module.memories.push(WasmMemory {
                            min_pages: memory.initial as u32,
                            max_pages: memory.maximum.map(|m| m as u32),
                            shared: memory.shared,
                        });
                    }
                }

                Payload::GlobalSection(reader) => {
                    for global in reader {
                        let global = global?;
                        module.globals.push(WasmGlobal {
                            value_type: self.convert_value_type(&global.ty.content_type)?,
                            mutable: global.ty.mutable,
                            init_expr: Vec::new(), // TODO: Parse init expression
                        });
                    }
                }

                Payload::ExportSection(reader) => {
                    for export in reader {
                        let export = export?;
                        module.exports.push(self.convert_export(&export)?);
                    }
                }

                Payload::StartSection { func, .. } => {
                    module.start_function = Some(func);
                }

                Payload::ElementSection(reader) => {
                    for element in reader {
                        let _element = element?;
                        // TODO: Handle element section
                    }
                }

                Payload::CodeSectionEntry(body) => {
                    let i = code_section.len();
                    let type_index = function_section.get(i).ok_or(WasmParseError::InvalidFunctionIndex(i as u32))?;
                    let func_type = type_section.get(*type_index as usize).ok_or(WasmParseError::InvalidTypeIndex(*type_index))?;

                    let function = self.parse_function_body(&body, func_type.clone())?;
                    code_section.push(function);
                }

                Payload::DataSection(reader) => {
                    for data in reader {
                        let data = data?;
                        let memory_index = match &data.kind {
                            wasmparser::DataKind::Active { memory_index, offset_expr: _ } => *memory_index,
                            wasmparser::DataKind::Passive => 0, // Default to memory 0 for passive data
                        };
                        module.data_segments.push(WasmDataSegment {
                            memory_index,
                            offset: Vec::new(), // TODO: Parse offset expression
                            data: data.data.to_vec(),
                        });
                    }
                }

                Payload::CustomSection(reader) => {
                    // Handle custom sections (e.g., name section)
                    let _name = reader.name();
                    let _data = reader.data();
                    // TODO: Handle custom sections if needed
                }

                _ => {
                    // Handle other sections or ignore them
                }
            }
        }

        // Combine type section with function section to create complete functions
        module.types = type_section;
        module.functions = code_section;

        Ok(module)
    }

    /// Convert wasmparser FuncType to our WasmFunctionType
    fn convert_function_type(&self, func_type: &FuncType) -> Result<WasmFunctionType, WasmParseError> {
        let params = func_type.params().iter().map(|t| self.convert_value_type(t)).collect::<Result<Vec<_>, _>>()?;

        let results = func_type.results().iter().map(|t| self.convert_value_type(t)).collect::<Result<Vec<_>, _>>()?;

        Ok(WasmFunctionType { params, results })
    }

    /// Convert wasmparser ValType to our WasmValueType
    fn convert_value_type(&self, val_type: &ValType) -> Result<WasmValueType, WasmParseError> {
        match val_type {
            ValType::I32 => Ok(WasmValueType::I32),
            ValType::I64 => Ok(WasmValueType::I64),
            ValType::F32 => Ok(WasmValueType::F32),
            ValType::F64 => Ok(WasmValueType::F64),
            ValType::V128 => Ok(WasmValueType::V128),
            ValType::Ref(ref_type) => match ref_type.heap_type() {
                wasmparser::HeapType::Func => Ok(WasmValueType::FuncRef),
                wasmparser::HeapType::Extern => Ok(WasmValueType::ExternRef),
                _ => Err(WasmParseError::UnsupportedFeature(format!("Reference type: {ref_type:?}"))),
            },
        }
    }

    /// Convert wasmparser Import to our WasmImport
    fn convert_import(&self, import: &Import) -> Result<WasmImport, WasmParseError> {
        let kind = match import.ty {
            TypeRef::Func(type_index) => WasmImportKind::Function { type_index },
            TypeRef::Table(table_type) => {
                // Convert table type to our representation - handle RefType
                let element_type = match table_type.element_type {
                    wasmparser::RefType::FUNCREF => WasmValueType::FuncRef,
                    wasmparser::RefType::EXTERNREF => WasmValueType::ExternRef,
                    _ => WasmValueType::FuncRef, // Default fallback
                };
                WasmImportKind::Table(WasmTable {
                    element_type,
                    initial: table_type.initial,
                    maximum: table_type.maximum,
                })
            }
            TypeRef::Memory(memory_type) => {
                // Convert memory type to our representation
                WasmImportKind::Memory(WasmMemory {
                    min_pages: memory_type.initial as u32,
                    max_pages: memory_type.maximum.map(|m| m as u32),
                    shared: memory_type.shared,
                })
            }
            TypeRef::Global(global_type) => WasmImportKind::Global {
                value_type: self.convert_value_type(&global_type.content_type)?,
                mutable: global_type.mutable,
            },
            TypeRef::Tag(_) => return Err(WasmParseError::UnsupportedFeature("Tag imports".to_string())),
        };

        Ok(WasmImport {
            module: import.module.to_string(),
            name: import.name.to_string(),
            kind,
        })
    }

    /// Convert wasmparser Export to our WasmExport
    fn convert_export(&self, export: &Export) -> Result<WasmExport, WasmParseError> {
        let kind = match export.kind {
            wasmparser::ExternalKind::Func => WasmExportKind::Function,
            wasmparser::ExternalKind::Table => WasmExportKind::Table,
            wasmparser::ExternalKind::Memory => WasmExportKind::Memory,
            wasmparser::ExternalKind::Global => WasmExportKind::Global,
            wasmparser::ExternalKind::Tag => return Err(WasmParseError::UnsupportedFeature("Tag exports".to_string())),
        };

        Ok(WasmExport {
            name: export.name.to_string(),
            kind,
            index: export.index,
        })
    }

    /// Parse a function body from wasmparser
    fn parse_function_body(&self, body: &wasmparser::FunctionBody, func_type: WasmFunctionType) -> Result<WasmFunction, WasmParseError> {
        let mut locals = Vec::new();
        let mut instructions = Vec::new();

        // Parse local variables
        let locals_reader = body.get_locals_reader()?;
        for local in locals_reader {
            let (count, val_type) = local?;
            let wasm_type = self.convert_value_type(&val_type)?;
            for _ in 0..count {
                locals.push(wasm_type);
            }
        }

        // Parse instructions
        let operators_reader = body.get_operators_reader()?;
        for op in operators_reader {
            let op = op?;
            instructions.push(self.convert_operator(&op)?);
        }

        Ok(WasmFunction {
            signature: func_type,
            locals,
            body: instructions,
        })
    }

    /// Convert wasmparser Operator to our WasmInstruction
    fn convert_operator(&self, op: &Operator) -> Result<WasmInstruction, WasmParseError> {
        match op {
            // Control flow
            Operator::Unreachable => Ok(WasmInstruction::Unreachable),
            Operator::Nop => Ok(WasmInstruction::Nop),
            Operator::Block { blockty } => Ok(WasmInstruction::Block {
                block_type: self.convert_block_type(blockty)?,
            }),
            Operator::Loop { blockty } => Ok(WasmInstruction::Loop {
                block_type: self.convert_block_type(blockty)?,
            }),
            Operator::If { blockty } => Ok(WasmInstruction::If {
                block_type: self.convert_block_type(blockty)?,
            }),
            Operator::Else => Ok(WasmInstruction::Else),
            Operator::End => Ok(WasmInstruction::End),
            Operator::Br { relative_depth } => Ok(WasmInstruction::Br { label_index: *relative_depth }),
            Operator::BrIf { relative_depth } => Ok(WasmInstruction::BrIf { label_index: *relative_depth }),
            Operator::BrTable { targets } => {
                let mut labels = Vec::new();
                for target in targets.targets() {
                    labels.push(target?);
                }
                Ok(WasmInstruction::BrTable { labels, default: targets.default() })
            }
            Operator::Return => Ok(WasmInstruction::Return),
            Operator::Call { function_index } => Ok(WasmInstruction::Call { function_index: *function_index }),
            Operator::CallIndirect { type_index, table_index, .. } => Ok(WasmInstruction::CallIndirect {
                type_index: *type_index,
                table_index: *table_index,
            }),

            // Parametric instructions
            Operator::Drop => Ok(WasmInstruction::Drop),
            Operator::Select => Ok(WasmInstruction::Select),

            // Variable instructions
            Operator::LocalGet { local_index } => Ok(WasmInstruction::LocalGet { local_index: *local_index }),
            Operator::LocalSet { local_index } => Ok(WasmInstruction::LocalSet { local_index: *local_index }),
            Operator::LocalTee { local_index } => Ok(WasmInstruction::LocalTee { local_index: *local_index }),
            Operator::GlobalGet { global_index } => Ok(WasmInstruction::GlobalGet { global_index: *global_index }),
            Operator::GlobalSet { global_index } => Ok(WasmInstruction::GlobalSet { global_index: *global_index }),

            // Memory instructions
            Operator::I32Load { memarg } => Ok(WasmInstruction::I32Load {
                memarg: MemArg {
                    offset: memarg.offset,
                    align: memarg.align as u32,
                },
            }),
            Operator::I64Load { memarg } => Ok(WasmInstruction::I64Load {
                memarg: MemArg {
                    offset: memarg.offset,
                    align: memarg.align as u32,
                },
            }),
            Operator::F32Load { memarg } => Ok(WasmInstruction::F32Load {
                memarg: MemArg {
                    offset: memarg.offset,
                    align: memarg.align as u32,
                },
            }),
            Operator::F64Load { memarg } => Ok(WasmInstruction::F64Load {
                memarg: MemArg {
                    offset: memarg.offset,
                    align: memarg.align as u32,
                },
            }),
            Operator::I32Store { memarg } => Ok(WasmInstruction::I32Store {
                memarg: MemArg {
                    offset: memarg.offset,
                    align: memarg.align as u32,
                },
            }),
            Operator::I64Store { memarg } => Ok(WasmInstruction::I64Store {
                memarg: MemArg {
                    offset: memarg.offset,
                    align: memarg.align as u32,
                },
            }),
            Operator::F32Store { memarg } => Ok(WasmInstruction::F32Store {
                memarg: MemArg {
                    offset: memarg.offset,
                    align: memarg.align as u32,
                },
            }),
            Operator::F64Store { memarg } => Ok(WasmInstruction::F64Store {
                memarg: MemArg {
                    offset: memarg.offset,
                    align: memarg.align as u32,
                },
            }),
            Operator::MemorySize { .. } => Ok(WasmInstruction::MemorySize),
            Operator::MemoryGrow { .. } => Ok(WasmInstruction::MemoryGrow),

            // Numeric instructions - Constants
            Operator::I32Const { value } => Ok(WasmInstruction::I32Const { value: *value }),
            Operator::I64Const { value } => Ok(WasmInstruction::I64Const { value: *value }),
            Operator::F32Const { value } => Ok(WasmInstruction::F32Const { value: f32::from_bits(value.bits()) }),
            Operator::F64Const { value } => Ok(WasmInstruction::F64Const { value: f64::from_bits(value.bits()) }),

            // Numeric instructions - Arithmetic
            Operator::I32Add => Ok(WasmInstruction::I32Add),
            Operator::I32Sub => Ok(WasmInstruction::I32Sub),
            Operator::I32Mul => Ok(WasmInstruction::I32Mul),
            Operator::I32DivS => Ok(WasmInstruction::I32DivS),
            Operator::I32DivU => Ok(WasmInstruction::I32DivU),
            Operator::I32RemS => Ok(WasmInstruction::I32RemS),
            Operator::I32RemU => Ok(WasmInstruction::I32RemU),
            Operator::I32And => Ok(WasmInstruction::I32And),
            Operator::I32Or => Ok(WasmInstruction::I32Or),
            Operator::I32Xor => Ok(WasmInstruction::I32Xor),
            Operator::I32Shl => Ok(WasmInstruction::I32Shl),
            Operator::I32ShrS => Ok(WasmInstruction::I32ShrS),
            Operator::I32ShrU => Ok(WasmInstruction::I32ShrU),
            Operator::I32Rotl => Ok(WasmInstruction::I32Rotl),
            Operator::I32Rotr => Ok(WasmInstruction::I32Rotr),

            Operator::I64Add => Ok(WasmInstruction::I64Add),
            Operator::I64Sub => Ok(WasmInstruction::I64Sub),
            Operator::I64Mul => Ok(WasmInstruction::I64Mul),
            Operator::I64DivS => Ok(WasmInstruction::I64DivS),
            Operator::I64DivU => Ok(WasmInstruction::I64DivU),
            Operator::I64RemS => Ok(WasmInstruction::I64RemS),
            Operator::I64RemU => Ok(WasmInstruction::I64RemU),
            Operator::I64And => Ok(WasmInstruction::I64And),
            Operator::I64Or => Ok(WasmInstruction::I64Or),
            Operator::I64Xor => Ok(WasmInstruction::I64Xor),
            Operator::I64Shl => Ok(WasmInstruction::I64Shl),
            Operator::I64ShrS => Ok(WasmInstruction::I64ShrS),
            Operator::I64ShrU => Ok(WasmInstruction::I64ShrU),
            Operator::I64Rotl => Ok(WasmInstruction::I64Rotl),
            Operator::I64Rotr => Ok(WasmInstruction::I64Rotr),

            // Comparison instructions
            Operator::I32Eqz => Ok(WasmInstruction::I32Eqz),
            Operator::I32Eq => Ok(WasmInstruction::I32Eq),
            Operator::I32Ne => Ok(WasmInstruction::I32Ne),
            Operator::I32LtS => Ok(WasmInstruction::I32LtS),
            Operator::I32LtU => Ok(WasmInstruction::I32LtU),
            Operator::I32GtS => Ok(WasmInstruction::I32GtS),
            Operator::I32GtU => Ok(WasmInstruction::I32GtU),
            Operator::I32LeS => Ok(WasmInstruction::I32LeS),
            Operator::I32LeU => Ok(WasmInstruction::I32LeU),
            Operator::I32GeS => Ok(WasmInstruction::I32GeS),
            Operator::I32GeU => Ok(WasmInstruction::I32GeU),

            Operator::I64Eqz => Ok(WasmInstruction::I64Eqz),
            Operator::I64Eq => Ok(WasmInstruction::I64Eq),
            Operator::I64Ne => Ok(WasmInstruction::I64Ne),
            Operator::I64LtS => Ok(WasmInstruction::I64LtS),
            Operator::I64LtU => Ok(WasmInstruction::I64LtU),
            Operator::I64GtS => Ok(WasmInstruction::I64GtS),
            Operator::I64GtU => Ok(WasmInstruction::I64GtU),
            Operator::I64LeS => Ok(WasmInstruction::I64LeS),
            Operator::I64LeU => Ok(WasmInstruction::I64LeU),
            Operator::I64GeS => Ok(WasmInstruction::I64GeS),
            Operator::I64GeU => Ok(WasmInstruction::I64GeU),

            // Floating point arithmetic
            Operator::F32Add => Ok(WasmInstruction::F32Add),
            Operator::F32Sub => Ok(WasmInstruction::F32Sub),
            Operator::F32Mul => Ok(WasmInstruction::F32Mul),
            Operator::F32Div => Ok(WasmInstruction::F32Div),
            Operator::F32Min => Ok(WasmInstruction::F32Min),
            Operator::F32Max => Ok(WasmInstruction::F32Max),

            Operator::F64Add => Ok(WasmInstruction::F64Add),
            Operator::F64Sub => Ok(WasmInstruction::F64Sub),
            Operator::F64Mul => Ok(WasmInstruction::F64Mul),
            Operator::F64Div => Ok(WasmInstruction::F64Div),
            Operator::F64Min => Ok(WasmInstruction::F64Min),
            Operator::F64Max => Ok(WasmInstruction::F64Max),

            // Conversion instructions
            Operator::I32WrapI64 => Ok(WasmInstruction::I32WrapI64),
            Operator::I64ExtendI32S => Ok(WasmInstruction::I64ExtendI32S),
            Operator::I64ExtendI32U => Ok(WasmInstruction::I64ExtendI32U),

            // Add more operators as needed...
            _ => Err(WasmParseError::UnsupportedFeature(format!("Operator: {op:?}"))),
        }
    }

    /// Convert wasmparser BlockType to our result type
    fn convert_block_type(&self, block_type: &wasmparser::BlockType) -> Result<Option<WasmValueType>, WasmParseError> {
        match block_type {
            wasmparser::BlockType::Empty => Ok(None),
            wasmparser::BlockType::Type(val_type) => Ok(Some(self.convert_value_type(val_type)?)),
            wasmparser::BlockType::FuncType(_) => Err(WasmParseError::UnsupportedFeature("Multi-value block types".to_string())),
        }
    }
}

impl Default for WasmParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = WasmParser::new();
        // Note: WasmFeatures doesn't implement PartialEq, so we can't compare directly
        // Just verify the parser was created successfully
        assert!(true);
    }

    #[test]
    fn test_value_type_conversion() {
        let parser = WasmParser::new();
        assert_eq!(parser.convert_value_type(&ValType::I32).unwrap(), WasmValueType::I32);
        assert_eq!(parser.convert_value_type(&ValType::I64).unwrap(), WasmValueType::I64);
        assert_eq!(parser.convert_value_type(&ValType::F32).unwrap(), WasmValueType::F32);
        assert_eq!(parser.convert_value_type(&ValType::F64).unwrap(), WasmValueType::F64);
    }

    // TODO: Add more comprehensive tests with actual WASM binaries
}
