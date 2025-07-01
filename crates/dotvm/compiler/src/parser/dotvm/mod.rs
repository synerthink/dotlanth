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

//! DotVM-specific parsing implementation

pub mod lexer;
pub mod syntax_parser;
pub mod semantic_analyzer;

pub use lexer::DotVMLexer;
pub use syntax_parser::{DotVMSyntaxParser, DotVMAst, AstNode};
pub use semantic_analyzer::DotVMSemanticAnalyzer;

use super::common::{ParseResult, ParseContext};
use super::traits::Parser;

/// Main DotVM parser that coordinates lexical, syntax, and semantic analysis
pub struct DotVMParser {
    lexer: DotVMLexer,
    syntax_parser: DotVMSyntaxParser,
    semantic_analyzer: DotVMSemanticAnalyzer,
    context: ParseContext,
}

impl DotVMParser {
    /// Create a new DotVM parser
    pub fn new(context: ParseContext) -> Self {
        Self {
            lexer: DotVMLexer::new(),
            syntax_parser: DotVMSyntaxParser::new(),
            semantic_analyzer: DotVMSemanticAnalyzer::new(),
            context,
        }
    }

    /// Parse DotVM source code into a validated AST
    pub fn parse_program(&mut self, input: &str) -> ParseResult<DotVMAst> {
        // Step 1: Lexical analysis
        let tokens = self.lexer.tokenize(input)?;
        
        // Step 2: Syntax analysis
        let mut ast = self.syntax_parser.parse_tokens(&tokens)?;
        
        // Step 3: Semantic analysis
        self.semantic_analyzer.analyze(&ast)?;
        
        // Step 4: Apply any transformations
        ast = self.apply_transformations(ast)?;
        
        Ok(ast)
    }

    /// Apply AST transformations
    fn apply_transformations(&mut self, ast: DotVMAst) -> ParseResult<DotVMAst> {
        // TODO: Implement AST transformations like:
        // - Constant folding
        // - Dead code elimination
        // - Type inference
        Ok(ast)
    }

    /// Get the current parsing context
    pub fn context(&self) -> &ParseContext {
        &self.context
    }

    /// Update the parsing context
    pub fn set_context(&mut self, context: ParseContext) {
        self.context = context;
    }
}

impl Parser<DotVMAst> for DotVMParser {
    fn parse(&mut self, input: &str) -> ParseResult<DotVMAst> {
        self.parse_program(input)
    }

    fn position(&self) -> super::common::Position {
        self.lexer.position()
    }

    fn is_at_end(&self) -> bool {
        self.lexer.is_at_end()
    }

    fn reset(&mut self) {
        self.lexer.reset();
        self.syntax_parser.reset();
        self.semantic_analyzer.reset();
    }

    fn name(&self) -> &'static str {
        "DotVMParser"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::ParserConfig;

    #[test]
    fn test_dotvm_parser_creation() {
        let context = ParseContext::new("test.dvm".to_string(), "".to_string());
        let parser = DotVMParser::new(context);
        assert_eq!(parser.name(), "DotVMParser");
    }

    #[test]
    fn test_dotvm_parser_empty_input() {
        let context = ParseContext::new("test.dvm".to_string(), "".to_string());
        let mut parser = DotVMParser::new(context);
        
        // Empty input should parse successfully (empty program)
        let result = parser.parse("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_dotvm_parser_simple_program() {
        let context = ParseContext::new("test.dvm".to_string(), "".to_string());
        let mut parser = DotVMParser::new(context);
        
        let input = "let x: i32 = 42;";
        let result = parser.parse(input);
        assert!(result.is_ok());
    }
}