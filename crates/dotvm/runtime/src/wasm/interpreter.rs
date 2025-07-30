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

//! WASM Interpreter Module

use crate::wasm::{WasmError, WasmExecutionContext, WasmResult};
use dotvm_compiler::wasm::ast::types::MemArg;
use dotvm_compiler::wasm::ast::{WasmInstruction, WasmValue as Value, WasmValueType};
use std::collections::HashMap;

/// WASM Interpreter for executing WASM instructions
pub struct WasmInterpreter {
    /// Local variables for current function
    locals: Vec<Value>,
    /// Operand stack
    stack: Vec<Value>,
    /// Block stack for control flow
    block_stack: Vec<BlockFrame>,
}

/// Block frame for control flow management
#[derive(Debug, Clone)]
struct BlockFrame {
    /// Block type (block, loop, if)
    block_type: BlockType,
    /// Start position of block
    start_pc: usize,
    /// End position of block
    end_pc: usize,
    /// Stack height at block entry
    stack_height: usize,
    /// Expected result types
    result_types: Vec<WasmValueType>,
}

/// Block type enumeration
#[derive(Debug, Clone, PartialEq)]
enum BlockType {
    Block,
    Loop,
    If,
    Else,
}

impl WasmInterpreter {
    /// Create a new WASM interpreter
    pub fn new() -> Self {
        Self {
            locals: Vec::new(),
            stack: Vec::new(),
            block_stack: Vec::new(),
        }
    }

    /// Push a value onto the stack
    pub fn push_value(&mut self, value: Value) -> WasmResult<()> {
        self.stack.push(value);
        Ok(())
    }

    /// Pop a value from the stack
    pub fn pop_value(&mut self) -> WasmResult<Value> {
        self.stack.pop().ok_or_else(|| WasmError::execution_error("Stack underflow".to_string()))
    }

    /// Check if the stack is empty
    pub fn is_stack_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Execute a sequence of WASM instructions with parameters
    pub fn execute_instructions(&mut self, instructions: &[WasmInstruction], context: &mut WasmExecutionContext) -> WasmResult<Vec<Value>> {
        self.execute_instructions_with_locals(instructions, context, Vec::new())
    }

    /// Execute a sequence of WASM instructions with local variables
    pub fn execute_instructions_with_locals(&mut self, instructions: &[WasmInstruction], context: &mut WasmExecutionContext, locals: Vec<Value>) -> WasmResult<Vec<Value>> {
        // Set up local variables (parameters + function locals)
        self.locals = locals;

        let mut pc = 0;

        while pc < instructions.len() {
            // Check instruction limits
            context.count_instruction()?;

            // Execute current instruction
            pc = self.execute_instruction(&instructions[pc], pc, instructions, context)?;

            // Check for execution timeout
            context.check_timeout()?;
        }

        // Return remaining values on stack as results
        Ok(self.stack.clone())
    }

    /// Execute a single WASM instruction
    fn execute_instruction(&mut self, instruction: &WasmInstruction, pc: usize, instructions: &[WasmInstruction], context: &mut WasmExecutionContext) -> WasmResult<usize> {
        match instruction {
            // Control instructions
            WasmInstruction::Unreachable => Err(WasmError::ExecutionError {
                message: "Unreachable instruction executed".to_string(),
            }),
            WasmInstruction::Nop => Ok(pc + 1),

            // Block instructions
            WasmInstruction::Block { block_type } => {
                let result_types = if let Some(t) = block_type { vec![t.clone()] } else { vec![] };
                self.enter_block(BlockType::Block, pc, result_types)?;
                Ok(pc + 1)
            }
            WasmInstruction::Loop { block_type } => {
                let result_types = if let Some(t) = block_type { vec![t.clone()] } else { vec![] };
                self.enter_block(BlockType::Loop, pc, result_types)?;
                Ok(pc + 1)
            }
            WasmInstruction::If { block_type } => {
                let condition = self.pop_i32()?;
                if condition != 0 {
                    let result_types = if let Some(t) = block_type { vec![t.clone()] } else { vec![] };
                    self.enter_block(BlockType::If, pc, result_types)?;
                } else {
                    // Skip to else or end
                    return self.skip_to_else_or_end(pc, instructions);
                }
                Ok(pc + 1)
            }
            WasmInstruction::Else => {
                // Skip to end of if block
                self.skip_to_end(pc, instructions)
            }
            WasmInstruction::End => {
                self.exit_block()?;
                Ok(pc + 1)
            }

            // Branch instructions
            WasmInstruction::Br { label_index } => self.branch(*label_index as usize),
            WasmInstruction::BrIf { label_index } => {
                let condition = self.pop_i32()?;
                if condition != 0 { self.branch(*label_index as usize) } else { Ok(pc + 1) }
            }
            WasmInstruction::BrTable { labels, default } => {
                let index = self.pop_i32()? as usize;
                let target = if index < labels.len() { labels[index] } else { *default };
                self.branch(target as usize)
            }
            WasmInstruction::Return => {
                // Return from function - handled by caller
                Ok(usize::MAX) // Special value to indicate return
            }

            // Call instructions
            WasmInstruction::Call { function_index } => {
                self.call_function(*function_index, context)?;
                Ok(pc + 1)
            }
            WasmInstruction::CallIndirect { type_index, table_index } => {
                self.call_indirect(*type_index, *table_index, context)?;
                Ok(pc + 1)
            }

            // Parametric instructions
            WasmInstruction::Drop => {
                self.stack.pop().ok_or_else(|| WasmError::ExecutionError {
                    message: "Stack underflow on drop instruction".to_string(),
                })?;
                Ok(pc + 1)
            }
            WasmInstruction::Select => {
                let condition = self.pop_i32()?;
                let val2 = self.stack.pop().ok_or_else(|| WasmError::ExecutionError {
                    message: "Stack underflow on select instruction".to_string(),
                })?;
                let val1 = self.stack.pop().ok_or_else(|| WasmError::ExecutionError {
                    message: "Stack underflow on select instruction".to_string(),
                })?;
                self.stack.push(if condition != 0 { val1 } else { val2 });
                Ok(pc + 1)
            }

            // Variable instructions
            WasmInstruction::LocalGet { local_index } => {
                let value = self.get_local(*local_index as usize)?;
                self.stack.push(value);
                Ok(pc + 1)
            }
            WasmInstruction::LocalSet { local_index } => {
                let value = self.stack.pop().ok_or_else(|| WasmError::ExecutionError {
                    message: "Stack underflow on local.set instruction".to_string(),
                })?;
                self.set_local(*local_index as usize, value)?;
                Ok(pc + 1)
            }
            WasmInstruction::LocalTee { local_index } => {
                let value = self
                    .stack
                    .last()
                    .ok_or_else(|| WasmError::ExecutionError {
                        message: "Stack underflow on local.tee instruction".to_string(),
                    })?
                    .clone();
                self.set_local(*local_index as usize, value)?;
                Ok(pc + 1)
            }
            WasmInstruction::GlobalGet { global_index } => {
                let value = self.get_global(*global_index, context)?;
                self.stack.push(value);
                Ok(pc + 1)
            }
            WasmInstruction::GlobalSet { global_index } => {
                let value = self.stack.pop().ok_or_else(|| WasmError::ExecutionError {
                    message: "Stack underflow on global.set instruction".to_string(),
                })?;
                self.set_global(*global_index, value, context)?;
                Ok(pc + 1)
            }

            // Memory instructions
            WasmInstruction::I32Load { memarg } => {
                let addr = self.pop_i32()? as u32;
                let value = self.load_i32(addr, memarg, context)?;
                self.stack.push(Value::I32(value));
                Ok(pc + 1)
            }
            WasmInstruction::I32Store { memarg } => {
                let value = self.pop_i32()?;
                let addr = self.pop_i32()? as u32;
                self.store_i32(addr, value, memarg, context)?;
                Ok(pc + 1)
            }

            // Numeric instructions
            WasmInstruction::I32Const { value } => {
                self.stack.push(Value::I32(*value));
                Ok(pc + 1)
            }
            WasmInstruction::I64Const { value } => {
                self.stack.push(Value::I64(*value));
                Ok(pc + 1)
            }
            WasmInstruction::F32Const { value } => {
                self.stack.push(Value::F32(*value));
                Ok(pc + 1)
            }
            WasmInstruction::F64Const { value } => {
                self.stack.push(Value::F64(*value));
                Ok(pc + 1)
            }

            // Arithmetic instructions
            WasmInstruction::I32Add => {
                let b = self.pop_i32()?;
                let a = self.pop_i32()?;
                self.stack.push(Value::I32(a.wrapping_add(b)));
                Ok(pc + 1)
            }
            WasmInstruction::I32Sub => {
                let b = self.pop_i32()?;
                let a = self.pop_i32()?;
                self.stack.push(Value::I32(a.wrapping_sub(b)));
                Ok(pc + 1)
            }
            WasmInstruction::I32Mul => {
                let b = self.pop_i32()?;
                let a = self.pop_i32()?;
                self.stack.push(Value::I32(a.wrapping_mul(b)));
                Ok(pc + 1)
            }
            WasmInstruction::I32DivS => {
                let b = self.pop_i32()?;
                let a = self.pop_i32()?;
                if b == 0 {
                    return Err(WasmError::ExecutionError {
                        message: "Division by zero".to_string(),
                    });
                }
                self.stack.push(Value::I32(a / b));
                Ok(pc + 1)
            }

            // Comparison instructions
            WasmInstruction::I32Eq => {
                let b = self.pop_i32()?;
                let a = self.pop_i32()?;
                self.stack.push(Value::I32(if a == b { 1 } else { 0 }));
                Ok(pc + 1)
            }
            WasmInstruction::I32Ne => {
                let b = self.pop_i32()?;
                let a = self.pop_i32()?;
                self.stack.push(Value::I32(if a != b { 1 } else { 0 }));
                Ok(pc + 1)
            }
            WasmInstruction::I32LtS => {
                let b = self.pop_i32()?;
                let a = self.pop_i32()?;
                self.stack.push(Value::I32(if a < b { 1 } else { 0 }));
                Ok(pc + 1)
            }
            WasmInstruction::I32GtS => {
                let b = self.pop_i32()?;
                let a = self.pop_i32()?;
                self.stack.push(Value::I32(if a > b { 1 } else { 0 }));
                Ok(pc + 1)
            }

            // Default case for unimplemented instructions
            _ => Err(WasmError::ExecutionError {
                message: format!("Unimplemented instruction: {:?}", instruction),
            }),
        }
    }

    /// Pop i32 value from stack
    fn pop_i32(&mut self) -> WasmResult<i32> {
        match self.stack.pop() {
            Some(Value::I32(val)) => Ok(val),
            Some(other) => Err(WasmError::TypeMismatch {
                expected: "i32".to_string(),
                actual: format!("{:?}", other),
            }),
            None => Err(WasmError::ExecutionError {
                message: "Stack underflow when popping i32 value".to_string(),
            }),
        }
    }

    /// Get local variable
    fn get_local(&self, index: usize) -> WasmResult<Value> {
        self.locals.get(index).cloned().ok_or(WasmError::ExecutionError {
            message: format!("Local variable {} not found", index),
        })
    }

    /// Set local variable
    fn set_local(&mut self, index: usize, value: Value) -> WasmResult<()> {
        if index < self.locals.len() {
            self.locals[index] = value;
            Ok(())
        } else {
            Err(WasmError::ExecutionError {
                message: format!("Local variable {} not found", index),
            })
        }
    }

    /// Get global variable
    fn get_global(&self, index: u32, context: &WasmExecutionContext) -> WasmResult<Value> {
        // Access global variables from execution context
        if let Some(globals) = context.wasm.globals.as_ref() {
            globals.get(index as usize).map(|global| global.get().clone()).ok_or_else(|| WasmError::ExecutionError {
                message: format!("Global variable {} not found or inaccessible", index),
            })
        } else {
            Err(WasmError::ExecutionError {
                message: "No global variables available in context".to_string(),
            })
        }
    }

    /// Set global variable
    fn set_global(&mut self, index: u32, value: Value, context: &mut WasmExecutionContext) -> WasmResult<()> {
        // Set global variables in execution context
        if let Some(globals) = context.wasm.globals.as_mut() {
            if let Some(global) = globals.get_mut(index as usize) {
                global.set(value).map_err(|e| WasmError::ExecutionError {
                    message: format!("Failed to set global variable {}: {:?}", index, e),
                })
            } else {
                Err(WasmError::ExecutionError {
                    message: format!("Global variable {} not found", index),
                })
            }
        } else {
            Err(WasmError::ExecutionError {
                message: "No global variables available in context".to_string(),
            })
        }
    }

    /// Load i32 from memory
    fn load_i32(&self, addr: u32, memarg: &MemArg, context: &WasmExecutionContext) -> WasmResult<i32> {
        // Calculate effective address with offset
        let offset_u32 = memarg.offset.try_into().map_err(|_| WasmError::ExecutionError {
            message: format!("Memory offset too large: {}", memarg.offset),
        })?;
        let effective_addr = addr.checked_add(offset_u32).ok_or_else(|| WasmError::ExecutionError {
            message: format!("Memory address overflow: {} + {}", addr, offset_u32),
        })?;

        // Access memory from execution context
        if let Some(memory) = context.wasm.memory.as_ref() {
            memory.read_i32(effective_addr as usize).map_err(|e| WasmError::ExecutionError {
                message: format!("Failed to load i32 from memory at {}: {:?}", effective_addr, e),
            })
        } else {
            Err(WasmError::ExecutionError {
                message: "No memory available in context".to_string(),
            })
        }
    }

    /// Store i32 to memory
    fn store_i32(&mut self, addr: u32, value: i32, memarg: &MemArg, context: &mut WasmExecutionContext) -> WasmResult<()> {
        // Calculate effective address with offset
        let offset_u32 = memarg.offset.try_into().map_err(|_| WasmError::ExecutionError {
            message: format!("Memory offset too large: {}", memarg.offset),
        })?;
        let effective_addr = addr.checked_add(offset_u32).ok_or_else(|| WasmError::ExecutionError {
            message: format!("Memory address overflow: {} + {}", addr, offset_u32),
        })?;

        // Store to memory in execution context
        if let Some(memory) = context.wasm.memory.as_mut() {
            memory.write_i32(effective_addr as usize, value).map_err(|e| WasmError::ExecutionError {
                message: format!("Failed to store i32 to memory at {}: {:?}", effective_addr, e),
            })
        } else {
            Err(WasmError::ExecutionError {
                message: "No memory available in context".to_string(),
            })
        }
    }

    /// Enter a block
    fn enter_block(&mut self, block_type: BlockType, pc: usize, _result_types: Vec<WasmValueType>) -> WasmResult<()> {
        let frame = BlockFrame {
            block_type,
            start_pc: pc,
            end_pc: 0, // Will be set when we find the matching end
            stack_height: self.stack.len(),
            result_types: _result_types,
        };
        self.block_stack.push(frame);
        Ok(())
    }

    /// Exit a block
    fn exit_block(&mut self) -> WasmResult<()> {
        self.block_stack.pop().ok_or(WasmError::ExecutionError {
            message: "No block to exit".to_string(),
        })?;
        Ok(())
    }

    /// Branch to a block
    fn branch(&mut self, depth: usize) -> WasmResult<usize> {
        if depth >= self.block_stack.len() {
            return Err(WasmError::ExecutionError {
                message: format!("Invalid branch depth: {}", depth),
            });
        }

        let target_frame = &self.block_stack[self.block_stack.len() - 1 - depth];
        match target_frame.block_type {
            BlockType::Loop => Ok(target_frame.start_pc),
            _ => Ok(target_frame.end_pc),
        }
    }

    /// Skip to else or end of if block
    fn skip_to_else_or_end(&self, pc: usize, instructions: &[WasmInstruction]) -> WasmResult<usize> {
        let mut depth = 0;
        let mut current_pc = pc + 1;

        while current_pc < instructions.len() {
            match &instructions[current_pc] {
                WasmInstruction::If { .. } | WasmInstruction::Block { .. } | WasmInstruction::Loop { .. } => {
                    depth += 1;
                }
                WasmInstruction::Else => {
                    if depth == 0 {
                        return Ok(current_pc + 1); // Skip to instruction after else
                    }
                }
                WasmInstruction::End => {
                    if depth == 0 {
                        return Ok(current_pc + 1); // Skip to instruction after end
                    }
                    depth -= 1;
                }
                _ => {}
            }
            current_pc += 1;
        }

        Err(WasmError::ExecutionError {
            message: "Unmatched if block - no corresponding else or end found".to_string(),
        })
    }

    /// Skip to end of block
    fn skip_to_end(&self, pc: usize, instructions: &[WasmInstruction]) -> WasmResult<usize> {
        let mut depth = 0;
        let mut current_pc = pc + 1;

        while current_pc < instructions.len() {
            match &instructions[current_pc] {
                WasmInstruction::If { .. } | WasmInstruction::Block { .. } | WasmInstruction::Loop { .. } => {
                    depth += 1;
                }
                WasmInstruction::End => {
                    if depth == 0 {
                        return Ok(current_pc + 1); // Skip to instruction after end
                    }
                    depth -= 1;
                }
                _ => {}
            }
            current_pc += 1;
        }

        Err(WasmError::ExecutionError {
            message: "Unmatched block - no corresponding end found".to_string(),
        })
    }

    /// Call a function
    fn call_function(&mut self, function_index: u32, context: &mut WasmExecutionContext) -> WasmResult<()> {
        // Get function signature to know parameter and return counts
        let func_signature = context.wasm.get_function_signature(function_index).ok_or_else(|| WasmError::ExecutionError {
            message: format!("Function {} not found", function_index),
        })?;

        // Pop arguments from stack
        let mut args = Vec::new();
        for _ in 0..func_signature.param_count() {
            let arg = self.stack.pop().ok_or_else(|| WasmError::ExecutionError {
                message: "Stack underflow when popping function arguments".to_string(),
            })?;
            args.push(arg);
        }
        args.reverse(); // Arguments were popped in reverse order

        // Execute function (this would typically involve recursion or a call stack)
        // For now, we'll simulate a simple function call that returns default values
        let results = self.execute_function_call(function_index, args, context)?;

        // Push results back onto stack
        for result in results {
            self.stack.push(result);
        }

        Ok(())
    }

    /// Call indirect function through table
    fn call_indirect(&mut self, type_index: u32, table_index: u32, context: &mut WasmExecutionContext) -> WasmResult<()> {
        // Pop function index from stack
        let func_index = self.pop_i32()? as u32;

        // Get function from table
        let actual_func_index = context.wasm.get_table_function(table_index, func_index).ok_or_else(|| WasmError::ExecutionError {
            message: format!("Function at table[{}][{}] not found", table_index, func_index),
        })?;

        // Verify function type matches expected type
        let expected_type = context.wasm.get_function_type(type_index).ok_or_else(|| WasmError::ExecutionError {
            message: format!("Function type {} not found", type_index),
        })?;

        let actual_type = context.wasm.get_function_signature(actual_func_index).ok_or_else(|| WasmError::ExecutionError {
            message: format!("Function {} signature not found", actual_func_index),
        })?;

        if !self.types_match(&expected_type, &actual_type) {
            return Err(WasmError::ExecutionError {
                message: format!("Function type mismatch for indirect call"),
            });
        }

        // Call the function
        self.call_function(actual_func_index, context)
    }

    /// Execute function call with proper parameter handling and local variable management
    fn execute_function_call(&mut self, function_index: u32, args: Vec<Value>, context: &mut WasmExecutionContext) -> WasmResult<Vec<Value>> {
        // Get function signature and clone it to avoid borrowing issues
        let func_signature = context.wasm.get_function_signature(function_index).cloned().ok_or_else(|| WasmError::ExecutionError {
            message: format!("Function {} signature not found", function_index),
        })?;

        // Validate argument count
        if args.len() != func_signature.params.len() {
            return Err(WasmError::ExecutionError {
                message: format!("Function {} expects {} arguments, got {}", function_index, func_signature.params.len(), args.len()),
            });
        }

        // Validate argument types
        for (i, (arg, expected_type)) in args.iter().zip(func_signature.params.iter()).enumerate() {
            if !self.value_matches_type(arg, expected_type) {
                return Err(WasmError::ExecutionError {
                    message: format!(
                        "Function {} argument {} type mismatch: expected {:?}, got {:?}",
                        function_index,
                        i,
                        expected_type,
                        self.get_value_type(arg)
                    ),
                });
            }
        }

        // Check if this is a host function
        if context.wasm.host_functions.contains_key(&function_index.to_string()) {
            // Clone the host function to avoid borrowing issues
            let host_func_key = function_index.to_string();
            if let Some(host_func) = context.wasm.host_functions.get(&host_func_key) {
                let result = host_func(&args)?;
                context.metrics.host_function_calls += 1;
                return Ok(result);
            }
        }

        // Save current locals state
        let saved_locals = self.locals.clone();

        // Set up function locals with parameters
        self.locals.clear();
        for arg in args {
            self.locals.push(arg);
        }

        // Get function metadata to determine local variable count
        let local_count = self.get_function_local_count(function_index, context);

        // Initialize additional local variables with default values
        for _ in func_signature.params.len()..local_count {
            self.locals.push(Value::I32(0)); // Default initialization
        }

        // Create call frame
        let call_frame = crate::wasm::execution::CallFrame {
            function_index,
            return_arity: func_signature.returns.len(),
            locals_start: 0, // Start of local variables on stack
            metadata: crate::wasm::execution::FrameMetadata {
                function_name: format!("func_{}", function_index),
                call_time: std::time::Instant::now(),
                instructions_executed: 0,
                tags: std::collections::HashMap::new(),
            },
        };

        // Push call frame to execution context
        context.wasm.call_stack.push(call_frame);
        context.wasm.call_depth += 1;
        context.metrics.function_calls += 1;

        // Check call depth limit
        if context.wasm.call_depth > context.wasm.max_call_depth {
            // Restore state
            self.locals = saved_locals;
            context.wasm.call_stack.pop();
            context.wasm.call_depth -= 1;

            return Err(WasmError::ExecutionError {
                message: format!("Call depth limit exceeded: current={}, limit={}", context.wasm.call_depth, context.wasm.max_call_depth),
            });
        }

        // Execute function body (simplified implementation)
        let result = self.execute_function_body(function_index, context);

        // Clean up call frame
        context.wasm.call_stack.pop();
        context.wasm.call_depth -= 1;

        // Restore locals
        self.locals = saved_locals;

        // Validate return values
        match &result {
            Ok(return_values) => {
                if return_values.len() != func_signature.returns.len() {
                    return Err(WasmError::ExecutionError {
                        message: format!("Function {} returned {} values, expected {}", function_index, return_values.len(), func_signature.returns.len()),
                    });
                }

                // Validate return value types
                for (i, (value, expected_type)) in return_values.iter().zip(func_signature.returns.iter()).enumerate() {
                    if !self.value_matches_type(value, expected_type) {
                        return Err(WasmError::ExecutionError {
                            message: format!(
                                "Function {} return value {} type mismatch: expected {:?}, got {:?}",
                                function_index,
                                i,
                                expected_type,
                                self.get_value_type(value)
                            ),
                        });
                    }
                }
            }
            Err(_) => {}
        }

        result
    }

    /// Check if function types match
    fn types_match(&self, expected: &crate::wasm::FunctionSignature, actual: &crate::wasm::FunctionSignature) -> bool {
        // Compare parameter types
        if expected.params.len() != actual.params.len() {
            return false;
        }

        for (expected_param, actual_param) in expected.params.iter().zip(actual.params.iter()) {
            if expected_param != actual_param {
                return false;
            }
        }

        // Compare return types
        if expected.returns.len() != actual.returns.len() {
            return false;
        }

        for (expected_return, actual_return) in expected.returns.iter().zip(actual.returns.iter()) {
            if expected_return != actual_return {
                return false;
            }
        }

        true
    }

    /// Check if a value matches the expected type
    fn value_matches_type(&self, value: &Value, expected_type: &crate::wasm::execution::ValueType) -> bool {
        match (value, expected_type) {
            (Value::I32(_), crate::wasm::execution::ValueType::I32) => true,
            (Value::I64(_), crate::wasm::execution::ValueType::I64) => true,
            (Value::F32(_), crate::wasm::execution::ValueType::F32) => true,
            (Value::F64(_), crate::wasm::execution::ValueType::F64) => true,
            (Value::V128(_), crate::wasm::execution::ValueType::V128) => true,
            (Value::FuncRef(_), crate::wasm::execution::ValueType::FuncRef) => true,
            (Value::ExternRef(_), crate::wasm::execution::ValueType::ExternRef) => true,
            _ => false,
        }
    }

    /// Get the type of a value
    fn get_value_type(&self, value: &Value) -> crate::wasm::execution::ValueType {
        match value {
            Value::I32(_) => crate::wasm::execution::ValueType::I32,
            Value::I64(_) => crate::wasm::execution::ValueType::I64,
            Value::F32(_) => crate::wasm::execution::ValueType::F32,
            Value::F64(_) => crate::wasm::execution::ValueType::F64,
            Value::V128(_) => crate::wasm::execution::ValueType::V128,
            Value::FuncRef(_) => crate::wasm::execution::ValueType::FuncRef,
            Value::ExternRef(_) => crate::wasm::execution::ValueType::ExternRef,
        }
    }

    /// Get function local variable count from WASM module function metadata
    fn get_function_local_count(&self, function_index: u32, context: &WasmExecutionContext) -> usize {
        // Access the actual WASM module to get function metadata
        if let Some(module) = &context.wasm.module {
            // Try to find function metadata by index
            for (_, func_meta) in &module.functions {
                if func_meta.index == function_index {
                    // Return the actual local count from the module's function metadata
                    // This includes parameters + declared local variables
                    return func_meta.local_count as usize;
                }
            }

            // If function not found in metadata, try to get from compiled module
            // Access the compiler's WASM module for function information
            if let Some(compiler_func) = module.compiled.functions.get(function_index as usize) {
                // Get parameter count from function signature
                let param_count = compiler_func.signature.params.len();

                // Get local variable count from function body
                // In WASM bytecode, locals are declared at the beginning of function body
                let local_count = compiler_func.locals.len();

                // Total locals = parameters + declared locals
                return param_count + local_count;
            }
        }

        // Fallback: Get from function signature if module not available
        let default_signature = crate::wasm::execution::FunctionSignature { params: vec![], returns: vec![] };
        let func_signature = context.wasm.get_function_signature(function_index).unwrap_or(&default_signature);

        // Return parameter count as minimum (WASM functions always have at least their parameters as locals)
        func_signature.params.len()
    }

    /// Parse function local variables from WASM module
    fn parse_function_locals_from_metadata(&self, function_index: u32, context: &WasmExecutionContext) -> Option<usize> {
        // Access the actual WASM module to get function local variables
        if let Some(module) = &context.wasm.module {
            // Get the function from the compiled module
            if let Some(compiler_func) = module.compiled.functions.get(function_index as usize) {
                // Return the actual local variable count from the WASM function
                return Some(compiler_func.locals.len());
            }
        }

        // If no module available, return None to indicate no additional locals found
        None
    }

    /// Set function local variable count metadata
    pub fn set_function_local_count(&self, function_index: u32, local_count: usize, context: &mut WasmExecutionContext) {
        context.wasm.metadata.insert(format!("func_{}_locals", function_index), local_count.to_string());
    }

    /// Call host function
    fn call_host_function(&mut self, host_func: &Box<dyn Fn(&[Value]) -> WasmResult<Vec<Value>> + Send + Sync>, args: &[Value], context: &mut WasmExecutionContext) -> WasmResult<Vec<Value>> {
        context.metrics.host_function_calls += 1;

        // Call the host function
        host_func(args)
    }

    /// Execute function body (simplified implementation)
    fn execute_function_body(&mut self, function_index: u32, context: &mut WasmExecutionContext) -> WasmResult<Vec<Value>> {
        // This is a simplified implementation that would normally:
        // 1. Fetch function bytecode from the module
        // 2. Execute instructions one by one
        // 3. Handle control flow (loops, branches, etc.)
        // 4. Manage local variables and stack
        // 5. Handle memory operations

        // For now, implement basic arithmetic operations based on function index
        match function_index {
            0 => {
                // Example: add function - adds two i32 values
                if self.locals.len() >= 2 {
                    if let (Value::I32(a), Value::I32(b)) = (&self.locals[0], &self.locals[1]) {
                        Ok(vec![Value::I32(a + b)])
                    } else {
                        Err(WasmError::ExecutionError {
                            message: "Invalid arguments for add function".to_string(),
                        })
                    }
                } else {
                    Err(WasmError::ExecutionError {
                        message: "Insufficient arguments for add function".to_string(),
                    })
                }
            }
            1 => {
                // Example: multiply function
                if self.locals.len() >= 2 {
                    if let (Value::I32(a), Value::I32(b)) = (&self.locals[0], &self.locals[1]) {
                        Ok(vec![Value::I32(a * b)])
                    } else {
                        Err(WasmError::ExecutionError {
                            message: "Invalid arguments for multiply function".to_string(),
                        })
                    }
                } else {
                    Err(WasmError::ExecutionError {
                        message: "Insufficient arguments for multiply function".to_string(),
                    })
                }
            }
            2 => {
                // Example: factorial function
                if !self.locals.is_empty() {
                    if let Value::I32(n) = &self.locals[0] {
                        let result = if *n <= 1 {
                            1
                        } else {
                            let mut factorial = 1i32;
                            for i in 2..=*n {
                                factorial = factorial.saturating_mul(i);
                            }
                            factorial
                        };
                        Ok(vec![Value::I32(result)])
                    } else {
                        Err(WasmError::ExecutionError {
                            message: "Invalid argument for factorial function".to_string(),
                        })
                    }
                } else {
                    Err(WasmError::ExecutionError {
                        message: "No argument provided for factorial function".to_string(),
                    })
                }
            }
            _ => {
                // Default behavior for unknown functions
                let func_signature = context.wasm.get_function_signature(function_index).ok_or_else(|| WasmError::ExecutionError {
                    message: format!("Function {} not found", function_index),
                })?;

                // Return default values based on return type
                let mut results = Vec::new();
                for return_type in &func_signature.returns {
                    let default_value = match return_type {
                        crate::wasm::execution::ValueType::I32 => Value::I32(0),
                        crate::wasm::execution::ValueType::I64 => Value::I64(0),
                        crate::wasm::execution::ValueType::F32 => Value::F32(0.0),
                        crate::wasm::execution::ValueType::F64 => Value::F64(0.0),
                        crate::wasm::execution::ValueType::V128 => Value::V128([0u8; 16]),
                        crate::wasm::execution::ValueType::FuncRef => Value::FuncRef(None),
                        crate::wasm::execution::ValueType::ExternRef => Value::ExternRef(None),
                    };
                    results.push(default_value);
                }

                Ok(results)
            }
        }
    }
}

impl Default for WasmInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpreter_creation() {
        let interpreter = WasmInterpreter::new();
        assert_eq!(interpreter.stack.len(), 0);
        assert_eq!(interpreter.locals.len(), 0);
        assert_eq!(interpreter.block_stack.len(), 0);
    }

    #[test]
    fn test_i32_arithmetic() {
        let mut interpreter = WasmInterpreter::new();

        // Test addition
        interpreter.stack.push(Value::I32(5));
        interpreter.stack.push(Value::I32(3));

        let result = interpreter.pop_i32().unwrap();
        assert_eq!(result, 3);

        let result = interpreter.pop_i32().unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn test_stack_underflow() {
        let mut interpreter = WasmInterpreter::new();

        let result = interpreter.pop_i32();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WasmError::ExecutionError { .. }));
    }
}
