// Integration test for database operations in DotVM

use dotvm_core::bytecode::{BytecodeFile, VmArchitecture, ConstantValue};
use dotvm_core::opcode::stack_opcodes::StackOpcode;
use dotvm_core::opcode::db_opcodes::DatabaseOpcode;
use dotvm_core::vm::executor::VmExecutor;

#[test]
fn test_database_operations() {
    // Create a bytecode program that tests database operations
    let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);
    
    // Constants
    let collection_const = bytecode.add_constant(ConstantValue::String("test_collection".to_string()));
    let doc_const = bytecode.add_constant(ConstantValue::String(r#"{"message": "Hello from VM!", "count": 42}"#.to_string()));
    
    // Program:
    // 1. Create collection
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbCreateCollection.as_u8(), &[]);
    
    // 2. Put document
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &doc_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbPut.as_u8(), &[]);
    
    // 3. Duplicate the document ID and get the document back
    bytecode.add_instruction(StackOpcode::Dup.as_u8(), &[]);
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Swap.as_u8(), &[]); // Put collection name first
    bytecode.add_instruction(DatabaseOpcode::DbGet.as_u8(), &[]);
    
    // Execute the program
    let mut executor = VmExecutor::new();
    executor.load_bytecode(bytecode).unwrap();
    let result = executor.execute().unwrap();
    
    // Verify execution
    assert!(result.instructions_executed > 0);
    assert_eq!(result.final_stack.len(), 2); // Document ID and retrieved document
    
    // The top of the stack should be the retrieved document
    let retrieved_doc = &result.final_stack[1];
    assert!(retrieved_doc.to_string().contains("Hello from VM!"));
    assert!(retrieved_doc.to_string().contains("42"));
}

#[test]
fn test_database_update_operations() {
    let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);
    
    // Constants
    let collection_const = bytecode.add_constant(ConstantValue::String("users".to_string()));
    let initial_doc_const = bytecode.add_constant(ConstantValue::String(r#"{"name": "Alice", "count": 5}"#.to_string()));
    let updated_doc_const = bytecode.add_constant(ConstantValue::String(r#"{"name": "Alice", "count": 6}"#.to_string()));
    
    // Program:
    // 1. Create collection
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbCreateCollection.as_u8(), &[]);
    
    // 2. Put initial document
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &initial_doc_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbPut.as_u8(), &[]);
    
    // 3. Update the document (stack has document ID)
    bytecode.add_instruction(StackOpcode::Dup.as_u8(), &[]); // Duplicate document ID
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Swap.as_u8(), &[]); // Rearrange: collection, doc_id
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &updated_doc_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbUpdate.as_u8(), &[]);
    
    // 4. Get the updated document
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Swap.as_u8(), &[]); // Put collection name first
    bytecode.add_instruction(DatabaseOpcode::DbGet.as_u8(), &[]);
    
    // Execute the program
    let mut executor = VmExecutor::new();
    executor.load_bytecode(bytecode).unwrap();
    let result = executor.execute().unwrap();
    
    // Verify execution
    assert!(result.instructions_executed > 0);
    assert_eq!(result.final_stack.len(), 1); // Retrieved updated document
    
    // The stack should contain the updated document
    let updated_doc = &result.final_stack[0];
    assert!(updated_doc.to_string().contains("Alice"));
    assert!(updated_doc.to_string().contains("6")); // Updated count
}

#[test]
fn test_database_list_operations() {
    let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);
    
    // Constants
    let collection_const = bytecode.add_constant(ConstantValue::String("items".to_string()));
    let doc1_const = bytecode.add_constant(ConstantValue::String(r#"{"item": "first"}"#.to_string()));
    let doc2_const = bytecode.add_constant(ConstantValue::String(r#"{"item": "second"}"#.to_string()));
    
    // Program:
    // 1. Create collection
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbCreateCollection.as_u8(), &[]);
    
    // 2. Put first document
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &doc1_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbPut.as_u8(), &[]);
    bytecode.add_instruction(StackOpcode::Pop.as_u8(), &[]); // Remove document ID
    
    // 3. Put second document
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &doc2_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbPut.as_u8(), &[]);
    bytecode.add_instruction(StackOpcode::Pop.as_u8(), &[]); // Remove document ID
    
    // 4. List documents
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbList.as_u8(), &[]);
    
    // Execute the program
    let mut executor = VmExecutor::new();
    executor.load_bytecode(bytecode).unwrap();
    let result = executor.execute().unwrap();
    
    // Verify execution
    assert!(result.instructions_executed > 0);
    assert_eq!(result.final_stack.len(), 1); // List of document IDs
    
    // The stack should contain a JSON array of document IDs
    let doc_list = &result.final_stack[0];
    let doc_list_str = doc_list.to_string();
    assert!(doc_list_str.starts_with('[') && doc_list_str.ends_with(']')); // Should be JSON array
}

#[test]
fn test_showcase_scenario() {
    // This test implements the exact showcase scenario from the implementation plan
    let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);
    
    // Constants for the showcase scenario
    let collection_const = bytecode.add_constant(ConstantValue::String("users".to_string()));
    let initial_doc_const = bytecode.add_constant(ConstantValue::String(r#"{"name": "Ada", "count": 5}"#.to_string()));
    let updated_doc_const = bytecode.add_constant(ConstantValue::String(r#"{"name": "Ada", "count": 6}"#.to_string()));
    
    // Showcase program:
    // 1. Create collection (equivalent to having the collection ready)
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbCreateCollection.as_u8(), &[]);
    
    // 2. Insert initial document (equivalent to `dotdb put users '{"name": "Ada", "count": 5}'`)
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &initial_doc_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbPut.as_u8(), &[]);
    
    // 3. Read the user, increment count, and save back
    // For the MVP, we simulate the increment by updating with a pre-calculated value
    bytecode.add_instruction(StackOpcode::Dup.as_u8(), &[]); // Duplicate document ID
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Swap.as_u8(), &[]); // Rearrange for update
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &updated_doc_const.to_le_bytes());
    bytecode.add_instruction(DatabaseOpcode::DbUpdate.as_u8(), &[]);
    
    // 4. Verify the change (equivalent to `dotdb get users <id>`)
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &collection_const.to_le_bytes());
    bytecode.add_instruction(StackOpcode::Swap.as_u8(), &[]); // Put collection name first
    bytecode.add_instruction(DatabaseOpcode::DbGet.as_u8(), &[]);
    
    // Execute the showcase program
    let mut executor = VmExecutor::new();
    executor.load_bytecode(bytecode).unwrap();
    let result = executor.execute().unwrap();
    
    // Verify the showcase scenario worked
    assert!(result.instructions_executed > 0);
    assert_eq!(result.final_stack.len(), 1); // Final document
    
    // The final document should show count: 6
    let final_doc = &result.final_stack[0];
    let final_doc_str = final_doc.to_string();
    assert!(final_doc_str.contains("Ada"));
    assert!(final_doc_str.contains("6")); // Count should be incremented to 6
    
    println!("✅ Showcase scenario completed successfully!");
    println!("   Final document: {}", final_doc_str);
}