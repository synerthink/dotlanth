// Create a bytecode file without constants

use dotvm_core::bytecode::{BytecodeFile, VmArchitecture};
use dotvm_core::opcode::stack_opcodes::StackOpcode;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple bytecode program
    let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);
    
    // Program: just push some immediate values
    bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[42]);
    bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[24]);
    bytecode.add_instruction(StackOpcode::Pop.as_u8(), &[]);
    
    // Save to file
    bytecode.save_to_file("no_constants.dotvm")?;
    println!("Created no_constants.dotvm with {} instructions", bytecode.code.len());
    println!("Constants: {}", bytecode.constants.len());
    
    Ok(())
}