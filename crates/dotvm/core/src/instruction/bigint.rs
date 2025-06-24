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

//! BigInt instruction implementations for 128-bit+ architectures
//!
//! This module provides instruction implementations for arbitrary precision
//! integer operations that are available in 128-bit and higher architectures.

use super::instruction::{ExecutorInterface, Instruction};
use crate::opcode::architecture_opcodes::BigIntOpcode;
use crate::vm::errors::VMError;
use num_bigint::{BigInt, Sign};
use num_traits::sign::Signed;

/// BigInt arithmetic instruction
/// Performs arbitrary precision integer operations
pub struct BigIntInstruction {
    opcode: BigIntOpcode,
}

impl BigIntInstruction {
    /// Create a new BigInt instruction
    pub fn new(opcode: BigIntOpcode) -> Self {
        Self { opcode }
    }

    /// Convert a stack value (f64) to BigInt
    /// For now, we'll use a simple conversion, but in a real implementation
    /// you'd want a more sophisticated encoding scheme
    fn stack_to_bigint(value: f64) -> Result<BigInt, VMError> {
        if value.is_finite() {
            Ok(BigInt::from(value as i64))
        } else {
            Err(VMError::InvalidOperand("Cannot convert non-finite value to BigInt".to_string()))
        }
    }

    /// Convert BigInt back to stack value (f64)
    /// This is a lossy conversion for demonstration purposes
    fn bigint_to_stack(value: &BigInt) -> Result<f64, VMError> {
        // For very large numbers, this will lose precision
        // In a real implementation, you'd use a different stack representation
        match value.to_string().parse::<f64>() {
            Ok(f) => Ok(f),
            Err(_) => {
                // If the number is too large, return infinity with appropriate sign
                if value.sign() == Sign::Minus { Ok(f64::NEG_INFINITY) } else { Ok(f64::INFINITY) }
            }
        }
    }

    /// Execute binary operation (takes two operands, pushes one result)
    fn execute_binary_op<F>(&self, executor: &mut dyn ExecutorInterface, op: F) -> Result<(), VMError>
    where
        F: FnOnce(BigInt, BigInt) -> Result<BigInt, VMError>,
    {
        let b = Self::stack_to_bigint(executor.pop_operand()?)?;
        let a = Self::stack_to_bigint(executor.pop_operand()?)?;
        let result = op(a, b)?;
        let stack_value = Self::bigint_to_stack(&result)?;
        executor.push_operand(stack_value);
        Ok(())
    }

    /// Execute unary operation (takes one operand, pushes one result)
    fn execute_unary_op<F>(&self, executor: &mut dyn ExecutorInterface, op: F) -> Result<(), VMError>
    where
        F: FnOnce(BigInt) -> Result<BigInt, VMError>,
    {
        let a = Self::stack_to_bigint(executor.pop_operand()?)?;
        let result = op(a)?;
        let stack_value = Self::bigint_to_stack(&result)?;
        executor.push_operand(stack_value);
        Ok(())
    }

    /// Execute comparison operation (takes two operands, pushes comparison result)
    fn execute_comparison<F>(&self, executor: &mut dyn ExecutorInterface, op: F) -> Result<(), VMError>
    where
        F: FnOnce(&BigInt, &BigInt) -> f64,
    {
        let b = Self::stack_to_bigint(executor.pop_operand()?)?;
        let a = Self::stack_to_bigint(executor.pop_operand()?)?;
        let result = op(&a, &b);
        executor.push_operand(result);
        Ok(())
    }

    /// Execute predicate operation (takes one operand, pushes boolean result)
    fn execute_predicate<F>(&self, executor: &mut dyn ExecutorInterface, op: F) -> Result<(), VMError>
    where
        F: FnOnce(&BigInt) -> bool,
    {
        let a = Self::stack_to_bigint(executor.pop_operand()?)?;
        let result = if op(&a) { 1.0 } else { 0.0 };
        executor.push_operand(result);
        Ok(())
    }
}

impl Instruction for BigIntInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        match self.opcode {
            BigIntOpcode::Add => self.execute_binary_op(executor, |a, b| Ok(a + b)),
            BigIntOpcode::Subtract => self.execute_binary_op(executor, |a, b| Ok(a - b)),
            BigIntOpcode::Multiply => self.execute_binary_op(executor, |a, b| Ok(a * b)),
            BigIntOpcode::Divide => self.execute_binary_op(executor, |a, b| if b.sign() == Sign::NoSign { Err(VMError::DivisionByZero) } else { Ok(a / b) }),
            BigIntOpcode::Modulus => self.execute_binary_op(executor, |a, b| if b.sign() == Sign::NoSign { Err(VMError::DivisionByZero) } else { Ok(a % b) }),
            BigIntOpcode::Power => {
                self.execute_binary_op(executor, |a, b| {
                    // Convert b to u32 for exponentiation
                    if b.sign() == Sign::Minus {
                        return Err(VMError::InvalidOperand("Negative exponent not supported".to_string()));
                    }

                    // For very large exponents, this could take forever or use too much memory
                    // In a real implementation, you'd want to limit the exponent size
                    match b.to_string().parse::<u32>() {
                        Ok(exp) if exp <= 10000 => {
                            // Reasonable limit
                            Ok(a.pow(exp))
                        }
                        _ => Err(VMError::InvalidOperand("Exponent too large".to_string())),
                    }
                })
            }
            BigIntOpcode::SquareRoot => {
                self.execute_unary_op(executor, |a| {
                    if a.sign() == Sign::Minus {
                        Err(VMError::InvalidOperand("Square root of negative number".to_string()))
                    } else {
                        // For positive numbers, convert to BigUint for sqrt operation
                        let (sign, magnitude) = a.into_parts();
                        if sign == Sign::Minus {
                            return Err(VMError::InvalidOperand("Square root of negative number".to_string()));
                        }

                        // Use integer square root
                        use num_integer::Roots;
                        let sqrt_result = magnitude.sqrt();
                        Ok(BigInt::from(sqrt_result))
                    }
                })
            }
            BigIntOpcode::Gcd => self.execute_binary_op(executor, |a, b| {
                use num_integer::Integer;
                Ok(a.gcd(&b))
            }),
            BigIntOpcode::Lcm => self.execute_binary_op(executor, |a, b| {
                use num_integer::Integer;
                Ok(a.lcm(&b))
            }),
            BigIntOpcode::FromInt => {
                // Convert regular integer (already on stack as f64) to BigInt representation
                // This is essentially a no-op in our current representation, but marks the intent
                let value = executor.pop_operand()?;
                let bigint = Self::stack_to_bigint(value)?;
                let result = Self::bigint_to_stack(&bigint)?;
                executor.push_operand(result);
                Ok(())
            }
            BigIntOpcode::ToInt => {
                // Convert BigInt to regular integer with overflow check
                let bigint = Self::stack_to_bigint(executor.pop_operand()?)?;

                // Check if it fits in i64 range
                if bigint > BigInt::from(i64::MAX) || bigint < BigInt::from(i64::MIN) {
                    return Err(VMError::IntegerOverflow);
                }

                // Safe to convert
                let result = bigint
                    .to_string()
                    .parse::<i64>()
                    .map_err(|_| VMError::InvalidOperand("Failed to convert BigInt to integer".to_string()))?;
                executor.push_operand(result as f64);
                Ok(())
            }
            BigIntOpcode::Compare => self.execute_comparison(executor, |a, b| {
                use std::cmp::Ordering;
                match a.cmp(b) {
                    Ordering::Less => -1.0,
                    Ordering::Equal => 0.0,
                    Ordering::Greater => 1.0,
                }
            }),
            BigIntOpcode::IsZero => self.execute_predicate(executor, |a| a.sign() == Sign::NoSign),
            BigIntOpcode::IsNegative => self.execute_predicate(executor, |a| a.sign() == Sign::Minus),
            BigIntOpcode::Abs => self.execute_unary_op(executor, |a| Ok(a.abs())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::multi_arch_executor::{Executor128, ExecutorFactory};

    fn create_test_executor() -> Executor128 {
        ExecutorFactory::create_128bit_executor().expect("Failed to create executor")
    }

    #[test]
    fn test_bigint_add() {
        let mut executor = create_test_executor();
        let instruction = BigIntInstruction::new(BigIntOpcode::Add);

        // Test: 123 + 456 = 579
        executor.push_operand(123.0);
        executor.push_operand(456.0);

        instruction.execute(&mut executor).expect("Addition failed");

        let result = executor.pop_operand().expect("No result on stack");
        assert_eq!(result, 579.0);
    }

    #[test]
    fn test_bigint_subtract() {
        let mut executor = create_test_executor();
        let instruction = BigIntInstruction::new(BigIntOpcode::Subtract);

        // Test: 1000 - 234 = 766
        executor.push_operand(1000.0);
        executor.push_operand(234.0);

        instruction.execute(&mut executor).expect("Subtraction failed");

        let result = executor.pop_operand().expect("No result on stack");
        assert_eq!(result, 766.0);
    }

    #[test]
    fn test_bigint_multiply() {
        let mut executor = create_test_executor();
        let instruction = BigIntInstruction::new(BigIntOpcode::Multiply);

        // Test: 12 * 34 = 408
        executor.push_operand(12.0);
        executor.push_operand(34.0);

        instruction.execute(&mut executor).expect("Multiplication failed");

        let result = executor.pop_operand().expect("No result on stack");
        assert_eq!(result, 408.0);
    }

    #[test]
    fn test_bigint_divide() {
        let mut executor = create_test_executor();
        let instruction = BigIntInstruction::new(BigIntOpcode::Divide);

        // Test: 100 / 4 = 25
        executor.push_operand(100.0);
        executor.push_operand(4.0);

        instruction.execute(&mut executor).expect("Division failed");

        let result = executor.pop_operand().expect("No result on stack");
        assert_eq!(result, 25.0);
    }

    #[test]
    fn test_bigint_divide_by_zero() {
        let mut executor = create_test_executor();
        let instruction = BigIntInstruction::new(BigIntOpcode::Divide);

        executor.push_operand(100.0);
        executor.push_operand(0.0);

        let result = instruction.execute(&mut executor);
        assert!(matches!(result, Err(VMError::DivisionByZero)));
    }

    #[test]
    fn test_bigint_power() {
        let mut executor = create_test_executor();
        let instruction = BigIntInstruction::new(BigIntOpcode::Power);

        // Test: 2^10 = 1024
        executor.push_operand(2.0);
        executor.push_operand(10.0);

        instruction.execute(&mut executor).expect("Power operation failed");

        let result = executor.pop_operand().expect("No result on stack");
        assert_eq!(result, 1024.0);
    }

    #[test]
    fn test_bigint_compare() {
        let mut executor = create_test_executor();
        let instruction = BigIntInstruction::new(BigIntOpcode::Compare);

        // Test: 100 compared to 50 should return 1 (greater)
        executor.push_operand(100.0);
        executor.push_operand(50.0);

        instruction.execute(&mut executor).expect("Comparison failed");

        let result = executor.pop_operand().expect("No result on stack");
        assert_eq!(result, 1.0);
    }

    #[test]
    fn test_bigint_is_zero() {
        let mut executor = create_test_executor();
        let instruction = BigIntInstruction::new(BigIntOpcode::IsZero);

        // Test: 0 should return true (1.0)
        executor.push_operand(0.0);

        instruction.execute(&mut executor).expect("IsZero failed");

        let result = executor.pop_operand().expect("No result on stack");
        assert_eq!(result, 1.0);

        // Test: 5 should return false (0.0)
        executor.push_operand(5.0);
        instruction.execute(&mut executor).expect("IsZero failed");

        let result = executor.pop_operand().expect("No result on stack");
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_bigint_abs() {
        let mut executor = create_test_executor();
        let instruction = BigIntInstruction::new(BigIntOpcode::Abs);

        // Test: abs(-42) = 42
        executor.push_operand(-42.0);

        instruction.execute(&mut executor).expect("Abs failed");

        let result = executor.pop_operand().expect("No result on stack");
        assert_eq!(result, 42.0);
    }
}
