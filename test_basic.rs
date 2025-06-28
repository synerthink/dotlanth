// Create a very basic bytecode file for testing

use dotvm_core::bytecode::{BytecodeFile, VmArchitecture, ConstantValue};
use dotvm_core::opcode::stack_opcodes::StackOpcode;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple bytecode program
    let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);
    
    // Add a simple constant and push it
    let hello_const = bytecode.add_constant(ConstantValue::String("Hello World!".to_string()));
    
    // Program: just push the constant
    bytecode.add_instruction(StackOpcode::Push.as_u8(), &hello_const.to_le_bytes());
    
    // Save to file
    bytecode.save_to_file("basic_program.dotvm")?;
    println!("Created basic_program.dotvm with {} instructions", bytecode.code.len());
    println!("Constants: {}", bytecode.constants.len());
    
    Ok(())
}