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

use crate::instruction::arithmetic::{
    Instruction, Operand, Operator, add_operands, divide_operands, modulus_operands,
    multiply_operands, subtract_operands,
};

/// Enum representing control flow instructions.
#[derive(Debug, PartialEq)]
pub enum ControlFlowInstruction {
    IfElse {
        condition: Condition,
        if_branch: Vec<Instruction>,
        else_branch: Vec<Instruction>,
    },
    ForLoop {
        initializer: Instruction,
        condition: Condition,
        updater: Instruction,
        body: Vec<Instruction>,
    },
    WhileLoop {
        condition: Condition,
        body: Vec<Instruction>,
    },
    DoWhileLoop {
        body: Vec<Instruction>,
        condition: Condition,
    },
    Jump {
        target: usize,
    },
}

/// Simple state structure for the virtual machine (VM).
#[derive(Debug, PartialEq)]
pub struct VM {
    pub register: Operand,
    pub stack: Vec<Operand>,
}

impl VM {
    pub fn new() -> Self {
        Self {
            register: Operand::Integer(0),
            stack: Vec::new(),
        }
    }
}

/// Conditional structure to be used in control flow instructions.
#[derive(Debug, PartialEq, Clone)]
pub struct Condition {
    pub left: Operand,
    pub operator: Operator,
    pub right: Operand,
}

// Helper: Convert an Operand to f64 for comparison.
fn operand_to_f64(op: &Operand) -> f64 {
    match op {
        Operand::Integer(i) => *i as f64,
        Operand::Float(f) => *f,
    }
}

/// This helper function is used if the condition in loops needs to be
/// dynamically evaluated with the value of a register in the VM.
fn evaluate_condition(current_value: i64, condition: &Condition) -> bool {
    let right = match condition.right {
        Operand::Integer(val) => val,
        _ => 0,
    };
    match condition.operator {
        Operator::LessThan => current_value < right,
        Operator::LessThanOrEqual => current_value <= right,
        Operator::GreaterThan => current_value > right,
        Operator::GreaterThanOrEqual => current_value >= right,
        Operator::Equal => current_value == right,
        Operator::NotEqual => current_value != right,
    }
}

/// Function that reflects the execution of simple arithmetic instructions on the virtual machine state.
fn execute_instruction(vm: &mut VM, instruction: &Instruction) -> Result<(), String> {
    match instruction {
        Instruction::Initialize => {
            vm.register = Operand::Integer(0);
            Ok(())
        }
        Instruction::Increment => {
            match vm.register {
                Operand::Integer(ref mut n) => *n += 1,
                Operand::Float(ref mut f) => *f += 1.0,
            }
            Ok(())
        }
        Instruction::Add => {
            if let Some(operand) = vm.stack.pop() {
                let result = add_operands(&vm.register, &operand);
                vm.register = result;
                Ok(())
            } else {
                Err("Stack underflow: Not enough operands for addition".to_string())
            }
        }
        Instruction::Subtract => {
            if let Some(operand) = vm.stack.pop() {
                let result = subtract_operands(&vm.register, &operand);
                vm.register = result;
                Ok(())
            } else {
                Err("Stack underflow: Not enough operands for subtraction".to_string())
            }
        }
        Instruction::Multiply => {
            if let Some(operand) = vm.stack.pop() {
                let result = multiply_operands(&vm.register, &operand);
                vm.register = result;
                Ok(())
            } else {
                Err("Stack underflow: Not enough operands for multiplication".to_string())
            }
        }
        Instruction::Divide => {
            if let Some(operand) = vm.stack.pop() {
                match divide_operands(&vm.register, &operand) {
                    Ok(result) => {
                        vm.register = result;
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                Err("Stack underflow: Not enough operands for division".to_string())
            }
        }
        Instruction::Modulus => {
            if let Some(operand) = vm.stack.pop() {
                match modulus_operands(&vm.register, &operand) {
                    Ok(result) => {
                        vm.register = result;
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                Err("Stack underflow: Not enough operands for modulus".to_string())
            }
        }
        Instruction::SomeInstruction => Ok(()),
    }
}

/// If-Else control flow: If the condition is true, instructions in the if_branch are executed,  
/// otherwise, instructions in the else_branch are executed.
pub fn execute_if_else(
    vm: &mut VM,
    condition: &Condition,
    if_branch: &Vec<Instruction>,
    else_branch: &Vec<Instruction>,
) -> Result<(), String> {
    // Condition is evaluated with the register value in the VM.
    let condition_result = match vm.register {
        Operand::Integer(n) => evaluate_condition(n, condition),
        _ => false,
    };
    if condition_result {
        for instr in if_branch {
            execute_instruction(vm, instr)?;
        }
    } else {
        for instr in else_branch {
            execute_instruction(vm, instr)?;
        }
    }
    Ok(())
}

/// For loop control flow:
/// - initializer: Initializes the VM before the loop.
/// - condition: Specifies the continuation condition for the loop (in this case, dynamically evaluated with the VM register value).
/// - updater: Updates the VM at the end of each loop iteration.
/// - body: Instructions within the loop body.
pub fn execute_for_loop(
    vm: &mut VM,
    initializer: &Instruction,
    condition: &Condition,
    updater: &Instruction,
    body: &Vec<Instruction>,
) -> Result<(), String> {
    // Set the initial value of the register
    execute_instruction(vm, initializer)?;

    let max_iterations = 1000;
    let mut iterations = 0;
    while {
        let counter = match vm.register {
            Operand::Integer(n) => n,
            _ => return Err("For loop condition expects integer in VM register".to_string()),
        };
        evaluate_condition(counter, condition) && iterations < max_iterations
    } {
        for instr in body {
            execute_instruction(vm, instr)?;
        }
        execute_instruction(vm, updater)?;
        iterations += 1;
    }
    Ok(())
}

/// While loop control flow: Instructions in the body are executed as long as the condition is true.
pub fn execute_while_loop(
    vm: &mut VM,
    condition: &Condition,
    body: &Vec<Instruction>,
) -> Result<(), String> {
    let max_iterations = 1000;
    let mut iterations = 0;
    while {
        let counter = match vm.register {
            Operand::Integer(n) => n,
            _ => return Err("While loop condition expects integer in VM register".to_string()),
        };
        evaluate_condition(counter, condition) && iterations < max_iterations
    } {
        for instr in body {
            execute_instruction(vm, instr)?;
        }
        iterations += 1;
    }
    Ok(())
}

/// Do-While loop control flow: The body is executed at least once, then the condition is checked.
pub fn execute_do_while_loop(
    vm: &mut VM,
    body: &Vec<Instruction>,
    condition: &Condition,
) -> Result<(), String> {
    let max_iterations = 1000;
    let mut iterations = 0;
    loop {
        for instr in body {
            execute_instruction(vm, instr)?;
        }
        iterations += 1;
        let counter = match vm.register {
            Operand::Integer(n) => n,
            _ => return Err("Do-while loop condition expects integer in VM register".to_string()),
        };
        if !evaluate_condition(counter, condition) || iterations >= max_iterations {
            break;
        }
    }
    Ok(())
}

/// Jump instruction: Updates the instruction pointer.
pub fn execute_jump(vm: &mut VM, target: usize) -> Result<(), String> {
    vm.register = Operand::Integer(target as i64);
    Ok(())
}

/// Function that executes the incoming control flow instruction appropriately.
pub fn execute_control_flow(
    vm: &mut VM,
    instruction: &ControlFlowInstruction,
) -> Result<(), String> {
    match instruction {
        ControlFlowInstruction::IfElse {
            condition,
            if_branch,
            else_branch,
        } => execute_if_else(vm, condition, if_branch, else_branch),
        ControlFlowInstruction::ForLoop {
            initializer,
            condition,
            updater,
            body,
        } => execute_for_loop(vm, initializer, condition, updater, body),
        ControlFlowInstruction::WhileLoop { condition, body } => {
            execute_while_loop(vm, condition, body)
        }
        ControlFlowInstruction::DoWhileLoop { body, condition } => {
            execute_do_while_loop(vm, body, condition)
        }
        ControlFlowInstruction::Jump { target } => execute_jump(vm, *target),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::arithmetic::{Instruction, Operand, Operator};

    #[test]
    fn test_if_else_instruction() {
        let mut vm = VM::new();
        // Initial value: 1
        vm.register = Operand::Integer(1);
        let condition = Condition {
            left: Operand::Integer(1), // Constant value; dynamic evaluation is done through VM.register.
            operator: Operator::GreaterThan,
            right: Operand::Integer(0),
        };
        // if_branch will increment the register with Increment; else_branch will reset it with Initialize.
        let if_branch = vec![Instruction::Increment];
        let else_branch = vec![Instruction::Initialize];
        let instruction = ControlFlowInstruction::IfElse {
            condition,
            if_branch,
            else_branch,
        };

        let result = execute_control_flow(&mut vm, &instruction);
        assert!(result.is_ok());
        // Since 1 > 0, the register should increment from 1 to 2.
        assert_eq!(vm.register, Operand::Integer(2));
    }

    #[test]
    fn test_for_loop_instruction() {
        let mut vm = VM::new();
        // For loop: The register starts at 0, and Increment is applied with the condition < 5.
        let initializer = Instruction::Initialize;
        let condition = Condition {
            left: Operand::Integer(0), // Initial value
            operator: Operator::LessThan,
            right: Operand::Integer(5),
        };
        let updater = Instruction::Increment;
        let body = vec![Instruction::SomeInstruction];
        let instruction = ControlFlowInstruction::ForLoop {
            initializer,
            condition,
            updater,
            body,
        };

        let result = execute_control_flow(&mut vm, &instruction);
        assert!(result.is_ok());
        // At the end of the loop, the register should be 5.
        assert_eq!(vm.register, Operand::Integer(5));
    }

    #[test]
    fn test_while_loop_instruction() {
        let mut vm = VM::new();
        // The initial value of the register is 5.
        vm.register = Operand::Integer(5);
        // Condition: register <= 7
        let condition = Condition {
            left: Operand::Integer(5), // Constant, but the evaluation is done through VM.register.
            operator: Operator::LessThanOrEqual,
            right: Operand::Integer(7),
        };
        // Body: Increments the register.
        let body = vec![Instruction::Increment];
        let instruction = ControlFlowInstruction::WhileLoop { condition, body };

        let result = execute_control_flow(&mut vm, &instruction);
        assert!(result.is_ok());
        // The loop runs until the register is 7, and then it should be 8.
        assert_eq!(vm.register, Operand::Integer(8));
    }

    #[test]
    fn test_do_while_loop_instruction() {
        let mut vm = VM::new();
        // The initial value of the register is 10.
        vm.register = Operand::Integer(10);
        // Condition: register < 12
        let condition = Condition {
            left: Operand::Integer(10),
            operator: Operator::LessThan,
            right: Operand::Integer(12),
        };
        // Body: Increments the register.
        let body = vec![Instruction::Increment];
        let instruction = ControlFlowInstruction::DoWhileLoop { body, condition };

        let result = execute_control_flow(&mut vm, &instruction);
        assert!(result.is_ok());
        // The loop runs at least once; starting from 10, it increments, and the final value should be 12.
        assert_eq!(vm.register, Operand::Integer(12));
    }

    #[test]
    fn test_jump_instruction() {
        let mut vm = VM::new();
        let target = 42;
        let instruction = ControlFlowInstruction::Jump { target };

        let result = execute_control_flow(&mut vm, &instruction);
        assert!(result.is_ok());
        // Jump should set the register to the target value (42).
        assert_eq!(vm.register, Operand::Integer(42));
    }
}
