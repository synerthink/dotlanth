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

#[derive(Debug, PartialEq)]
pub enum Instruction {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,
    SomeInstruction,
    Initialize,
    Increment,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Operand {
    Integer(i64),
    Float(f64),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Operator {
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

// Helper: Converts the operand to f64.
fn operand_to_f64(op: &Operand) -> f64 {
    match op {
        Operand::Integer(i) => *i as f64,
        Operand::Float(f) => *f,
    }
}

/// Adds two operands. If both operands are integers, the result is an integer.
/// Otherwise, the operands are converted to f64 and the result is a float.
pub fn add_operands(a: &Operand, b: &Operand) -> Operand {
    match (a, b) {
        (Operand::Integer(a_val), Operand::Integer(b_val)) => Operand::Integer(a_val + b_val),
        _ => Operand::Float(operand_to_f64(a) + operand_to_f64(b)),
    }
}

/// Subtracts the second operand from the first. If both operands are integers, the result is an integer.
/// Otherwise, the operands are converted to f64 and the result is a float.
pub fn subtract_operands(a: &Operand, b: &Operand) -> Operand {
    match (a, b) {
        (Operand::Integer(a_val), Operand::Integer(b_val)) => Operand::Integer(a_val - b_val),
        _ => Operand::Float(operand_to_f64(a) - operand_to_f64(b)),
    }
}

/// Multiplies two operands. If both operands are integers, the result is an integer.
/// Otherwise, the operands are converted to f64 and the result is a float.
pub fn multiply_operands(a: &Operand, b: &Operand) -> Operand {
    match (a, b) {
        (Operand::Integer(a_val), Operand::Integer(b_val)) => Operand::Integer(a_val * b_val),
        _ => Operand::Float(operand_to_f64(a) * operand_to_f64(b)),
    }
}

/// Divides the first operand by the second. If both operands are integers and the division is exact,
/// the result is an integer. Otherwise, the result is a float.
/// Returns an error if division by zero is attempted.
pub fn divide_operands(a: &Operand, b: &Operand) -> Result<Operand, String> {
    if operand_to_f64(b).abs() < 1e-9 {
        return Err("Division by zero".to_string());
    }
    match (a, b) {
        (Operand::Integer(a_val), Operand::Integer(b_val)) => {
            if a_val % b_val == 0 {
                Ok(Operand::Integer(a_val / b_val))
            } else {
                Ok(Operand::Float(*a_val as f64 / *b_val as f64))
            }
        }
        _ => Ok(Operand::Float(operand_to_f64(a) / operand_to_f64(b))),
    }
}

/// Computes the modulus of the first operand by the second.
/// This operation is only supported for integer operands.
/// Returns an error if division by zero or non-integer operands are provided.
pub fn modulus_operands(a: &Operand, b: &Operand) -> Result<Operand, String> {
    match (a, b) {
        (Operand::Integer(a_val), Operand::Integer(b_val)) => {
            if *b_val == 0 {
                Err("Modulus by zero".to_string())
            } else {
                Ok(Operand::Integer(a_val % b_val))
            }
        }
        _ => Err("Modulus operation only supports integers".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addition_with_integers() {
        let operand1 = Operand::Integer(5);
        let operand2 = Operand::Integer(3);
        let result = add_operands(&operand1, &operand2);
        assert_eq!(result, Operand::Integer(8));
    }

    #[test]
    fn test_addition_with_floats() {
        let operand1 = Operand::Float(5.5);
        let operand2 = Operand::Float(3.2);
        let result = add_operands(&operand1, &operand2);
        match result {
            Operand::Float(val) => assert!((val - 8.7).abs() < 1e-6),
            _ => panic!("Expected Float result"),
        }
    }

    #[test]
    fn test_addition_with_mixed_types_integer_first() {
        let operand1 = Operand::Integer(2);
        let operand2 = Operand::Float(3.5);
        let result = add_operands(&operand1, &operand2);
        match result {
            Operand::Float(val) => assert!((val - 5.5).abs() < 1e-6),
            _ => panic!("Expected Float result"),
        }
    }

    #[test]
    fn test_addition_with_mixed_types_float_first() {
        let operand1 = Operand::Float(2.5);
        let operand2 = Operand::Integer(3);
        let result = add_operands(&operand1, &operand2);
        match result {
            Operand::Float(val) => assert!((val - 5.5).abs() < 1e-6),
            _ => panic!("Expected Float result"),
        }
    }

    #[test]
    fn test_subtraction_with_integers() {
        let operand1 = Operand::Integer(10);
        let operand2 = Operand::Integer(4);
        let result = subtract_operands(&operand1, &operand2);
        assert_eq!(result, Operand::Integer(6));
    }

    #[test]
    fn test_subtraction_with_floats() {
        let operand1 = Operand::Float(10.5);
        let operand2 = Operand::Float(4.2);
        let result = subtract_operands(&operand1, &operand2);
        match result {
            Operand::Float(val) => assert!((val - 6.3).abs() < 1e-6),
            _ => panic!("Expected Float result"),
        }
    }

    #[test]
    fn test_subtraction_with_mixed_types_integer_first() {
        let operand1 = Operand::Integer(5);
        let operand2 = Operand::Float(2.5);
        let result = subtract_operands(&operand1, &operand2);
        match result {
            Operand::Float(val) => assert!((val - 2.5).abs() < 1e-6),
            _ => panic!("Expected Float result"),
        }
    }

    #[test]
    fn test_subtraction_with_mixed_types_float_first() {
        let operand1 = Operand::Float(5.5);
        let operand2 = Operand::Integer(2);
        let result = subtract_operands(&operand1, &operand2);
        match result {
            Operand::Float(val) => assert!((val - 3.5).abs() < 1e-6),
            _ => panic!("Expected Float result"),
        }
    }

    #[test]
    fn test_multiplication_with_integers() {
        let operand1 = Operand::Integer(7);
        let operand2 = Operand::Integer(6);
        let result = multiply_operands(&operand1, &operand2);
        assert_eq!(result, Operand::Integer(42));
    }

    #[test]
    fn test_multiplication_with_floats() {
        let operand1 = Operand::Float(7.5);
        let operand2 = Operand::Float(6.0);
        let result = multiply_operands(&operand1, &operand2);
        match result {
            Operand::Float(val) => assert!((val - 45.0).abs() < 1e-6),
            _ => panic!("Expected Float result"),
        }
    }

    #[test]
    fn test_multiplication_with_mixed_types_integer_first() {
        let operand1 = Operand::Integer(3);
        let operand2 = Operand::Float(2.5);
        let result = multiply_operands(&operand1, &operand2);
        match result {
            Operand::Float(val) => assert!((val - 7.5).abs() < 1e-6),
            _ => panic!("Expected Float result"),
        }
    }

    #[test]
    fn test_multiplication_with_mixed_types_float_first() {
        let operand1 = Operand::Float(3.0);
        let operand2 = Operand::Integer(2);
        let result = multiply_operands(&operand1, &operand2);
        match result {
            Operand::Float(val) => assert!((val - 6.0).abs() < 1e-6),
            _ => panic!("Expected Float result"),
        }
    }

    #[test]
    fn test_division_with_integers() {
        let operand1 = Operand::Integer(20);
        let operand2 = Operand::Integer(4);
        let result = divide_operands(&operand1, &operand2).unwrap();
        assert_eq!(result, Operand::Integer(5));
    }

    #[test]
    fn test_division_with_floats() {
        let operand1 = Operand::Float(20.0);
        let operand2 = Operand::Float(4.0);
        let result = divide_operands(&operand1, &operand2).unwrap();
        match result {
            Operand::Float(val) => assert!((val - 5.0).abs() < 1e-6),
            _ => panic!("Expected Float result"),
        }
    }

    #[test]
    fn test_division_with_mixed_types_integer_first() {
        let operand1 = Operand::Integer(10);
        let operand2 = Operand::Float(2.0);
        let result = divide_operands(&operand1, &operand2).unwrap();
        match result {
            Operand::Float(val) => assert!((val - 5.0).abs() < 1e-6),
            _ => panic!("Expected Float result"),
        }
    }

    #[test]
    fn test_division_with_mixed_types_float_first() {
        let operand1 = Operand::Float(10.0);
        let operand2 = Operand::Integer(2);
        let result = divide_operands(&operand1, &operand2).unwrap();
        match result {
            Operand::Float(val) => assert!((val - 5.0).abs() < 1e-6),
            _ => panic!("Expected Float result"),
        }
    }

    #[test]
    fn test_division_by_zero() {
        let operand1 = Operand::Integer(10);
        let operand2 = Operand::Integer(0);
        let result = divide_operands(&operand1, &operand2);
        assert!(result.is_err());
    }

    #[test]
    fn test_modulus_with_integers() {
        let operand1 = Operand::Integer(10);
        let operand2 = Operand::Integer(3);
        let result = modulus_operands(&operand1, &operand2).unwrap();
        assert_eq!(result, Operand::Integer(1));
    }

    #[test]
    fn test_modulus_with_floats() {
        let operand1 = Operand::Float(10.5);
        let operand2 = Operand::Float(3.2);
        let result = modulus_operands(&operand1, &operand2);
        assert!(result.is_err());
    }

    #[test]
    fn test_modulus_with_mixed_types_integer_first() {
        let operand1 = Operand::Integer(10);
        let operand2 = Operand::Float(3.2);
        let result = modulus_operands(&operand1, &operand2);
        assert!(result.is_err());
    }

    #[test]
    fn test_modulus_with_mixed_types_float_first() {
        let operand1 = Operand::Float(10.5);
        let operand2 = Operand::Integer(3);
        let result = modulus_operands(&operand1, &operand2);
        assert!(result.is_err());
    }

    #[test]
    fn test_modulus_by_zero() {
        let operand1 = Operand::Integer(10);
        let operand2 = Operand::Integer(0);
        let result = modulus_operands(&operand1, &operand2);
        assert!(result.is_err());
    }
}
