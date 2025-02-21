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
    // ... other instructions goes here
}

#[derive(Debug, PartialEq)]
pub enum Operand {
    Integer(i64),
    Float(f64),
}

// TODO: Implement functions for each arithmetic operation
pub fn add_operands(a: &Operand, b: &Operand) -> Operand {
    // Implementation goes here
    todo!()
}

pub fn subtract_operands(a: &Operand, b: &Operand) -> Operand {
    // Implementation goes here
    todo!()
}

pub fn multiply_operands(a: &Operand, b: &Operand) -> Operand {
    // Implementation goes here
    todo!()
}

pub fn divide_operands(a: &Operand, b: &Operand) -> Result<Operand, String> {
    // Implementation goes here
    todo!()
}

pub fn modulus_operands(a: &Operand, b: &Operand) -> Result<Operand, String> {
    // Implementation goes here
    todo!()
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
        // Depending on implementation, might not support modulus with floats.
        assert!(result.is_err());
    }

    #[test]
    fn test_modulus_by_zero() {
        let operand1 = Operand::Integer(10);
        let operand2 = Operand::Integer(0);
        let result = modulus_operands(&operand1, &operand2);
        assert!(result.is_err());
    }

    // TODO: Implement additional tests for mixed types if applicable
}
