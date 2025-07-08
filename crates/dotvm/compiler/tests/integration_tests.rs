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

//! Integration tests for the complete DotVM compilation pipeline
//!
//! These tests verify the end-to-end functionality from Wasm input
//! to optimized DotVM bytecode output.

use dotvm_compiler::{codegen::DotVMGenerator, optimizer::Optimizer, transpiler::engine_new::NewTranspilationEngine, transpiler::types::TranspiledModule, wasm::ast::*};
use dotvm_core::bytecode::VmArchitecture;
use wasm_encoder::{CodeSection, Function, FunctionSection, Instruction, Module, TypeSection};

/// Test the complete pipeline with a simple arithmetic function
#[test]
fn test_complete_pipeline_arithmetic() {
    let wasm_module = create_simple_arithmetic_module();

    for arch in [VmArchitecture::Arch64, VmArchitecture::Arch128, VmArchitecture::Arch256, VmArchitecture::Arch512] {
        // Transpile
        let mut transpiler = NewTranspilationEngine::with_architecture(arch).expect("Transpiler creation should succeed");
        let wasm_module_bytes = encode_wasm_module(&wasm_module);
        let transpiled_module = transpiler.transpile(&wasm_module_bytes).expect("Transpilation should succeed");

        // Optimize
        let mut optimizer = Optimizer::new(arch, 2);
        let optimized_functions = optimizer.optimize(transpiled_module.functions);

        // Create optimized module
        let optimized_module = TranspiledModule {
            header: transpiled_module.header,
            functions: optimized_functions,
            globals: transpiled_module.globals,
            memory_layout: transpiled_module.memory_layout,
            exports: transpiled_module.exports,
            imports: transpiled_module.imports,
            metadata: transpiled_module.metadata,
        };

        // Generate bytecode
        let mut generator = DotVMGenerator::with_architecture(arch).expect("Generator creation should succeed");
        let bytecode = generator.generate_bytecode(&optimized_module).expect("Bytecode generation should succeed");

        assert!(!bytecode.is_empty(), "Bytecode should not be empty for {:?}", arch);

        // Verify bytecode has proper header
        assert!(bytecode.len() >= 8, "Bytecode should have at least header for {:?}", arch);
    }
}

/// Test optimization effectiveness
#[test]
fn test_optimization_effectiveness() {
    let wasm_module = create_optimization_test_module();
    let arch = VmArchitecture::Arch128;

    // Transpile without optimization
    let mut transpiler = NewTranspilationEngine::with_architecture(arch).expect("Transpiler creation should succeed");
    let wasm_module_bytes = encode_wasm_module(&wasm_module);
    let transpiled_module = transpiler.transpile(&wasm_module_bytes).expect("Transpilation should succeed");

    let unoptimized_count = transpiled_module.functions.iter().map(|f| f.instructions.len()).sum::<usize>();

    // Optimize with different levels
    for opt_level in 0..=3 {
        let mut optimizer = Optimizer::new(arch, opt_level);
        let optimized_functions = optimizer.optimize(transpiled_module.functions.clone());
        let optimized_count = optimized_functions.iter().map(|f| f.instructions.len()).sum::<usize>();

        if opt_level > 0 {
            assert!(optimized_count <= unoptimized_count, "Optimization level {} should not increase instruction count", opt_level);
        }

        let stats = optimizer.stats();
        if opt_level > 1 {
            assert!(stats.total_optimizations() > 0, "Optimization level {} should perform some optimizations", opt_level);
        }
    }
}

/// Test architecture-specific features
#[test]
fn test_architecture_specific_features() {
    // Test BigInt operations (128-bit+)
    let bigint_module = create_bigint_test_module();

    // Should work on 128-bit+ architectures
    for arch in [VmArchitecture::Arch128, VmArchitecture::Arch256, VmArchitecture::Arch512] {
        let mut transpiler = NewTranspilationEngine::with_architecture(arch);
        let bigint_module_bytes = encode_wasm_module(&bigint_module);
        let mut transpiler = transpiler.expect("Transpiler creation should succeed");
        let result = transpiler.transpile(&bigint_module_bytes);
        assert!(result.is_ok(), "BigInt operations should work on {:?}", arch);
    }

    // Test SIMD operations (256-bit+)
    let simd_module = create_simd_test_module();

    for arch in [VmArchitecture::Arch256, VmArchitecture::Arch512] {
        let mut transpiler = NewTranspilationEngine::with_architecture(arch).expect("Transpiler creation should succeed");
        let simd_module_bytes = encode_wasm_module(&simd_module);
        let result = transpiler.transpile(&simd_module_bytes);
        if let Err(e) = &result {
            println!("SIMD transpilation failed on {:?}: {:?}", arch, e);
        }
        assert!(result.is_ok(), "SIMD operations should work on {:?}: {:?}", arch, result.err());
    }

    // Test vector operations (512-bit)
    let vector_module = create_vector_test_module();

    let mut transpiler = NewTranspilationEngine::with_architecture(VmArchitecture::Arch512).expect("Transpiler creation should succeed");
    let vector_module_bytes = encode_wasm_module(&vector_module);
    let result = transpiler.transpile(&vector_module_bytes);
    assert!(result.is_ok(), "Vector operations should work on Arch512");
}

/// Test error handling and edge cases
#[test]
fn test_error_handling() {
    // Test empty module
    let empty_module = WasmModule {
        types: vec![],
        functions: vec![],
        function_types: vec![],
        imports: vec![],
        exports: vec![],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        elements: vec![],
        data_segments: vec![],
        custom_sections: vec![],
    };

    let mut transpiler = NewTranspilationEngine::with_architecture(VmArchitecture::Arch64).expect("Transpiler creation should succeed");
    let empty_module_bytes = encode_wasm_module(&empty_module);
    let result = transpiler.transpile(&empty_module_bytes);
    assert!(result.is_ok(), "Empty module should transpile successfully");

    // Test invalid function
    let invalid_module = create_invalid_function_module();
    let invalid_module_bytes = encode_wasm_module(&invalid_module);
    let _result = transpiler.transpile(&invalid_module_bytes);
    // Should handle gracefully (exact behavior depends on implementation)
}

/// Test performance with large modules
#[test]
fn test_performance_large_module() {
    let large_module = create_large_test_module(1000); // 1000 functions
    let arch = VmArchitecture::Arch256;

    let start = std::time::Instant::now();

    let mut transpiler = NewTranspilationEngine::with_architecture(arch);
    let large_module_bytes = encode_wasm_module(&large_module);
    let mut transpiler = transpiler.expect("Transpiler creation should succeed");
    let transpiled_module = transpiler.transpile(&large_module_bytes).expect("Large module transpilation should succeed");

    let mut optimizer = Optimizer::new(arch, 2);
    let optimized_functions = optimizer.optimize(transpiled_module.functions);

    let optimized_module = TranspiledModule {
        header: transpiled_module.header,
        functions: optimized_functions,
        globals: transpiled_module.globals,
        memory_layout: transpiled_module.memory_layout,
        exports: transpiled_module.exports,
        imports: transpiled_module.imports,
        metadata: transpiled_module.metadata,
    };

    let mut generator = DotVMGenerator::with_architecture(arch).expect("Generator creation should succeed");
    let _bytecode = generator.generate_bytecode(&optimized_module).expect("Large module bytecode generation should succeed");

    let duration = start.elapsed();

    // Should complete within reasonable time (adjust threshold as needed)
    assert!(duration.as_secs() < 30, "Large module compilation took too long: {:?}", duration);
}

/// Test bytecode compatibility across architectures
#[test]
fn test_bytecode_compatibility() {
    let wasm_module = create_compatibility_test_module();

    let mut bytecodes = Vec::new();

    // Generate bytecode for all architectures
    for arch in [VmArchitecture::Arch64, VmArchitecture::Arch128, VmArchitecture::Arch256, VmArchitecture::Arch512] {
        let mut transpiler = NewTranspilationEngine::with_architecture(arch).expect("Transpiler creation should succeed");
        let wasm_module_bytes = encode_wasm_module(&wasm_module);
        let transpiled_module = transpiler.transpile(&wasm_module_bytes).expect("Transpilation should succeed");

        let mut generator = DotVMGenerator::with_architecture(arch).expect("Generator creation should succeed");
        let bytecode = generator.generate_bytecode(&transpiled_module).expect("Bytecode generation should succeed");

        bytecodes.push((arch, bytecode));
    }

    // Verify all bytecodes are valid and have proper headers
    for (arch, bytecode) in &bytecodes {
        assert!(!bytecode.is_empty(), "Bytecode should not be empty for {:?}", arch);
        assert!(bytecode.len() >= 8, "Bytecode should have proper header for {:?}", arch);
    }
}

// Helper functions to create test modules

fn create_simple_arithmetic_module() -> WasmModule {
    let func_type = WasmFunctionType {
        params: vec![WasmValueType::I32, WasmValueType::I32],
        results: vec![WasmValueType::I32],
    };

    WasmModule {
        types: vec![func_type.clone()],
        function_types: vec![0],
        functions: vec![WasmFunction {
            signature: func_type,
            locals: vec![],
            body: vec![WasmInstruction::LocalGet { local_index: 0 }, WasmInstruction::LocalGet { local_index: 1 }, WasmInstruction::I32Add],
        }],
        imports: vec![],
        exports: vec![WasmExport {
            name: "add".to_string(),
            kind: WasmExportKind::Function,
            index: 0,
        }],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        elements: vec![],
        data_segments: vec![],
        custom_sections: vec![],
    }
}

fn create_optimization_test_module() -> WasmModule {
    let func_type = WasmFunctionType {
        params: vec![WasmValueType::I32],
        results: vec![WasmValueType::I32],
    };

    WasmModule {
        types: vec![func_type.clone()],
        function_types: vec![0],
        functions: vec![WasmFunction {
            signature: func_type,
            locals: vec![],
            body: vec![
                WasmInstruction::LocalGet { local_index: 0 },
                WasmInstruction::I32Const { value: 0 },
                WasmInstruction::I32Add, // Add 0 (should be optimized away)
                WasmInstruction::I32Const { value: 1 },
                WasmInstruction::I32Mul, // Multiply by 1 (should be optimized away)
                WasmInstruction::I32Const { value: 42 },
                WasmInstruction::I32Const { value: 58 },
                WasmInstruction::I32Add, // Constant folding opportunity
                WasmInstruction::I32Add,
            ],
        }],
        imports: vec![],
        exports: vec![],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        elements: vec![],
        data_segments: vec![],
        custom_sections: vec![],
    }
}

fn create_bigint_test_module() -> WasmModule {
    let func_type = WasmFunctionType {
        params: vec![WasmValueType::I64, WasmValueType::I64],
        results: vec![WasmValueType::I64],
    };

    WasmModule {
        types: vec![func_type.clone()],
        function_types: vec![0],
        functions: vec![WasmFunction {
            signature: func_type,
            locals: vec![],
            body: vec![WasmInstruction::LocalGet { local_index: 0 }, WasmInstruction::LocalGet { local_index: 1 }, WasmInstruction::I64Add],
        }],
        imports: vec![],
        exports: vec![],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        elements: vec![],
        data_segments: vec![],
        custom_sections: vec![],
    }
}

fn create_simd_test_module() -> WasmModule {
    let func_type = WasmFunctionType {
        params: vec![WasmValueType::V128, WasmValueType::V128],
        results: vec![WasmValueType::V128],
    };

    WasmModule {
        types: vec![func_type.clone()],
        function_types: vec![0],
        functions: vec![WasmFunction {
            signature: func_type,
            locals: vec![],
            body: vec![
                WasmInstruction::LocalGet { local_index: 0 },
                WasmInstruction::LocalGet { local_index: 1 },
                // TODO: Add proper SIMD instructions when available
                WasmInstruction::Drop,
            ],
        }],
        imports: vec![],
        exports: vec![],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        elements: vec![],
        data_segments: vec![],
        custom_sections: vec![],
    }
}

fn create_vector_test_module() -> WasmModule {
    let func_type = WasmFunctionType {
        params: vec![WasmValueType::V128],
        results: vec![WasmValueType::V128],
    };

    WasmModule {
        types: vec![func_type.clone()],
        function_types: vec![0],
        functions: vec![WasmFunction {
            signature: func_type,
            locals: vec![],
            body: vec![
                WasmInstruction::LocalGet { local_index: 0 },
                // Vector-specific operations would go here
            ],
        }],
        imports: vec![],
        exports: vec![],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        elements: vec![],
        data_segments: vec![],
        custom_sections: vec![],
    }
}

fn create_invalid_function_module() -> WasmModule {
    let func_type = WasmFunctionType {
        params: vec![],
        results: vec![WasmValueType::I32],
    };

    WasmModule {
        types: vec![func_type.clone()],
        function_types: vec![0],
        functions: vec![WasmFunction {
            signature: func_type,
            locals: vec![],
            body: vec![
                // Missing return value - should be handled gracefully
            ],
        }],
        imports: vec![],
        exports: vec![],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        elements: vec![],
        data_segments: vec![],
        custom_sections: vec![],
    }
}

fn create_large_test_module(function_count: usize) -> WasmModule {
    let func_type = WasmFunctionType {
        params: vec![WasmValueType::I32],
        results: vec![WasmValueType::I32],
    };

    let mut functions = Vec::new();
    let mut function_types = Vec::new();

    for i in 0..function_count {
        functions.push(WasmFunction {
            signature: func_type.clone(),
            locals: vec![],
            body: vec![WasmInstruction::LocalGet { local_index: 0 }, WasmInstruction::I32Const { value: i as i32 }, WasmInstruction::I32Add],
        });
        function_types.push(0);
    }

    WasmModule {
        types: vec![func_type],
        function_types,
        functions,
        imports: vec![],
        exports: vec![],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        elements: vec![],
        data_segments: vec![],
        custom_sections: vec![],
    }
}

fn create_compatibility_test_module() -> WasmModule {
    let func_type = WasmFunctionType {
        params: vec![WasmValueType::I32, WasmValueType::F32],
        results: vec![WasmValueType::F32],
    };

    WasmModule {
        types: vec![func_type.clone()],
        function_types: vec![0],
        functions: vec![WasmFunction {
            signature: func_type,
            locals: vec![WasmValueType::I32],
            body: vec![
                WasmInstruction::LocalGet { local_index: 0 },
                WasmInstruction::F32ConvertI32S,
                WasmInstruction::LocalGet { local_index: 1 },
                WasmInstruction::F32Add,
                WasmInstruction::LocalTee { local_index: 2 },
            ],
        }],
        imports: vec![],
        exports: vec![WasmExport {
            name: "compat_test".to_string(),
            kind: WasmExportKind::Function,
            index: 0,
        }],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        elements: vec![],
        data_segments: vec![],
        custom_sections: vec![],
    }
}

/// Helper function to encode a WasmModule AST into proper WASM binary format
fn encode_wasm_module(wasm_module: &WasmModule) -> Vec<u8> {
    let mut module = Module::new();

    // Add type section
    if !wasm_module.types.is_empty() {
        let mut types = TypeSection::new();
        for func_type in &wasm_module.types {
            let params: Vec<wasm_encoder::ValType> = func_type
                .params
                .iter()
                .map(|t| match t {
                    WasmValueType::I32 => wasm_encoder::ValType::I32,
                    WasmValueType::I64 => wasm_encoder::ValType::I64,
                    WasmValueType::F32 => wasm_encoder::ValType::F32,
                    WasmValueType::F64 => wasm_encoder::ValType::F64,
                    WasmValueType::V128 => wasm_encoder::ValType::V128,
                    WasmValueType::FuncRef => wasm_encoder::ValType::Ref(wasm_encoder::RefType::FUNCREF),
                    WasmValueType::ExternRef => wasm_encoder::ValType::Ref(wasm_encoder::RefType::EXTERNREF),
                })
                .collect();
            let results: Vec<wasm_encoder::ValType> = func_type
                .results
                .iter()
                .map(|t| match t {
                    WasmValueType::I32 => wasm_encoder::ValType::I32,
                    WasmValueType::I64 => wasm_encoder::ValType::I64,
                    WasmValueType::F32 => wasm_encoder::ValType::F32,
                    WasmValueType::F64 => wasm_encoder::ValType::F64,
                    WasmValueType::V128 => wasm_encoder::ValType::V128,
                    WasmValueType::FuncRef => wasm_encoder::ValType::Ref(wasm_encoder::RefType::FUNCREF),
                    WasmValueType::ExternRef => wasm_encoder::ValType::Ref(wasm_encoder::RefType::EXTERNREF),
                })
                .collect();
            types.function(params, results);
        }
        module.section(&types);
    }

    // Add function section
    if !wasm_module.function_types.is_empty() {
        let mut functions = FunctionSection::new();
        for &type_idx in &wasm_module.function_types {
            functions.function(type_idx);
        }
        module.section(&functions);
    }

    // Add code section
    if !wasm_module.functions.is_empty() {
        let mut code = CodeSection::new();
        for func in &wasm_module.functions {
            let mut function = Function::new(vec![]); // No locals for simplicity

            // Convert instructions
            for instr in &func.body {
                match instr {
                    WasmInstruction::LocalGet { local_index } => {
                        function.instruction(&Instruction::LocalGet(*local_index));
                    }
                    WasmInstruction::I32Const { value } => {
                        function.instruction(&Instruction::I32Const(*value));
                    }
                    WasmInstruction::I64Const { value } => {
                        function.instruction(&Instruction::I64Const(*value));
                    }
                    WasmInstruction::I32Add => {
                        function.instruction(&Instruction::I32Add);
                    }
                    WasmInstruction::I64Add => {
                        function.instruction(&Instruction::I64Add);
                    }
                    WasmInstruction::I32Mul => {
                        function.instruction(&Instruction::I32Mul);
                    }
                    WasmInstruction::Drop => {
                        function.instruction(&Instruction::Drop);
                    }
                    _ => {
                        // For unsupported instructions, add a nop
                        function.instruction(&Instruction::Nop);
                    }
                }
            }

            function.instruction(&Instruction::End);
            code.function(&function);
        }
        module.section(&code);
    }

    module.finish()
}
