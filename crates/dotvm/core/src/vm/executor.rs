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
use crate::vm::database_bridge::DatabaseBridge;
use crate::vm::stack::{OperandStack, StackError, StackValue};
use std::collections::HashMap;
use std::path::Path;

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
        }
    }

    /// Reset the execution context
    pub fn reset(&mut self) {
        self.pc = 0;
        self.stack.clear();
        self.locals.clear();
        self.flags = ExecutionFlags::default();
        self.instruction_count = 0;
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
}

impl VmExecutor {
    /// Create a new VM executor
    pub fn new() -> Self {
        Self {
            bytecode: None,
            context: ExecutionContext::new(),
            debug_info: DebugInfo::new(),
            database_bridge: DatabaseBridge::new(),
        }
    }

    /// Create a new VM executor with a custom database bridge
    pub fn with_database_bridge(database_bridge: DatabaseBridge) -> Self {
        Self {
            bytecode: None,
            context: ExecutionContext::new(),
            debug_info: DebugInfo::new(),
            database_bridge,
        }
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

        Err(ExecutorError::UnknownOpcode(opcode_byte))
    }

    /// Execute a decoded instruction
    fn execute_instruction(&mut self, instruction: &Instruction) -> Result<(), ExecutorError> {
        match instruction {
            Instruction::Stack(stack_instr) => {
                self.execute_stack_instruction(stack_instr)?;
            }
            Instruction::Arithmetic(arith_opcode) => {
                self.execute_arithmetic_instruction(*arith_opcode)?;
            }
            Instruction::Database(db_opcode) => {
                self.execute_database_instruction(*db_opcode)?;
            }
            Instruction::ControlFlow(cf_opcode) => {
                self.execute_control_flow_instruction(*cf_opcode)?;
            }
        }

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
        let mut executor = VmExecutor::new();
        let bytecode = create_test_bytecode();

        executor.load_bytecode(bytecode).unwrap();
        let result = executor.execute().unwrap();

        assert!(result.instructions_executed > 0);
        assert!(result.final_stack.is_empty()); // Should be empty after PUSH, PUSH, POP, POP
    }

    #[test]
    fn test_arithmetic_operations() {
        let mut executor = VmExecutor::new();
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
        let mut executor = VmExecutor::new();
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
        let mut executor = VmExecutor::new();
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
        let mut executor = VmExecutor::new();
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
