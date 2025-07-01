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

//! Token definitions for the DotVM language

use super::{Position, position::Span};
use std::fmt;

/// A token in the source code
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The type of token
    pub token_type: TokenType,
    /// The source text that produced this token
    pub lexeme: String,
    /// The position where this token starts
    pub span: Span,
}

impl Token {
    /// Create a new token
    pub fn new(token_type: TokenType, lexeme: String, span: Span) -> Self {
        Self { token_type, lexeme, span }
    }

    /// Create a token at a single position
    pub fn at_position(token_type: TokenType, lexeme: String, position: Position) -> Self {
        let span = Span::single(position);
        Self::new(token_type, lexeme, span)
    }

    /// Check if this token is of a specific type
    pub fn is_type(&self, token_type: &TokenType) -> bool {
        &self.token_type == token_type
    }

    /// Check if this token is a keyword
    pub fn is_keyword(&self) -> bool {
        matches!(self.token_type, TokenType::Keyword(_))
    }

    /// Check if this token is an operator
    pub fn is_operator(&self) -> bool {
        matches!(self.token_type, TokenType::Operator(_))
    }

    /// Check if this token is a delimiter
    pub fn is_delimiter(&self) -> bool {
        matches!(self.token_type, TokenType::Delimiter(_))
    }

    /// Check if this token is a literal
    pub fn is_literal(&self) -> bool {
        matches!(
            self.token_type,
            TokenType::IntegerLiteral(_) | TokenType::FloatLiteral(_) | TokenType::StringLiteral(_) | TokenType::BooleanLiteral(_) | TokenType::CharLiteral(_)
        )
    }

    /// Get the keyword if this token is a keyword
    pub fn as_keyword(&self) -> Option<&Keyword> {
        match &self.token_type {
            TokenType::Keyword(kw) => Some(kw),
            _ => None,
        }
    }

    /// Get the operator if this token is an operator
    pub fn as_operator(&self) -> Option<&Operator> {
        match &self.token_type {
            TokenType::Operator(op) => Some(op),
            _ => None,
        }
    }

    /// Get the delimiter if this token is a delimiter
    pub fn as_delimiter(&self) -> Option<&Delimiter> {
        match &self.token_type {
            TokenType::Delimiter(delim) => Some(delim),
            _ => None,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} '{}'", self.token_type, self.lexeme)
    }
}

/// Types of tokens in the DotVM language
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    IntegerLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BooleanLiteral(bool),
    CharLiteral(char),

    // Identifiers
    Identifier(String),

    // Keywords
    Keyword(Keyword),

    // Operators
    Operator(Operator),

    // Delimiters
    Delimiter(Delimiter),

    // Comments
    Comment(String),

    // Whitespace
    Whitespace,

    // End of file
    Eof,

    // Invalid token
    Invalid(String),
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::IntegerLiteral(_) => write!(f, "integer"),
            TokenType::FloatLiteral(_) => write!(f, "float"),
            TokenType::StringLiteral(_) => write!(f, "string"),
            TokenType::BooleanLiteral(_) => write!(f, "boolean"),
            TokenType::CharLiteral(_) => write!(f, "character"),
            TokenType::Identifier(_) => write!(f, "identifier"),
            TokenType::Keyword(kw) => write!(f, "keyword '{}'", kw),
            TokenType::Operator(op) => write!(f, "operator '{}'", op),
            TokenType::Delimiter(delim) => write!(f, "delimiter '{}'", delim),
            TokenType::Comment(_) => write!(f, "comment"),
            TokenType::Whitespace => write!(f, "whitespace"),
            TokenType::Eof => write!(f, "end of file"),
            TokenType::Invalid(_) => write!(f, "invalid token"),
        }
    }
}

/// Keywords in the DotVM language
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Keyword {
    // Control flow
    If,
    Else,
    While,
    For,
    Loop,
    Break,
    Continue,
    Return,
    Match,

    // Declarations
    Let,
    Const,
    Fn,
    Struct,
    Enum,
    Trait,
    Impl,
    Mod,
    Use,
    Import,

    // Types
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
    Char,
    String,
    Void,

    // Literals
    True,
    False,
    Null,

    // Memory management
    New,
    Delete,
    Ref,
    Deref,

    // Visibility
    Pub,
    Priv,

    // Other
    As,
    In,
    Mut,
    Static,
    Extern,
    Unsafe,
    Async,
    Await,
    Self_,
    Super,
    Crate,

    // Custom keyword for extensions
    Custom(String),
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::While => "while",
            Keyword::For => "for",
            Keyword::Loop => "loop",
            Keyword::Break => "break",
            Keyword::Continue => "continue",
            Keyword::Return => "return",
            Keyword::Match => "match",
            Keyword::Let => "let",
            Keyword::Const => "const",
            Keyword::Fn => "fn",
            Keyword::Struct => "struct",
            Keyword::Enum => "enum",
            Keyword::Trait => "trait",
            Keyword::Impl => "impl",
            Keyword::Mod => "mod",
            Keyword::Use => "use",
            Keyword::Import => "import",
            Keyword::I8 => "i8",
            Keyword::I16 => "i16",
            Keyword::I32 => "i32",
            Keyword::I64 => "i64",
            Keyword::U8 => "u8",
            Keyword::U16 => "u16",
            Keyword::U32 => "u32",
            Keyword::U64 => "u64",
            Keyword::F32 => "f32",
            Keyword::F64 => "f64",
            Keyword::Bool => "bool",
            Keyword::Char => "char",
            Keyword::String => "string",
            Keyword::Void => "void",
            Keyword::True => "true",
            Keyword::False => "false",
            Keyword::Null => "null",
            Keyword::New => "new",
            Keyword::Delete => "delete",
            Keyword::Ref => "ref",
            Keyword::Deref => "deref",
            Keyword::Pub => "pub",
            Keyword::Priv => "priv",
            Keyword::As => "as",
            Keyword::In => "in",
            Keyword::Mut => "mut",
            Keyword::Static => "static",
            Keyword::Extern => "extern",
            Keyword::Unsafe => "unsafe",
            Keyword::Async => "async",
            Keyword::Await => "await",
            Keyword::Self_ => "self",
            Keyword::Super => "super",
            Keyword::Crate => "crate",
            Keyword::Custom(s) => s,
        };
        write!(f, "{}", s)
    }
}

impl Keyword {
    /// Get keyword from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "if" => Some(Keyword::If),
            "else" => Some(Keyword::Else),
            "while" => Some(Keyword::While),
            "for" => Some(Keyword::For),
            "loop" => Some(Keyword::Loop),
            "break" => Some(Keyword::Break),
            "continue" => Some(Keyword::Continue),
            "return" => Some(Keyword::Return),
            "match" => Some(Keyword::Match),
            "let" => Some(Keyword::Let),
            "const" => Some(Keyword::Const),
            "fn" => Some(Keyword::Fn),
            "struct" => Some(Keyword::Struct),
            "enum" => Some(Keyword::Enum),
            "trait" => Some(Keyword::Trait),
            "impl" => Some(Keyword::Impl),
            "mod" => Some(Keyword::Mod),
            "use" => Some(Keyword::Use),
            "import" => Some(Keyword::Import),
            "i8" => Some(Keyword::I8),
            "i16" => Some(Keyword::I16),
            "i32" => Some(Keyword::I32),
            "i64" => Some(Keyword::I64),
            "u8" => Some(Keyword::U8),
            "u16" => Some(Keyword::U16),
            "u32" => Some(Keyword::U32),
            "u64" => Some(Keyword::U64),
            "f32" => Some(Keyword::F32),
            "f64" => Some(Keyword::F64),
            "bool" => Some(Keyword::Bool),
            "char" => Some(Keyword::Char),
            "string" => Some(Keyword::String),
            "void" => Some(Keyword::Void),
            "true" => Some(Keyword::True),
            "false" => Some(Keyword::False),
            "null" => Some(Keyword::Null),
            "new" => Some(Keyword::New),
            "delete" => Some(Keyword::Delete),
            "ref" => Some(Keyword::Ref),
            "deref" => Some(Keyword::Deref),
            "pub" => Some(Keyword::Pub),
            "priv" => Some(Keyword::Priv),
            "as" => Some(Keyword::As),
            "in" => Some(Keyword::In),
            "mut" => Some(Keyword::Mut),
            "static" => Some(Keyword::Static),
            "extern" => Some(Keyword::Extern),
            "unsafe" => Some(Keyword::Unsafe),
            "async" => Some(Keyword::Async),
            "await" => Some(Keyword::Await),
            "self" => Some(Keyword::Self_),
            "super" => Some(Keyword::Super),
            "crate" => Some(Keyword::Crate),
            _ => None,
        }
    }

    /// Check if this keyword is a type keyword
    pub fn is_type_keyword(&self) -> bool {
        matches!(
            self,
            Keyword::I8
                | Keyword::I16
                | Keyword::I32
                | Keyword::I64
                | Keyword::U8
                | Keyword::U16
                | Keyword::U32
                | Keyword::U64
                | Keyword::F32
                | Keyword::F64
                | Keyword::Bool
                | Keyword::Char
                | Keyword::String
                | Keyword::Void
        )
    }

    /// Check if this keyword is a control flow keyword
    pub fn is_control_flow(&self) -> bool {
        matches!(
            self,
            Keyword::If | Keyword::Else | Keyword::While | Keyword::For | Keyword::Loop | Keyword::Break | Keyword::Continue | Keyword::Return | Keyword::Match
        )
    }
}

/// Operators in the DotVM language
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operator {
    // Arithmetic
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Power,

    // Assignment
    Assign,
    PlusAssign,
    MinusAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,

    // Comparison
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    // Logical
    And,
    Or,
    Not,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
    LeftShift,
    RightShift,

    // Other
    Arrow,
    FatArrow,
    Dot,
    Range,
    RangeInclusive,
    Question,
    DoubleColon,
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Operator::Plus => "+",
            Operator::Minus => "-",
            Operator::Multiply => "*",
            Operator::Divide => "/",
            Operator::Modulo => "%",
            Operator::Power => "**",
            Operator::Assign => "=",
            Operator::PlusAssign => "+=",
            Operator::MinusAssign => "-=",
            Operator::MultiplyAssign => "*=",
            Operator::DivideAssign => "/=",
            Operator::ModuloAssign => "%=",
            Operator::Equal => "==",
            Operator::NotEqual => "!=",
            Operator::Less => "<",
            Operator::LessEqual => "<=",
            Operator::Greater => ">",
            Operator::GreaterEqual => ">=",
            Operator::And => "&&",
            Operator::Or => "||",
            Operator::Not => "!",
            Operator::BitAnd => "&",
            Operator::BitOr => "|",
            Operator::BitXor => "^",
            Operator::BitNot => "~",
            Operator::LeftShift => "<<",
            Operator::RightShift => ">>",
            Operator::Arrow => "->",
            Operator::FatArrow => "=>",
            Operator::Dot => ".",
            Operator::Range => "..",
            Operator::RangeInclusive => "..=",
            Operator::Question => "?",
            Operator::DoubleColon => "::",
        };
        write!(f, "{}", s)
    }
}

impl Operator {
    /// Get operator from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "+" => Some(Operator::Plus),
            "-" => Some(Operator::Minus),
            "*" => Some(Operator::Multiply),
            "/" => Some(Operator::Divide),
            "%" => Some(Operator::Modulo),
            "**" => Some(Operator::Power),
            "=" => Some(Operator::Assign),
            "+=" => Some(Operator::PlusAssign),
            "-=" => Some(Operator::MinusAssign),
            "*=" => Some(Operator::MultiplyAssign),
            "/=" => Some(Operator::DivideAssign),
            "%=" => Some(Operator::ModuloAssign),
            "==" => Some(Operator::Equal),
            "!=" => Some(Operator::NotEqual),
            "<" => Some(Operator::Less),
            "<=" => Some(Operator::LessEqual),
            ">" => Some(Operator::Greater),
            ">=" => Some(Operator::GreaterEqual),
            "&&" => Some(Operator::And),
            "||" => Some(Operator::Or),
            "!" => Some(Operator::Not),
            "&" => Some(Operator::BitAnd),
            "|" => Some(Operator::BitOr),
            "^" => Some(Operator::BitXor),
            "~" => Some(Operator::BitNot),
            "<<" => Some(Operator::LeftShift),
            ">>" => Some(Operator::RightShift),
            "->" => Some(Operator::Arrow),
            "=>" => Some(Operator::FatArrow),
            "." => Some(Operator::Dot),
            ".." => Some(Operator::Range),
            "..=" => Some(Operator::RangeInclusive),
            "?" => Some(Operator::Question),
            "::" => Some(Operator::DoubleColon),
            _ => None,
        }
    }

    /// Get operator precedence (higher number = higher precedence)
    pub fn precedence(&self) -> u8 {
        match self {
            Operator::Assign | Operator::PlusAssign | Operator::MinusAssign | Operator::MultiplyAssign | Operator::DivideAssign | Operator::ModuloAssign => 1,

            Operator::Or => 2,
            Operator::And => 3,
            Operator::BitOr => 4,
            Operator::BitXor => 5,
            Operator::BitAnd => 6,

            Operator::Equal | Operator::NotEqual => 7,

            Operator::Less | Operator::LessEqual | Operator::Greater | Operator::GreaterEqual => 8,

            Operator::LeftShift | Operator::RightShift => 9,
            Operator::Plus | Operator::Minus => 10,
            Operator::Multiply | Operator::Divide | Operator::Modulo => 11,
            Operator::Power => 12,

            Operator::Not | Operator::BitNot => 13,
            Operator::Dot | Operator::DoubleColon => 14,

            _ => 0,
        }
    }

    /// Check if this operator is left-associative
    pub fn is_left_associative(&self) -> bool {
        !matches!(
            self,
            Operator::Power | Operator::Assign | Operator::PlusAssign | Operator::MinusAssign | Operator::MultiplyAssign | Operator::DivideAssign | Operator::ModuloAssign
        )
    }
}

/// Delimiters in the DotVM language
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Delimiter {
    // Parentheses
    LeftParen,
    RightParen,

    // Brackets
    LeftBracket,
    RightBracket,

    // Braces
    LeftBrace,
    RightBrace,

    // Other
    Comma,
    Semicolon,
    Colon,
}

impl fmt::Display for Delimiter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Delimiter::LeftParen => "(",
            Delimiter::RightParen => ")",
            Delimiter::LeftBracket => "[",
            Delimiter::RightBracket => "]",
            Delimiter::LeftBrace => "{",
            Delimiter::RightBrace => "}",
            Delimiter::Comma => ",",
            Delimiter::Semicolon => ";",
            Delimiter::Colon => ":",
        };
        write!(f, "{}", s)
    }
}

impl Delimiter {
    /// Get delimiter from character
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '(' => Some(Delimiter::LeftParen),
            ')' => Some(Delimiter::RightParen),
            '[' => Some(Delimiter::LeftBracket),
            ']' => Some(Delimiter::RightBracket),
            '{' => Some(Delimiter::LeftBrace),
            '}' => Some(Delimiter::RightBrace),
            ',' => Some(Delimiter::Comma),
            ';' => Some(Delimiter::Semicolon),
            ':' => Some(Delimiter::Colon),
            _ => None,
        }
    }

    /// Check if this delimiter opens a group
    pub fn is_opening(&self) -> bool {
        matches!(self, Delimiter::LeftParen | Delimiter::LeftBracket | Delimiter::LeftBrace)
    }

    /// Check if this delimiter closes a group
    pub fn is_closing(&self) -> bool {
        matches!(self, Delimiter::RightParen | Delimiter::RightBracket | Delimiter::RightBrace)
    }

    /// Get the matching delimiter
    pub fn matching(&self) -> Option<Self> {
        match self {
            Delimiter::LeftParen => Some(Delimiter::RightParen),
            Delimiter::RightParen => Some(Delimiter::LeftParen),
            Delimiter::LeftBracket => Some(Delimiter::RightBracket),
            Delimiter::RightBracket => Some(Delimiter::LeftBracket),
            Delimiter::LeftBrace => Some(Delimiter::RightBrace),
            Delimiter::RightBrace => Some(Delimiter::LeftBrace),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_creation() {
        let pos = Position::new(1, 1);
        let span = Span::single(pos);
        let token = Token::new(TokenType::Keyword(Keyword::Let), "let".to_string(), span);

        assert!(token.is_keyword());
        assert!(!token.is_operator());
        assert_eq!(token.as_keyword(), Some(&Keyword::Let));
    }

    #[test]
    fn test_keyword_from_str() {
        assert_eq!(Keyword::from_str("if"), Some(Keyword::If));
        assert_eq!(Keyword::from_str("let"), Some(Keyword::Let));
        assert_eq!(Keyword::from_str("i32"), Some(Keyword::I32));
        assert_eq!(Keyword::from_str("invalid"), None);
    }

    #[test]
    fn test_keyword_categories() {
        assert!(Keyword::I32.is_type_keyword());
        assert!(!Keyword::If.is_type_keyword());

        assert!(Keyword::If.is_control_flow());
        assert!(!Keyword::I32.is_control_flow());
    }

    #[test]
    fn test_operator_from_str() {
        assert_eq!(Operator::from_str("+"), Some(Operator::Plus));
        assert_eq!(Operator::from_str("=="), Some(Operator::Equal));
        assert_eq!(Operator::from_str("->"), Some(Operator::Arrow));
        assert_eq!(Operator::from_str("invalid"), None);
    }

    #[test]
    fn test_operator_precedence() {
        assert!(Operator::Multiply.precedence() > Operator::Plus.precedence());
        assert!(Operator::Plus.precedence() > Operator::Equal.precedence());
        assert!(Operator::Equal.precedence() > Operator::And.precedence());
    }

    #[test]
    fn test_operator_associativity() {
        assert!(Operator::Plus.is_left_associative());
        assert!(!Operator::Power.is_left_associative());
        assert!(!Operator::Assign.is_left_associative());
    }

    #[test]
    fn test_delimiter_from_char() {
        assert_eq!(Delimiter::from_char('('), Some(Delimiter::LeftParen));
        assert_eq!(Delimiter::from_char('}'), Some(Delimiter::RightBrace));
        assert_eq!(Delimiter::from_char(','), Some(Delimiter::Comma));
        assert_eq!(Delimiter::from_char('x'), None);
    }

    #[test]
    fn test_delimiter_matching() {
        assert_eq!(Delimiter::LeftParen.matching(), Some(Delimiter::RightParen));
        assert_eq!(Delimiter::RightBrace.matching(), Some(Delimiter::LeftBrace));
        assert_eq!(Delimiter::Comma.matching(), None);
    }

    #[test]
    fn test_delimiter_categories() {
        assert!(Delimiter::LeftParen.is_opening());
        assert!(!Delimiter::RightParen.is_opening());

        assert!(Delimiter::RightBrace.is_closing());
        assert!(!Delimiter::LeftBrace.is_closing());

        assert!(!Delimiter::Comma.is_opening());
        assert!(!Delimiter::Comma.is_closing());
    }

    #[test]
    fn test_token_display() {
        let pos = Position::new(1, 1);
        let span = Span::single(pos);
        let token = Token::new(TokenType::IntegerLiteral(42), "42".to_string(), span);

        assert_eq!(format!("{}", token), "integer '42'");
    }

    #[test]
    fn test_token_type_display() {
        assert_eq!(format!("{}", TokenType::IntegerLiteral(42)), "integer");
        assert_eq!(format!("{}", TokenType::Keyword(Keyword::Let)), "keyword 'let'");
        assert_eq!(format!("{}", TokenType::Operator(Operator::Plus)), "operator '+'");
    }
}
