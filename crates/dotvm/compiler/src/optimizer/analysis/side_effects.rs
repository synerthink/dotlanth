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

//! Side effect analysis for optimization

use crate::transpiler::types::{TranspiledFunction, TranspiledInstruction};
use std::collections::{HashMap, HashSet};

/// Types of side effects
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SideEffect {
    /// Memory write
    MemoryWrite,
    /// Memory read (for volatile memory)
    VolatileMemoryRead,
    /// Global variable modification
    GlobalWrite,
    /// Function call (may have unknown side effects)
    FunctionCall(String),
    /// System call
    SystemCall,
    /// Exception throwing
    Exception,
    /// Control flow modification
    ControlFlow,
}

/// Side effect information for instructions
#[derive(Debug, Clone)]
pub struct SideEffectInfo {
    /// Instructions that have side effects
    pub side_effect_instructions: HashMap<usize, HashSet<SideEffect>>,
    /// Instructions that are pure (no side effects)
    pub pure_instructions: HashSet<usize>,
    /// Instructions that only read memory
    pub read_only_instructions: HashSet<usize>,
    /// Instructions that can be eliminated if result is unused
    pub eliminable_instructions: HashSet<usize>,
}

/// Side effect analyzer
pub struct SideEffectAnalyzer {
    info: SideEffectInfo,
}

impl SideEffectAnalyzer {
    /// Create a new side effect analyzer
    pub fn new() -> Self {
        Self {
            info: SideEffectInfo {
                side_effect_instructions: HashMap::new(),
                pure_instructions: HashSet::new(),
                read_only_instructions: HashSet::new(),
                eliminable_instructions: HashSet::new(),
            },
        }
    }

    /// Analyze side effects in a function
    pub fn analyze(&mut self, function: &TranspiledFunction) -> &SideEffectInfo {
        self.info = SideEffectInfo {
            side_effect_instructions: HashMap::new(),
            pure_instructions: HashSet::new(),
            read_only_instructions: HashSet::new(),
            eliminable_instructions: HashSet::new(),
        };

        for (index, instruction) in function.instructions.iter().enumerate() {
            self.analyze_instruction(index, instruction);
        }

        &self.info
    }

    /// Analyze a single instruction for side effects
    fn analyze_instruction(&mut self, index: usize, instruction: &TranspiledInstruction) {
        let side_effects = self.get_instruction_side_effects(instruction);

        if side_effects.is_empty() {
            // Pure instruction
            self.info.pure_instructions.insert(index);
            self.info.eliminable_instructions.insert(index);
        } else {
            // Has side effects
            self.info.side_effect_instructions.insert(index, side_effects.clone());

            // Check if it's read-only
            if side_effects.len() == 1 && side_effects.contains(&SideEffect::VolatileMemoryRead) {
                self.info.read_only_instructions.insert(index);
            }

            // Some side-effect instructions can still be eliminated under certain conditions
            if self.is_conditionally_eliminable(instruction, &side_effects) {
                self.info.eliminable_instructions.insert(index);
            }
        }
    }

    /// Get side effects for a specific instruction
    fn get_instruction_side_effects(&self, instruction: &TranspiledInstruction) -> HashSet<SideEffect> {
        let mut effects = HashSet::new();

        match instruction.opcode.as_str() {
            // Pure arithmetic operations
            "ADD" | "SUB" | "MUL" | "DIV" | "MOD" | "I32_ADD" | "I32_SUB" | "I32_MUL" | "I32_DIV_S" | "I32_DIV_U" | "F32_ADD" | "F32_SUB" | "F32_MUL" | "F32_DIV" => {
                // No side effects
            }

            // Memory operations
            "LOAD" | "I32_LOAD" | "I64_LOAD" | "F32_LOAD" | "F64_LOAD" => {
                effects.insert(SideEffect::VolatileMemoryRead);
            }
            "STORE" | "I32_STORE" | "I64_STORE" | "F32_STORE" | "F64_STORE" => {
                effects.insert(SideEffect::MemoryWrite);
            }

            // Local variable operations (usually no side effects)
            "GET_LOCAL" | "SET_LOCAL" | "TEE_LOCAL" => {
                // No side effects for local variables
            }

            // Global variable operations
            "GET_GLOBAL" => {
                // Reading globals might have side effects in some cases
                effects.insert(SideEffect::VolatileMemoryRead);
            }
            "SET_GLOBAL" => {
                effects.insert(SideEffect::GlobalWrite);
            }

            // Control flow
            "JUMP" | "JUMP_IF" | "BR" | "BR_IF" | "BR_TABLE" | "RETURN" => {
                effects.insert(SideEffect::ControlFlow);
            }

            // Function calls
            "CALL" | "CALL_INDIRECT" => {
                let func_name = instruction
                    .operands
                    .first()
                    .and_then(|op| match op {
                        crate::transpiler::types::instruction::Operand::Label(name) => Some(name.clone()),
                        _ => Some("unknown".to_string()),
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                effects.insert(SideEffect::FunctionCall(func_name));
                // Assume function calls may have any side effect
                effects.insert(SideEffect::MemoryWrite);
                effects.insert(SideEffect::GlobalWrite);
            }

            // System calls
            "SYSCALL" => {
                effects.insert(SideEffect::SystemCall);
                effects.insert(SideEffect::MemoryWrite);
                effects.insert(SideEffect::GlobalWrite);
            }

            // Exception handling
            "THROW" | "UNREACHABLE" => {
                effects.insert(SideEffect::Exception);
                effects.insert(SideEffect::ControlFlow);
            }

            // Constants and other pure operations
            "CONST" | "CONST_I32" | "CONST_I64" | "CONST_F32" | "CONST_F64" | "NOP" => {
                // No side effects
            }

            // Add more instruction types as needed
            _ => {
                // Conservative: assume unknown instructions have side effects
                effects.insert(SideEffect::MemoryWrite);
            }
        }

        effects
    }

    /// Check if an instruction with side effects can be eliminated under certain conditions
    fn is_conditionally_eliminable(&self, instruction: &TranspiledInstruction, side_effects: &HashSet<SideEffect>) -> bool {
        match instruction.opcode.as_str() {
            // Pure function calls (if we know they're pure)
            "CALL" | "CALL_INDIRECT" => {
                let func_name = instruction
                    .operands
                    .first()
                    .and_then(|op| match op {
                        crate::transpiler::types::instruction::Operand::Label(name) => Some(name.as_str()),
                        _ => None,
                    })
                    .unwrap_or("unknown");
                self.is_pure_function(func_name)
            }

            // Read-only operations can sometimes be eliminated
            _ if side_effects.len() == 1 && side_effects.contains(&SideEffect::VolatileMemoryRead) => {
                // Can be eliminated if result is unused and memory is not volatile
                true
            }

            _ => false,
        }
    }

    /// Check if a function is known to be pure
    fn is_pure_function(&self, func_name: &str) -> bool {
        // List of known pure functions
        matches!(func_name, "sqrt" | "sin" | "cos" | "tan" | "log" | "exp" | "abs" | "floor" | "ceil" | "round" | "min" | "max")
    }

    /// Get side effect information
    pub fn info(&self) -> &SideEffectInfo {
        &self.info
    }

    /// Check if an instruction has side effects
    pub fn has_side_effects(&self, instruction_index: usize) -> bool {
        self.info.side_effect_instructions.contains_key(&instruction_index)
    }

    /// Check if an instruction is pure
    pub fn is_pure(&self, instruction_index: usize) -> bool {
        self.info.pure_instructions.contains(&instruction_index)
    }

    /// Check if an instruction can be eliminated
    pub fn is_eliminable(&self, instruction_index: usize) -> bool {
        self.info.eliminable_instructions.contains(&instruction_index)
    }

    /// Get specific side effects for an instruction
    pub fn get_side_effects(&self, instruction_index: usize) -> Option<&HashSet<SideEffect>> {
        self.info.side_effect_instructions.get(&instruction_index)
    }

    /// Check if two instructions can be reordered
    pub fn can_reorder(&self, inst1_index: usize, inst2_index: usize) -> bool {
        let inst1_effects = self.info.side_effect_instructions.get(&inst1_index);
        let inst2_effects = self.info.side_effect_instructions.get(&inst2_index);

        match (inst1_effects, inst2_effects) {
            (None, None) => true, // Both pure
            (None, Some(_)) | (Some(_), None) => {
                // One pure, one with side effects - check for conflicts
                true // Simplified - would need more detailed analysis
            }
            (Some(effects1), Some(effects2)) => {
                // Both have side effects - check for conflicts
                !self.effects_conflict(effects1, effects2)
            }
        }
    }

    /// Check if two sets of side effects conflict
    fn effects_conflict(&self, effects1: &HashSet<SideEffect>, effects2: &HashSet<SideEffect>) -> bool {
        // Memory writes conflict with any memory operation
        if (effects1.contains(&SideEffect::MemoryWrite) && (effects2.contains(&SideEffect::MemoryWrite) || effects2.contains(&SideEffect::VolatileMemoryRead)))
            || (effects2.contains(&SideEffect::MemoryWrite) && (effects1.contains(&SideEffect::MemoryWrite) || effects1.contains(&SideEffect::VolatileMemoryRead)))
        {
            return true;
        }

        // Global writes conflict with global operations
        if effects1.contains(&SideEffect::GlobalWrite) && effects2.contains(&SideEffect::GlobalWrite) {
            return true;
        }

        // Control flow changes conflict with everything
        if effects1.contains(&SideEffect::ControlFlow) || effects2.contains(&SideEffect::ControlFlow) {
            return true;
        }

        // Function calls and system calls are conservative
        if effects1.iter().any(|e| matches!(e, SideEffect::FunctionCall(_) | SideEffect::SystemCall)) || effects2.iter().any(|e| matches!(e, SideEffect::FunctionCall(_) | SideEffect::SystemCall)) {
            return true;
        }

        false
    }
}
