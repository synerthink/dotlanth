#[derive(Debug)]
pub enum CompilerError {
    LexerError(String),
    ParserError(String),
    BytecodeError(String),
}

impl std::fmt::Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerError::LexerError(msg) => write!(f, "Lexer Error: {}", msg),
            CompilerError::ParserError(msg) => write!(f, "Parser Error: {}", msg),
            CompilerError::BytecodeError(msg) => write!(f, "Bytecode Error: {}", msg),
        }
    }
}

impl std::error::Error for CompilerError {}
