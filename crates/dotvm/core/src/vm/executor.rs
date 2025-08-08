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

//! VM Executor
//!
//! This module implements the core execution engine for the DotVM.
//! It provides the fetch-decode-execute cycle for running bytecode.

use crate::bytecode::BytecodeFile;
use crate::opcode::arithmetic_opcodes::ArithmeticOpcode;
use crate::opcode::control_flow_opcodes::ControlFlowOpcode;
use crate::opcode::db_opcodes::DatabaseOpcode;
use crate::opcode::stack_opcodes::{StackInstruction, StackOpcode};
use crate::opcode::state_opcodes::StateOpcode;
use crate::security::types::{CurrentResourceUsage, OpcodeResult, ResourceCost, SecurityMetadata, SideEffect};
use crate::security::{CustomOpcode, DotVMContext, OpcodeType, SecurityLevel, SecuritySandbox};
use crate::vm::database_bridge::DatabaseBridge;
use crate::vm::database_executor::DatabaseOpcodeExecutor;
use crate::vm::stack::{OperandStack, StackError, StackValue};
use crate::vm::state_executor::{MerkleOperation, SnapshotId, StateOpcodeExecutor};
use crate::vm::state_management::{StateKey, StateValue};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use std::time::{Duration, SystemTime};

/// Maximum number of instructions to execute (to prevent infinite loops)
pub const MAX_INSTRUCTIONS: usize = 1_000_000;

/// VM execution context
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Program counter (instruction pointer)
    pub pc: usize,
    /// Operand stack
    pub stack: OperandStack,
    /// Local variables (for future use)
    pub locals: HashMap<String, StackValue>,
    /// Execution flags
    pub flags: ExecutionFlags,
    /// Instruction count (for debugging and limits)
    pub instruction_count: usize,
    /// Dot ID for security tracking
    pub dot_id: String,
    /// Security level
    pub security_level: SecurityLevel,
    /// Resource usage tracking
    pub resource_usage: CurrentResourceUsage,
    /// Execution start time for resource tracking
    pub execution_start: Instant,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new() -> Self {
        Self {
            pc: 0,
            stack: OperandStack::new(),
            locals: HashMap::new(),
            flags: ExecutionFlags::default(),
            instruction_count: 0,
            dot_id: "default".to_string(),
            security_level: SecurityLevel::Standard,
            resource_usage: CurrentResourceUsage::default(),
            execution_start: Instant::now(),
        }
    }

    /// Create a new execution context with a specific dot ID
    pub fn new_with_dot_id(dot_id: String) -> Self {
        Self {
            pc: 0,
            stack: OperandStack::new(),
            locals: HashMap::new(),
            flags: ExecutionFlags::default(),
            instruction_count: 0,
            dot_id,
            security_level: SecurityLevel::Standard,
            resource_usage: CurrentResourceUsage::default(),
            execution_start: Instant::now(),
        }
    }

    /// Reset the execution context
    pub fn reset(&mut self) {
        self.pc = 0;
        self.stack.clear();
        self.locals.clear();
        self.flags = ExecutionFlags::default();
        self.instruction_count = 0;
        self.resource_usage = CurrentResourceUsage::default();
        self.execution_start = Instant::now();
        // Note: dot_id and security_level are preserved during reset
    }

    /// Check if execution should halt
    pub fn should_halt(&self) -> bool {
        self.flags.halt || self.instruction_count >= MAX_INSTRUCTIONS
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution flags
#[derive(Debug, Clone, Default)]
pub struct ExecutionFlags {
    /// Halt execution
    pub halt: bool,
    /// Debug mode
    pub debug: bool,
    /// Step mode (pause after each instruction)
    pub step: bool,
}

/// VM Executor
#[derive(Debug)]
pub struct VmExecutor {
    /// Loaded bytecode
    bytecode: Option<BytecodeFile>,
    /// Execution context
    context: ExecutionContext,
    /// Debug information
    debug_info: DebugInfo,
    /// Database bridge for database operations
    database_bridge: DatabaseBridge,
    /// Database opcode executor for new key-value operations
    database_executor: Option<DatabaseOpcodeExecutor>,
    /// State opcode executor for advanced state management
    state_executor: Option<StateOpcodeExecutor>,
    /// Security sandbox for opcode security checks
    pub security_sandbox: SecuritySandbox,
}

impl VmExecutor {
    /// Create a new VM executor
    pub fn new() -> Self {
        Self {
            bytecode: None,
            context: ExecutionContext::new(),
            debug_info: DebugInfo::new(),
            database_bridge: DatabaseBridge::new(),
            database_executor: None,
            state_executor: None,
            security_sandbox: SecuritySandbox::new(),
        }
    }

    /// Create a new VM executor with a specific dot ID
    pub fn new_with_dot_id(dot_id: String) -> Self {
        let mut executor = Self {
            bytecode: None,
            context: ExecutionContext::new_with_dot_id(dot_id.clone()),
            debug_info: DebugInfo::new(),
            database_bridge: DatabaseBridge::new(),
            database_executor: None,
            state_executor: None,
            security_sandbox: SecuritySandbox::new(),
        };

        // Initialize security context for this dot
        if let Err(e) = executor.security_sandbox.initialize_dot_security_context(dot_id.clone(), SecurityLevel::Standard) {
            // Log error but continue - security will be disabled for this dot
            eprintln!("Warning: Failed to initialize security context for dot {}: {}", dot_id, e);
        }

        executor
    }

    /// Create a new VM executor with a custom database bridge
    pub fn with_database_bridge(database_bridge: DatabaseBridge) -> Self {
        Self {
            bytecode: None,
            context: ExecutionContext::new(),
            debug_info: DebugInfo::new(),
            database_bridge,
            database_executor: None,
            state_executor: None,
            security_sandbox: SecuritySandbox::new(),
        }
    }

    /// Set the database executor for new key-value operations
    pub fn with_database_executor(mut self, database_executor: DatabaseOpcodeExecutor) -> Self {
        self.database_executor = Some(database_executor);
        self
    }

    /// Set the state executor for advanced state management operations
    pub fn with_state_executor(mut self, state_executor: StateOpcodeExecutor) -> Self {
        self.state_executor = Some(state_executor);
        self
    }

    /// Get reference to the security sandbox
    pub fn security_sandbox(&self) -> &SecuritySandbox {
        &self.security_sandbox
    }

    /// Get mutable reference to the security sandbox
    pub fn security_sandbox_mut(&mut self) -> &mut SecuritySandbox {
        &mut self.security_sandbox
    }

    /// Set security level for the current execution context
    pub fn set_security_level(&mut self, level: SecurityLevel) {
        self.context.security_level = level;
    }

    /// Set dot ID for the current execution context
    pub fn set_dot_id(&mut self, dot_id: String) -> Result<(), ExecutorError> {
        // Update context
        self.context.dot_id = dot_id.clone();

        // Initialize security context for this dot
        self.security_sandbox
            .initialize_dot_security_context(dot_id, self.context.security_level.clone())
            .map_err(|e| ExecutorError::SecurityError(e.to_string()))?;

        Ok(())
    }

    /// Load bytecode from a file
    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), ExecutorError> {
        let bytecode = BytecodeFile::load_from_file(path)?;
        self.load_bytecode(bytecode)
    }

    /// Load bytecode directly
    pub fn load_bytecode(&mut self, bytecode: BytecodeFile) -> Result<(), ExecutorError> {
        // Validate bytecode
        self.validate_bytecode(&bytecode)?;

        // Reset execution context
        self.context.reset();
        self.context.pc = bytecode.entry_point() as usize;

        // Store bytecode
        self.bytecode = Some(bytecode);

        Ok(())
    }

    /// Execute the loaded bytecode
    pub fn execute(&mut self) -> Result<ExecutionResult, ExecutorError> {
        if self.bytecode.is_none() {
            return Err(ExecutorError::NoBytecodeLoaded);
        }

        let start_time = std::time::Instant::now();

        loop {
            // Check halt conditions
            if self.context.should_halt() {
                break;
            }

            // Check bounds
            let code_len = self.bytecode.as_ref().unwrap().code.len();
            if self.context.pc >= code_len {
                break;
            }

            // Fetch instruction
            let instruction = self.fetch_instruction()?;

            // Debug output
            if self.context.flags.debug {
                self.debug_info.log_instruction(self.context.pc, &instruction);
            }

            // Decode and execute
            self.execute_instruction(&instruction)?;

            // Increment instruction count
            self.context.instruction_count += 1;

            // Step mode pause
            if self.context.flags.step {
                break;
            }
        }

        let execution_time = start_time.elapsed();

        Ok(ExecutionResult {
            instructions_executed: self.context.instruction_count,
            execution_time,
            final_stack: self.context.stack.snapshot(),
            halted: self.context.flags.halt,
            pc: self.context.pc,
        })
    }

    /// Execute a single instruction (for step mode)
    pub fn step(&mut self) -> Result<StepResult, ExecutorError> {
        if self.context.should_halt() {
            return Ok(StepResult::Halted);
        }

        let bytecode = self.bytecode.as_ref().ok_or(ExecutorError::NoBytecodeLoaded)?;

        if self.context.pc >= bytecode.code.len() {
            return Ok(StepResult::EndOfCode);
        }

        let instruction = self.fetch_instruction()?;
        self.execute_instruction(&instruction)?;
        self.context.instruction_count += 1;

        Ok(StepResult::Executed {
            instruction,
            pc: self.context.pc,
            stack_size: self.context.stack.size(),
        })
    }

    /// Fetch the next instruction from bytecode
    fn fetch_instruction(&self) -> Result<Instruction, ExecutorError> {
        let bytecode = self.bytecode.as_ref().unwrap();

        if self.context.pc >= bytecode.code.len() {
            return Err(ExecutorError::ProgramCounterOutOfBounds(self.context.pc));
        }

        let opcode_byte = bytecode.code[self.context.pc];

        // Try to decode as different instruction types
        if let Some(stack_opcode) = StackOpcode::from_u8(opcode_byte) {
            let operand_size = stack_opcode.operand_size();
            if self.context.pc + 1 + operand_size > bytecode.code.len() {
                return Err(ExecutorError::InsufficientBytecode);
            }

            let operands = if operand_size > 0 {
                bytecode.code[self.context.pc + 1..self.context.pc + 1 + operand_size].to_vec()
            } else {
                vec![]
            };

            return Ok(Instruction::Stack(StackInstruction::new(stack_opcode, operands)));
        }

        if let Some(arith_opcode) = ArithmeticOpcode::from_u8(opcode_byte) {
            return Ok(Instruction::Arithmetic(arith_opcode));
        }

        if let Some(db_opcode) = DatabaseOpcode::from_u8(opcode_byte) {
            return Ok(Instruction::Database(db_opcode));
        }

        if let Some(cf_opcode) = ControlFlowOpcode::from_u8(opcode_byte) {
            return Ok(Instruction::ControlFlow(cf_opcode));
        }

        if let Ok(state_opcode) = StateOpcode::from_u8(opcode_byte) {
            return Ok(Instruction::State(state_opcode));
        }

        Err(ExecutorError::UnknownOpcode(opcode_byte))
    }

    /// Execute a decoded instruction
    fn execute_instruction(&mut self, instruction: &Instruction) -> Result<(), ExecutorError> {
        // Convert instruction to CustomOpcode for security checks
        let custom_opcode = self.instruction_to_custom_opcode(instruction);

        // Update resource usage
        self.update_resource_usage(instruction)?;

        // Create DotVMContext for security checks
        let dot_context = self.create_dot_context();

        // Convert resource usage to the format expected by security sandbox
        let security_resource_usage = crate::security::resource_limiter::ResourceUsage {
            memory_bytes: self.context.resource_usage.memory_bytes,
            cpu_time_ms: self.context.resource_usage.cpu_time_ms,
            instruction_count: self.context.resource_usage.instruction_count,
            file_descriptors: self.context.resource_usage.file_descriptors,
            network_bytes: self.context.resource_usage.network_bytes,
            storage_bytes: self.context.resource_usage.storage_bytes,
            call_stack_depth: self.context.resource_usage.call_stack_depth,
            last_updated: Some(SystemTime::now()),
        };

        // Perform comprehensive security check
        let required_permissions = vec![]; // No specific permissions required for basic operations
        match self
            .security_sandbox
            .comprehensive_security_check(&dot_context, &custom_opcode, &required_permissions, &security_resource_usage)
        {
            Ok(check_result) => {
                if !check_result.allowed {
                    let violation_msg = check_result
                        .violations
                        .iter()
                        .map(|v| format!("{}: {}", v.violation_type, v.description))
                        .collect::<Vec<_>>()
                        .join("; ");
                    // Audit the security violation
                    let failure_result = OpcodeResult {
                        success: false,
                        return_value: None,
                        resource_consumed: ResourceCost {
                            cpu_cycles: 0,
                            memory_bytes: 0,
                            storage_bytes: 0,
                            network_bytes: 0,
                            execution_time_ms: 0,
                        },
                        execution_time: Duration::from_millis(0),
                        side_effects: vec![],
                        errors: vec![violation_msg.clone()],
                    };
                    self.security_sandbox.audit_opcode_call(&dot_context, &custom_opcode, &failure_result);
                    return Err(ExecutorError::SecurityError(violation_msg));
                }
            }
            Err(security_error) => {
                // Audit the security violation
                let failure_result = OpcodeResult {
                    success: false,
                    return_value: None,
                    resource_consumed: ResourceCost {
                        cpu_cycles: 0,
                        memory_bytes: 0,
                        storage_bytes: 0,
                        network_bytes: 0,
                        execution_time_ms: 0,
                    },
                    execution_time: Duration::from_millis(0),
                    side_effects: vec![],
                    errors: vec![format!("Security check failed: {}", security_error)],
                };
                self.security_sandbox.audit_opcode_call(&dot_context, &custom_opcode, &failure_result);
                return Err(ExecutorError::SecurityError(security_error.to_string()));
            }
        }

        // Execute the instruction
        let execution_result = match instruction {
            Instruction::Stack(stack_instr) => self.execute_stack_instruction(stack_instr),
            Instruction::Arithmetic(arith_opcode) => self.execute_arithmetic_instruction(*arith_opcode),
            Instruction::Database(db_opcode) => self.execute_database_instruction(*db_opcode),
            Instruction::ControlFlow(cf_opcode) => self.execute_control_flow_instruction(*cf_opcode),
            Instruction::State(state_opcode) => self.execute_state_instruction(*state_opcode),
        };

        // Audit the opcode call (both success and failure)
        let audit_result = match &execution_result {
            Ok(()) => OpcodeResult {
                success: true,
                return_value: None,
                resource_consumed: ResourceCost {
                    cpu_cycles: 1000,
                    memory_bytes: 64,
                    storage_bytes: 0,
                    network_bytes: 0,
                    execution_time_ms: 1,
                },
                execution_time: self.context.execution_start.elapsed(),
                side_effects: vec![],
                errors: vec![],
            },
            Err(e) => OpcodeResult {
                success: false,
                return_value: None,
                resource_consumed: ResourceCost {
                    cpu_cycles: 0,
                    memory_bytes: 0,
                    storage_bytes: 0,
                    network_bytes: 0,
                    execution_time_ms: 0,
                },
                execution_time: self.context.execution_start.elapsed(),
                side_effects: vec![],
                errors: vec![e.to_string()],
            },
        };
        self.security_sandbox.audit_opcode_call(&dot_context, &custom_opcode, &audit_result);

        execution_result
    }

    /// Create a DotVMContext from the current execution context
    fn create_dot_context(&self) -> DotVMContext {
        // Create a minimal execution context to avoid circular dependency
        let exec_context = crate::vm::executor::ExecutionContext {
            pc: self.context.pc,
            stack: self.context.stack.clone(),
            locals: HashMap::new(),
            flags: self.context.flags.clone(),
            instruction_count: self.context.instruction_count,
            dot_id: self.context.dot_id.clone(),
            security_level: self.context.security_level.clone(),
            resource_usage: self.context.resource_usage.clone(),
            execution_start: self.context.execution_start,
        };

        DotVMContext {
            execution_context: exec_context,
            dot_id: self.context.dot_id.clone(),
            session_id: format!("session_{}", self.context.dot_id),
            security_level: self.context.security_level.clone(),
            caller_context: None,
            security_metadata: SecurityMetadata {
                start_time: SystemTime::now(),
                permissions_checked: vec![],
                capabilities_used: vec![],
                resource_allocations: vec![],
                audit_trail: vec![],
            },
            resource_usage: self.context.resource_usage.clone(),
        }
    }

    /// Convert VM instruction to CustomOpcode for security system
    fn instruction_to_custom_opcode(&self, instruction: &Instruction) -> CustomOpcode {
        match instruction {
            Instruction::Stack(stack_instr) => CustomOpcode {
                opcode_type: OpcodeType::Standard {
                    architecture: crate::security::types::OpcodeArchitecture::Arch64,
                    category: crate::security::types::OpcodeCategory::Stack,
                },
                parameters: vec![],
                metadata: crate::security::types::OpcodeMetadata {
                    source_location: Some(format!("PC:{:04X}", self.context.pc)),
                    call_stack_depth: 1,
                    execution_count: 1,
                    estimated_cost: ResourceCost {
                        cpu_cycles: 1000,
                        memory_bytes: 64,
                        storage_bytes: 0,
                        network_bytes: 0,
                        execution_time_ms: 1,
                    },
                },
            },
            Instruction::Arithmetic(arith_opcode) => CustomOpcode {
                opcode_type: OpcodeType::Standard {
                    architecture: crate::security::types::OpcodeArchitecture::Arch64,
                    category: crate::security::types::OpcodeCategory::Arithmetic,
                },
                parameters: vec![],
                metadata: crate::security::types::OpcodeMetadata {
                    source_location: Some(format!("PC:{:04X}", self.context.pc)),
                    call_stack_depth: 1,
                    execution_count: 1,
                    estimated_cost: ResourceCost {
                        cpu_cycles: 2000,
                        memory_bytes: 64,
                        storage_bytes: 0,
                        network_bytes: 0,
                        execution_time_ms: 2,
                    },
                },
            },
            Instruction::Database(db_opcode) => CustomOpcode {
                opcode_type: OpcodeType::Database {
                    operation: crate::security::types::DatabaseOperation::Read,
                },
                parameters: vec![],
                metadata: crate::security::types::OpcodeMetadata {
                    source_location: Some(format!("PC:{:04X}", self.context.pc)),
                    call_stack_depth: 1,
                    execution_count: 1,
                    estimated_cost: ResourceCost {
                        cpu_cycles: 10000,
                        memory_bytes: 128,
                        storage_bytes: 100,
                        network_bytes: 0,
                        execution_time_ms: 10,
                    },
                },
            },
            Instruction::ControlFlow(cf_opcode) => CustomOpcode {
                opcode_type: OpcodeType::Standard {
                    architecture: crate::security::types::OpcodeArchitecture::Arch64,
                    category: crate::security::types::OpcodeCategory::ControlFlow,
                },
                parameters: vec![],
                metadata: crate::security::types::OpcodeMetadata {
                    source_location: Some(format!("PC:{:04X}", self.context.pc)),
                    call_stack_depth: 1,
                    execution_count: 1,
                    estimated_cost: ResourceCost {
                        cpu_cycles: 3000,
                        memory_bytes: 64,
                        storage_bytes: 0,
                        network_bytes: 0,
                        execution_time_ms: 3,
                    },
                },
            },
            Instruction::State(state_opcode) => CustomOpcode {
                opcode_type: OpcodeType::System {
                    operation: crate::security::types::SystemOperation::MemoryAllocation,
                },
                parameters: vec![],
                metadata: crate::security::types::OpcodeMetadata {
                    source_location: Some(format!("PC:{:04X}", self.context.pc)),
                    call_stack_depth: 1,
                    execution_count: 1,
                    estimated_cost: ResourceCost {
                        cpu_cycles: 20000,
                        memory_bytes: 256,
                        storage_bytes: 200,
                        network_bytes: 0,
                        execution_time_ms: 20,
                    },
                },
            },
        }
    }

    /// Update resource usage for the current instruction
    fn update_resource_usage(&mut self, instruction: &Instruction) -> Result<(), ExecutorError> {
        let elapsed = self.context.execution_start.elapsed();

        // Update CPU time
        self.context.resource_usage.cpu_time_ms = elapsed.as_millis() as u64;

        // Update instruction count
        self.context.resource_usage.instruction_count = self.context.instruction_count as u64;

        // Update memory usage (simplified calculation based on stack size)
        self.context.resource_usage.memory_bytes = (self.context.stack.size() * 64) as u64; // Estimate 64 bytes per stack item

        // Update call stack depth
        self.context.resource_usage.call_stack_depth = 1; // Simplified for now

        // Estimate additional resources based on instruction type
        match instruction {
            Instruction::Database(_) => {
                self.context.resource_usage.storage_bytes += 100; // Estimate
                self.context.resource_usage.file_descriptors += 1;
            }
            Instruction::State(_) => {
                self.context.resource_usage.storage_bytes += 200; // Estimate
            }
            _ => {}
        }

        // Create resource cost for tracking
        let resource_cost = ResourceCost {
            cpu_cycles: 1000, // Per instruction estimate
            memory_bytes: 64, // Per instruction estimate
            storage_bytes: 0,
            network_bytes: 0,
            execution_time_ms: 1, // Per instruction estimate
        };

        // Update resource tracking in security sandbox
        self.security_sandbox
            .resource_limiter
            .update_usage(&self.context.dot_id, &resource_cost)
            .map_err(|e| ExecutorError::SecurityError(format!("Resource tracking failed: {}", e)))?;

        Ok(())
    }

    /// Execute a stack instruction
    fn execute_stack_instruction(&mut self, instruction: &StackInstruction) -> Result<(), ExecutorError> {
        let bytecode = self.bytecode.as_ref().unwrap();

        match instruction.opcode {
            StackOpcode::Push => {
                let constant_id = u32::from_le_bytes([instruction.operands[0], instruction.operands[1], instruction.operands[2], instruction.operands[3]]);

                let constant = bytecode.get_constant(constant_id).ok_or(ExecutorError::InvalidConstantId(constant_id))?;

                let stack_value = StackValue::from_constant(constant);
                self.context.stack.push(stack_value)?;

                self.context.pc += 1 + instruction.operands.len();
            }

            StackOpcode::Pop => {
                self.context.stack.pop()?;
                self.context.pc += 1;
            }

            StackOpcode::Dup => {
                self.context.stack.dup()?;
                self.context.pc += 1;
            }

            StackOpcode::Swap => {
                self.context.stack.swap()?;
                self.context.pc += 1;
            }

            StackOpcode::PushNull => {
                self.context.stack.push(StackValue::Null)?;
                self.context.pc += 1;
            }

            StackOpcode::PushTrue => {
                self.context.stack.push(StackValue::Bool(true))?;
                self.context.pc += 1;
            }

            StackOpcode::PushFalse => {
                self.context.stack.push(StackValue::Bool(false))?;
                self.context.pc += 1;
            }

            StackOpcode::PushInt8 => {
                let value = instruction.operands[0] as i8;
                self.context.stack.push(StackValue::Int64(value as i64))?;
                self.context.pc += 1 + instruction.operands.len();
            }

            StackOpcode::PushInt32 => {
                let value = i32::from_le_bytes([instruction.operands[0], instruction.operands[1], instruction.operands[2], instruction.operands[3]]);
                self.context.stack.push(StackValue::Int64(value as i64))?;
                self.context.pc += 1 + instruction.operands.len();
            }

            StackOpcode::PushInt64 => {
                let value = i64::from_le_bytes([
                    instruction.operands[0],
                    instruction.operands[1],
                    instruction.operands[2],
                    instruction.operands[3],
                    instruction.operands[4],
                    instruction.operands[5],
                    instruction.operands[6],
                    instruction.operands[7],
                ]);
                self.context.stack.push(StackValue::Int64(value))?;
                self.context.pc += 1 + instruction.operands.len();
            }

            StackOpcode::PushFloat64 => {
                let value = f64::from_le_bytes([
                    instruction.operands[0],
                    instruction.operands[1],
                    instruction.operands[2],
                    instruction.operands[3],
                    instruction.operands[4],
                    instruction.operands[5],
                    instruction.operands[6],
                    instruction.operands[7],
                ]);
                self.context.stack.push(StackValue::Float64(value))?;
                self.context.pc += 1 + instruction.operands.len();
            }

            StackOpcode::DupN => {
                let depth = instruction.operands[0] as usize;
                let value = self.context.stack.peek_at(depth)?.clone();
                self.context.stack.push(value)?;
                self.context.pc += 1 + instruction.operands.len();
            }

            StackOpcode::Rotate => {
                // Simple rotation implementation - can be improved
                let count = instruction.operands[0] as usize;
                if count > 0 && self.context.stack.size() >= count {
                    let mut values = Vec::new();
                    for _ in 0..count {
                        values.push(self.context.stack.pop()?);
                    }
                    // Rotate by moving the last element to the front
                    if let Some(last) = values.pop() {
                        self.context.stack.push(last)?;
                        for value in values.into_iter().rev() {
                            self.context.stack.push(value)?;
                        }
                    }
                }
                self.context.pc += 1 + instruction.operands.len();
            }
        }

        Ok(())
    }

    /// Execute an arithmetic instruction
    fn execute_arithmetic_instruction(&mut self, opcode: ArithmeticOpcode) -> Result<(), ExecutorError> {
        match opcode {
            ArithmeticOpcode::Add => {
                let (a, b) = self.context.stack.pop_two()?;
                let result = self.add_values(&a, &b)?;
                self.context.stack.push(result)?;
            }

            ArithmeticOpcode::Subtract => {
                let (a, b) = self.context.stack.pop_two()?;
                let result = self.subtract_values(&a, &b)?;
                self.context.stack.push(result)?;
            }

            ArithmeticOpcode::Multiply => {
                let (a, b) = self.context.stack.pop_two()?;
                let result = self.multiply_values(&a, &b)?;
                self.context.stack.push(result)?;
            }

            ArithmeticOpcode::Divide => {
                let (a, b) = self.context.stack.pop_two()?;
                let result = self.divide_values(&a, &b)?;
                self.context.stack.push(result)?;
            }

            ArithmeticOpcode::Modulus => {
                let (a, b) = self.context.stack.pop_two()?;
                let result = self.modulus_values(&a, &b)?;
                self.context.stack.push(result)?;
            }
        }

        self.context.pc += 1; // Arithmetic opcodes have no operands
        Ok(())
    }

    /// Execute a database instruction
    fn execute_database_instruction(&mut self, opcode: DatabaseOpcode) -> Result<(), ExecutorError> {
        match opcode {
            // New key-value opcodes
            DatabaseOpcode::DbRead => {
                // Stack: [table_id, key] -> [value] or [null]
                let key = self.context.stack.pop()?.as_bytes_value();
                let table_id = self.context.stack.pop()?.as_u32_value();

                if let Some(ref executor) = self.database_executor {
                    match executor.execute_db_read(table_id, key) {
                        Ok(Some(value)) => {
                            self.context.stack.push(StackValue::Bytes(value))?;
                        }
                        Ok(None) => {
                            self.context.stack.push(StackValue::Null)?;
                        }
                        Err(e) => {
                            return Err(ExecutorError::DatabaseError(e.to_string()));
                        }
                    }
                } else {
                    return Err(ExecutorError::DatabaseError("Database executor not configured".to_string()));
                }
            }

            DatabaseOpcode::DbWrite => {
                // Stack: [table_id, key, value] -> []
                let value = self.context.stack.pop()?.as_bytes_value();
                let key = self.context.stack.pop()?.as_bytes_value();
                let table_id = self.context.stack.pop()?.as_u32_value();

                if let Some(ref executor) = self.database_executor {
                    if let Err(e) = executor.execute_db_write(table_id, key, value) {
                        return Err(ExecutorError::DatabaseError(e.to_string()));
                    }
                } else {
                    return Err(ExecutorError::DatabaseError("Database executor not configured".to_string()));
                }
            }

            DatabaseOpcode::DbQuery => {
                // Stack: [query_spec_json] -> [query_result_json]
                let query_spec_json = self.context.stack.pop()?.as_string_value();

                if let Some(ref executor) = self.database_executor {
                    // Parse query spec from JSON
                    let query_spec: crate::vm::database_executor::QuerySpec = serde_json::from_str(&query_spec_json).map_err(|e| ExecutorError::DatabaseError(format!("Invalid query spec: {}", e)))?;

                    match executor.execute_db_query(query_spec) {
                        Ok(result) => {
                            let result_json = serde_json::to_string(&result).map_err(|e| ExecutorError::DatabaseError(format!("Failed to serialize result: {}", e)))?;
                            self.context.stack.push(StackValue::String(result_json))?;
                        }
                        Err(e) => {
                            return Err(ExecutorError::DatabaseError(e.to_string()));
                        }
                    }
                } else {
                    return Err(ExecutorError::DatabaseError("Database executor not configured".to_string()));
                }
            }

            DatabaseOpcode::DbTransaction => {
                // Stack: [transaction_ops_json] -> [transaction_result_json]
                let tx_ops_json = self.context.stack.pop()?.as_string_value();

                if let Some(ref executor) = self.database_executor {
                    // Parse transaction operations from JSON
                    let tx_ops: Vec<crate::vm::database_executor::TransactionOp> =
                        serde_json::from_str(&tx_ops_json).map_err(|e| ExecutorError::DatabaseError(format!("Invalid transaction ops: {}", e)))?;

                    match executor.execute_db_transaction(tx_ops) {
                        Ok(result) => {
                            let result_json = serde_json::to_string(&result).map_err(|e| ExecutorError::DatabaseError(format!("Failed to serialize result: {}", e)))?;
                            self.context.stack.push(StackValue::String(result_json))?;
                        }
                        Err(e) => {
                            return Err(ExecutorError::DatabaseError(e.to_string()));
                        }
                    }
                } else {
                    return Err(ExecutorError::DatabaseError("Database executor not configured".to_string()));
                }
            }

            DatabaseOpcode::DbIndex => {
                // Stack: [index_operation_json] -> []
                let index_op_json = self.context.stack.pop()?.as_string_value();

                if let Some(ref executor) = self.database_executor {
                    // Parse index operation from JSON
                    let index_op: crate::vm::database_executor::IndexOperation =
                        serde_json::from_str(&index_op_json).map_err(|e| ExecutorError::DatabaseError(format!("Invalid index operation: {}", e)))?;

                    if let Err(e) = executor.execute_db_index(index_op) {
                        return Err(ExecutorError::DatabaseError(e.to_string()));
                    }
                } else {
                    return Err(ExecutorError::DatabaseError("Database executor not configured".to_string()));
                }
            }

            DatabaseOpcode::DbStream => {
                // Stack: [stream_spec_json] -> [stream_result_json]
                let stream_spec_json = self.context.stack.pop()?.as_string_value();

                if let Some(ref executor) = self.database_executor {
                    // Parse stream spec from JSON
                    let stream_spec: crate::vm::database_executor::StreamSpec =
                        serde_json::from_str(&stream_spec_json).map_err(|e| ExecutorError::DatabaseError(format!("Invalid stream spec: {}", e)))?;

                    match executor.execute_db_stream(stream_spec) {
                        Ok(result) => {
                            let result_json = serde_json::to_string(&result).map_err(|e| ExecutorError::DatabaseError(format!("Failed to serialize result: {}", e)))?;
                            self.context.stack.push(StackValue::String(result_json))?;
                        }
                        Err(e) => {
                            return Err(ExecutorError::DatabaseError(e.to_string()));
                        }
                    }
                } else {
                    return Err(ExecutorError::DatabaseError("Database executor not configured".to_string()));
                }
            }

            // Legacy document opcodes
            DatabaseOpcode::DbGet => {
                // Stack: [collection_name, document_id] -> [document_json]
                let document_id = self.context.stack.pop()?.as_string_value();
                let collection_name = self.context.stack.pop()?.as_string_value();

                match self.database_bridge.get_document(&collection_name, &document_id) {
                    Ok(Some(document_json)) => {
                        self.context.stack.push(StackValue::String(document_json))?;
                    }
                    Ok(None) => {
                        self.context.stack.push(StackValue::Null)?;
                    }
                    Err(e) => {
                        return Err(ExecutorError::DatabaseError(e.to_string()));
                    }
                }
            }

            DatabaseOpcode::DbPut => {
                // Stack: [collection_name, document_json] -> [document_id]
                let document_json = self.context.stack.pop()?.as_string_value();
                let collection_name = self.context.stack.pop()?.as_string_value();

                match self.database_bridge.put_document(&collection_name, &document_json) {
                    Ok(document_id) => {
                        self.context.stack.push(StackValue::String(document_id))?;
                    }
                    Err(e) => {
                        return Err(ExecutorError::DatabaseError(e.to_string()));
                    }
                }
            }

            DatabaseOpcode::DbUpdate => {
                // Stack: [collection_name, document_id, document_json] -> []
                let document_json = self.context.stack.pop()?.as_string_value();
                let document_id = self.context.stack.pop()?.as_string_value();
                let collection_name = self.context.stack.pop()?.as_string_value();

                if let Err(e) = self.database_bridge.update_document(&collection_name, &document_id, &document_json) {
                    return Err(ExecutorError::DatabaseError(e.to_string()));
                }
            }

            DatabaseOpcode::DbDelete => {
                // Stack: [collection_name, document_id] -> []
                let document_id = self.context.stack.pop()?.as_string_value();
                let collection_name = self.context.stack.pop()?.as_string_value();

                if let Err(e) = self.database_bridge.delete_document(&collection_name, &document_id) {
                    return Err(ExecutorError::DatabaseError(e.to_string()));
                }
            }

            DatabaseOpcode::DbList => {
                // Stack: [collection_name] -> [document_ids_array]
                let collection_name = self.context.stack.pop()?.as_string_value();

                match self.database_bridge.list_documents(&collection_name) {
                    Ok(document_ids) => {
                        // Convert to JSON array string
                        let json_array = serde_json::to_string(&document_ids).map_err(|e| ExecutorError::DatabaseError(format!("Failed to serialize document IDs: {e}")))?;
                        self.context.stack.push(StackValue::String(json_array))?;
                    }
                    Err(e) => {
                        return Err(ExecutorError::DatabaseError(e.to_string()));
                    }
                }
            }

            DatabaseOpcode::DbCreateCollection => {
                // Stack: [collection_name] -> []
                let collection_name = self.context.stack.pop()?.as_string_value();

                if let Err(e) = self.database_bridge.create_collection(&collection_name) {
                    return Err(ExecutorError::DatabaseError(e.to_string()));
                }
            }

            DatabaseOpcode::DbDeleteCollection => {
                // Stack: [collection_name] -> []
                let collection_name = self.context.stack.pop()?.as_string_value();

                if let Err(e) = self.database_bridge.delete_collection(&collection_name) {
                    return Err(ExecutorError::DatabaseError(e.to_string()));
                }
            }
        }

        self.context.pc += 1; // Database opcodes have no operands
        Ok(())
    }

    /// Execute a control flow instruction
    fn execute_control_flow_instruction(&mut self, opcode: ControlFlowOpcode) -> Result<(), ExecutorError> {
        match opcode {
            ControlFlowOpcode::Jump => {
                // For now, implement a simple unconditional jump
                // In a real implementation, this would need jump targets
                // For the MVP, we'll implement a simple relative jump using the top stack value
                let offset = self.context.stack.pop()?.to_i64().ok_or_else(|| ExecutorError::TypeMismatch {
                    operation: "jump".to_string(),
                    left: "stack_value".to_string(),
                    right: "integer".to_string(),
                })?;

                // Calculate new PC (with bounds checking)
                let new_pc = if offset >= 0 {
                    self.context.pc.saturating_add(offset as usize)
                } else {
                    self.context.pc.saturating_sub((-offset) as usize)
                };

                // Validate bounds
                let bytecode = self.bytecode.as_ref().unwrap();
                if new_pc >= bytecode.code.len() {
                    return Err(ExecutorError::ProgramCounterOutOfBounds(new_pc));
                }

                self.context.pc = new_pc;
            }

            ControlFlowOpcode::IfElse => {
                // Conditional execution based on stack top
                // Stack: [condition, true_offset, false_offset] -> []
                let false_offset = self.context.stack.pop()?.to_i64().ok_or_else(|| ExecutorError::TypeMismatch {
                    operation: "if_else".to_string(),
                    left: "stack_value".to_string(),
                    right: "integer".to_string(),
                })?;

                let true_offset = self.context.stack.pop()?.to_i64().ok_or_else(|| ExecutorError::TypeMismatch {
                    operation: "if_else".to_string(),
                    left: "stack_value".to_string(),
                    right: "integer".to_string(),
                })?;

                let condition = self.context.stack.pop()?.to_bool();

                let offset = if condition { true_offset } else { false_offset };

                // Calculate new PC
                let new_pc = if offset >= 0 {
                    self.context.pc.saturating_add(offset as usize)
                } else {
                    self.context.pc.saturating_sub((-offset) as usize)
                };

                // Validate bounds
                let bytecode = self.bytecode.as_ref().unwrap();
                if new_pc >= bytecode.code.len() {
                    return Err(ExecutorError::ProgramCounterOutOfBounds(new_pc));
                }

                self.context.pc = new_pc;
            }

            ControlFlowOpcode::ForLoop | ControlFlowOpcode::WhileLoop | ControlFlowOpcode::DoWhileLoop => {
                // For the MVP, we'll implement these as simple jumps
                // In a full implementation, these would have more sophisticated loop handling
                let offset = self.context.stack.pop()?.to_i64().ok_or_else(|| ExecutorError::TypeMismatch {
                    operation: "loop".to_string(),
                    left: "stack_value".to_string(),
                    right: "integer".to_string(),
                })?;

                let new_pc = if offset >= 0 {
                    self.context.pc.saturating_add(offset as usize)
                } else {
                    self.context.pc.saturating_sub((-offset) as usize)
                };

                let bytecode = self.bytecode.as_ref().unwrap();
                if new_pc >= bytecode.code.len() {
                    return Err(ExecutorError::ProgramCounterOutOfBounds(new_pc));
                }

                self.context.pc = new_pc;
            }
        }

        // Note: Control flow instructions manage PC themselves, so we don't increment here
        Ok(())
    }

    /// Execute a state management instruction
    fn execute_state_instruction(&mut self, opcode: StateOpcode) -> Result<(), ExecutorError> {
        let state_executor = self.state_executor.as_mut().ok_or_else(|| ExecutorError::DatabaseError("State executor not configured".to_string()))?;

        match opcode {
            StateOpcode::StateRead => {
                // Stack: [state_key] -> [value] or [null]
                let key_bytes = self.context.stack.pop()?.as_bytes_value();
                let state_key = StateKey::new(key_bytes);

                match state_executor.execute_state_read(state_key) {
                    Ok(Some(value)) => {
                        self.context.stack.push(StackValue::Bytes(value.as_bytes().to_vec()))?;
                    }
                    Ok(None) => {
                        self.context.stack.push(StackValue::Null)?;
                    }
                    Err(e) => {
                        return Err(ExecutorError::DatabaseError(format!("State read error: {}", e)));
                    }
                }
            }

            StateOpcode::StateWrite => {
                // Stack: [state_key, value] -> []
                let value_bytes = self.context.stack.pop()?.as_bytes_value();
                let key_bytes = self.context.stack.pop()?.as_bytes_value();

                let state_key = StateKey::new(key_bytes);
                let state_value = StateValue::new(value_bytes);

                if let Err(e) = state_executor.execute_state_write(state_key, state_value) {
                    return Err(ExecutorError::DatabaseError(format!("State write error: {}", e)));
                }
            }

            StateOpcode::StateCommit => {
                // Stack: [] -> [state_root_hash]
                match state_executor.execute_state_commit() {
                    Ok(root_hash) => {
                        self.context.stack.push(StackValue::Bytes(root_hash))?;
                    }
                    Err(e) => {
                        return Err(ExecutorError::DatabaseError(format!("State commit error: {}", e)));
                    }
                }
            }

            StateOpcode::StateRollback => {
                // Stack: [] -> []
                if let Err(e) = state_executor.execute_state_rollback() {
                    return Err(ExecutorError::DatabaseError(format!("State rollback error: {}", e)));
                }
            }

            StateOpcode::StateMerkle => {
                // Stack: [operation_type, key] -> [proof_data] or [verification_result]
                let key_bytes = self.context.stack.pop()?.as_bytes_value();
                let operation_type = self.context.stack.pop()?.as_u32_value();

                let state_key = StateKey::new(key_bytes);

                let operation = match operation_type {
                    0 => MerkleOperation::GenerateProof { key: state_key },
                    1 => {
                        // For verification, we need additional data from the stack
                        // This is a simplified implementation
                        return Err(ExecutorError::DatabaseError("Merkle verification not fully implemented".to_string()));
                    }
                    _ => {
                        return Err(ExecutorError::DatabaseError("Invalid Merkle operation type".to_string()));
                    }
                };

                match state_executor.execute_state_merkle(operation) {
                    Ok(result) => {
                        self.context.stack.push(StackValue::Bytes(result))?;
                    }
                    Err(e) => {
                        return Err(ExecutorError::DatabaseError(format!("Merkle operation error: {}", e)));
                    }
                }
            }

            StateOpcode::StateSnapshot => {
                // Stack: [snapshot_id] -> []
                let snapshot_id_bytes = self.context.stack.pop()?.as_bytes_value();
                let snapshot_id = String::from_utf8(snapshot_id_bytes).map_err(|e| ExecutorError::DatabaseError(format!("Invalid snapshot ID: {}", e)))?;

                if let Err(e) = state_executor.execute_state_snapshot(snapshot_id) {
                    return Err(ExecutorError::DatabaseError(format!("Snapshot creation error: {}", e)));
                }
            }

            StateOpcode::StateRestore => {
                // Stack: [snapshot_id] -> []
                let snapshot_id_bytes = self.context.stack.pop()?.as_bytes_value();
                let snapshot_id = String::from_utf8(snapshot_id_bytes).map_err(|e| ExecutorError::DatabaseError(format!("Invalid snapshot ID: {}", e)))?;

                if let Err(e) = state_executor.execute_state_restore(snapshot_id) {
                    return Err(ExecutorError::DatabaseError(format!("Snapshot restore error: {}", e)));
                }
            }

            // Handle legacy opcodes (these would be implemented separately)
            _ => {
                return Err(ExecutorError::DatabaseError(format!("Legacy state opcode not implemented: {:?}", opcode)));
            }
        }

        self.context.pc += 1; // State opcodes have no operands
        Ok(())
    }

    /// Add two stack values
    fn add_values(&self, a: &StackValue, b: &StackValue) -> Result<StackValue, ExecutorError> {
        match (a, b) {
            (StackValue::Int64(x), StackValue::Int64(y)) => Ok(StackValue::Int64(x + y)),
            (StackValue::Float64(x), StackValue::Float64(y)) => Ok(StackValue::Float64(x + y)),
            (StackValue::Int64(x), StackValue::Float64(y)) => Ok(StackValue::Float64(*x as f64 + y)),
            (StackValue::Float64(x), StackValue::Int64(y)) => Ok(StackValue::Float64(x + *y as f64)),
            (StackValue::String(x), StackValue::String(y)) => Ok(StackValue::String(format!("{x}{y}"))),
            _ => Err(ExecutorError::TypeMismatch {
                operation: "add".to_string(),
                left: a.type_name().to_string(),
                right: b.type_name().to_string(),
            }),
        }
    }

    /// Subtract two stack values
    fn subtract_values(&self, a: &StackValue, b: &StackValue) -> Result<StackValue, ExecutorError> {
        match (a, b) {
            (StackValue::Int64(x), StackValue::Int64(y)) => Ok(StackValue::Int64(x - y)),
            (StackValue::Float64(x), StackValue::Float64(y)) => Ok(StackValue::Float64(x - y)),
            (StackValue::Int64(x), StackValue::Float64(y)) => Ok(StackValue::Float64(*x as f64 - y)),
            (StackValue::Float64(x), StackValue::Int64(y)) => Ok(StackValue::Float64(x - *y as f64)),
            _ => Err(ExecutorError::TypeMismatch {
                operation: "subtract".to_string(),
                left: a.type_name().to_string(),
                right: b.type_name().to_string(),
            }),
        }
    }

    /// Multiply two stack values
    fn multiply_values(&self, a: &StackValue, b: &StackValue) -> Result<StackValue, ExecutorError> {
        match (a, b) {
            (StackValue::Int64(x), StackValue::Int64(y)) => Ok(StackValue::Int64(x * y)),
            (StackValue::Float64(x), StackValue::Float64(y)) => Ok(StackValue::Float64(x * y)),
            (StackValue::Int64(x), StackValue::Float64(y)) => Ok(StackValue::Float64(*x as f64 * y)),
            (StackValue::Float64(x), StackValue::Int64(y)) => Ok(StackValue::Float64(x * *y as f64)),
            _ => Err(ExecutorError::TypeMismatch {
                operation: "multiply".to_string(),
                left: a.type_name().to_string(),
                right: b.type_name().to_string(),
            }),
        }
    }

    /// Divide two stack values
    fn divide_values(&self, a: &StackValue, b: &StackValue) -> Result<StackValue, ExecutorError> {
        match (a, b) {
            (StackValue::Int64(x), StackValue::Int64(y)) => {
                if *y == 0 {
                    return Err(ExecutorError::DivisionByZero);
                }
                Ok(StackValue::Int64(x / y))
            }
            (StackValue::Float64(x), StackValue::Float64(y)) => {
                if *y == 0.0 {
                    return Err(ExecutorError::DivisionByZero);
                }
                Ok(StackValue::Float64(x / y))
            }
            (StackValue::Int64(x), StackValue::Float64(y)) => {
                if *y == 0.0 {
                    return Err(ExecutorError::DivisionByZero);
                }
                Ok(StackValue::Float64(*x as f64 / y))
            }
            (StackValue::Float64(x), StackValue::Int64(y)) => {
                if *y == 0 {
                    return Err(ExecutorError::DivisionByZero);
                }
                Ok(StackValue::Float64(x / *y as f64))
            }
            _ => Err(ExecutorError::TypeMismatch {
                operation: "divide".to_string(),
                left: a.type_name().to_string(),
                right: b.type_name().to_string(),
            }),
        }
    }

    /// Modulus of two stack values
    fn modulus_values(&self, a: &StackValue, b: &StackValue) -> Result<StackValue, ExecutorError> {
        match (a, b) {
            (StackValue::Int64(x), StackValue::Int64(y)) => {
                if *y == 0 {
                    return Err(ExecutorError::DivisionByZero);
                }
                Ok(StackValue::Int64(x % y))
            }
            (StackValue::Float64(x), StackValue::Float64(y)) => {
                if *y == 0.0 {
                    return Err(ExecutorError::DivisionByZero);
                }
                Ok(StackValue::Float64(x % y))
            }
            _ => Err(ExecutorError::TypeMismatch {
                operation: "modulus".to_string(),
                left: a.type_name().to_string(),
                right: b.type_name().to_string(),
            }),
        }
    }

    /// Validate bytecode before execution
    fn validate_bytecode(&self, bytecode: &BytecodeFile) -> Result<(), ExecutorError> {
        if bytecode.code.is_empty() {
            return Err(ExecutorError::EmptyBytecode);
        }

        if bytecode.entry_point() as usize >= bytecode.code.len() {
            return Err(ExecutorError::InvalidEntryPoint(bytecode.entry_point()));
        }

        Ok(())
    }

    /// Get the current execution context (for debugging)
    pub fn context(&self) -> &ExecutionContext {
        &self.context
    }

    /// Get mutable access to execution context (for debugging)
    pub fn context_mut(&mut self) -> &mut ExecutionContext {
        &mut self.context
    }

    /// Enable debug mode
    pub fn enable_debug(&mut self) {
        self.context.flags.debug = true;
    }

    /// Enable step mode
    pub fn enable_step(&mut self) {
        self.context.flags.step = true;
    }

    /// Halt execution
    pub fn halt(&mut self) {
        self.context.flags.halt = true;
    }

    /// Clean shutdown - cleanup security context
    pub fn shutdown(&mut self) -> Result<(), ExecutorError> {
        // Clean up security context for this dot
        self.security_sandbox
            .cleanup_dot_security_context(&self.context.dot_id)
            .map_err(|e| ExecutorError::SecurityError(e.to_string()))?;

        // Reset execution context
        self.context.reset();

        Ok(())
    }
}

impl Default for VmExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Instruction types that can be executed
#[derive(Debug, Clone)]
pub enum Instruction {
    Stack(StackInstruction),
    Arithmetic(ArithmeticOpcode),
    Database(DatabaseOpcode),
    ControlFlow(ControlFlowOpcode),
    State(StateOpcode),
}

/// Result of executing bytecode
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub instructions_executed: usize,
    pub execution_time: std::time::Duration,
    pub final_stack: Vec<StackValue>,
    pub halted: bool,
    pub pc: usize,
}

/// Result of executing a single step
#[derive(Debug, Clone)]
pub enum StepResult {
    Executed { instruction: Instruction, pc: usize, stack_size: usize },
    Halted,
    EndOfCode,
}

/// Debug information for execution
#[derive(Debug, Default)]
pub struct DebugInfo {
    pub instruction_log: Vec<String>,
}

impl DebugInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn log_instruction(&mut self, pc: usize, instruction: &Instruction) {
        let log_entry = format!("PC:{pc:04X} {instruction:?}");
        self.instruction_log.push(log_entry);
    }
}

/// Executor errors
#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Stack error: {0}")]
    Stack(#[from] StackError),

    #[error("No bytecode loaded")]
    NoBytecodeLoaded,

    #[error("Program counter out of bounds: {0}")]
    ProgramCounterOutOfBounds(usize),

    #[error("Unknown opcode: 0x{0:02X}")]
    UnknownOpcode(u8),

    #[error("Invalid constant ID: {0}")]
    InvalidConstantId(u32),

    #[error("Insufficient bytecode for instruction")]
    InsufficientBytecode,

    #[error("Empty bytecode")]
    EmptyBytecode,

    #[error("Invalid entry point: {0}")]
    InvalidEntryPoint(u32),

    #[error("Type mismatch in {operation}: {left} and {right}")]
    TypeMismatch { operation: String, left: String, right: String },

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Execution limit exceeded")]
    ExecutionLimitExceeded,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Security error: {0}")]
    SecurityError(String),
}

/// Type alias for executor operation results
pub type ExecutorResult<T> = Result<T, ExecutorError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{ConstantValue, VmArchitecture};

    fn create_test_bytecode() -> BytecodeFile {
        let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);

        // Add constants
        let str_id = bytecode.add_constant(ConstantValue::String("hello".to_string()));
        let int_id = bytecode.add_constant(ConstantValue::Int64(42));

        // Add instructions: PUSH "hello", PUSH 42, POP, POP
        bytecode.add_instruction(StackOpcode::Push.as_u8(), &str_id.to_le_bytes());
        bytecode.add_instruction(StackOpcode::Push.as_u8(), &int_id.to_le_bytes());
        bytecode.add_instruction(StackOpcode::Pop.as_u8(), &[]);
        bytecode.add_instruction(StackOpcode::Pop.as_u8(), &[]);

        bytecode
    }

    /// Helper function to create a test executor with security capabilities
    fn create_test_executor() -> VmExecutor {
        use crate::security::capability_manager::{Capability, CapabilityMetadata};
        use crate::security::resource_limiter::ResourceLimits;
        use crate::security::types::{OpcodeArchitecture, OpcodeCategory, OpcodeType, SecurityLevel};
        use std::collections::HashMap;
        use std::time::SystemTime;

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
    fn test_executor_creation() {
        let executor = VmExecutor::new();
        assert!(executor.bytecode.is_none());
        assert_eq!(executor.context.pc, 0);
        assert!(executor.context.stack.is_empty());
    }

    #[test]
    fn test_load_bytecode() {
        let mut executor = VmExecutor::new();
        let bytecode = create_test_bytecode();

        executor.load_bytecode(bytecode).unwrap();
        assert!(executor.bytecode.is_some());
        assert_eq!(executor.context.pc, 0); // Entry point
    }

    #[test]
    fn test_execute_stack_operations() {
        let mut executor = create_test_executor();
        let bytecode = create_test_bytecode();

        executor.load_bytecode(bytecode).unwrap();
        let result = executor.execute().unwrap();

        assert!(result.instructions_executed > 0);
        assert!(result.final_stack.is_empty()); // Should be empty after PUSH, PUSH, POP, POP
    }

    #[test]
    fn test_arithmetic_operations() {
        let mut executor = create_test_executor();
        let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);

        // Create program: PUSH 10, PUSH 5, ADD
        bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[10]);
        bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[5]);
        bytecode.add_instruction(ArithmeticOpcode::Add.as_u8(), &[]);

        executor.load_bytecode(bytecode).unwrap();
        let result = executor.execute().unwrap();

        assert_eq!(result.final_stack.len(), 1);
        assert_eq!(result.final_stack[0], StackValue::Int64(15));
    }

    #[test]
    fn test_step_execution() {
        let mut executor = create_test_executor();
        let bytecode = create_test_bytecode();

        executor.load_bytecode(bytecode).unwrap();
        executor.enable_step();

        // Execute first instruction
        let step_result = executor.step().unwrap();
        match step_result {
            StepResult::Executed { pc, stack_size, .. } => {
                assert!(pc > 0);
                assert_eq!(stack_size, 1); // One value pushed
            }
            _ => panic!("Expected executed result"),
        }
    }

    #[test]
    fn test_invalid_bytecode() {
        let mut executor = VmExecutor::new();
        let bytecode = BytecodeFile::new(VmArchitecture::Arch64);
        // Empty bytecode

        let result = executor.load_bytecode(bytecode);
        assert!(matches!(result, Err(ExecutorError::EmptyBytecode)));
    }

    #[test]
    fn test_division_by_zero() {
        let mut executor = create_test_executor();
        let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);

        // Create program: PUSH 10, PUSH 0, DIV
        bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[10]);
        bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[0]);
        bytecode.add_instruction(ArithmeticOpcode::Divide.as_u8(), &[]);

        executor.load_bytecode(bytecode).unwrap();
        let result = executor.execute();

        assert!(matches!(result, Err(ExecutorError::DivisionByZero)));
    }

    #[test]
    fn test_type_mismatch() {
        let mut executor = create_test_executor();
        let mut bytecode = BytecodeFile::new(VmArchitecture::Arch64);

        // Add string constant
        let str_id = bytecode.add_constant(ConstantValue::String("hello".to_string()));

        // Create program: PUSH "hello", PUSH 5, ADD (should fail)
        bytecode.add_instruction(StackOpcode::Push.as_u8(), &str_id.to_le_bytes());
        bytecode.add_instruction(StackOpcode::PushInt8.as_u8(), &[5]);
        bytecode.add_instruction(ArithmeticOpcode::Add.as_u8(), &[]);

        executor.load_bytecode(bytecode).unwrap();
        let result = executor.execute();

        assert!(matches!(result, Err(ExecutorError::TypeMismatch { .. })));
    }
}
