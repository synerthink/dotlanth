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

//! Operand Stack Management
//!
//! This module provides the operand stack implementation for the DotVM.
//! The stack is used to store intermediate values during bytecode execution.

use crate::bytecode::ConstantValue;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Maximum stack size to prevent stack overflow
pub const MAX_STACK_SIZE: usize = 10000;

/// Stack value that can be stored on the operand stack
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StackValue {
    /// 64-bit signed integer
    Int64(i64),
    /// 64-bit floating point
    Float64(f64),
    /// UTF-8 string
    String(String),
    /// Boolean value
    Bool(bool),
    /// Null value
    Null,
    /// JSON value for database operations
    Json(serde_json::Value),
    /// Document ID for database operations
    DocumentId(String),
    /// Collection name for database operations
    Collection(String),
}

impl StackValue {
    /// Convert a constant value to a stack value
    pub fn from_constant(constant: &ConstantValue) -> Self {
        match constant {
            ConstantValue::Int64(v) => StackValue::Int64(*v),
            ConstantValue::Float64(v) => StackValue::Float64(*v),
            ConstantValue::String(v) => StackValue::String(v.clone()),
            ConstantValue::Bool(v) => StackValue::Bool(*v),
            ConstantValue::Null => StackValue::Null,
            ConstantValue::Json(v) => StackValue::Json(v.clone()),
        }
    }

    /// Convert to a constant value
    pub fn to_constant(&self) -> ConstantValue {
        match self {
            StackValue::Int64(v) => ConstantValue::Int64(*v),
            StackValue::Float64(v) => ConstantValue::Float64(*v),
            StackValue::String(v) => ConstantValue::String(v.clone()),
            StackValue::Bool(v) => ConstantValue::Bool(*v),
            StackValue::Null => ConstantValue::Null,
            StackValue::Json(v) => ConstantValue::Json(v.clone()),
            StackValue::DocumentId(v) => ConstantValue::String(v.clone()),
            StackValue::Collection(v) => ConstantValue::String(v.clone()),
        }
    }

    /// Get the type name of this value
    pub fn type_name(&self) -> &'static str {
        match self {
            StackValue::Int64(_) => "int64",
            StackValue::Float64(_) => "float64",
            StackValue::String(_) => "string",
            StackValue::Bool(_) => "bool",
            StackValue::Null => "null",
            StackValue::Json(_) => "json",
            StackValue::DocumentId(_) => "document_id",
            StackValue::Collection(_) => "collection",
        }
    }

    /// Check if this value is truthy (for conditional operations)
    pub fn is_truthy(&self) -> bool {
        match self {
            StackValue::Bool(b) => *b,
            StackValue::Int64(i) => *i != 0,
            StackValue::Float64(f) => *f != 0.0,
            StackValue::String(s) => !s.is_empty(),
            StackValue::Null => false,
            StackValue::Json(v) => !v.is_null(),
            StackValue::DocumentId(s) => !s.is_empty(),
            StackValue::Collection(s) => !s.is_empty(),
        }
    }

    /// Convert to JSON value for database operations
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            StackValue::Int64(v) => serde_json::Value::Number((*v).into()),
            StackValue::Float64(v) => serde_json::Value::Number(serde_json::Number::from_f64(*v).unwrap_or_else(|| 0.into())),
            StackValue::String(v) => serde_json::Value::String(v.clone()),
            StackValue::Bool(v) => serde_json::Value::Bool(*v),
            StackValue::Null => serde_json::Value::Null,
            StackValue::Json(v) => v.clone(),
            StackValue::DocumentId(v) => serde_json::Value::String(v.clone()),
            StackValue::Collection(v) => serde_json::Value::String(v.clone()),
        }
    }

    /// Try to convert to string
    pub fn as_string(&self) -> Option<String> {
        match self {
            StackValue::String(s) => Some(s.clone()),
            StackValue::DocumentId(s) => Some(s.clone()),
            StackValue::Collection(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Try to convert to i64
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            StackValue::Int64(i) => Some(*i),
            StackValue::Float64(f) => Some(*f as i64),
            StackValue::Bool(b) => Some(if *b { 1 } else { 0 }),
            _ => None,
        }
    }

    /// Try to convert to f64
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            StackValue::Float64(f) => Some(*f),
            StackValue::Int64(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Convert to i64 (for compatibility with executor)
    pub fn to_i64(&self) -> Option<i64> {
        self.as_i64()
    }

    /// Convert to bool (for conditional operations)
    pub fn to_bool(&self) -> bool {
        self.is_truthy()
    }

    /// Convert to string (for database operations)
    pub fn to_string(&self) -> String {
        match self {
            StackValue::String(s) => s.clone(),
            StackValue::DocumentId(s) => s.clone(),
            StackValue::Collection(s) => s.clone(),
            StackValue::Int64(i) => i.to_string(),
            StackValue::Float64(f) => f.to_string(),
            StackValue::Bool(b) => b.to_string(),
            StackValue::Null => "null".to_string(),
            StackValue::Json(v) => v.to_string(),
        }
    }
}

impl fmt::Display for StackValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StackValue::Int64(v) => write!(f, "{}", v),
            StackValue::Float64(v) => write!(f, "{}", v),
            StackValue::String(v) => write!(f, "\"{}\"", v),
            StackValue::Bool(v) => write!(f, "{}", v),
            StackValue::Null => write!(f, "null"),
            StackValue::Json(v) => write!(f, "{}", v),
            StackValue::DocumentId(v) => write!(f, "doc:{}", v),
            StackValue::Collection(v) => write!(f, "col:{}", v),
        }
    }
}

/// Operand stack for VM execution
#[derive(Debug, Clone)]
pub struct OperandStack {
    /// Stack storage
    stack: Vec<StackValue>,
    /// Maximum allowed stack size
    max_size: usize,
}

impl OperandStack {
    /// Create a new operand stack with default maximum size
    pub fn new() -> Self {
        Self::with_max_size(MAX_STACK_SIZE)
    }

    /// Create a new operand stack with specified maximum size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            stack: Vec::with_capacity(std::cmp::min(max_size, 1000)),
            max_size,
        }
    }

    /// Push a value onto the stack
    pub fn push(&mut self, value: StackValue) -> Result<(), StackError> {
        if self.stack.len() >= self.max_size {
            return Err(StackError::Overflow);
        }
        self.stack.push(value);
        Ok(())
    }

    /// Pop a value from the stack
    pub fn pop(&mut self) -> Result<StackValue, StackError> {
        self.stack.pop().ok_or(StackError::Underflow)
    }

    /// Peek at the top value without removing it
    pub fn peek(&self) -> Result<&StackValue, StackError> {
        self.stack.last().ok_or(StackError::Underflow)
    }

    /// Peek at the value at the given depth (0 = top, 1 = second from top, etc.)
    pub fn peek_at(&self, depth: usize) -> Result<&StackValue, StackError> {
        if depth >= self.stack.len() {
            return Err(StackError::Underflow);
        }
        let index = self.stack.len() - 1 - depth;
        Ok(&self.stack[index])
    }

    /// Get the current stack size
    pub fn size(&self) -> usize {
        self.stack.len()
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Clear the stack
    pub fn clear(&mut self) {
        self.stack.clear();
    }

    /// Duplicate the top value on the stack
    pub fn dup(&mut self) -> Result<(), StackError> {
        let value = self.peek()?.clone();
        self.push(value)
    }

    /// Swap the top two values on the stack
    pub fn swap(&mut self) -> Result<(), StackError> {
        if self.stack.len() < 2 {
            return Err(StackError::Underflow);
        }
        let len = self.stack.len();
        self.stack.swap(len - 1, len - 2);
        Ok(())
    }

    /// Pop two values and push them in reverse order (useful for non-commutative operations)
    pub fn pop_two(&mut self) -> Result<(StackValue, StackValue), StackError> {
        let b = self.pop()?;
        let a = self.pop()?;
        Ok((a, b))
    }

    /// Pop three values
    pub fn pop_three(&mut self) -> Result<(StackValue, StackValue, StackValue), StackError> {
        let c = self.pop()?;
        let b = self.pop()?;
        let a = self.pop()?;
        Ok((a, b, c))
    }

    /// Get a snapshot of the current stack for debugging
    pub fn snapshot(&self) -> Vec<StackValue> {
        self.stack.clone()
    }

    /// Restore the stack from a snapshot (for debugging/testing)
    pub fn restore(&mut self, snapshot: Vec<StackValue>) -> Result<(), StackError> {
        if snapshot.len() > self.max_size {
            return Err(StackError::Overflow);
        }
        self.stack = snapshot;
        Ok(())
    }

    /// Get the maximum stack size
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Check if the stack has at least n elements
    pub fn has_at_least(&self, n: usize) -> bool {
        self.stack.len() >= n
    }
}

impl Default for OperandStack {
    fn default() -> Self {
        Self::new()
    }
}

/// Stack operation errors
#[derive(Debug, thiserror::Error)]
pub enum StackError {
    #[error("Stack overflow - maximum size ({}) exceeded", MAX_STACK_SIZE)]
    Overflow,

    #[error("Stack underflow - attempted to pop from empty stack")]
    Underflow,

    #[error("Type error: expected {expected}, found {found}")]
    TypeError { expected: String, found: String },

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Type alias for stack operation results
pub type StackResult<T> = Result<T, StackError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_basic_operations() {
        let mut stack = OperandStack::new();

        // Test push and pop
        assert!(stack.is_empty());
        stack.push(StackValue::Int64(42)).unwrap();
        stack.push(StackValue::String("hello".to_string())).unwrap();

        assert_eq!(stack.size(), 2);
        assert!(!stack.is_empty());

        let value = stack.pop().unwrap();
        assert_eq!(value, StackValue::String("hello".to_string()));

        let value = stack.pop().unwrap();
        assert_eq!(value, StackValue::Int64(42));

        assert!(stack.is_empty());
    }

    #[test]
    fn test_stack_underflow() {
        let mut stack = OperandStack::new();
        let result = stack.pop();
        assert!(matches!(result, Err(StackError::Underflow)));
    }

    #[test]
    fn test_stack_overflow() {
        let mut stack = OperandStack::with_max_size(2);
        stack.push(StackValue::Int64(1)).unwrap();
        stack.push(StackValue::Int64(2)).unwrap();

        let result = stack.push(StackValue::Int64(3));
        assert!(matches!(result, Err(StackError::Overflow)));
    }

    #[test]
    fn test_stack_peek() {
        let mut stack = OperandStack::new();
        stack.push(StackValue::Bool(true)).unwrap();
        stack.push(StackValue::Float64(3.14)).unwrap();

        let top = stack.peek().unwrap();
        assert_eq!(*top, StackValue::Float64(3.14));
        assert_eq!(stack.size(), 2); // Peek doesn't remove

        let second = stack.peek_at(1).unwrap();
        assert_eq!(*second, StackValue::Bool(true));
    }

    #[test]
    fn test_stack_dup() {
        let mut stack = OperandStack::new();
        stack.push(StackValue::String("test".to_string())).unwrap();
        stack.dup().unwrap();

        assert_eq!(stack.size(), 2);
        assert_eq!(stack.pop().unwrap(), StackValue::String("test".to_string()));
        assert_eq!(stack.pop().unwrap(), StackValue::String("test".to_string()));
    }

    #[test]
    fn test_stack_swap() {
        let mut stack = OperandStack::new();
        stack.push(StackValue::Int64(1)).unwrap();
        stack.push(StackValue::Int64(2)).unwrap();
        stack.swap().unwrap();

        assert_eq!(stack.pop().unwrap(), StackValue::Int64(1));
        assert_eq!(stack.pop().unwrap(), StackValue::Int64(2));
    }

    #[test]
    fn test_stack_pop_two() {
        let mut stack = OperandStack::new();
        stack.push(StackValue::Int64(1)).unwrap();
        stack.push(StackValue::Int64(2)).unwrap();

        let (a, b) = stack.pop_two().unwrap();
        assert_eq!(a, StackValue::Int64(1));
        assert_eq!(b, StackValue::Int64(2));
        assert!(stack.is_empty());
    }

    #[test]
    fn test_stack_value_conversions() {
        let constant = ConstantValue::String("test".to_string());
        let stack_value = StackValue::from_constant(&constant);
        assert_eq!(stack_value, StackValue::String("test".to_string()));

        let back_to_constant = stack_value.to_constant();
        assert_eq!(back_to_constant, constant);
    }

    #[test]
    fn test_stack_value_truthiness() {
        assert!(StackValue::Bool(true).is_truthy());
        assert!(!StackValue::Bool(false).is_truthy());
        assert!(StackValue::Int64(1).is_truthy());
        assert!(!StackValue::Int64(0).is_truthy());
        assert!(StackValue::String("hello".to_string()).is_truthy());
        assert!(!StackValue::String("".to_string()).is_truthy());
        assert!(!StackValue::Null.is_truthy());
    }

    #[test]
    fn test_stack_value_type_conversions() {
        let int_val = StackValue::Int64(42);
        assert_eq!(int_val.as_i64(), Some(42));
        assert_eq!(int_val.as_f64(), Some(42.0));
        assert_eq!(int_val.as_string(), None);

        let str_val = StackValue::String("hello".to_string());
        assert_eq!(str_val.as_string(), Some("hello".to_string()));
        assert_eq!(str_val.as_i64(), None);

        let bool_val = StackValue::Bool(true);
        assert_eq!(bool_val.as_i64(), Some(1));
    }

    #[test]
    fn test_stack_snapshot_restore() {
        let mut stack = OperandStack::new();
        stack.push(StackValue::Int64(1)).unwrap();
        stack.push(StackValue::String("test".to_string())).unwrap();

        let snapshot = stack.snapshot();
        stack.clear();
        assert!(stack.is_empty());

        stack.restore(snapshot).unwrap();
        assert_eq!(stack.size(), 2);
        assert_eq!(stack.pop().unwrap(), StackValue::String("test".to_string()));
        assert_eq!(stack.pop().unwrap(), StackValue::Int64(1));
    }
}
