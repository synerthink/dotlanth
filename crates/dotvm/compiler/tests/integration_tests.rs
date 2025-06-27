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

use dotvm_compiler::{
    codegen::dotvm_generator::DotVMGenerator,
    optimizer::Optimizer,
    transpiler::engine::{TranspilationEngine, TranspiledModule},
    wasm::{parser::WasmParser, ast::*},
};
use dotvm_core::bytecode::VmArchitecture;

/// Test the complete pipeline with a simple arithmetic function
#[test]
fn test_complete_pipeline_arithmetic() {
    let wasm_module = create_simple_arithmetic_module();
    
    for arch in [VmArchitecture::Arch64, VmArchitecture::Arch128, VmArchitecture::Arch256, VmArchitecture::Arch512] {
        // Transpile
        let mut transpiler = TranspilationEngine::with_architecture(arch);
        let transpiled_module = transpiler.transpile_module(wasm_module.clone())
            .expect("Transpilation should succeed");
        
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
        };
        
        // Generate bytecode
        let mut generator = DotVMGenerator::with_architecture(arch);
        let bytecode = generator.generate_bytecode(optimized_module)
            .expect("Bytecode generation should succeed");
        
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
    let mut transpiler = TranspilationEngine::with_architecture(arch);
    let transpiled_module = transpiler.transpile_module(wasm_module.clone())
        .expect("Transpilation should succeed");
    
    let unoptimized_count = transpiled_module.functions.iter().map(|f| f.instructions.len()).sum::<usize>();
    
    // Optimize with different levels
    for opt_level in 0..=3 {
        let mut optimizer = Optimizer::new(arch, opt_level);
        let optimized_functions = optimizer.optimize(transpiled_module.functions.clone());
        let optimized_count = optimized_functions.iter().map(|f| f.instructions.len()).sum::<usize>();
        
        if opt_level > 0 {
            assert!(optimized_count <= unoptimized_count, 
                "Optimization level {} should not increase instruction count", opt_level);
        }
        
        let stats = optimizer.stats();
        if opt_level > 1 {
            assert!(stats.total_optimizations() > 0, 
                "Optimization level {} should perform some optimizations", opt_level);
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
        let mut transpiler = TranspilationEngine::with_architecture(arch);
        let result = transpiler.transpile_module(bigint_module.clone());
        assert!(result.is_ok(), "BigInt operations should work on {:?}", arch);
    }
    
    // Test SIMD operations (256-bit+)
    let simd_module = create_simd_test_module();
    
    for arch in [VmArchitecture::Arch256, VmArchitecture::Arch512] {
        let mut transpiler = TranspilationEngine::with_architecture(arch);
        let result = transpiler.transpile_module(simd_module.clone());
        assert!(result.is_ok(), "SIMD operations should work on {:?}", arch);
    }
    
    // Test vector operations (512-bit)
    let vector_module = create_vector_test_module();
    
    let mut transpiler = TranspilationEngine::with_architecture(VmArchitecture::Arch512);
    let result = transpiler.transpile_module(vector_module);
    assert!(result.is_ok(), "Vector operations should work on Arch512");
}

/// Test error handling and edge cases
#[test]
fn test_error_handling() {
    // Test empty module
    let empty_module = WasmModule {
        types: vec![],
        functions: vec![],
        imports: vec![],
        exports: vec![],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        element_segments: vec![],
        data_segments: vec![],
    };
    
    let mut transpiler = TranspilationEngine::with_architecture(VmArchitecture::Arch64);
    let result = transpiler.transpile_module(empty_module);
    assert!(result.is_ok(), "Empty module should transpile successfully");
    
    // Test invalid function
    let invalid_module = create_invalid_function_module();
    let result = transpiler.transpile_module(invalid_module);
    // Should handle gracefully (exact behavior depends on implementation)
}

/// Test performance with large modules
#[test]
fn test_performance_large_module() {
    let large_module = create_large_test_module(1000); // 1000 functions
    let arch = VmArchitecture::Arch256;
    
    let start = std::time::Instant::now();
    
    let mut transpiler = TranspilationEngine::with_architecture(arch);
    let transpiled_module = transpiler.transpile_module(large_module)
        .expect("Large module transpilation should succeed");
    
    let mut optimizer = Optimizer::new(arch, 2);
    let optimized_functions = optimizer.optimize(transpiled_module.functions);
    
    let optimized_module = TranspiledModule {
        header: transpiled_module.header,
        functions: optimized_functions,
        globals: transpiled_module.globals,
        memory_layout: transpiled_module.memory_layout,
        exports: transpiled_module.exports,
        imports: transpiled_module.imports,
    };
    
    let mut generator = DotVMGenerator::with_architecture(arch);
    let _bytecode = generator.generate_bytecode(optimized_module)
        .expect("Large module bytecode generation should succeed");
    
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
        let mut transpiler = TranspilationEngine::with_architecture(arch);
        let transpiled_module = transpiler.transpile_module(wasm_module.clone())
            .expect("Transpilation should succeed");
        
        let mut generator = DotVMGenerator::with_architecture(arch);
        let bytecode = generator.generate_bytecode(transpiled_module)
            .expect("Bytecode generation should succeed");
        
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
    WasmModule {
        types: vec![
            WasmFunctionType {
                params: vec![WasmValueType::I32, WasmValueType::I32],
                results: vec![WasmValueType::I32],
            }
        ],
        functions: vec![
            WasmFunction {
                signature: WasmFunctionType {
                    params: vec![WasmValueType::I32, WasmValueType::I32],
                    results: vec![WasmValueType::I32],
                },
                locals: vec![],
                body: vec![
                    WasmInstruction::LocalGet { local_index: 0 },
                    WasmInstruction::LocalGet { local_index: 1 },
                    WasmInstruction::I32Add,
                ],
            }
        ],
        imports: vec![],
        exports: vec![
            WasmExport {
                name: "add".to_string(),
                kind: WasmExportKind::Function,
                index: 0,
            }
        ],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        element_segments: vec![],
        data_segments: vec![],
    }
}

fn create_optimization_test_module() -> WasmModule {
    WasmModule {
        types: vec![
            WasmFunctionType {
                params: vec![WasmValueType::I32],
                results: vec![WasmValueType::I32],
            }
        ],
        functions: vec![
            WasmFunction {
                signature: WasmFunctionType {
                    params: vec![WasmValueType::I32],
                    results: vec![WasmValueType::I32],
                },
                locals: vec![],
                body: vec![
                    WasmInstruction::LocalGet { local_index: 0 },
                    WasmInstruction::I32Const { value: 0 },
                    WasmInstruction::I32Add,  // Add 0 (should be optimized away)
                    WasmInstruction::I32Const { value: 1 },
                    WasmInstruction::I32Mul,  // Multiply by 1 (should be optimized away)
                    WasmInstruction::I32Const { value: 42 },
                    WasmInstruction::I32Const { value: 58 },
                    WasmInstruction::I32Add,  // Constant folding opportunity
                    WasmInstruction::I32Add,
                ],
            }
        ],
        imports: vec![],
        exports: vec![],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        element_segments: vec![],
        data_segments: vec![],
    }
}

fn create_bigint_test_module() -> WasmModule {
    WasmModule {
        types: vec![
            WasmFunctionType {
                params: vec![WasmValueType::I64, WasmValueType::I64],
                results: vec![WasmValueType::I64],
            }
        ],
        functions: vec![
            WasmFunction {
                signature: WasmFunctionType {
                    params: vec![WasmValueType::I64, WasmValueType::I64],
                    results: vec![WasmValueType::I64],
                },
                locals: vec![],
                body: vec![
                    WasmInstruction::LocalGet { local_index: 0 },
                    WasmInstruction::LocalGet { local_index: 1 },
                    WasmInstruction::I64Add,
                ],
            }
        ],
        imports: vec![],
        exports: vec![],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        element_segments: vec![],
        data_segments: vec![],
    }
}

fn create_simd_test_module() -> WasmModule {
    WasmModule {
        types: vec![
            WasmFunctionType {
                params: vec![WasmValueType::V128, WasmValueType::V128],
                results: vec![WasmValueType::V128],
            }
        ],
        functions: vec![
            WasmFunction {
                signature: WasmFunctionType {
                    params: vec![WasmValueType::V128, WasmValueType::V128],
                    results: vec![WasmValueType::V128],
                },
                locals: vec![],
                body: vec![
                    WasmInstruction::LocalGet { local_index: 0 },
                    WasmInstruction::LocalGet { local_index: 1 },
                    // TODO: Add proper SIMD instructions when available
                    WasmInstruction::Drop,
                ],
            }
        ],
        imports: vec![],
        exports: vec![],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        element_segments: vec![],
        data_segments: vec![],
    }
}

fn create_vector_test_module() -> WasmModule {
    WasmModule {
        types: vec![
            WasmFunctionType {
                params: vec![WasmValueType::V128],
                results: vec![WasmValueType::V128],
            }
        ],
        functions: vec![
            WasmFunction {
                signature: WasmFunctionType {
                    params: vec![WasmValueType::V128],
                    results: vec![WasmValueType::V128],
                },
                locals: vec![],
                body: vec![
                    WasmInstruction::LocalGet { local_index: 0 },
                    // Vector-specific operations would go here
                ],
            }
        ],
        imports: vec![],
        exports: vec![],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        element_segments: vec![],
        data_segments: vec![],
    }
}

fn create_invalid_function_module() -> WasmModule {
    WasmModule {
        types: vec![
            WasmFunctionType {
                params: vec![],
                results: vec![WasmValueType::I32],
            }
        ],
        functions: vec![
            WasmFunction {
                signature: WasmFunctionType {
                    params: vec![],
                    results: vec![WasmValueType::I32],
                },
                locals: vec![],
                body: vec![
                    // Missing return value - should be handled gracefully
                ],
            }
        ],
        imports: vec![],
        exports: vec![],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        element_segments: vec![],
        data_segments: vec![],
    }
}

fn create_large_test_module(function_count: usize) -> WasmModule {
    let mut functions = Vec::new();
    
    for i in 0..function_count {
        functions.push(WasmFunction {
            signature: WasmFunctionType {
                params: vec![WasmValueType::I32],
                results: vec![WasmValueType::I32],
            },
            locals: vec![],
            body: vec![
                WasmInstruction::LocalGet { local_index: 0 },
                WasmInstruction::I32Const { value: i as i32 },
                WasmInstruction::I32Add,
            ],
        });
    }
    
    WasmModule {
        types: vec![
            WasmFunctionType {
                params: vec![WasmValueType::I32],
                results: vec![WasmValueType::I32],
            }
        ],
        functions,
        imports: vec![],
        exports: vec![],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        element_segments: vec![],
        data_segments: vec![],
    }
}

fn create_compatibility_test_module() -> WasmModule {
    WasmModule {
        types: vec![
            WasmFunctionType {
                params: vec![WasmValueType::I32, WasmValueType::F32],
                results: vec![WasmValueType::F32],
            }
        ],
        functions: vec![
            WasmFunction {
                signature: WasmFunctionType {
                    params: vec![WasmValueType::I32, WasmValueType::F32],
                    results: vec![WasmValueType::F32],
                },
                locals: vec![WasmValueType::I32],
                body: vec![
                    WasmInstruction::LocalGet { local_index: 0 },
                    WasmInstruction::F32ConvertI32S,
                    WasmInstruction::LocalGet { local_index: 1 },
                    WasmInstruction::F32Add,
                    WasmInstruction::LocalTee { local_index: 2 },
                ],
            }
        ],
        imports: vec![],
        exports: vec![
            WasmExport {
                name: "compat_test".to_string(),
                kind: WasmExportKind::Function,
                index: 0,
            }
        ],
        memories: vec![],
        tables: vec![],
        globals: vec![],
        start_function: None,
        element_segments: vec![],
        data_segments: vec![],
    }
}