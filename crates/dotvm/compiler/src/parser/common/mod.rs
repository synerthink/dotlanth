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

//! Common parsing utilities and types

pub mod error;
pub mod position;
pub mod token;

pub use error::{ParseError, ParseErrorKind, ParseResult};
pub use position::Position;
pub use token::{Delimiter, Keyword, Operator, Token, TokenType};

use std::collections::HashMap;

/// Configuration for parsers
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Maximum recursion depth for parsing
    pub max_recursion_depth: usize,
    /// Whether to include debug information in AST
    pub include_debug_info: bool,
    /// Whether to perform strict type checking
    pub strict_type_checking: bool,
    /// Custom keywords to recognize
    pub custom_keywords: HashMap<String, Keyword>,
    /// Whether to allow experimental features
    pub allow_experimental: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            max_recursion_depth: 1000,
            include_debug_info: true,
            strict_type_checking: true,
            custom_keywords: HashMap::new(),
            allow_experimental: false,
        }
    }
}

impl ParserConfig {
    /// Create a new parser configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum recursion depth
    pub fn with_max_recursion_depth(mut self, depth: usize) -> Self {
        self.max_recursion_depth = depth;
        self
    }

    /// Enable or disable debug information
    pub fn with_debug_info(mut self, enable: bool) -> Self {
        self.include_debug_info = enable;
        self
    }

    /// Enable or disable strict type checking
    pub fn with_strict_types(mut self, strict: bool) -> Self {
        self.strict_type_checking = strict;
        self
    }

    /// Add a custom keyword
    pub fn with_custom_keyword(mut self, word: String, keyword: Keyword) -> Self {
        self.custom_keywords.insert(word, keyword);
        self
    }

    /// Enable or disable experimental features
    pub fn with_experimental(mut self, allow: bool) -> Self {
        self.allow_experimental = allow;
        self
    }
}

/// Context information for parsing
#[derive(Debug, Clone)]
pub struct ParseContext {
    /// Current file being parsed
    pub file_name: String,
    /// Current recursion depth
    pub recursion_depth: usize,
    /// Parser configuration
    pub config: ParserConfig,
    /// Source code being parsed
    pub source: String,
}

impl ParseContext {
    /// Create a new parse context
    pub fn new(file_name: String, source: String) -> Self {
        Self {
            file_name,
            recursion_depth: 0,
            config: ParserConfig::default(),
            source,
        }
    }

    /// Create context with custom configuration
    pub fn with_config(file_name: String, source: String, config: ParserConfig) -> Self {
        Self {
            file_name,
            recursion_depth: 0,
            config,
            source,
        }
    }

    /// Enter a new recursion level
    pub fn enter_recursion(&mut self) -> ParseResult<()> {
        if self.recursion_depth >= self.config.max_recursion_depth {
            return Err(ParseError::new(
                ParseErrorKind::RecursionLimitExceeded,
                Position::new(0, 0),
                format!("Maximum recursion depth of {} exceeded", self.config.max_recursion_depth),
            ));
        }
        self.recursion_depth += 1;
        Ok(())
    }

    /// Exit the current recursion level
    pub fn exit_recursion(&mut self) {
        if self.recursion_depth > 0 {
            self.recursion_depth -= 1;
        }
    }

    /// Get a line from the source code
    pub fn get_line(&self, line_number: usize) -> Option<&str> {
        self.source.lines().nth(line_number.saturating_sub(1))
    }

    /// Get context around a position for error reporting
    pub fn get_context(&self, position: Position, context_lines: usize) -> Vec<String> {
        let start_line = position.line.saturating_sub(context_lines);
        let end_line = position.line + context_lines;

        self.source
            .lines()
            .enumerate()
            .skip(start_line.saturating_sub(1))
            .take(end_line - start_line + 1)
            .map(|(i, line)| format!("{:4} | {}", i + 1, line))
            .collect()
    }
}

/// Utility functions for parsing
pub mod utils {
    use super::*;

    /// Check if a character is a valid identifier start
    pub fn is_identifier_start(c: char) -> bool {
        c.is_alphabetic() || c == '_'
    }

    /// Check if a character is a valid identifier continuation
    pub fn is_identifier_continue(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    /// Check if a string is a valid identifier
    pub fn is_valid_identifier(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        let mut chars = s.chars();
        if let Some(first) = chars.next() {
            if !is_identifier_start(first) {
                return false;
            }
        }

        chars.all(is_identifier_continue)
    }

    /// Check if a character is whitespace
    pub fn is_whitespace(c: char) -> bool {
        matches!(c, ' ' | '\t' | '\r' | '\n')
    }

    /// Check if a character starts a number
    pub fn is_digit_start(c: char) -> bool {
        c.is_ascii_digit()
    }

    /// Check if a character can be part of a number
    pub fn is_digit_continue(c: char) -> bool {
        c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-'
    }

    /// Escape a string for display
    pub fn escape_string(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                '\n' => "\\n".to_string(),
                '\r' => "\\r".to_string(),
                '\t' => "\\t".to_string(),
                '\\' => "\\\\".to_string(),
                '"' => "\\\"".to_string(),
                c if c.is_control() => format!("\\u{{{:04x}}}", c as u32),
                c => c.to_string(),
            })
            .collect()
    }

    /// Unescape a string literal
    pub fn unescape_string(s: &str) -> ParseResult<String> {
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some('u') => {
                        // Unicode escape sequence
                        if chars.next() != Some('{') {
                            return Err(ParseError::new(
                                ParseErrorKind::InvalidEscapeSequence,
                                Position::new(0, 0),
                                "Invalid unicode escape sequence".to_string(),
                            ));
                        }

                        let mut hex_digits = String::new();
                        while let Some(c) = chars.peek() {
                            if *c == '}' {
                                chars.next();
                                break;
                            }
                            if c.is_ascii_hexdigit() {
                                hex_digits.push(chars.next().unwrap());
                            } else {
                                return Err(ParseError::new(
                                    ParseErrorKind::InvalidEscapeSequence,
                                    Position::new(0, 0),
                                    "Invalid hex digit in unicode escape".to_string(),
                                ));
                            }
                        }

                        if let Ok(code_point) = u32::from_str_radix(&hex_digits, 16) {
                            if let Some(unicode_char) = char::from_u32(code_point) {
                                result.push(unicode_char);
                            } else {
                                return Err(ParseError::new(ParseErrorKind::InvalidEscapeSequence, Position::new(0, 0), "Invalid unicode code point".to_string()));
                            }
                        } else {
                            return Err(ParseError::new(
                                ParseErrorKind::InvalidEscapeSequence,
                                Position::new(0, 0),
                                "Invalid hex number in unicode escape".to_string(),
                            ));
                        }
                    }
                    Some(c) => {
                        return Err(ParseError::new(ParseErrorKind::InvalidEscapeSequence, Position::new(0, 0), format!("Unknown escape sequence: \\{}", c)));
                    }
                    None => {
                        return Err(ParseError::new(ParseErrorKind::InvalidEscapeSequence, Position::new(0, 0), "Incomplete escape sequence".to_string()));
                    }
                }
            } else {
                result.push(c);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_config_creation() {
        let config = ParserConfig::new()
            .with_max_recursion_depth(500)
            .with_debug_info(false)
            .with_strict_types(false)
            .with_experimental(true);

        assert_eq!(config.max_recursion_depth, 500);
        assert!(!config.include_debug_info);
        assert!(!config.strict_type_checking);
        assert!(config.allow_experimental);
    }

    #[test]
    fn test_parse_context_recursion() {
        let mut context = ParseContext::new("test.dvm".to_string(), "test code".to_string());

        assert_eq!(context.recursion_depth, 0);
        assert!(context.enter_recursion().is_ok());
        assert_eq!(context.recursion_depth, 1);

        context.exit_recursion();
        assert_eq!(context.recursion_depth, 0);
    }

    #[test]
    fn test_utils_identifier_validation() {
        assert!(utils::is_valid_identifier("valid_name"));
        assert!(utils::is_valid_identifier("_private"));
        assert!(utils::is_valid_identifier("name123"));
        assert!(!utils::is_valid_identifier("123invalid"));
        assert!(!utils::is_valid_identifier(""));
        assert!(!utils::is_valid_identifier("with-dash"));
    }

    #[test]
    fn test_utils_string_escaping() {
        assert_eq!(utils::escape_string("hello\nworld"), "hello\\nworld");
        assert_eq!(utils::escape_string("tab\there"), "tab\\there");
        assert_eq!(utils::escape_string("quote\"here"), "quote\\\"here");
    }

    #[test]
    fn test_utils_string_unescaping() {
        assert_eq!(utils::unescape_string("hello\\nworld").unwrap(), "hello\nworld");
        assert_eq!(utils::unescape_string("tab\\there").unwrap(), "tab\there");
        assert_eq!(utils::unescape_string("quote\\\"here").unwrap(), "quote\"here");
        assert!(utils::unescape_string("invalid\\x").is_err());
    }

    #[test]
    fn test_context_get_line() {
        let source = "line 1\nline 2\nline 3";
        let context = ParseContext::new("test".to_string(), source.to_string());

        assert_eq!(context.get_line(1), Some("line 1"));
        assert_eq!(context.get_line(2), Some("line 2"));
        assert_eq!(context.get_line(3), Some("line 3"));
        assert_eq!(context.get_line(4), None);
    }
}
