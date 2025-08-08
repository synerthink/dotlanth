// Integration test for Week 2: Core Execution Engine
use dotvm_core::bytecode::{BytecodeFile, ConstantValue, VmArchitecture};
use dotvm_core::opcode::arithmetic_opcodes::ArithmeticOpcode;
use dotvm_core::opcode::stack_opcodes::StackOpcode;
use dotvm_core::security::capability_manager::{Capability, CapabilityMetadata};
use dotvm_core::security::resource_limiter::ResourceLimits;
use dotvm_core::security::types::{OpcodeArchitecture, OpcodeCategory, OpcodeType, SecurityLevel};
use dotvm_core::vm::executor::VmExecutor;
use dotvm_core::vm::stack::StackValue;
use std::collections::HashMap;
use std::time::SystemTime;

/// Helper function to create a test executor with security capabilities
fn create_test_executor() -> VmExecutor {
    let mut executor = VmExecutor::new_with_dot_id("test_dot".to_string());

    // Grant all necessary capabilities for testing
    let capabilities = vec![
        Capability {
            id: "test_stack_cap".to_string(),
            opcode_type: OpcodeType::Standard {
                architecture: OpcodeArchitecture::Arch64,
                category: OpcodeCategory::Stack,
            },
            permissions: vec![],
            resource_limits: ResourceLimits::default(),
            expiration: None,
            metadata: CapabilityMetadata {
                created_at: SystemTime::now(),
                granted_by: "test_system".to_string(),
                purpose: "Testing stack operations".to_string(),
                usage_count: 0,
                last_used: None,
                custom_data: HashMap::new(),
            },
            delegatable: false,
            required_security_level: SecurityLevel::Development,
        },
        Capability {
            id: "test_arithmetic_cap".to_string(),
            opcode_type: OpcodeType::Standard {
                architecture: OpcodeArchitecture::Arch64,
                category: OpcodeCategory::Arithmetic,
            },
            permissions: vec![],
            resource_limits: ResourceLimits::default(),
            expiration: None,
            metadata: CapabilityMetadata {
                created_at: SystemTime::now(),
                granted_by: "test_system".to_string(),
                purpose: "Testing arithmetic operations".to_string(),
                usage_count: 0,
                last_used: None,
                custom_data: HashMap::new(),
            },
            delegatable: false,
            required_security_level: SecurityLevel::Development,
        },
    ];

    // Grant capabilities to the test dot
    for capability in capabilities {
        if let Err(e) = executor
            .security_sandbox
            .capability_manager
            .grant_capability("test_dot".to_string(), capability, "test_system".to_string())
        {
            eprintln!("Warning: Failed to grant test capability: {}", e);
        }
    }

    executor
}

#[test]
fn test_week2_core_execution_engine() {
    println!("=== Week 2: Core Execution Engine Integration Test ===");

    // Create a VM executor with security capabilities
    let mut executor = create_test_executor();

    // Create a simple program that demonstrates our capabilities
    let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);

    // Add constants to the constant pool
    let hello_id = bytecode.add_constant(ConstantValue::String("Hello".to_string()));
    let world_id = bytecode.add_constant(ConstantValue::String(" World!".to_string()));
    let number_id = bytecode.add_constant(ConstantValue::Int64(42));

    println!("\n1. Creating bytecode program...");
    println!("   Constants added:");
    println!("   - String: 'Hello' (ID: {})", hello_id);
    println!("   - String: ' World!' (ID: {})", world_id);
    println!("   - Int64: 42 (ID: {})", number_id);

    // Create a program that:
    // 1. Pushes "Hello" and " World!" and concatenates them (string addition)
    // 2. Pushes some numbers and does arithmetic
    // 3. Demonstrates stack operations

    // Program: String concatenation
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &hello_id.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &world_id.to_le_bytes());
    bytecode.add_instruction(ArithmeticOpcode::Add.as_u8(), &[]); // String concatenation

    // Program: Arithmetic operations
    bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[10]); // Push 10
    bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[5]); // Push 5
    bytecode.add_instruction(ArithmeticOpcode::Add.as_u8(), &[]); // 10 + 5 = 15

    bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[3]); // Push 3
    bytecode.add_instruction(ArithmeticOpcode::Multiply.as_u8(), &[]); // 15 * 3 = 45

    // Program: Stack operations
    bytecode.add_instruction(StackOpcode::Dup.as_u8(), &[]); // Duplicate 45
    bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[5]); // Push 5
    bytecode.add_instruction(ArithmeticOpcode::Subtract.as_u8(), &[]); // 45 - 5 = 40
    bytecode.add_instruction(StackOpcode::Swap.as_u8(), &[]); // Swap top two values

    println!("\n2. Program created with {} bytes of bytecode", bytecode.code.len());
    println!("   Instructions:");
    println!("   - PUSH 'Hello'");
    println!("   - PUSH ' World!'");
    println!("   - ADD (string concatenation)");
    println!("   - PUSH_INT8 10");
    println!("   - PUSH_INT8 5");
    println!("   - ADD (10 + 5 = 15)");
    println!("   - PUSH_INT8 3");
    println!("   - MUL (15 * 3 = 45)");
    println!("   - DUP (duplicate 45)");
    println!("   - PUSH_INT8 5");
    println!("   - SUB (45 - 5 = 40)");
    println!("   - SWAP (swap top two values)");

    // Load and execute the bytecode
    println!("\n3. Loading bytecode into VM...");
    executor.load_bytecode(bytecode).unwrap();

    println!("\n4. Executing bytecode...");
    let result = executor.execute().unwrap();

    println!("\n5. Execution completed!");
    println!("   Instructions executed: {}", result.instructions_executed);
    println!("   Execution time: {:?}", result.execution_time);
    println!("   Final stack size: {}", result.final_stack.len());

    // Verify the final stack state
    println!("\n6. Final stack contents:");
    for (i, value) in result.final_stack.iter().enumerate() {
        println!("   Stack[{}]: {}", i, value);
    }

    // Expected stack (from bottom to top):
    // 0: "Hello World!" (string concatenation result)
    // 1: 40 (45 - 5 result) - after SWAP, this moved down
    // 2: 45 (duplicated value) - after SWAP, this moved up

    assert_eq!(result.final_stack.len(), 3);
    assert_eq!(result.final_stack[0], StackValue::String("Hello World!".to_string()));
    assert_eq!(result.final_stack[1], StackValue::Int64(40));
    assert_eq!(result.final_stack[2], StackValue::Int64(45));

    println!("\n✅ Week 2 Core Execution Engine test completed successfully!");
    println!("   - ✅ Bytecode format working");
    println!("   - ✅ Constant pool working");
    println!("   - ✅ Stack operations working (PUSH, POP, DUP, SWAP)");
    println!("   - ✅ Arithmetic operations working (ADD, SUB, MUL)");
    println!("   - ✅ String concatenation working");
    println!("   - ✅ VM execution loop working");
    println!("   - ✅ Fetch-decode-execute cycle working");
}

#[test]
fn test_step_by_step_execution() {
    println!("=== Step-by-Step Execution Test ===");

    let mut executor = create_test_executor();
    let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);

    // Simple program: PUSH 1, PUSH 2, ADD
    bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[1]);
    bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[2]);
    bytecode.add_instruction(ArithmeticOpcode::Add.as_u8(), &[]);

    executor.load_bytecode(bytecode).unwrap();
    executor.enable_step();

    println!("\n1. Executing step by step...");

    // Step 1: PUSH 1
    let step1 = executor.step().unwrap();
    println!("   Step 1: {:?}", step1);
    assert_eq!(executor.context().stack.size(), 1);

    // Step 2: PUSH 2
    let step2 = executor.step().unwrap();
    println!("   Step 2: {:?}", step2);
    assert_eq!(executor.context().stack.size(), 2);

    // Step 3: ADD
    let step3 = executor.step().unwrap();
    println!("   Step 3: {:?}", step3);
    assert_eq!(executor.context().stack.size(), 1);

    // Verify final result
    let final_value = executor.context().stack.peek().unwrap();
    assert_eq!(*final_value, StackValue::Int64(3));

    println!("\n✅ Step-by-step execution working correctly!");
}

#[test]
fn test_error_handling() {
    println!("=== Error Handling Test ===");

    let mut executor = create_test_executor();
    let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);

    // Create a program that will cause division by zero
    bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[10]);
    bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[0]);
    bytecode.add_instruction(ArithmeticOpcode::Divide.as_u8(), &[]);

    executor.load_bytecode(bytecode).unwrap();

    println!("\n1. Testing division by zero error handling...");
    let result = executor.execute();

    match result {
        Err(e) => {
            println!("   ✅ Correctly caught error: {}", e);
            assert!(e.to_string().contains("Division by zero"));
        }
        Ok(_) => panic!("Expected division by zero error"),
    }

    println!("\n✅ Error handling working correctly!");
}
