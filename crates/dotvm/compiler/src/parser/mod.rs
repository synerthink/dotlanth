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

//! Parser module for the DotVM compiler
//!
//! This module provides comprehensive parsing infrastructure including:
//! - Lexical analysis (tokenization)
//! - Syntax analysis (AST generation)
//! - Semantic analysis (type checking, symbol resolution)
//! - Error handling and reporting
//! - Position tracking for debugging
//!
//! # Architecture
//!
//! The parser is organized into several layers:
//! - **Common**: Shared types, utilities, and error handling
//! - **Traits**: Interfaces for different parser components
//! - **DotVM**: Language-specific parsing logic
//! - **Validation**: Semantic analysis and validation
//!
//! # Example Usage
//!
//! ```rust
//! use dotvm_compiler::parser::{ParserConfig, ParseContext};
//!
//! let config = ParserConfig::new()
//!     .with_strict_types(true)
//!     .with_debug_info(true);
//!     
//! let context = ParseContext::with_config(
//!     "example.dvm".to_string(),
//!     "let x: i32 = 42;".to_string(),
//!     config
//! );
//! ```

pub mod common;
pub mod traits;
// TODO: Implement in future phases
// pub mod dotvm;
// pub mod validation;

// Re-export commonly used types
pub use common::{Delimiter, Keyword, Operator, ParseContext, ParseError, ParseErrorKind, ParseResult, ParserConfig, Position, Token, TokenType, position::Span};

pub use traits::{AstTransformer, AstVisitor, BaseType, ConstantValue, Lexer, Parser, SemanticAnalyzer, SymbolInfo, SymbolTable, SymbolType, SyntaxParser, Validator};

/// Create a default parser configuration
pub fn default_config() -> ParserConfig {
    ParserConfig::new()
}

/// Create a strict parser configuration for production use
pub fn strict_config() -> ParserConfig {
    ParserConfig::new().with_strict_types(true).with_debug_info(false).with_max_recursion_depth(500)
}

/// Create a development parser configuration with extra debugging
pub fn dev_config() -> ParserConfig {
    ParserConfig::new()
        .with_strict_types(false)
        .with_debug_info(true)
        .with_experimental(true)
        .with_max_recursion_depth(1000)
}

/// Quick parse function for simple use cases
pub fn quick_parse_tokens(source: &str) -> ParseResult<Vec<Token>> {
    // This is a placeholder - will be implemented when we add the lexer
    // For now, return an empty token list
    Ok(vec![Token::new(TokenType::Eof, "".to_string(), Span::single(Position::start()))])
}
