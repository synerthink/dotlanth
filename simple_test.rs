// Simple test to verify basic VM and database integration

use dotvm_core::bytecode::{BytecodeFile, VmArchitecture, ConstantValue};
use dotvm_core::opcode::stack_opcodes::StackOpcode;
use dotvm_core::opcode::db_opcodes::DatabaseOpcode;
use dotvm_core::vm::executor::VmExecutor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Simple DotVM + DotDB Test ===");
    
    // Create a simple bytecode program
    let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);
    
    // Test 1: Basic stack operations
    println!("\n1. Testing basic stack operations...");
    let hello_const = bytecode.add_constant(ConstantValue::String("Hello".to_string()));
    let world_const = bytecode.add_constant(ConstantValue::String("World".to_string()));
    
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &hello_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &world_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Pop.as_u8(), &[]);
    
    // Test 2: Database operations
    println!("2. Testing database operations...");
    let collection_const = bytecode.add_constant(ConstantValue::String("test_collection".to_string()));
    let doc_const = bytecode.add_constant(ConstantValue::String(r#"{"message": "Hello from VM!"}"#.to_string()));
    
    // Create collection
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbCreateCollection.as_u8(), &[]);
    
    // Put document
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &doc_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbPut.as_u8(), &[]);
    
    // The document ID should now be on the stack
    // Let's duplicate it and use it to get the document back
    bytecode.add_instruction(StackOpcode::Dup.as_u8(), &[]);
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Swap.as_u8(), &[]); // Put collection name first
    bytecode.add_instruction(DatabaseOpcode::DbGet.as_u8(), &[]);
    
    println!("   Created bytecode with {} instructions", bytecode.code.len());
    
    // Execute the program
    println!("\n3. Executing program...");
    let mut executor = VmExecutor::new();
    executor.enable_debug();
    
    executor.load_bytecode(bytecode)?;
    let result = executor.execute()?;
    
    println!("   Execution completed!");
    println!("   Instructions executed: {}", result.instructions_executed);
    println!("   Execution time: {:?}", result.execution_time);
    println!("   Final stack size: {}", result.final_stack.len());
    
    if !result.final_stack.is_empty() {
        println!("   Final stack contents:");
        for (i, value) in result.final_stack.iter().enumerate() {
            println!("     [{}]: {}", i, value);
        }
    }
    
    println!("\n✅ Test completed successfully!");
    
    Ok(())
}