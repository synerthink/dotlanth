use crate::core::dotlang::{ast::ast::AstNode, lexer::lexer::Token};

use super::statement::parse_statement;

pub fn parse(tokens: &[Token]) -> AstNode {
    let mut current = 0;
    let mut nodes = Vec::new();

    while current < tokens.len() && tokens[current] != Token::EOF {
        nodes.push(parse_statement(tokens, &mut current));
    }

    AstNode::Block(nodes)
}
