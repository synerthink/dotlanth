# DOTVM-12: Core Instruction Set Implementation Plan

## Overview

This document outlines the Test-Driven Development (TDD) approach for implementing the **Control Flow Instructions** in the DOTVM virtual machine. The goal is to develop robust and maintainable control flow mechanisms that manage the execution sequence within the VM, ensuring optimized performance and comprehensive testing for handling edge cases.

## Objectives

1. **Conditional Branching**
   - Implement `if-else` structures to allow decision-making based on runtime conditions.
   
2. **Loops**
   - Support `for`, `while`, and `do-while` loops to enable repetitive execution of code blocks.
   
3. **Jump Instructions**
   - Implement jump operations to alter the execution flow unconditionally.

## File Structure

The implementation will involve creating and modifying the following files within the project:

### 1. Arithmetic Instructions

- **File:** `crates/dotvm/core/src/instruction/arithmetic.rs`

  - **Enums:**
    - `Instruction`
    - `Operand`
    - `Operator`
  
  - **Functions:**
    - `add_operands`
    - `subtract_operands`
    - `multiply_operands`
    - `divide_operands`
    - `modulus_operands`
  
  - **Unit Tests:** Comprehensive tests covering integer operations, float operations, mixed-type operations, and edge cases.

### 2. Control Flow Instructions

- **File:** `crates/dotvm/core/src/instruction/control_flow.rs`

  - **Enums:**
    - `ControlFlowInstruction`
    - `Condition`
  
  - **Functions:**
    - `execute_control_flow`
  
  - **Unit Tests:** Comprehensive tests covering each control flow variant (`IfElse`, `ForLoop`, `WhileLoop`, `DoWhileLoop`, `Jump`) and their execution scenarios.

### 3. Documentation

- **File:** `docs/src/DOTVM-12_Core_Instruction_Set_Implementation_Plan.md`

  - **Content:** Detailed plan and documentation for implementing control flow instructions, outlining the file structures, data structures, unit tests, and step-by-step implementation guidance.

## Implementation Steps

Following the TDD methodology, each feature will be developed by first writing the unit tests, then implementing the corresponding functionality to pass those tests.

### Step 1: Implement Arithmetic Operations

1. **Add Operands**
   - **Function:** `add_operands`
   - **Description:** Adds two operands, supporting both integers and floats, including mixed-type operations.
   - **Placeholder:** `todo!("Implement addition of operands")`
   - **Unit Tests:** 
     - Test addition with integers.
     - Test addition with floats.
     - Test addition with mixed types (integer first).
     - Test addition with mixed types (float first).

2. **Subtract Operands**
   - **Function:** `subtract_operands`
   - **Description:** Subtracts the second operand from the first, supporting both integers and floats, including mixed-type operations.
   - **Placeholder:** `todo!("Implement subtraction of operands")`
   - **Unit Tests:** 
     - Test subtraction with integers.
     - Test subtraction with floats.
     - Test subtraction with mixed types (integer first).
     - Test subtraction with mixed types (float first).

3. **Multiply Operands**
   - **Function:** `multiply_operands`
   - **Description:** Multiplies two operands, supporting both integers and floats, including mixed-type operations.
   - **Placeholder:** `todo!("Implement multiplication of operands")`
   - **Unit Tests:** 
     - Test multiplication with integers.
     - Test multiplication with floats.
     - Test multiplication with mixed types (integer first).
     - Test multiplication with mixed types (float first).

4. **Divide Operands**
   - **Function:** `divide_operands`
   - **Description:** Divides the first operand by the second, handling division by zero and supporting both integers and floats, including mixed-type operations.
   - **Placeholder:** `todo!("Implement division of operands with error handling for division by zero")`
   - **Unit Tests:** 
     - Test division with integers.
     - Test division with floats.
     - Test division with mixed types (integer first).
     - Test division with mixed types (float first).
     - Test division by zero.

5. **Modulus Operands**
   - **Function:** `modulus_operands`
   - **Description:** Computes the modulus of the first operand by the second, handling unsupported types (floats) and division by zero.
   - **Placeholder:** `todo!("Implement modulus of operands with error handling for unsupported types and division by zero")`
   - **Unit Tests:** 
     - Test modulus with integers.
     - Test modulus with floats.
     - Test modulus with mixed types (integer first).
     - Test modulus with mixed types (float first).
     - Test modulus by zero.

### Step 2: Implement Control Flow Instructions

1. **If-Else Instruction**
   - **Enum Variant:** `ControlFlowInstruction::IfElse`
   - **Description:** Implements conditional branching based on runtime conditions.
   - **Function:** `execute_control_flow`
   - **Placeholder:** `todo!("Implement execution logic for IfElse control flow")`
   - **Unit Tests:** 
     - Test If-Else with condition true.
     - Test If-Else with condition false.

2. **For Loop Instruction**
   - **Enum Variant:** `ControlFlowInstruction::ForLoop`
   - **Description:** Implements `for` loops with initializer, condition, updater, and body.
   - **Function:** `execute_control_flow`
   - **Placeholder:** `todo!("Implement execution logic for ForLoop control flow")`
   - **Unit Tests:** 
     - Test For Loop iterations.
     - Test For Loop with different loop bodies.

3. **While Loop Instruction**
   - **Enum Variant:** `ControlFlowInstruction::WhileLoop`
   - **Description:** Implements `while` loops that continue as long as the condition is true.
   - **Function:** `execute_control_flow`
   - **Placeholder:** `todo!("Implement execution logic for WhileLoop control flow")`
   - **Unit Tests:** 
     - Test While Loop with condition true.
     - Test While Loop with condition false.

4. **Do-While Loop Instruction**
   - **Enum Variant:** `ControlFlowInstruction::DoWhileLoop`
   - **Description:** Implements `do-while` loops that execute the body at least once.
   - **Function:** `execute_control_flow`
   - **Placeholder:** `todo!("Implement execution logic for DoWhileLoop control flow")`
   - **Unit Tests:** 
     - Test Do-While Loop executes once.
     - Test Do-While Loop with condition true and false.

5. **Jump Instruction**
   - **Enum Variant:** `ControlFlowInstruction::Jump`
   - **Description:** Implements unconditional jump operations to alter the execution flow.
   - **Function:** `execute_control_flow`
   - **Placeholder:** `todo!("Implement execution logic for Jump control flow")`
   - **Unit Tests:** 
     - Test Jump to a valid instruction index.
     - Test Jump to an invalid instruction index (error handling).

## Documentation

All control flow instructions and their corresponding execution logic are documented in this file. The implementation follows the TDD approach, ensuring that each functionality is thoroughly tested before actual implementation.

## Next Steps

1. **Implement Arithmetic Functions:**
   - Start by implementing each arithmetic function to pass the corresponding unit tests.
   
2. **Implement Control Flow Execution:**
   - Develop the `execute_control_flow` function to handle each `ControlFlowInstruction` variant.
   
3. **Run and Validate Tests:**
   - Execute `cargo test` to ensure all unit tests pass upon implementing each function.
   
4. **Optimize and Refactor:**
   - After passing all tests, review the code for optimizations and maintainability improvements.

5. **Update Documentation:**
   - Continuously update this documentation as new features are implemented and tested.

## Conclusion

Following this TDD approach ensures that the control flow mechanisms within the DOTVM virtual machine are reliable, maintainable, and optimized for performance. Comprehensive testing guarantees that edge cases are handled gracefully, contributing to the overall robustness of the system.
