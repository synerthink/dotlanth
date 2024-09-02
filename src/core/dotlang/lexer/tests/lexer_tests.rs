#[cfg(test)]
mod tests {
    use crate::core::dotlang::lexer::lexer::{tokenize, Token};

    #[test]
    fn test_tokenize_empty_input() {
        let input = "";
        let tokens = tokenize(input);
        assert_eq!(tokens, vec![Token::EOF]);
    }

    #[test]
    fn test_tokenize_whitespace() {
        let input = "   \t\n\r  ";
        let tokens = tokenize(input);
        assert_eq!(tokens, vec![Token::EOF]);
    }

    #[test]
    fn test_tokenize_single_char_tokens() {
        let input = "+ - * / ; ( ) { } . =";
        let tokens = tokenize(input);
        assert_eq!(
            tokens,
            vec![
                Token::Plus,
                Token::Minus,
                Token::Asterisk,
                Token::Slash,
                Token::Semicolon,
                Token::LeftParen,
                Token::RightParen,
                Token::LeftBrace,
                Token::RightBrace,
                Token::Comma,
                Token::Equal,
                Token::EOF
            ]
        );
    }

    #[test]
    fn test_tokenize_numbers() {
        let input = "123 456 789";
        let tokens = tokenize(input);
        assert_eq!(
            tokens,
            vec![
                Token::Number(123),
                Token::Number(456),
                Token::Number(789),
                Token::EOF
            ]
        );
    }

    #[test]
    fn test_tokenize_identifiers() {
        let input = "abc x_y_z _123";
        let tokens = tokenize(input);
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("abc".to_string()),
                Token::Identifier("x_y_z".to_string()),
                Token::Identifier("_123".to_string()),
                Token::EOF
            ]
        );
    }

    #[test]
    fn test_tokenize_keywords() {
        let input = "let fn if else while";
        let tokens = tokenize(input);
        assert_eq!(tokens, vec![Token::Let, Token::Fn, Token::If, Token::Else, Token::While, Token::EOF]);
    }

    #[test]
    fn test_tokenize_mixed_input() {
        let input = "let x = 5 + 3; fn add(a, b) { a + b }";
        let tokens = tokenize(input);
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Identifier("x".to_string()),
                Token::Equal,
                Token::Number(5),
                Token::Plus,
                Token::Number(3),
                Token::Semicolon,
                Token::Fn,
                Token::Identifier("add".to_string()),
                Token::LeftParen,
                Token::Identifier("a".to_string()),
                Token::Identifier("b".to_string()),
                Token::RightParen,
                Token::LeftBrace,
                Token::Identifier("a".to_string()),
                Token::Plus,
                Token::Identifier("b".to_string()),
                Token::RightBrace,
                Token::EOF
            ]
        );
    }

    #[test]
    fn test_tokenize_ignore_invalid_chars() {
        let input = "abc!@#def";
        let tokens = tokenize(input);
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("abc".to_string()),
                Token::Identifier("def".to_string()),
                Token::EOF
            ]
        );
    }
}
