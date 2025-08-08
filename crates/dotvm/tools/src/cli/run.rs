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

//! Run command for executing DotVM bytecode

use clap::Args;
use dotvm_core::security::capability_manager::{Capability, CapabilityMetadata};
use dotvm_core::security::resource_limiter::ResourceLimits;
use dotvm_core::security::types::{OpcodeArchitecture, OpcodeCategory, OpcodeType, SecurityLevel};
use dotvm_core::vm::database_bridge::DatabaseBridge;
use dotvm_core::vm::executor::VmExecutor;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Instant, SystemTime};

/// Arguments for the run command
#[derive(Args, Debug)]
pub struct RunArgs {
    /// Path to the bytecode file to execute
    #[arg(value_name = "BYTECODE_FILE")]
    pub bytecode_file: PathBuf,

    /// Enable debug mode (shows instruction execution)
    #[arg(short, long)]
    pub debug: bool,

    /// Enable step mode (pause after each instruction)
    #[arg(short, long)]
    pub step: bool,

    /// Maximum number of instructions to execute (safety limit)
    #[arg(long, default_value = "1000000")]
    pub max_instructions: usize,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

/// Helper function to create a VM executor with security capabilities for CLI operations
fn create_cli_executor() -> VmExecutor {
    let database_bridge = DatabaseBridge::new();
    let mut executor = VmExecutor::with_database_bridge(database_bridge);

    // Set the dot ID for security context
    executor.context_mut().dot_id = "cli_executor".to_string();

    // Initialize security context for this dot
    if let Err(e) = executor.security_sandbox.initialize_dot_security_context("cli_executor".to_string(), SecurityLevel::Development) {
        eprintln!("Warning: Failed to initialize CLI security context: {}", e);
    }

    // Grant all necessary capabilities for CLI execution
    let capabilities = vec![
        Capability {
            id: "cli_stack_cap".to_string(),
            opcode_type: OpcodeType::Standard {
                architecture: OpcodeArchitecture::Arch64,
                category: OpcodeCategory::Stack,
            },
            permissions: vec![],
            resource_limits: ResourceLimits::default(),
            expiration: None,
            metadata: CapabilityMetadata {
                created_at: SystemTime::now(),
                granted_by: "cli_system".to_string(),
                purpose: "CLI stack operations".to_string(),
                usage_count: 0,
                last_used: None,
                custom_data: HashMap::new(),
            },
            delegatable: false,
            required_security_level: SecurityLevel::Development,
        },
        Capability {
            id: "cli_arithmetic_cap".to_string(),
            opcode_type: OpcodeType::Standard {
                architecture: OpcodeArchitecture::Arch64,
                category: OpcodeCategory::Arithmetic,
            },
            permissions: vec![],
            resource_limits: ResourceLimits::default(),
            expiration: None,
            metadata: CapabilityMetadata {
                created_at: SystemTime::now(),
                granted_by: "cli_system".to_string(),
                purpose: "CLI arithmetic operations".to_string(),
                usage_count: 0,
                last_used: None,
                custom_data: HashMap::new(),
            },
            delegatable: false,
            required_security_level: SecurityLevel::Development,
        },
        Capability {
            id: "cli_control_flow_cap".to_string(),
            opcode_type: OpcodeType::Standard {
                architecture: OpcodeArchitecture::Arch64,
                category: OpcodeCategory::ControlFlow,
            },
            permissions: vec![],
            resource_limits: ResourceLimits::default(),
            expiration: None,
            metadata: CapabilityMetadata {
                created_at: SystemTime::now(),
                granted_by: "cli_system".to_string(),
                purpose: "CLI control flow operations".to_string(),
                usage_count: 0,
                last_used: None,
                custom_data: HashMap::new(),
            },
            delegatable: false,
            required_security_level: SecurityLevel::Development,
        },
    ];

    // Grant capabilities to the CLI executor
    for capability in capabilities {
        if let Err(e) = executor
            .security_sandbox
            .capability_manager
            .grant_capability("cli_executor".to_string(), capability, "cli_system".to_string())
        {
            eprintln!("Warning: Failed to grant CLI capability: {}", e);
        }
    }

    executor
}

/// Execute the run command
pub fn run_bytecode(args: RunArgs) -> Result<(), Box<dyn std::error::Error>> {
    if args.verbose {
        println!("Loading bytecode from: {}", args.bytecode_file.display());
    }

    // Create VM executor with security capabilities
    let mut executor = create_cli_executor();

    // Configure execution flags
    if args.debug {
        executor.enable_debug();
        println!("Debug mode enabled");
    }

    if args.step {
        executor.enable_step();
        println!("Step mode enabled");
    }

    // Load bytecode file
    let start_load = Instant::now();
    executor.load_file(&args.bytecode_file)?;
    let load_time = start_load.elapsed();

    if args.verbose {
        println!("Bytecode loaded in {load_time:?}");
        println!("Starting execution...");
    }

    // Execute bytecode
    let start_exec = Instant::now();
    let result = if args.step { execute_step_mode(&mut executor, args.verbose)? } else { executor.execute()? };
    let exec_time = start_exec.elapsed();

    // Print results
    println!("Execution completed!");
    println!("Instructions executed: {}", result.instructions_executed);
    println!("Execution time: {exec_time:?}");
    println!("Total time: {:?}", load_time + exec_time);

    if args.debug || args.verbose {
        println!("Final stack size: {}", result.final_stack.len());
        if !result.final_stack.is_empty() {
            println!("Final stack contents:");
            for (i, value) in result.final_stack.iter().enumerate() {
                println!("  [{i}]: {value}");
            }
        }
        println!("Program counter: {}", result.pc);
        println!("Halted: {}", result.halted);
    }

    Ok(())
}

/// Execute in step mode (interactive debugging)
fn execute_step_mode(executor: &mut VmExecutor, verbose: bool) -> Result<dotvm_core::vm::executor::ExecutionResult, Box<dyn std::error::Error>> {
    use std::io::{self, Write};

    let mut instruction_count = 0;
    let start_time = Instant::now();

    println!("Step mode: Press Enter to execute next instruction, 'q' to quit, 'c' to continue");

    loop {
        // Show current state
        let context = executor.context();
        println!("\n--- Step {} ---", instruction_count + 1);
        println!("PC: {}", context.pc);
        println!("Stack size: {}", context.stack.size());

        if verbose && !context.stack.is_empty() {
            println!("Stack contents:");
            let snapshot = context.stack.snapshot();
            for (i, value) in snapshot.iter().enumerate() {
                println!("  [{i}]: {value}");
            }
        }

        // Wait for user input
        print!("Step> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            "q" | "quit" => {
                println!("Execution halted by user");
                break;
            }
            "c" | "continue" => {
                // Disable step mode and continue execution
                executor.context_mut().flags.step = false;
                let result = executor.execute()?;
                return Ok(result);
            }
            "" => {
                // Execute next step
                match executor.step()? {
                    dotvm_core::vm::executor::StepResult::Executed { instruction, pc, stack_size } => {
                        instruction_count += 1;
                        if verbose {
                            println!("Executed: {instruction:?}");
                            println!("New PC: {pc}, Stack size: {stack_size}");
                        }
                    }
                    dotvm_core::vm::executor::StepResult::Halted => {
                        println!("Execution halted");
                        break;
                    }
                    dotvm_core::vm::executor::StepResult::EndOfCode => {
                        println!("End of code reached");
                        break;
                    }
                }
            }
            _ => {
                println!("Unknown command. Use Enter to step, 'c' to continue, 'q' to quit");
            }
        }
    }

    let execution_time = start_time.elapsed();
    let final_stack = executor.context().stack.snapshot();
    let pc = executor.context().pc;

    Ok(dotvm_core::vm::executor::ExecutionResult {
        instructions_executed: instruction_count,
        execution_time,
        final_stack,
        halted: true,
        pc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotvm_core::bytecode::{BytecodeFile, VmArchitecture};
    use dotvm_core::opcode::stack_opcodes::StackOpcode;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_run_simple_bytecode() {
        // Create a simple bytecode file
        let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);

        // Use PushInt8 instead of Push with constants since constants aren't persisted yet
        bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[42]);
        bytecode.add_instruction(StackOpcode::Pop.as_u8(), &[]);

        // Write to temporary file
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.dotvm");
        bytecode.save_to_file(&file_path).unwrap();

        // Test run command
        let args = RunArgs {
            bytecode_file: file_path,
            debug: false,
            step: false,
            max_instructions: 1000,
            verbose: false,
        };

        let result = run_bytecode(args);
        if let Err(e) = &result {
            println!("Error: {}", e);
        }
        assert!(result.is_ok());
    }
}
