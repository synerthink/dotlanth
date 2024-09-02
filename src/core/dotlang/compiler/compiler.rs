use crate::core::dotlang::{
    bytecode::bytecode::generate_bytecode,
    lexer::lexer::tokenize,
    parser::parser::parse,
};

use super::errors::CompilerError;

/// The DotLangCompiler is responsible for orchestrating the compilation process.
/// It compiles source code written in dotLang into bytecode that can be executed by the execution engine.
pub struct DotLangCompiler;

impl DotLangCompiler {
    /// Compiles DotLang source code into bytecode.
    ///
    /// # Arguments
    ///
    /// * `source` - A string slice containing the DotLang source code.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, CompilerError>` - The compiled bytecode or an error if the compilation fails.
    pub fn compile(&self, source: &str) -> Result<Vec<u8>, CompilerError> {
        let tokens = tokenize(source);

        let ast = parse(&tokens).map_err(|e| CompilerError::ParserError(e.to_string()))?;

        let bytecode = generate_bytecode(&ast)?;

        Ok(bytecode)
    }
}