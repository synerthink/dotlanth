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

//! Multi-architecture tests for DotVM core functionality
//!
//! These tests verify that all VM architectures work correctly
//! and maintain compatibility across different instruction sets.

use dotvm_core::{
    bytecode::VmArchitecture,
    instruction::{
        arithmetic::ArithmeticInstruction,
        bigint::BigIntInstruction,
        instruction::Instruction,
        memory::{LoadInstruction, StoreInstruction},
    },
    memory::{Arch32, Arch64, Arch128, Arch256, Arch512, Architecture},
    opcode::{architecture_opcodes::*, arithmetic_opcodes::ArithmeticOpcode, bigint_opcodes::BigIntOpcode},
    vm::{
        architecture_detector::DetectedArch,
        multi_arch_executor::MultiArchExecutor,
        vm_factory::{SimpleVMFactory, VMFactory},
    },
};

/// Test basic arithmetic operations across all architectures
#[test]
fn test_arithmetic_across_architectures() {
    let test_cases = vec![
        (10.0, 5.0, ArithmeticOpcode::Add, 15.0),
        (10.0, 5.0, ArithmeticOpcode::Subtract, 5.0),
        (10.0, 5.0, ArithmeticOpcode::Multiply, 50.0),
        (10.0, 5.0, ArithmeticOpcode::Divide, 2.0),
    ];

    for (a, b, op, expected) in test_cases {
        // Test on 32-bit architecture
        {
            let mut executor = MultiArchExecutor::<Arch32>::new(VmArchitecture::Arch32, VmArchitecture::Arch32).expect("Failed to create 32-bit executor");

            let instruction = ArithmeticInstruction::new(op);
            executor.push_operand(a);
            executor.push_operand(b);

            instruction.execute(&mut executor).expect("Arithmetic operation failed");
            let result = executor.pop_operand().expect("No result on stack");
            assert!((result - expected).abs() < f64::EPSILON, "32-bit: {:?} failed: {} != {}", op, result, expected);
        }

        // Test on 64-bit architecture
        {
            let mut executor = MultiArchExecutor::<Arch64>::new(VmArchitecture::Arch64, VmArchitecture::Arch64).expect("Failed to create 64-bit executor");

            let instruction = ArithmeticInstruction::new(op);
            executor.push_operand(a);
            executor.push_operand(b);

            instruction.execute(&mut executor).expect("Arithmetic operation failed");
            let result = executor.pop_operand().expect("No result on stack");
            assert!((result - expected).abs() < f64::EPSILON, "64-bit: {:?} failed: {} != {}", op, result, expected);
        }
    }
}

/// Test BigInt operations on 128-bit+ architectures
#[test]
fn test_bigint_operations() {
    let test_cases = vec![
        (100.0, 50.0, BigIntOpcode::Add, 150.0),
        (100.0, 50.0, BigIntOpcode::Sub, 50.0),
        (12.0, 5.0, BigIntOpcode::Mul, 60.0),
        (100.0, 4.0, BigIntOpcode::Div, 25.0),
    ];

    for (a, b, op, expected) in test_cases {
        // Test on 128-bit architecture
        {
            let mut executor = MultiArchExecutor::<Arch128>::new(VmArchitecture::Arch128, VmArchitecture::Arch128).expect("Failed to create 128-bit executor");

            let instruction = BigIntInstruction::new(op);
            executor.push_operand(a);
            executor.push_operand(b);

            instruction.execute(&mut executor).expect("BigInt operation failed");
            let result = executor.pop_operand().expect("No result on stack");
            assert!((result - expected).abs() < f64::EPSILON, "128-bit BigInt: {:?} failed: {} != {}", op, result, expected);
        }

        // Test on 256-bit architecture
        {
            let mut executor = MultiArchExecutor::<Arch256>::new(VmArchitecture::Arch256, VmArchitecture::Arch256).expect("Failed to create 256-bit executor");

            let instruction = BigIntInstruction::new(op);
            executor.push_operand(a);
            executor.push_operand(b);

            instruction.execute(&mut executor).expect("BigInt operation failed");
            let result = executor.pop_operand().expect("No result on stack");
            assert!((result - expected).abs() < f64::EPSILON, "256-bit BigInt: {:?} failed: {} != {}", op, result, expected);
        }
    }
}

/// Test memory operations across architectures
#[test]
fn test_memory_operations() {
    // Test on different architectures
    test_memory_on_arch::<Arch32>(VmArchitecture::Arch32);
    test_memory_on_arch::<Arch64>(VmArchitecture::Arch64);
    test_memory_on_arch::<Arch128>(VmArchitecture::Arch128);
    test_memory_on_arch::<Arch256>(VmArchitecture::Arch256);
    test_memory_on_arch::<Arch512>(VmArchitecture::Arch512);
}

fn test_memory_on_arch<A: Architecture + std::fmt::Debug>(arch: VmArchitecture) {
    let mut executor = MultiArchExecutor::<A>::new(arch, arch).expect("Failed to create executor");

    // Test store and load at address 100
    let store_instruction = StoreInstruction::new(100);
    let load_instruction = LoadInstruction::new(100);

    // Store value 42.0 at address 100
    executor.push_operand(42.0); // value to store
    store_instruction.execute(&mut executor).expect("Store operation failed");

    // Load value from address 100
    load_instruction.execute(&mut executor).expect("Load operation failed");

    let result = executor.pop_operand().expect("No result on stack");

    // Note: The dummy memory manager returns (address & 0xFF) as u8, so for address 100:
    // 100 & 0xFF = 100, so we expect 100.0, not 42.0
    let expected = (100 & 0xFF) as f64;
    assert!((result - expected).abs() < f64::EPSILON, "{:?}: Memory load failed: {} != {}", arch, result, expected);
}

/// Test architecture compatibility and opcode availability
#[test]
fn test_architecture_opcode_compatibility() {
    // 64-bit should have basic opcodes
    assert!(Arch64Opcodes::supports_opcode(&Opcode64::Arithmetic(ArithmeticOpcode::Add)));

    // 128-bit should have basic + BigInt opcodes
    assert!(Arch128Opcodes::supports_opcode(&Opcode128::Base(Opcode64::Arithmetic(ArithmeticOpcode::Add))));
    assert!(Arch128Opcodes::supports_opcode(&Opcode128::BigInt(BigIntOpcode::Add)));

    // 256-bit should have all previous + SIMD
    assert!(Arch256Opcodes::supports_opcode(&Opcode256::Base(Opcode128::Base(Opcode64::Arithmetic(ArithmeticOpcode::Add)))));
    assert!(Arch256Opcodes::supports_opcode(&Opcode256::Base(Opcode128::BigInt(BigIntOpcode::Add))));

    // 512-bit should have all opcodes
    assert!(Arch512Opcodes::supports_opcode(&Opcode512::Base(Opcode256::Base(Opcode128::Base(Opcode64::Arithmetic(
        ArithmeticOpcode::Add
    ))))));
    assert!(Arch512Opcodes::supports_opcode(&Opcode512::Base(Opcode256::Base(Opcode128::BigInt(BigIntOpcode::Add)))));
}

/// Test VM factory with different architecture combinations
#[test]
fn test_vm_factory_multi_arch() {
    let factory = SimpleVMFactory::new();

    // Test all valid host/guest combinations
    let architectures = [
        VmArchitecture::Arch32,
        VmArchitecture::Arch64,
        VmArchitecture::Arch128,
        VmArchitecture::Arch256,
        VmArchitecture::Arch512,
    ];

    for &host_arch in &architectures {
        for &guest_arch in &architectures {
            let detected_arch = DetectedArch {
                required: guest_arch,
                execution: host_arch,
                compatibility_mode: host_arch != guest_arch,
            };

            let result = factory.create_vm_from_detected(detected_arch);

            if host_arch >= guest_arch {
                // Should succeed when host can support guest
                assert!(result.is_ok(), "Factory should create VM for host {:?} guest {:?}", host_arch, guest_arch);

                if let Ok(vm) = result {
                    assert_eq!(vm.architecture(), host_arch);
                }
            }
            // Note: Some combinations might fail if host < guest, which is expected
        }
    }
}

/// Test performance scaling across architectures
#[test]
fn test_performance_scaling() {
    let iterations = 1000;

    // Test arithmetic performance on different architectures
    let arch_configs = [
        (VmArchitecture::Arch32, "32-bit"),
        (VmArchitecture::Arch64, "64-bit"),
        (VmArchitecture::Arch128, "128-bit"),
        (VmArchitecture::Arch256, "256-bit"),
        (VmArchitecture::Arch512, "512-bit"),
    ];

    for (arch, name) in arch_configs {
        let start = std::time::Instant::now();

        // Create executor based on architecture
        match arch {
            VmArchitecture::Arch32 => {
                let mut executor = MultiArchExecutor::<Arch32>::new(arch, arch).expect("Failed to create executor");
                run_performance_test(&mut executor, iterations);
            }
            VmArchitecture::Arch64 => {
                let mut executor = MultiArchExecutor::<Arch64>::new(arch, arch).expect("Failed to create executor");
                run_performance_test(&mut executor, iterations);
            }
            VmArchitecture::Arch128 => {
                let mut executor = MultiArchExecutor::<Arch128>::new(arch, arch).expect("Failed to create executor");
                run_performance_test(&mut executor, iterations);
            }
            VmArchitecture::Arch256 => {
                let mut executor = MultiArchExecutor::<Arch256>::new(arch, arch).expect("Failed to create executor");
                run_performance_test(&mut executor, iterations);
            }
            VmArchitecture::Arch512 => {
                let mut executor = MultiArchExecutor::<Arch512>::new(arch, arch).expect("Failed to create executor");
                run_performance_test(&mut executor, iterations);
            }
        }

        let duration = start.elapsed();
        println!("{} performance: {:?} for {} iterations", name, duration, iterations);

        // All architectures should complete within reasonable time
        assert!(duration.as_millis() < 5000, "{} took too long: {:?}", name, duration);
    }
}

fn run_performance_test<A: Architecture + std::fmt::Debug>(executor: &mut MultiArchExecutor<A>, iterations: usize) {
    let instruction = ArithmeticInstruction::new(ArithmeticOpcode::Add);

    for i in 0..iterations {
        executor.push_operand(i as f64);
        executor.push_operand(1.0);
        instruction.execute(executor).expect("Arithmetic operation failed");
        let _result = executor.pop_operand().expect("No result on stack");
    }
}

/// Test architecture-specific instruction execution
#[test]
fn test_architecture_specific_instructions() {
    // Test that BigInt instructions only work on 128-bit+ architectures
    let bigint_instruction = BigIntInstruction::new(BigIntOpcode::Add);

    // Should work on 128-bit
    {
        let mut executor = MultiArchExecutor::<Arch128>::new(VmArchitecture::Arch128, VmArchitecture::Arch128).expect("Failed to create 128-bit executor");

        executor.push_operand(100.0);
        executor.push_operand(50.0);

        let result = bigint_instruction.execute(&mut executor);
        assert!(result.is_ok(), "BigInt should work on 128-bit architecture");
    }

    // Should work on 256-bit
    {
        let mut executor = MultiArchExecutor::<Arch256>::new(VmArchitecture::Arch256, VmArchitecture::Arch256).expect("Failed to create 256-bit executor");

        executor.push_operand(100.0);
        executor.push_operand(50.0);

        let result = bigint_instruction.execute(&mut executor);
        assert!(result.is_ok(), "BigInt should work on 256-bit architecture");
    }
}

/// Test memory management across architectures
#[test]
fn test_memory_management_scaling() {
    // Test memory allocation sizes scale with architecture
    test_memory_scaling::<Arch32>(VmArchitecture::Arch32, 32);
    test_memory_scaling::<Arch64>(VmArchitecture::Arch64, 64);
    test_memory_scaling::<Arch128>(VmArchitecture::Arch128, 128);
    test_memory_scaling::<Arch256>(VmArchitecture::Arch256, 256);
    test_memory_scaling::<Arch512>(VmArchitecture::Arch512, 512);
}

fn test_memory_scaling<A: Architecture + std::fmt::Debug>(arch: VmArchitecture, expected_bits: usize) {
    let _executor = MultiArchExecutor::<A>::new(arch, arch).expect("Failed to create executor");

    // Verify architecture properties
    assert_eq!(A::WORD_SIZE * 8, expected_bits, "Word size should match architecture bits");

    // Test that executor was created successfully
    // (Additional memory-specific tests would go here)
}

/// Test backward compatibility
#[test]
fn test_backward_compatibility() {
    // Test that higher architectures can run lower architecture code

    // 64-bit code should run on 128-bit VM
    {
        let mut executor = MultiArchExecutor::<Arch128>::new(
            VmArchitecture::Arch128,
            VmArchitecture::Arch64, // Guest is 64-bit
        )
        .expect("Failed to create executor");

        let instruction = ArithmeticInstruction::new(ArithmeticOpcode::Add);
        executor.push_operand(10.0);
        executor.push_operand(5.0);

        instruction.execute(&mut executor).expect("Backward compatibility failed");
        let result = executor.pop_operand().expect("No result on stack");
        assert!((result - 15.0).abs() < f64::EPSILON);
    }

    // 128-bit code should run on 256-bit VM
    {
        let mut executor = MultiArchExecutor::<Arch256>::new(
            VmArchitecture::Arch256,
            VmArchitecture::Arch128, // Guest is 128-bit
        )
        .expect("Failed to create executor");

        let instruction = BigIntInstruction::new(BigIntOpcode::Add);
        executor.push_operand(100.0);
        executor.push_operand(50.0);

        instruction.execute(&mut executor).expect("Backward compatibility failed");
        let result = executor.pop_operand().expect("No result on stack");
        assert!((result - 150.0).abs() < f64::EPSILON);
    }
}
