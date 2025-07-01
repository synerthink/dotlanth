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

//! Parser error types and handling

use super::Position;
use std::fmt;
use thiserror::Error;

/// Result type for parsing operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Main error type for parsing operations
#[derive(Error, Debug, Clone)]
pub struct ParseError {
    /// The kind of error
    pub kind: ParseErrorKind,
    /// Position where the error occurred
    pub position: Position,
    /// Human-readable error message
    pub message: String,
    /// Additional context or suggestions
    pub context: Option<String>,
    /// Related errors (for error chains)
    pub related: Vec<ParseError>,
}

impl ParseError {
    /// Create a new parse error
    pub fn new(kind: ParseErrorKind, position: Position, message: String) -> Self {
        Self {
            kind,
            position,
            message,
            context: None,
            related: Vec::new(),
        }
    }

    /// Create an error with context
    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }

    /// Add a related error
    pub fn with_related(mut self, related: ParseError) -> Self {
        self.related.push(related);
        self
    }

    /// Add multiple related errors
    pub fn with_related_errors(mut self, related: Vec<ParseError>) -> Self {
        self.related.extend(related);
        self
    }

    /// Check if this is a fatal error that should stop parsing
    pub fn is_fatal(&self) -> bool {
        matches!(self.kind, ParseErrorKind::InternalError | ParseErrorKind::RecursionLimitExceeded | ParseErrorKind::OutOfMemory)
    }

    /// Get a user-friendly error message
    pub fn user_message(&self) -> String {
        format!("{} at line {}, column {}: {}", self.kind.description(), self.position.line, self.position.column, self.message)
    }

    /// Get detailed error information for debugging
    pub fn debug_message(&self) -> String {
        let mut msg = format!(
            "[{}] {} at {}:{}: {}",
            self.kind.code(),
            self.kind.description(),
            self.position.line,
            self.position.column,
            self.message
        );

        if let Some(context) = &self.context {
            msg.push_str(&format!("\nContext: {}", context));
        }

        if !self.related.is_empty() {
            msg.push_str("\nRelated errors:");
            for (i, error) in self.related.iter().enumerate() {
                msg.push_str(&format!("\n  {}: {}", i + 1, error.user_message()));
            }
        }

        msg
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

/// Categories of parse errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// Lexical errors (tokenization)
    #[error("Lexical error")]
    LexicalError,

    /// Syntax errors (grammar violations)
    #[error("Syntax error")]
    SyntaxError,

    /// Semantic errors (type checking, etc.)
    #[error("Semantic error")]
    SemanticError,

    /// Unexpected token
    #[error("Unexpected token")]
    UnexpectedToken,

    /// Expected token not found
    #[error("Expected token")]
    ExpectedToken,

    /// Unexpected end of file
    #[error("Unexpected end of file")]
    UnexpectedEof,

    /// Invalid character
    #[error("Invalid character")]
    InvalidCharacter,

    /// Invalid number format
    #[error("Invalid number")]
    InvalidNumber,

    /// Invalid string literal
    #[error("Invalid string literal")]
    InvalidString,

    /// Invalid escape sequence
    #[error("Invalid escape sequence")]
    InvalidEscapeSequence,

    /// Unterminated string literal
    #[error("Unterminated string")]
    UnterminatedString,

    /// Unterminated comment
    #[error("Unterminated comment")]
    UnterminatedComment,

    /// Invalid identifier
    #[error("Invalid identifier")]
    InvalidIdentifier,

    /// Duplicate declaration
    #[error("Duplicate declaration")]
    DuplicateDeclaration,

    /// Undefined symbol
    #[error("Undefined symbol")]
    UndefinedSymbol,

    /// Type mismatch
    #[error("Type mismatch")]
    TypeMismatch,

    /// Invalid assignment
    #[error("Invalid assignment")]
    InvalidAssignment,

    /// Invalid operation
    #[error("Invalid operation")]
    InvalidOperation,

    /// Recursion limit exceeded
    #[error("Recursion limit exceeded")]
    RecursionLimitExceeded,

    /// Out of memory
    #[error("Out of memory")]
    OutOfMemory,

    /// Internal compiler error
    #[error("Internal error")]
    InternalError,

    /// Custom error with message
    #[error("Custom error")]
    Custom(String),
}

impl ParseErrorKind {
    /// Get a short error code for this kind
    pub fn code(&self) -> &'static str {
        match self {
            ParseErrorKind::LexicalError => "E001",
            ParseErrorKind::SyntaxError => "E002",
            ParseErrorKind::SemanticError => "E003",
            ParseErrorKind::UnexpectedToken => "E004",
            ParseErrorKind::ExpectedToken => "E005",
            ParseErrorKind::UnexpectedEof => "E006",
            ParseErrorKind::InvalidCharacter => "E007",
            ParseErrorKind::InvalidNumber => "E008",
            ParseErrorKind::InvalidString => "E009",
            ParseErrorKind::InvalidEscapeSequence => "E010",
            ParseErrorKind::UnterminatedString => "E011",
            ParseErrorKind::UnterminatedComment => "E012",
            ParseErrorKind::InvalidIdentifier => "E013",
            ParseErrorKind::DuplicateDeclaration => "E014",
            ParseErrorKind::UndefinedSymbol => "E015",
            ParseErrorKind::TypeMismatch => "E016",
            ParseErrorKind::InvalidAssignment => "E017",
            ParseErrorKind::InvalidOperation => "E018",
            ParseErrorKind::RecursionLimitExceeded => "E019",
            ParseErrorKind::OutOfMemory => "E020",
            ParseErrorKind::InternalError => "E021",
            ParseErrorKind::Custom(_) => "E999",
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            ParseErrorKind::LexicalError => "Lexical error",
            ParseErrorKind::SyntaxError => "Syntax error",
            ParseErrorKind::SemanticError => "Semantic error",
            ParseErrorKind::UnexpectedToken => "Unexpected token",
            ParseErrorKind::ExpectedToken => "Expected token",
            ParseErrorKind::UnexpectedEof => "Unexpected end of file",
            ParseErrorKind::InvalidCharacter => "Invalid character",
            ParseErrorKind::InvalidNumber => "Invalid number format",
            ParseErrorKind::InvalidString => "Invalid string literal",
            ParseErrorKind::InvalidEscapeSequence => "Invalid escape sequence",
            ParseErrorKind::UnterminatedString => "Unterminated string literal",
            ParseErrorKind::UnterminatedComment => "Unterminated comment",
            ParseErrorKind::InvalidIdentifier => "Invalid identifier",
            ParseErrorKind::DuplicateDeclaration => "Duplicate declaration",
            ParseErrorKind::UndefinedSymbol => "Undefined symbol",
            ParseErrorKind::TypeMismatch => "Type mismatch",
            ParseErrorKind::InvalidAssignment => "Invalid assignment",
            ParseErrorKind::InvalidOperation => "Invalid operation",
            ParseErrorKind::RecursionLimitExceeded => "Recursion limit exceeded",
            ParseErrorKind::OutOfMemory => "Out of memory",
            ParseErrorKind::InternalError => "Internal compiler error",
            ParseErrorKind::Custom(_) => "Custom error",
        }
    }

    /// Get severity level (0 = info, 1 = warning, 2 = error, 3 = fatal)
    pub fn severity(&self) -> u8 {
        match self {
            ParseErrorKind::LexicalError
            | ParseErrorKind::SyntaxError
            | ParseErrorKind::UnexpectedToken
            | ParseErrorKind::ExpectedToken
            | ParseErrorKind::UnexpectedEof
            | ParseErrorKind::InvalidCharacter
            | ParseErrorKind::InvalidNumber
            | ParseErrorKind::InvalidString
            | ParseErrorKind::InvalidEscapeSequence
            | ParseErrorKind::UnterminatedString
            | ParseErrorKind::UnterminatedComment
            | ParseErrorKind::InvalidIdentifier => 2, // Error

            ParseErrorKind::SemanticError
            | ParseErrorKind::DuplicateDeclaration
            | ParseErrorKind::UndefinedSymbol
            | ParseErrorKind::TypeMismatch
            | ParseErrorKind::InvalidAssignment
            | ParseErrorKind::InvalidOperation => 2, // Error

            ParseErrorKind::RecursionLimitExceeded | ParseErrorKind::OutOfMemory | ParseErrorKind::InternalError => 3, // Fatal

            ParseErrorKind::Custom(_) => 2, // Error by default
        }
    }
}

/// Helper functions for creating common errors
impl ParseError {
    /// Create a syntax error
    pub fn syntax_error(position: Position, message: String) -> Self {
        Self::new(ParseErrorKind::SyntaxError, position, message)
    }

    /// Create an unexpected token error
    pub fn unexpected_token(position: Position, found: String, expected: Option<String>) -> Self {
        let message = if let Some(exp) = expected {
            format!("Found '{}', expected '{}'", found, exp)
        } else {
            format!("Unexpected token '{}'", found)
        };
        Self::new(ParseErrorKind::UnexpectedToken, position, message)
    }

    /// Create an unexpected EOF error
    pub fn unexpected_eof(position: Position) -> Self {
        Self::new(ParseErrorKind::UnexpectedEof, position, "Unexpected end of file".to_string())
    }

    /// Create an undefined symbol error
    pub fn undefined_symbol(position: Position, symbol: String) -> Self {
        Self::new(ParseErrorKind::UndefinedSymbol, position, format!("Undefined symbol '{}'", symbol))
    }

    /// Create a type mismatch error
    pub fn type_mismatch(position: Position, expected: String, found: String) -> Self {
        Self::new(ParseErrorKind::TypeMismatch, position, format!("Type mismatch: expected '{}', found '{}'", expected, found))
    }

    /// Create a duplicate declaration error
    pub fn duplicate_declaration(position: Position, name: String) -> Self {
        Self::new(ParseErrorKind::DuplicateDeclaration, position, format!("Duplicate declaration of '{}'", name))
    }

    /// Create an internal error
    pub fn internal_error(position: Position, message: String) -> Self {
        Self::new(ParseErrorKind::InternalError, position, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_creation() {
        let pos = Position::new(10, 5);
        let error = ParseError::new(ParseErrorKind::SyntaxError, pos, "Test error".to_string());

        assert_eq!(error.kind, ParseErrorKind::SyntaxError);
        assert_eq!(error.position, pos);
        assert_eq!(error.message, "Test error");
        assert!(error.context.is_none());
        assert!(error.related.is_empty());
    }

    #[test]
    fn test_error_with_context() {
        let error = ParseError::syntax_error(Position::new(1, 1), "Test error".to_string()).with_context("Additional context".to_string());

        assert_eq!(error.context, Some("Additional context".to_string()));
    }

    #[test]
    fn test_error_severity() {
        assert_eq!(ParseErrorKind::SyntaxError.severity(), 2);
        assert_eq!(ParseErrorKind::InternalError.severity(), 3);
        assert_eq!(ParseErrorKind::TypeMismatch.severity(), 2);
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(ParseErrorKind::SyntaxError.code(), "E002");
        assert_eq!(ParseErrorKind::UnexpectedToken.code(), "E004");
        assert_eq!(ParseErrorKind::InternalError.code(), "E021");
    }

    #[test]
    fn test_fatal_errors() {
        let fatal_error = ParseError::new(ParseErrorKind::InternalError, Position::new(1, 1), "Fatal".to_string());
        assert!(fatal_error.is_fatal());

        let normal_error = ParseError::syntax_error(Position::new(1, 1), "Normal".to_string());
        assert!(!normal_error.is_fatal());
    }

    #[test]
    fn test_helper_constructors() {
        let pos = Position::new(5, 10);

        let syntax_err = ParseError::syntax_error(pos, "syntax".to_string());
        assert_eq!(syntax_err.kind, ParseErrorKind::SyntaxError);

        let unexpected_err = ParseError::unexpected_token(pos, "found".to_string(), Some("expected".to_string()));
        assert_eq!(unexpected_err.kind, ParseErrorKind::UnexpectedToken);
        assert!(unexpected_err.message.contains("Found 'found', expected 'expected'"));

        let eof_err = ParseError::unexpected_eof(pos);
        assert_eq!(eof_err.kind, ParseErrorKind::UnexpectedEof);

        let undef_err = ParseError::undefined_symbol(pos, "symbol".to_string());
        assert_eq!(undef_err.kind, ParseErrorKind::UndefinedSymbol);
        assert!(undef_err.message.contains("symbol"));
    }

    #[test]
    fn test_error_messages() {
        let error = ParseError::syntax_error(Position::new(10, 5), "Test message".to_string());

        let user_msg = error.user_message();
        assert!(user_msg.contains("line 10"));
        assert!(user_msg.contains("column 5"));
        assert!(user_msg.contains("Test message"));

        let debug_msg = error.debug_message();
        assert!(debug_msg.contains("E002"));
        assert!(debug_msg.contains("Syntax error"));
    }
}
