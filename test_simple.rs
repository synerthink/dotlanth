// Create a simple bytecode file for testing

use dotvm_core::bytecode::{BytecodeFile, VmArchitecture, ConstantValue};
use dotvm_core::opcode::stack_opcodes::StackOpcode;
use dotvm_core::opcode::db_opcodes::DatabaseOpcode;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple bytecode program
    let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);
    
    // Add constants
    let collection_const = bytecode.add_constant(ConstantValue::String("test_collection".to_string()));
    let doc_const = bytecode.add_constant(ConstantValue::String(r#"{"message": "Hello from DotVM!", "count": 42}"#.to_string()));
    
    // Program:
    // 1. Create collection
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbCreateCollection.as_u8(), &[]);
    
    // 2. Put document
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &doc_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbPut.as_u8(), &[]);
    
    // 3. Get the document back
    bytecode.add_instruction(StackOpcode::Dup.as_u8(), &[]);
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Swap.as_u8(), &[]);
    bytecode.add_instruction(DatabaseOpcode::DbGet.as_u8(), &[]);
    
    // Save to file
    bytecode.save_to_file("test_program.dotvm")?;
    println!("Created test_program.dotvm with {} instructions", bytecode.code.len());
    
    Ok(())
}