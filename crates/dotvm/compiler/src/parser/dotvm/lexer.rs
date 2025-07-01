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

//! DotVM lexical analyzer (tokenizer)

use crate::parser::common::{
    ParseError, ParseErrorKind, ParseResult, Position, Token, TokenType,
    Keyword, Operator, Delimiter, position::{Span, PositionTracker}, utils,
};
use crate::parser::traits::Lexer;

/// DotVM lexical analyzer
pub struct DotVMLexer {
    /// Position tracker for the source
    tracker: Option<PositionTracker>,
    /// Current tokens (for peek functionality)
    current_token: Option<Token>,
    /// Whether we've reached the end
    at_end: bool,
}

impl DotVMLexer {
    /// Create a new DotVM lexer
    pub fn new() -> Self {
        Self {
            tracker: None,
            current_token: None,
            at_end: false,
        }
    }

    /// Tokenize the entire input
    pub fn tokenize(&mut self, input: &str) -> ParseResult<Vec<Token>> {
        self.tracker = Some(PositionTracker::new(input.to_string()));
        self.at_end = false;
        self.current_token = None;

        let mut tokens = Vec::new();
        
        loop {
            let token = self.next_token()?;
            let is_eof = matches!(token.token_type, TokenType::Eof);
            tokens.push(token);
            
            if is_eof {
                break;
            }
        }
        
        Ok(tokens)
    }

    /// Scan the next token from input
    fn scan_token(&mut self) -> ParseResult<Token> {
        let tracker = self.tracker.as_mut().unwrap();
        
        // Skip whitespace
        tracker.skip_whitespace();
        
        let start_pos = tracker.position();
        
        if tracker.is_at_end() {
            return Ok(Token::new(
                TokenType::Eof,
                "".to_string(),
                Span::single(start_pos),
            ));
        }

        let ch = tracker.next_char().unwrap();
        
        match ch {
            // Single-character tokens
            '(' => self.make_token(TokenType::Delimiter(Delimiter::LeftParen), ch.to_string(), start_pos),
            ')' => self.make_token(TokenType::Delimiter(Delimiter::RightParen), ch.to_string(), start_pos),
            '[' => self.make_token(TokenType::Delimiter(Delimiter::LeftBracket), ch.to_string(), start_pos),
            ']' => self.make_token(TokenType::Delimiter(Delimiter::RightBracket), ch.to_string(), start_pos),
            '{' => self.make_token(TokenType::Delimiter(Delimiter::LeftBrace), ch.to_string(), start_pos),
            '}' => self.make_token(TokenType::Delimiter(Delimiter::RightBrace), ch.to_string(), start_pos),
            ',' => self.make_token(TokenType::Delimiter(Delimiter::Comma), ch.to_string(), start_pos),
            ';' => self.make_token(TokenType::Delimiter(Delimiter::Semicolon), ch.to_string(), start_pos),
            
            // Operators (may be multi-character)
            '+' => self.scan_operator_starting_with('+', start_pos),
            '-' => self.scan_operator_starting_with('-', start_pos),
            '*' => self.scan_operator_starting_with('*', start_pos),
            '/' => self.scan_slash_or_comment(start_pos),
            '%' => self.scan_operator_starting_with('%', start_pos),
            '=' => self.scan_operator_starting_with('=', start_pos),
            '!' => self.scan_operator_starting_with('!', start_pos),
            '<' => self.scan_operator_starting_with('<', start_pos),
            '>' => self.scan_operator_starting_with('>', start_pos),
            '&' => self.scan_operator_starting_with('&', start_pos),
            '|' => self.scan_operator_starting_with('|', start_pos),
            '^' => self.make_token(TokenType::Operator(Operator::BitXor), ch.to_string(), start_pos),
            '~' => self.make_token(TokenType::Operator(Operator::BitNot), ch.to_string(), start_pos),
            '?' => self.make_token(TokenType::Operator(Operator::Question), ch.to_string(), start_pos),
            
            // Dot and ranges
            '.' => self.scan_dot_or_range(start_pos),
            
            // Colon
            ':' => self.scan_colon(start_pos),
            
            // String literals
            '"' => self.scan_string_literal(start_pos),
            '\'' => self.scan_char_literal(start_pos),
            
            // Numbers
            c if c.is_ascii_digit() => {
                // Put the character back and scan number
                let mut pos = tracker.position();
                pos.column -= 1;
                tracker.seek_to(tracker.byte_offset() - ch.len_utf8()).unwrap();
                self.scan_number(start_pos)
            }
            
            // Identifiers and keywords
            c if utils::is_identifier_start(c) => {
                // Put the character back and scan identifier
                let mut pos = tracker.position();
                pos.column -= 1;
                tracker.seek_to(tracker.byte_offset() - ch.len_utf8()).unwrap();
                self.scan_identifier_or_keyword(start_pos)
            }
            
            // Invalid character
            _ => Err(ParseError::new(
                ParseErrorKind::InvalidCharacter,
                start_pos,
                format!("Unexpected character '{}'", ch),
            )),
        }
    }

    /// Create a token with proper span
    fn make_token(&self, token_type: TokenType, lexeme: String, start_pos: Position) -> ParseResult<Token> {
        let tracker = self.tracker.as_ref().unwrap();
        let end_pos = tracker.position();
        let span = Span::new(start_pos, end_pos);
        Ok(Token::new(token_type, lexeme, span))
    }

    /// Scan operators that may have multiple characters
    fn scan_operator_starting_with(&mut self, first_char: char, start_pos: Position) -> ParseResult<Token> {
        let tracker = self.tracker.as_mut().unwrap();
        let mut lexeme = first_char.to_string();
        
        let token_type = match first_char {
            '+' => {
                if tracker.peek_char() == Some('=') {
                    tracker.next_char();
                    lexeme.push('=');
                    TokenType::Operator(Operator::PlusAssign)
                } else {
                    TokenType::Operator(Operator::Plus)
                }
            }
            '-' => {
                match tracker.peek_char() {
                    Some('=') => {
                        tracker.next_char();
                        lexeme.push('=');
                        TokenType::Operator(Operator::MinusAssign)
                    }
                    Some('>') => {
                        tracker.next_char();
                        lexeme.push('>');
                        TokenType::Operator(Operator::Arrow)
                    }
                    _ => TokenType::Operator(Operator::Minus),
                }
            }
            '*' => {
                match tracker.peek_char() {
                    Some('=') => {
                        tracker.next_char();
                        lexeme.push('=');
                        TokenType::Operator(Operator::MultiplyAssign)
                    }
                    Some('*') => {
                        tracker.next_char();
                        lexeme.push('*');
                        TokenType::Operator(Operator::Power)
                    }
                    _ => TokenType::Operator(Operator::Multiply),
                }
            }
            '%' => {
                if tracker.peek_char() == Some('=') {
                    tracker.next_char();
                    lexeme.push('=');
                    TokenType::Operator(Operator::ModuloAssign)
                } else {
                    TokenType::Operator(Operator::Modulo)
                }
            }
            '=' => {
                match tracker.peek_char() {
                    Some('=') => {
                        tracker.next_char();
                        lexeme.push('=');
                        TokenType::Operator(Operator::Equal)
                    }
                    Some('>') => {
                        tracker.next_char();
                        lexeme.push('>');
                        TokenType::Operator(Operator::FatArrow)
                    }
                    _ => TokenType::Operator(Operator::Assign),
                }
            }
            '!' => {
                if tracker.peek_char() == Some('=') {
                    tracker.next_char();
                    lexeme.push('=');
                    TokenType::Operator(Operator::NotEqual)
                } else {
                    TokenType::Operator(Operator::Not)
                }
            }
            '<' => {
                match tracker.peek_char() {
                    Some('=') => {
                        tracker.next_char();
                        lexeme.push('=');
                        TokenType::Operator(Operator::LessEqual)
                    }
                    Some('<') => {
                        tracker.next_char();
                        lexeme.push('<');
                        TokenType::Operator(Operator::LeftShift)
                    }
                    _ => TokenType::Operator(Operator::Less),
                }
            }
            '>' => {
                match tracker.peek_char() {
                    Some('=') => {
                        tracker.next_char();
                        lexeme.push('=');
                        TokenType::Operator(Operator::GreaterEqual)
                    }
                    Some('>') => {
                        tracker.next_char();
                        lexeme.push('>');
                        TokenType::Operator(Operator::RightShift)
                    }
                    _ => TokenType::Operator(Operator::Greater),
                }
            }
            '&' => {
                if tracker.peek_char() == Some('&') {
                    tracker.next_char();
                    lexeme.push('&');
                    TokenType::Operator(Operator::And)
                } else {
                    TokenType::Operator(Operator::BitAnd)
                }
            }
            '|' => {
                if tracker.peek_char() == Some('|') {
                    tracker.next_char();
                    lexeme.push('|');
                    TokenType::Operator(Operator::Or)
                } else {
                    TokenType::Operator(Operator::BitOr)
                }
            }
            _ => unreachable!(),
        };
        
        self.make_token(token_type, lexeme, start_pos)
    }

    /// Scan slash or comment
    fn scan_slash_or_comment(&mut self, start_pos: Position) -> ParseResult<Token> {
        let tracker = self.tracker.as_mut().unwrap();
        
        match tracker.peek_char() {
            Some('/') => {
                // Line comment
                tracker.next_char(); // consume second '/'
                let mut comment = String::new();
                
                while let Some(ch) = tracker.peek_char() {
                    if ch == '\n' {
                        break;
                    }
                    comment.push(tracker.next_char().unwrap());
                }
                
                self.make_token(TokenType::Comment(comment), format!("//{}", comment), start_pos)
            }
            Some('*') => {
                // Block comment
                tracker.next_char(); // consume '*'
                let mut comment = String::new();
                let mut depth = 1;
                
                while depth > 0 && !tracker.is_at_end() {
                    let ch = tracker.next_char().unwrap();
                    comment.push(ch);
                    
                    if ch == '/' && tracker.peek_char() == Some('*') {
                        tracker.next_char();
                        comment.push('*');
                        depth += 1;
                    } else if ch == '*' && tracker.peek_char() == Some('/') {
                        tracker.next_char();
                        comment.push('/');
                        depth -= 1;
                    }
                }
                
                if depth > 0 {
                    return Err(ParseError::new(
                        ParseErrorKind::UnterminatedComment,
                        start_pos,
                        "Unterminated block comment".to_string(),
                    ));
                }
                
                self.make_token(TokenType::Comment(comment), format!("/*{}*/", comment), start_pos)
            }
            Some('=') => {
                // /= operator
                tracker.next_char();
                self.make_token(TokenType::Operator(Operator::DivideAssign), "/=".to_string(), start_pos)
            }
            _ => {
                // Just division
                self.make_token(TokenType::Operator(Operator::Divide), "/".to_string(), start_pos)
            }
        }
    }

    /// Scan dot or range operators
    fn scan_dot_or_range(&mut self, start_pos: Position) -> ParseResult<Token> {
        let tracker = self.tracker.as_mut().unwrap();
        
        match tracker.peek_char() {
            Some('.') => {
                tracker.next_char();
                if tracker.peek_char() == Some('=') {
                    tracker.next_char();
                    self.make_token(TokenType::Operator(Operator::RangeInclusive), "..=".to_string(), start_pos)
                } else {
                    self.make_token(TokenType::Operator(Operator::Range), "..".to_string(), start_pos)
                }
            }
            _ => self.make_token(TokenType::Operator(Operator::Dot), ".".to_string(), start_pos),
        }
    }

    /// Scan colon or double colon
    fn scan_colon(&mut self, start_pos: Position) -> ParseResult<Token> {
        let tracker = self.tracker.as_mut().unwrap();
        
        if tracker.peek_char() == Some(':') {
            tracker.next_char();
            self.make_token(TokenType::Operator(Operator::DoubleColon), "::".to_string(), start_pos)
        } else {
            self.make_token(TokenType::Delimiter(Delimiter::Colon), ":".to_string(), start_pos)
        }
    }

    /// Scan string literal
    fn scan_string_literal(&mut self, start_pos: Position) -> ParseResult<Token> {
        let tracker = self.tracker.as_mut().unwrap();
        let mut value = String::new();
        let mut lexeme = String::from("\"");
        
        while let Some(ch) = tracker.peek_char() {
            if ch == '"' {
                tracker.next_char();
                lexeme.push('"');
                break;
            } else if ch == '\\' {
                tracker.next_char();
                lexeme.push('\\');
                
                if let Some(escaped) = tracker.next_char() {
                    lexeme.push(escaped);
                    match escaped {
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        '\\' => value.push('\\'),
                        '"' => value.push('"'),
                        _ => {
                            return Err(ParseError::new(
                                ParseErrorKind::InvalidEscapeSequence,
                                tracker.position(),
                                format!("Invalid escape sequence: \\{}", escaped),
                            ));
                        }
                    }
                } else {
                    return Err(ParseError::new(
                        ParseErrorKind::UnterminatedString,
                        start_pos,
                        "Unterminated string literal".to_string(),
                    ));
                }
            } else {
                value.push(tracker.next_char().unwrap());
                lexeme.push(ch);
            }
        }
        
        if !lexeme.ends_with('"') {
            return Err(ParseError::new(
                ParseErrorKind::UnterminatedString,
                start_pos,
                "Unterminated string literal".to_string(),
            ));
        }
        
        self.make_token(TokenType::StringLiteral(value), lexeme, start_pos)
    }

    /// Scan character literal
    fn scan_char_literal(&mut self, start_pos: Position) -> ParseResult<Token> {
        let tracker = self.tracker.as_mut().unwrap();
        let mut lexeme = String::from("'");
        
        let ch = if let Some(ch) = tracker.next_char() {
            lexeme.push(ch);
            if ch == '\\' {
                // Escaped character
                if let Some(escaped) = tracker.next_char() {
                    lexeme.push(escaped);
                    match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '\\' => '\\',
                        '\'' => '\'',
                        _ => {
                            return Err(ParseError::new(
                                ParseErrorKind::InvalidEscapeSequence,
                                tracker.position(),
                                format!("Invalid escape sequence: \\{}", escaped),
                            ));
                        }
                    }
                } else {
                    return Err(ParseError::new(
                        ParseErrorKind::InvalidCharacter,
                        start_pos,
                        "Incomplete character literal".to_string(),
                    ));
                }
            } else {
                ch
            }
        } else {
            return Err(ParseError::new(
                ParseErrorKind::InvalidCharacter,
                start_pos,
                "Empty character literal".to_string(),
            ));
        };
        
        if tracker.next_char() != Some('\'') {
            return Err(ParseError::new(
                ParseErrorKind::InvalidCharacter,
                start_pos,
                "Unterminated character literal".to_string(),
            ));
        }
        lexeme.push('\'');
        
        self.make_token(TokenType::CharLiteral(ch), lexeme, start_pos)
    }

    /// Scan number literal
    fn scan_number(&mut self, start_pos: Position) -> ParseResult<Token> {
        let tracker = self.tracker.as_mut().unwrap();
        let mut lexeme = String::new();
        let mut is_float = false;
        
        // Scan integer part
        while let Some(ch) = tracker.peek_char() {
            if ch.is_ascii_digit() {
                lexeme.push(tracker.next_char().unwrap());
            } else {
                break;
            }
        }
        
        // Check for decimal point
        if tracker.peek_char() == Some('.') {
            // Look ahead to see if it's a decimal or range operator
            let next_chars = tracker.peek_chars(2);
            if next_chars.len() >= 2 && next_chars.chars().nth(1).unwrap().is_ascii_digit() {
                is_float = true;
                lexeme.push(tracker.next_char().unwrap()); // consume '.'
                
                // Scan fractional part
                while let Some(ch) = tracker.peek_char() {
                    if ch.is_ascii_digit() {
                        lexeme.push(tracker.next_char().unwrap());
                    } else {
                        break;
                    }
                }
            }
        }
        
        // Check for scientific notation
        if let Some(ch) = tracker.peek_char() {
            if ch == 'e' || ch == 'E' {
                is_float = true;
                lexeme.push(tracker.next_char().unwrap());
                
                // Optional sign
                if let Some(sign) = tracker.peek_char() {
                    if sign == '+' || sign == '-' {
                        lexeme.push(tracker.next_char().unwrap());
                    }
                }
                
                // Exponent digits
                let mut has_exponent_digits = false;
                while let Some(ch) = tracker.peek_char() {
                    if ch.is_ascii_digit() {
                        lexeme.push(tracker.next_char().unwrap());
                        has_exponent_digits = true;
                    } else {
                        break;
                    }
                }
                
                if !has_exponent_digits {
                    return Err(ParseError::new(
                        ParseErrorKind::InvalidNumber,
                        start_pos,
                        "Invalid number: missing exponent digits".to_string(),
                    ));
                }
            }
        }
        
        let token_type = if is_float {
            match lexeme.parse::<f64>() {
                Ok(value) => TokenType::FloatLiteral(value),
                Err(_) => {
                    return Err(ParseError::new(
                        ParseErrorKind::InvalidNumber,
                        start_pos,
                        format!("Invalid float literal: {}", lexeme),
                    ));
                }
            }
        } else {
            match lexeme.parse::<i64>() {
                Ok(value) => TokenType::IntegerLiteral(value),
                Err(_) => {
                    return Err(ParseError::new(
                        ParseErrorKind::InvalidNumber,
                        start_pos,
                        format!("Invalid integer literal: {}", lexeme),
                    ));
                }
            }
        };
        
        self.make_token(token_type, lexeme, start_pos)
    }

    /// Scan identifier or keyword
    fn scan_identifier_or_keyword(&mut self, start_pos: Position) -> ParseResult<Token> {
        let tracker = self.tracker.as_mut().unwrap();
        let mut lexeme = String::new();
        
        // First character (already validated as identifier start)
        if let Some(ch) = tracker.next_char() {
            lexeme.push(ch);
        }
        
        // Remaining characters
        while let Some(ch) = tracker.peek_char() {
            if utils::is_identifier_continue(ch) {
                lexeme.push(tracker.next_char().unwrap());
            } else {
                break;
            }
        }
        
        let token_type = if let Some(keyword) = Keyword::from_str(&lexeme) {
            match keyword {
                Keyword::True => TokenType::BooleanLiteral(true),
                Keyword::False => TokenType::BooleanLiteral(false),
                _ => TokenType::Keyword(keyword),
            }
        } else {
            TokenType::Identifier(lexeme.clone())
        };
        
        self.make_token(token_type, lexeme, start_pos)
    }
}

impl Lexer for DotVMLexer {
    fn next_token(&mut self) -> ParseResult<Token> {
        if let Some(token) = self.current_token.take() {
            return Ok(token);
        }
        
        if self.at_end {
            let pos = self.tracker.as_ref().map_or(Position::start(), |t| t.position());
            return Ok(Token::new(TokenType::Eof, "".to_string(), Span::single(pos)));
        }
        
        let token = self.scan_token()?;
        if matches!(token.token_type, TokenType::Eof) {
            self.at_end = true;
        }
        
        Ok(token)
    }

    fn peek_token(&self) -> ParseResult<Token> {
        if let Some(ref token) = self.current_token {
            Ok(token.clone())
        } else {
            // This is a bit tricky since we need to peek without consuming
            // For now, we'll return EOF if we don't have a current token
            let pos = self.tracker.as_ref().map_or(Position::start(), |t| t.position());
            Ok(Token::new(TokenType::Eof, "".to_string(), Span::single(pos)))
        }
    }

    fn has_more_tokens(&self) -> bool {
        !self.at_end
    }

    fn position(&self) -> Position {
        self.tracker.as_ref().map_or(Position::start(), |t| t.position())
    }

    fn skip_whitespace(&mut self) {
        if let Some(tracker) = self.tracker.as_mut() {
            tracker.skip_whitespace();
        }
    }

    fn reset(&mut self) {
        if let Some(tracker) = self.tracker.as_mut() {
            tracker.reset();
        }
        self.current_token = None;
        self.at_end = false;
    }
}

impl Default for DotVMLexer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_creation() {
        let lexer = DotVMLexer::new();
        assert!(lexer.tracker.is_none());
        assert!(!lexer.has_more_tokens());
    }

    #[test]
    fn test_tokenize_empty() {
        let mut lexer = DotVMLexer::new();
        let tokens = lexer.tokenize("").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].token_type, TokenType::Eof));
    }

    #[test]
    fn test_tokenize_simple_tokens() {
        let mut lexer = DotVMLexer::new();
        let tokens = lexer.tokenize("( ) [ ] { } , ;").unwrap();
        
        assert_eq!(tokens.len(), 9); // 8 delimiters + EOF
        assert!(matches!(tokens[0].token_type, TokenType::Delimiter(Delimiter::LeftParen)));
        assert!(matches!(tokens[1].token_type, TokenType::Delimiter(Delimiter::RightParen)));
        assert!(matches!(tokens[2].token_type, TokenType::Delimiter(Delimiter::LeftBracket)));
        assert!(matches!(tokens[3].token_type, TokenType::Delimiter(Delimiter::RightBracket)));
    }

    #[test]
    fn test_tokenize_operators() {
        let mut lexer = DotVMLexer::new();
        let tokens = lexer.tokenize("+ - * / % == != <= >= && ||").unwrap();
        
        // Should have operators + EOF
        assert!(tokens.len() > 10);
        assert!(matches!(tokens[0].token_type, TokenType::Operator(Operator::Plus)));
        assert!(matches!(tokens[1].token_type, TokenType::Operator(Operator::Minus)));
        assert!(matches!(tokens[2].token_type, TokenType::Operator(Operator::Multiply)));
    }

    #[test]
    fn test_tokenize_numbers() {
        let mut lexer = DotVMLexer::new();
        let tokens = lexer.tokenize("42 3.14 1e10").unwrap();
        
        assert_eq!(tokens.len(), 4); // 3 numbers + EOF
        assert!(matches!(tokens[0].token_type, TokenType::IntegerLiteral(42)));
        assert!(matches!(tokens[1].token_type, TokenType::FloatLiteral(_)));
        assert!(matches!(tokens[2].token_type, TokenType::FloatLiteral(_)));
    }

    #[test]
    fn test_tokenize_strings() {
        let mut lexer = DotVMLexer::new();
        let tokens = lexer.tokenize(r#""hello" "world\n""#).unwrap();
        
        assert_eq!(tokens.len(), 3); // 2 strings + EOF
        if let TokenType::StringLiteral(s) = &tokens[0].token_type {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_tokenize_identifiers_and_keywords() {
        let mut lexer = DotVMLexer::new();
        let tokens = lexer.tokenize("let x if true false").unwrap();
        
        assert_eq!(tokens.len(), 6); // 5 tokens + EOF
        assert!(matches!(tokens[0].token_type, TokenType::Keyword(Keyword::Let)));
        assert!(matches!(tokens[1].token_type, TokenType::Identifier(_)));
        assert!(matches!(tokens[2].token_type, TokenType::Keyword(Keyword::If)));
        assert!(matches!(tokens[3].token_type, TokenType::BooleanLiteral(true)));
        assert!(matches!(tokens[4].token_type, TokenType::BooleanLiteral(false)));
    }

    #[test]
    fn test_tokenize_comments() {
        let mut lexer = DotVMLexer::new();
        let tokens = lexer.tokenize("// line comment\n/* block comment */").unwrap();
        
        assert_eq!(tokens.len(), 3); // 2 comments + EOF
        assert!(matches!(tokens[0].token_type, TokenType::Comment(_)));
        assert!(matches!(tokens[1].token_type, TokenType::Comment(_)));
    }

    #[test]
    fn test_error_handling() {
        let mut lexer = DotVMLexer::new();
        
        // Unterminated string
        let result = lexer.tokenize(r#""unterminated"#);
        assert!(result.is_err());
        
        // Invalid character
        let result = lexer.tokenize("@");
        assert!(result.is_err());
    }
}