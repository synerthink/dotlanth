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

//! WASM to DotVM Bytecode Transpiler

use crate::wasm::{WasmError as RuntimeError, WasmResult as RuntimeResult};
use dotvm_compiler::wasm::{WasmError, WasmInstruction, WasmModule, WasmParser, WasmResult};
use dotvm_core::bytecode::{BytecodeFile, ConstantValue, VmArchitecture};
use std::collections::HashMap;

/// WASM to DotVM transpiler configuration
#[derive(Debug, Clone)]
pub struct TranspilerConfig {
    /// Target DotVM architecture
    pub target_architecture: VmArchitecture,
    /// Enable custom DotVM opcode injection
    pub enable_custom_opcodes: bool,
    /// Enable optimization passes
    pub enable_optimizations: bool,
    /// Maximum function size for inlining
    pub max_inline_size: usize,
    /// Enable debug information preservation
    pub preserve_debug_info: bool,
}

impl Default for TranspilerConfig {
    fn default() -> Self {
        Self {
            target_architecture: VmArchitecture::Arch64,
            enable_custom_opcodes: true,
            enable_optimizations: true,
            max_inline_size: 100,
            preserve_debug_info: false,
        }
    }
}

/// Standard WASM opcode to DotVM opcode mapping
#[derive(Debug, Clone)]
pub struct OpcodeMapping {
    /// Standard WASM instruction mappings
    standard_mappings: HashMap<String, Vec<u8>>,
    /// Custom DotVM opcode mappings
    custom_mappings: HashMap<String, Vec<u8>>,
}

impl Default for OpcodeMapping {
    fn default() -> Self {
        let mut mapping = Self {
            standard_mappings: HashMap::new(),
            custom_mappings: HashMap::new(),
        };
        mapping.initialize_standard_mappings();
        mapping.initialize_custom_mappings();
        mapping
    }
}

impl OpcodeMapping {
    /// Initialize standard WASM to DotVM opcode mappings
    fn initialize_standard_mappings(&mut self) {
        // Control flow opcodes
        self.standard_mappings.insert("nop".to_string(), vec![0x00]);
        self.standard_mappings.insert("unreachable".to_string(), vec![0x01]);
        self.standard_mappings.insert("block".to_string(), vec![0x02]);
        self.standard_mappings.insert("loop".to_string(), vec![0x03]);
        self.standard_mappings.insert("if".to_string(), vec![0x04]);
        self.standard_mappings.insert("else".to_string(), vec![0x05]);
        self.standard_mappings.insert("end".to_string(), vec![0x0B]);
        self.standard_mappings.insert("br".to_string(), vec![0x0C]);
        self.standard_mappings.insert("br_if".to_string(), vec![0x0D]);
        self.standard_mappings.insert("br_table".to_string(), vec![0x0E]);
        self.standard_mappings.insert("return".to_string(), vec![0x0F]);
        self.standard_mappings.insert("call".to_string(), vec![0x10]);
        self.standard_mappings.insert("call_indirect".to_string(), vec![0x11]);

        // Variable access opcodes
        self.standard_mappings.insert("local.get".to_string(), vec![0x20]);
        self.standard_mappings.insert("local.set".to_string(), vec![0x21]);
        self.standard_mappings.insert("local.tee".to_string(), vec![0x22]);
        self.standard_mappings.insert("global.get".to_string(), vec![0x23]);
        self.standard_mappings.insert("global.set".to_string(), vec![0x24]);

        // Memory opcodes
        self.standard_mappings.insert("i32.load".to_string(), vec![0x28]);
        self.standard_mappings.insert("i64.load".to_string(), vec![0x29]);
        self.standard_mappings.insert("f32.load".to_string(), vec![0x2A]);
        self.standard_mappings.insert("f64.load".to_string(), vec![0x2B]);
        self.standard_mappings.insert("i32.store".to_string(), vec![0x36]);
        self.standard_mappings.insert("i64.store".to_string(), vec![0x37]);
        self.standard_mappings.insert("f32.store".to_string(), vec![0x38]);
        self.standard_mappings.insert("f64.store".to_string(), vec![0x39]);
        self.standard_mappings.insert("memory.size".to_string(), vec![0x3F]);
        self.standard_mappings.insert("memory.grow".to_string(), vec![0x40]);

        // Constant opcodes
        self.standard_mappings.insert("i32.const".to_string(), vec![0x41]);
        self.standard_mappings.insert("i64.const".to_string(), vec![0x42]);
        self.standard_mappings.insert("f32.const".to_string(), vec![0x43]);
        self.standard_mappings.insert("f64.const".to_string(), vec![0x44]);

        // Arithmetic opcodes (i32)
        self.standard_mappings.insert("i32.add".to_string(), vec![0x6A]);
        self.standard_mappings.insert("i32.sub".to_string(), vec![0x6B]);
        self.standard_mappings.insert("i32.mul".to_string(), vec![0x6C]);
        self.standard_mappings.insert("i32.div_s".to_string(), vec![0x6D]);
        self.standard_mappings.insert("i32.div_u".to_string(), vec![0x6E]);
        self.standard_mappings.insert("i32.rem_s".to_string(), vec![0x6F]);
        self.standard_mappings.insert("i32.rem_u".to_string(), vec![0x70]);
        self.standard_mappings.insert("i32.and".to_string(), vec![0x71]);
        self.standard_mappings.insert("i32.or".to_string(), vec![0x72]);
        self.standard_mappings.insert("i32.xor".to_string(), vec![0x73]);
        self.standard_mappings.insert("i32.shl".to_string(), vec![0x74]);
        self.standard_mappings.insert("i32.shr_s".to_string(), vec![0x75]);
        self.standard_mappings.insert("i32.shr_u".to_string(), vec![0x76]);

        // Arithmetic opcodes (i64)
        self.standard_mappings.insert("i64.add".to_string(), vec![0x7C]);
        self.standard_mappings.insert("i64.sub".to_string(), vec![0x7D]);
        self.standard_mappings.insert("i64.mul".to_string(), vec![0x7E]);
        self.standard_mappings.insert("i64.div_s".to_string(), vec![0x7F]);
        self.standard_mappings.insert("i64.div_u".to_string(), vec![0x80]);

        // Floating point opcodes (f32)
        self.standard_mappings.insert("f32.add".to_string(), vec![0x92]);
        self.standard_mappings.insert("f32.sub".to_string(), vec![0x93]);
        self.standard_mappings.insert("f32.mul".to_string(), vec![0x94]);
        self.standard_mappings.insert("f32.div".to_string(), vec![0x95]);

        // Floating point opcodes (f64)
        self.standard_mappings.insert("f64.add".to_string(), vec![0xA0]);
        self.standard_mappings.insert("f64.sub".to_string(), vec![0xA1]);
        self.standard_mappings.insert("f64.mul".to_string(), vec![0xA2]);
        self.standard_mappings.insert("f64.div".to_string(), vec![0xA3]);

        // Stack manipulation
        self.standard_mappings.insert("drop".to_string(), vec![0x1A]);
        self.standard_mappings.insert("select".to_string(), vec![0x1B]);
    }

    /// Initialize custom DotVM opcode mappings
    fn initialize_custom_mappings(&mut self) {
        // Custom DotVM opcodes for "dot" features
        self.custom_mappings.insert("dot.state.get".to_string(), vec![0xF0, 0x01]);
        self.custom_mappings.insert("dot.state.set".to_string(), vec![0xF0, 0x02]);
        self.custom_mappings.insert("dot.db.query".to_string(), vec![0xF1, 0x01]);
        self.custom_mappings.insert("dot.db.insert".to_string(), vec![0xF1, 0x02]);
        self.custom_mappings.insert("dot.crypto.hash".to_string(), vec![0xF2, 0x01]);
        self.custom_mappings.insert("dot.crypto.verify".to_string(), vec![0xF2, 0x02]);
        self.custom_mappings.insert("dot.parallel.spawn".to_string(), vec![0xF3, 0x01]);
        self.custom_mappings.insert("dot.parallel.join".to_string(), vec![0xF3, 0x02]);
        self.custom_mappings.insert("dot.system.call".to_string(), vec![0xF4, 0x01]);
        self.custom_mappings.insert("dot.math.bigint_add".to_string(), vec![0xF5, 0x01]);
        self.custom_mappings.insert("dot.math.bigint_mul".to_string(), vec![0xF5, 0x02]);
    }

    /// Get opcode mapping for a WASM instruction
    pub fn get_mapping(&self, instruction_name: &str) -> Option<&Vec<u8>> {
        self.standard_mappings.get(instruction_name).or_else(|| self.custom_mappings.get(instruction_name))
    }

    /// Check if instruction is a custom DotVM opcode
    pub fn is_custom_opcode(&self, instruction_name: &str) -> bool {
        self.custom_mappings.contains_key(instruction_name)
    }
}

/// WASM to DotVM transpiler
pub struct WasmTranspiler {
    /// Transpiler configuration
    config: TranspilerConfig,
    /// Opcode mapping table
    opcode_mapping: OpcodeMapping,
    /// WASM parser
    parser: WasmParser,
    /// Transpiler statistics
    statistics: TranspilerStatistics,
}

impl std::fmt::Debug for WasmTranspiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmTranspiler").field("config", &self.config).field("opcode_mapping", &self.opcode_mapping).finish()
    }
}

impl WasmTranspiler {
    /// Create a new WASM transpiler
    pub fn new(config: TranspilerConfig) -> Self {
        Self {
            config,
            opcode_mapping: OpcodeMapping::default(),
            parser: WasmParser::new(),
            statistics: TranspilerStatistics::default(),
        }
    }

    /// Create transpiler with default configuration
    pub fn default() -> Self {
        Self::new(TranspilerConfig::default())
    }

    /// Transpile WASM binary to DotVM bytecode
    pub fn transpile(&mut self, wasm_binary: &[u8]) -> RuntimeResult<BytecodeFile> {
        // Parse WASM module
        let wasm_module = self.parser.parse(wasm_binary).map_err(|e| RuntimeError::execution_error(format!("WASM parsing failed: {}", e)))?;

        // Create DotVM bytecode file
        let mut bytecode = BytecodeFile::new(self.config.target_architecture);

        // Transpile functions
        self.transpile_functions(&wasm_module, &mut bytecode)?;

        // Transpile globals
        self.transpile_globals(&wasm_module, &mut bytecode)?;

        // Transpile memory sections
        self.transpile_memory(&wasm_module, &mut bytecode)?;

        // Apply optimizations if enabled
        if self.config.enable_optimizations {
            self.optimize_bytecode(&mut bytecode)?;
        }

        Ok(bytecode)
    }

    /// Transpile WASM functions to DotVM bytecode
    fn transpile_functions(&self, module: &WasmModule, bytecode: &mut BytecodeFile) -> RuntimeResult<()> {
        for function in &module.functions {
            for instruction in &function.body {
                self.transpile_instruction(instruction, bytecode)?;
            }
        }
        Ok(())
    }

    /// Transpile a single WASM instruction to DotVM opcodes
    fn transpile_instruction(&self, instruction: &WasmInstruction, bytecode: &mut BytecodeFile) -> RuntimeResult<()> {
        let instruction_name = instruction.name();

        // Check for custom DotVM opcodes first
        if self.config.enable_custom_opcodes && self.should_inject_custom_opcode(instruction) {
            return self.inject_custom_opcode(instruction, bytecode);
        }

        // Get standard mapping
        if let Some(opcodes) = self.opcode_mapping.get_mapping(&instruction_name) {
            // Add the mapped opcodes
            for &opcode in opcodes {
                bytecode.add_instruction(opcode, &[]);
            }

            // Add operands based on instruction type
            self.add_instruction_operands(instruction, bytecode)?;
        } else {
            return Err(RuntimeError::execution_error(format!("Unsupported WASM instruction: {}", instruction_name)));
        }

        Ok(())
    }

    /// Check if custom DotVM opcode should be injected
    fn should_inject_custom_opcode(&self, instruction: &WasmInstruction) -> bool {
        // Only inject custom opcodes if enabled in config
        if !self.config.enable_custom_opcodes {
            return false;
        }

        // Inject custom opcodes for specific patterns or function calls
        match instruction {
            WasmInstruction::Call { function_index } => {
                // Inject custom opcodes for high-frequency function calls
                // This could be based on function index patterns or call frequency
                *function_index < 10 // Inject for first 10 functions (system/builtin functions)
            }
            WasmInstruction::I32Add | WasmInstruction::I32Sub | WasmInstruction::I32Mul | WasmInstruction::I32DivS => {
                // Inject optimized arithmetic opcodes
                true
            }
            WasmInstruction::LocalGet { .. } | WasmInstruction::LocalSet { .. } => {
                // Inject optimized local variable access
                true
            }
            _ => false,
        }
    }

    /// Inject custom DotVM opcode
    fn inject_custom_opcode(&self, instruction: &WasmInstruction, bytecode: &mut BytecodeFile) -> RuntimeResult<()> {
        match instruction {
            WasmInstruction::Call { function_index } => {
                // Example: inject dot.state.get for state access
                if let Some(opcodes) = self.opcode_mapping.get_mapping("dot.state.get") {
                    for &opcode in opcodes {
                        bytecode.add_instruction(opcode, &[]);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Add operands for WASM instruction
    fn add_instruction_operands(&self, instruction: &WasmInstruction, bytecode: &mut BytecodeFile) -> RuntimeResult<()> {
        match instruction {
            WasmInstruction::LocalGet { local_index } | WasmInstruction::LocalSet { local_index } | WasmInstruction::LocalTee { local_index } => {
                bytecode.add_instruction(0, &local_index.to_le_bytes());
            }
            WasmInstruction::GlobalGet { global_index } | WasmInstruction::GlobalSet { global_index } => {
                bytecode.add_instruction(0, &global_index.to_le_bytes());
            }
            WasmInstruction::I32Const { value } => {
                let const_id = bytecode.add_constant(ConstantValue::Int64(*value as i64));
                bytecode.add_instruction(0, &const_id.to_le_bytes());
            }
            WasmInstruction::I64Const { value } => {
                let const_id = bytecode.add_constant(ConstantValue::Int64(*value));
                bytecode.add_instruction(0, &const_id.to_le_bytes());
            }
            WasmInstruction::F32Const { value } => {
                let const_id = bytecode.add_constant(ConstantValue::Float64(*value as f64));
                bytecode.add_instruction(0, &const_id.to_le_bytes());
            }
            WasmInstruction::F64Const { value } => {
                let const_id = bytecode.add_constant(ConstantValue::Float64(*value));
                bytecode.add_instruction(0, &const_id.to_le_bytes());
            }
            WasmInstruction::Call { function_index } => {
                bytecode.add_instruction(0, &function_index.to_le_bytes());
            }
            _ => {
                // No operands for this instruction
            }
        }
        Ok(())
    }

    /// Transpile global variables
    fn transpile_globals(&self, module: &WasmModule, bytecode: &mut BytecodeFile) -> RuntimeResult<()> {
        for global in &module.globals {
            // Add global initialization code
            for instruction in &global.init_expr {
                self.transpile_instruction(instruction, bytecode)?;
            }
        }
        Ok(())
    }

    /// Transpile memory sections
    fn transpile_memory(&self, module: &WasmModule, bytecode: &mut BytecodeFile) -> RuntimeResult<()> {
        for memory in &module.memories {
            // Add memory initialization opcodes
            // This would involve setting up memory regions in DotVM
            let memory_size = memory.memory_type.initial * 65536; // WASM page size
            let const_id = bytecode.add_constant(ConstantValue::Int64(memory_size as i64));

            // Add memory allocation instruction
            bytecode.add_instruction(0x3F, &const_id.to_le_bytes()); // memory.size equivalent
        }
        Ok(())
    }

    /// Apply optimization passes to the bytecode
    fn optimize_bytecode(&mut self, bytecode: &mut BytecodeFile) -> RuntimeResult<()> {
        if !self.config.enable_optimizations {
            return Ok(());
        }

        self.statistics.optimizations_applied += 1;

        // 1. Dead code elimination - remove unreachable instructions
        self.eliminate_dead_code(bytecode)?;

        // 2. Constant folding - evaluate constant expressions at compile time
        self.fold_constants(bytecode)?;

        // 3. Instruction combining - merge adjacent compatible instructions
        self.combine_instructions(bytecode)?;

        // 4. Peephole optimizations - local instruction pattern optimizations
        self.apply_peephole_optimizations(bytecode)?;

        Ok(())
    }

    /// Eliminate dead code from bytecode
    fn eliminate_dead_code(&mut self, bytecode: &mut BytecodeFile) -> RuntimeResult<()> {
        // Track reachable instructions starting from entry points
        let mut reachable = std::collections::HashSet::new();
        let mut worklist = vec![0]; // Start from first instruction

        // Start analysis from entry point (beginning of code)
        if !bytecode.code.is_empty() {
            worklist.push(0);
        }

        // Perform reachability analysis
        while let Some(pc) = worklist.pop() {
            if reachable.contains(&pc) || pc >= bytecode.code.len() {
                continue;
            }
            reachable.insert(pc);

            // Analyze current instruction for control flow
            let opcode = bytecode.code[pc];

            // Add next instruction (for non-terminal instructions)
            match opcode {
                // Jump instructions
                0x0C | 0x0D => {
                    // br, br_if
                    if pc + 5 < bytecode.code.len() {
                        // Extract jump target from instruction operand
                        let target = u32::from_le_bytes([bytecode.code[pc + 1], bytecode.code[pc + 2], bytecode.code[pc + 3], bytecode.code[pc + 4]]) as usize;
                        if target < bytecode.code.len() {
                            worklist.push(target);
                        }
                    }
                    // For conditional branches, also add fall-through
                    if opcode == 0x0D && pc + 5 < bytecode.code.len() {
                        worklist.push(pc + 5);
                    }
                }
                // Call instructions
                0x10 => {
                    // call
                    if pc + 5 < bytecode.code.len() {
                        worklist.push(pc + 5); // Continue after call
                    }
                }
                // Return instruction - no next instruction
                0x0F => {} // return
                // Unreachable instruction - no next instruction
                0x00 => {} // unreachable
                // All other instructions continue to next
                _ => {
                    if pc + 1 < bytecode.code.len() {
                        worklist.push(pc + 1);
                    }
                }
            }
        }

        // Remove unreachable instructions
        let original_count = bytecode.code.len();
        let mut new_code = Vec::new();
        let mut instruction_map = std::collections::HashMap::new();

        for (old_pc, &instruction) in bytecode.code.iter().enumerate() {
            if reachable.contains(&old_pc) {
                instruction_map.insert(old_pc, new_code.len());
                new_code.push(instruction);
            }
        }

        // Update jump targets in remaining instructions
        for i in 0..new_code.len() {
            let opcode = new_code[i];
            match opcode {
                0x0C | 0x0D | 0x10 => {
                    // br, br_if, call
                    if i + 4 < new_code.len() {
                        let old_target = u32::from_le_bytes([new_code[i + 1], new_code[i + 2], new_code[i + 3], new_code[i + 4]]) as usize;

                        if let Some(&new_target) = instruction_map.get(&old_target) {
                            let new_target_bytes = (new_target as u32).to_le_bytes();
                            new_code[i + 1] = new_target_bytes[0];
                            new_code[i + 2] = new_target_bytes[1];
                            new_code[i + 3] = new_target_bytes[2];
                            new_code[i + 4] = new_target_bytes[3];
                        }
                    }
                }
                _ => {}
            }
        }

        bytecode.code = new_code;
        let eliminated_count = original_count - bytecode.code.len();
        self.statistics.dead_code_eliminated += eliminated_count as u64;

        Ok(())
    }

    /// Fold constants in bytecode
    fn fold_constants(&mut self, bytecode: &mut BytecodeFile) -> RuntimeResult<()> {
        // Look for patterns like: CONST a, CONST b, ADD -> CONST (a+b)
        let mut i = 0;
        let mut folded_count = 0;
        let mut new_code = Vec::new();

        while i < bytecode.code.len() {
            // Check for constant folding opportunities
            if i + 8 < bytecode.code.len() {
                let op1 = bytecode.code[i];
                let op2 = bytecode.code[i + 5];
                let op3 = bytecode.code[i + 10];

                // Pattern: i32.const a, i32.const b, i32.add -> i32.const (a+b)
                if op1 == 0x41 && op2 == 0x41 && op3 == 0x6A {
                    // i32.const, i32.const, i32.add
                    let val1 = i32::from_le_bytes([bytecode.code[i + 1], bytecode.code[i + 2], bytecode.code[i + 3], bytecode.code[i + 4]]);
                    let val2 = i32::from_le_bytes([bytecode.code[i + 6], bytecode.code[i + 7], bytecode.code[i + 8], bytecode.code[i + 9]]);

                    // Perform constant addition
                    if let Some(result) = val1.checked_add(val2) {
                        // Replace with single constant
                        new_code.push(0x41); // i32.const
                        new_code.extend_from_slice(&result.to_le_bytes());
                        i += 11; // Skip the folded instructions
                        folded_count += 1;
                        continue;
                    }
                }

                // Pattern: i32.const a, i32.const b, i32.mul -> i32.const (a*b)
                if op1 == 0x41 && op2 == 0x41 && op3 == 0x6C {
                    // i32.const, i32.const, i32.mul
                    let val1 = i32::from_le_bytes([bytecode.code[i + 1], bytecode.code[i + 2], bytecode.code[i + 3], bytecode.code[i + 4]]);
                    let val2 = i32::from_le_bytes([bytecode.code[i + 6], bytecode.code[i + 7], bytecode.code[i + 8], bytecode.code[i + 9]]);

                    // Perform constant multiplication
                    if let Some(result) = val1.checked_mul(val2) {
                        // Replace with single constant
                        new_code.push(0x41); // i32.const
                        new_code.extend_from_slice(&result.to_le_bytes());
                        i += 11; // Skip the folded instructions
                        folded_count += 1;
                        continue;
                    }
                }

                // Pattern: i32.const a, i32.const b, i32.sub -> i32.const (a-b)
                if op1 == 0x41 && op2 == 0x41 && op3 == 0x6B {
                    // i32.const, i32.const, i32.sub
                    let val1 = i32::from_le_bytes([bytecode.code[i + 1], bytecode.code[i + 2], bytecode.code[i + 3], bytecode.code[i + 4]]);
                    let val2 = i32::from_le_bytes([bytecode.code[i + 6], bytecode.code[i + 7], bytecode.code[i + 8], bytecode.code[i + 9]]);

                    // Perform constant subtraction
                    if let Some(result) = val1.checked_sub(val2) {
                        // Replace with single constant
                        new_code.push(0x41); // i32.const
                        new_code.extend_from_slice(&result.to_le_bytes());
                        i += 11; // Skip the folded instructions
                        folded_count += 1;
                        continue;
                    }
                }
            }

            // No folding opportunity, copy instruction as-is
            new_code.push(bytecode.code[i]);
            i += 1;
        }

        // Update bytecode with folded constants
        if folded_count > 0 {
            bytecode.code = new_code;
        }

        self.statistics.constants_folded += folded_count;
        Ok(())
    }

    /// Combine adjacent compatible instructions
    fn combine_instructions(&mut self, bytecode: &mut BytecodeFile) -> RuntimeResult<()> {
        // Look for patterns like multiple consecutive LOADs that can be combined
        let mut combined_count = 0;
        let mut new_code = Vec::new();
        let mut i = 0;

        while i < bytecode.code.len() {
            // Pattern: consecutive local.get instructions -> combined load
            if i + 9 < bytecode.code.len() {
                let op1 = bytecode.code[i];
                let op2 = bytecode.code[i + 5];

                // local.get x, local.get y -> load_locals x,y (custom combined instruction)
                if op1 == 0x20 && op2 == 0x20 {
                    // local.get, local.get
                    let local1 = u32::from_le_bytes([bytecode.code[i + 1], bytecode.code[i + 2], bytecode.code[i + 3], bytecode.code[i + 4]]);
                    let local2 = u32::from_le_bytes([bytecode.code[i + 6], bytecode.code[i + 7], bytecode.code[i + 8], bytecode.code[i + 9]]);

                    // Check if locals are consecutive
                    if local2 == local1 + 1 {
                        // Create combined load instruction (custom opcode 0xF0)
                        new_code.push(0xF0); // load_locals_consecutive
                        new_code.extend_from_slice(&local1.to_le_bytes());
                        new_code.extend_from_slice(&2u32.to_le_bytes()); // count
                        i += 10; // Skip both instructions
                        combined_count += 1;
                        continue;
                    }
                }
            }

            // Pattern: consecutive i32.const with same value -> single const + dup
            if i + 9 < bytecode.code.len() {
                let op1 = bytecode.code[i];
                let op2 = bytecode.code[i + 5];

                if op1 == 0x41 && op2 == 0x41 {
                    // i32.const, i32.const
                    let val1 = i32::from_le_bytes([bytecode.code[i + 1], bytecode.code[i + 2], bytecode.code[i + 3], bytecode.code[i + 4]]);
                    let val2 = i32::from_le_bytes([bytecode.code[i + 6], bytecode.code[i + 7], bytecode.code[i + 8], bytecode.code[i + 9]]);

                    // Same constant values -> const + dup
                    if val1 == val2 {
                        new_code.push(0x41); // i32.const
                        new_code.extend_from_slice(&val1.to_le_bytes());
                        new_code.push(0xF1); // dup (custom opcode)
                        i += 10; // Skip both instructions
                        combined_count += 1;
                        continue;
                    }
                }
            }

            // No combination opportunity, copy instruction as-is
            new_code.push(bytecode.code[i]);
            i += 1;
        }

        // Update bytecode with combined instructions
        if combined_count > 0 {
            bytecode.code = new_code;
        }

        self.statistics.instructions_combined += combined_count as u64;
        Ok(())
    }

    /// Apply peephole optimizations
    fn apply_peephole_optimizations(&mut self, bytecode: &mut BytecodeFile) -> RuntimeResult<()> {
        let mut optimized_count = 0;
        let mut new_code = Vec::new();
        let mut i = 0;

        while i < bytecode.code.len() {
            let mut skip_count = 0;

            // Pattern: local.get x, local.set x -> (remove both - redundant load/store)
            if i + 9 < bytecode.code.len() {
                let op1 = bytecode.code[i];
                let op2 = bytecode.code[i + 5];

                if op1 == 0x20 && op2 == 0x21 {
                    // local.get, local.set
                    let local1 = u32::from_le_bytes([bytecode.code[i + 1], bytecode.code[i + 2], bytecode.code[i + 3], bytecode.code[i + 4]]);
                    let local2 = u32::from_le_bytes([bytecode.code[i + 6], bytecode.code[i + 7], bytecode.code[i + 8], bytecode.code[i + 9]]);

                    // Same local variable -> redundant
                    if local1 == local2 {
                        skip_count = 10; // Skip both instructions
                        optimized_count += 1;
                    }
                }
            }

            // Pattern: i32.const 0, i32.add -> (remove both - adding zero)
            if skip_count == 0 && i + 5 < bytecode.code.len() {
                let op1 = bytecode.code[i];
                let op2 = bytecode.code[i + 5];

                if op1 == 0x41 && op2 == 0x6A {
                    // i32.const, i32.add
                    let val = i32::from_le_bytes([bytecode.code[i + 1], bytecode.code[i + 2], bytecode.code[i + 3], bytecode.code[i + 4]]);

                    // Adding zero is redundant
                    if val == 0 {
                        skip_count = 6; // Skip both instructions
                        optimized_count += 1;
                    }
                }
            }

            // Pattern: i32.const 1, i32.mul -> (remove both - multiplying by one)
            if skip_count == 0 && i + 5 < bytecode.code.len() {
                let op1 = bytecode.code[i];
                let op2 = bytecode.code[i + 5];

                if op1 == 0x41 && op2 == 0x6C {
                    // i32.const, i32.mul
                    let val = i32::from_le_bytes([bytecode.code[i + 1], bytecode.code[i + 2], bytecode.code[i + 3], bytecode.code[i + 4]]);

                    // Multiplying by one is redundant
                    if val == 1 {
                        skip_count = 6; // Skip both instructions
                        optimized_count += 1;
                    }
                }
            }

            // Pattern: br 0 (unconditional branch to next instruction) -> remove
            if skip_count == 0 && i + 4 < bytecode.code.len() {
                let op = bytecode.code[i];

                if op == 0x0C {
                    // br (unconditional branch)
                    let target = u32::from_le_bytes([bytecode.code[i + 1], bytecode.code[i + 2], bytecode.code[i + 3], bytecode.code[i + 4]]) as usize;

                    // Branch to next instruction is redundant
                    if target == i + 5 {
                        skip_count = 5; // Skip branch instruction
                        optimized_count += 1;
                    }
                }
            }

            // Pattern: drop, drop -> drop2 (custom optimization)
            if skip_count == 0 && i + 1 < bytecode.code.len() {
                let op1 = bytecode.code[i];
                let op2 = bytecode.code[i + 1];

                if op1 == 0x1A && op2 == 0x1A {
                    // drop, drop
                    new_code.push(0xF2); // drop2 (custom opcode)
                    skip_count = 2;
                    optimized_count += 1;
                }
            }

            if skip_count > 0 {
                i += skip_count;
            } else {
                // No optimization opportunity, copy instruction as-is
                new_code.push(bytecode.code[i]);
                i += 1;
            }
        }

        // Update bytecode with optimized instructions
        if optimized_count > 0 {
            bytecode.code = new_code;
        }

        self.statistics.peephole_optimizations += optimized_count;
        Ok(())
    }

    /// Get transpiler statistics
    pub fn get_statistics(&self) -> TranspilerStatistics {
        TranspilerStatistics {
            instructions_transpiled: 0,
            custom_opcodes_injected: 0,
            optimizations_applied: 0,
            dead_code_eliminated: 0,
            constants_folded: 0,
            instructions_combined: 0,
            peephole_optimizations: 0,
        }
    }
}

/// Transpiler statistics
#[derive(Debug, Clone, Default)]
pub struct TranspilerStatistics {
    /// Number of instructions transpiled
    pub instructions_transpiled: usize,
    /// Number of custom opcodes injected
    pub custom_opcodes_injected: usize,
    /// Number of optimizations applied
    pub optimizations_applied: usize,
    /// Number of dead code instructions eliminated
    pub dead_code_eliminated: u64,
    /// Number of constants folded
    pub constants_folded: u64,
    /// Number of instructions combined
    pub instructions_combined: u64,
    /// Number of peephole optimizations applied
    pub peephole_optimizations: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpiler_creation() {
        let config = TranspilerConfig::default();
        let transpiler = WasmTranspiler::new(config);
        assert_eq!(transpiler.config.target_architecture, VmArchitecture::Arch64);
        assert!(transpiler.config.enable_custom_opcodes);
    }

    #[test]
    fn test_opcode_mapping() {
        let mapping = OpcodeMapping::default();

        // Test standard mappings
        assert!(mapping.get_mapping("i32.add").is_some());
        assert!(mapping.get_mapping("f64.mul").is_some());
        assert!(mapping.get_mapping("local.get").is_some());

        // Test custom mappings
        assert!(mapping.get_mapping("dot.state.get").is_some());
        assert!(mapping.get_mapping("dot.crypto.hash").is_some());

        // Test non-existent mapping
        assert!(mapping.get_mapping("nonexistent.opcode").is_none());
    }

    #[test]
    fn test_custom_opcode_detection() {
        let mapping = OpcodeMapping::default();

        assert!(!mapping.is_custom_opcode("i32.add"));
        assert!(mapping.is_custom_opcode("dot.state.get"));
        assert!(mapping.is_custom_opcode("dot.crypto.hash"));
    }

    #[test]
    fn test_transpiler_config() {
        let mut config = TranspilerConfig::default();
        config.target_architecture = VmArchitecture::Arch128;
        config.enable_custom_opcodes = false;

        let transpiler = WasmTranspiler::new(config);
        assert_eq!(transpiler.config.target_architecture, VmArchitecture::Arch128);
        assert!(!transpiler.config.enable_custom_opcodes);
    }
}
