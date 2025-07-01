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

//! Parser traits and interfaces for the DotVM compiler

use super::common::{ParseError, ParseResult, Position, Token};
use std::fmt::Debug;

/// Core trait for all parsers in the DotVM compiler
pub trait Parser<T> {
    /// Parse the input and return the parsed result
    fn parse(&mut self, input: &str) -> ParseResult<T>;

    /// Get the current position in the input
    fn position(&self) -> Position;

    /// Check if the parser has reached the end of input
    fn is_at_end(&self) -> bool;

    /// Reset the parser to the beginning
    fn reset(&mut self);

    /// Get the parser name for debugging
    fn name(&self) -> &'static str;
}

/// Trait for lexical analyzers (tokenizers)
pub trait Lexer {
    /// Get the next token from the input
    fn next_token(&mut self) -> ParseResult<Token>;

    /// Peek at the next token without consuming it
    fn peek_token(&self) -> ParseResult<Token>;

    /// Check if there are more tokens
    fn has_more_tokens(&self) -> bool;

    /// Get the current position
    fn position(&self) -> Position;

    /// Skip whitespace and comments
    fn skip_whitespace(&mut self);

    /// Reset to the beginning of input
    fn reset(&mut self);
}

/// Trait for syntax parsers that work with tokens
pub trait SyntaxParser<T> {
    /// Parse tokens into an AST node
    fn parse_tokens(&mut self, tokens: &[Token]) -> ParseResult<T>;

    /// Parse a specific construct starting from current position
    fn parse_construct(&mut self) -> ParseResult<T>;

    /// Check if the current token matches expected type
    fn expect_token(&mut self, expected: &Token) -> ParseResult<()>;

    /// Consume a token if it matches the expected type
    fn consume_if(&mut self, expected: &Token) -> bool;

    /// Get the current token without consuming it
    fn current_token(&self) -> Option<&Token>;
}

/// Trait for semantic analyzers
pub trait SemanticAnalyzer<T> {
    /// Perform semantic analysis on the parsed AST
    fn analyze(&mut self, ast: &T) -> ParseResult<()>;

    /// Check for semantic errors
    fn check_semantics(&self, ast: &T) -> Vec<ParseError>;

    /// Get symbol table information
    fn get_symbols(&self) -> &dyn SymbolTable;
}

/// Trait for symbol tables
pub trait SymbolTable {
    /// Define a new symbol
    fn define(&mut self, name: String, symbol_type: SymbolType) -> ParseResult<()>;

    /// Look up a symbol
    fn lookup(&self, name: &str) -> Option<&SymbolInfo>;

    /// Enter a new scope
    fn enter_scope(&mut self);

    /// Exit the current scope
    fn exit_scope(&mut self);

    /// Get the current scope level
    fn scope_level(&self) -> usize;
}

/// Information about a symbol
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub symbol_type: SymbolType,
    pub position: Position,
    pub scope_level: usize,
    pub is_mutable: bool,
}

/// Types of symbols in the language
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolType {
    Variable(VariableType),
    Function(FunctionType),
    Type(TypeInfo),
    Module(ModuleInfo),
    Constant(ConstantType),
}

/// Variable type information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableType {
    pub base_type: BaseType,
    pub is_array: bool,
    pub array_size: Option<usize>,
}

/// Function type information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionType {
    pub parameters: Vec<VariableType>,
    pub return_type: Option<VariableType>,
    pub is_external: bool,
}

/// Type information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeInfo {
    pub name: String,
    pub size: usize,
    pub alignment: usize,
}

/// Module information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleInfo {
    pub name: String,
    pub path: String,
    pub exports: Vec<String>,
}

/// Constant type information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantType {
    pub base_type: BaseType,
    pub value: ConstantValue,
}

/// Base types in the language
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    String,
    Void,
    Custom(String),
}

/// Constant values
#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
    Integer(i64),
    UnsignedInteger(u64),
    Float(f64),
    Boolean(bool),
    String(String),
    Null,
}

impl Eq for ConstantValue {}

/// Trait for validating parsed constructs
pub trait Validator<T> {
    /// Validate the parsed construct
    fn validate(&self, item: &T) -> Vec<ParseError>;

    /// Check if the construct is valid
    fn is_valid(&self, item: &T) -> bool {
        self.validate(item).is_empty()
    }
}

/// Trait for AST visitors
pub trait AstVisitor<T> {
    /// Visit an AST node
    fn visit(&mut self, node: &T) -> ParseResult<()>;

    /// Visit all children of a node
    fn visit_children(&mut self, node: &T) -> ParseResult<()>;
}

/// Trait for AST transformers
pub trait AstTransformer<T> {
    /// Transform an AST node
    fn transform(&mut self, node: T) -> ParseResult<T>;

    /// Transform all children of a node
    fn transform_children(&mut self, node: T) -> ParseResult<T>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_type_creation() {
        let var_type = SymbolType::Variable(VariableType {
            base_type: BaseType::I32,
            is_array: false,
            array_size: None,
        });

        match var_type {
            SymbolType::Variable(vt) => {
                assert_eq!(vt.base_type, BaseType::I32);
                assert!(!vt.is_array);
            }
            _ => panic!("Expected variable type"),
        }
    }

    #[test]
    fn test_function_type_creation() {
        let func_type = SymbolType::Function(FunctionType {
            parameters: vec![VariableType {
                base_type: BaseType::I32,
                is_array: false,
                array_size: None,
            }],
            return_type: Some(VariableType {
                base_type: BaseType::Bool,
                is_array: false,
                array_size: None,
            }),
            is_external: false,
        });

        match func_type {
            SymbolType::Function(ft) => {
                assert_eq!(ft.parameters.len(), 1);
                assert!(ft.return_type.is_some());
                assert!(!ft.is_external);
            }
            _ => panic!("Expected function type"),
        }
    }

    #[test]
    fn test_constant_value_equality() {
        let val1 = ConstantValue::Integer(42);
        let val2 = ConstantValue::Integer(42);
        let val3 = ConstantValue::Integer(24);

        assert_eq!(val1, val2);
        assert_ne!(val1, val3);
    }

    #[test]
    fn test_base_type_variants() {
        let types = vec![BaseType::I32, BaseType::F64, BaseType::Bool, BaseType::String, BaseType::Custom("MyType".to_string())];

        assert_eq!(types.len(), 5);
        assert_eq!(types[4], BaseType::Custom("MyType".to_string()));
    }
}
