#[derive(Debug)]
pub enum ParserError {
    UnexpectedEndOfInput,
    ExpectedVariableName,
    ExpectedEqual,
    ExpectedExpression,
    ExpectedSemicolon,
    UnknownStatementType,
    ExpectedParameterName,
    ExpectedLeftBrace,
    ExpectedLeftParen,
    ExpectedFunctionName,
    ExpectedRightParen,
    UnexpectedToken,
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParserError::UnexpectedEndOfInput => write!(f, "Unexpected end of input"),
            ParserError::ExpectedVariableName => write!(f, "Expected variable name after 'let'"),
            ParserError::ExpectedEqual => write!(f, "Expected '=' after variable name"),
            ParserError::ExpectedExpression => write!(f, "Expected expression after '='"),
            ParserError::ExpectedSemicolon => write!(f, "Expected ';' after variable declaration"),
            ParserError::UnknownStatementType => write!(f, "Unknown statement type"),
            ParserError::ExpectedParameterName => write!(f, "Expected parameter name"),
            ParserError::ExpectedLeftBrace => write!(f, "Expected left brace"),
            ParserError::ExpectedLeftParen => write!(f, "Expected left paren"),
            ParserError::ExpectedFunctionName => write!(f, "Expected function name"),
            ParserError::ExpectedRightParen => write!(f, "Expected right paren"),
            ParserError::UnexpectedToken => write!(f, "Unexpected token"),
        }
    }
}

impl std::error::Error for ParserError {}
