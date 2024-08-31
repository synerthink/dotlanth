use crate::core::dotlang::{ast::ast::AstNode, lexer::lexer::Token};

use super::expression::parse_expression;

pub fn parse_statement(tokens: &[Token], current: &mut usize) -> AstNode {
    match tokens.get(*current) {
        Some(Token::Let) => parse_variable_declaration(tokens, current),
        Some(Token::Fn) => parse_function(tokens, current),
        Some(Token::If) => parse_if(tokens, current),
        Some(Token::While) => parse_while(tokens, current),
        Some(Token::LeftBrace) => {
            *current += 1;
            parse_block(tokens, current)
        }
        _ => parse_expression(tokens, current),
    }
}

fn parse_variable_declaration(tokens: &[Token], current: &mut usize) -> AstNode {
    if let Token::Let = tokens[*current] {
        *current += 1;
        if let Token::Identifier(name) = &tokens[*current] {
            *current += 1;
            if let Token::Equal = tokens[*current] {
                *current += 1;
                let value = parse_expression(tokens, current);
                if let Token::Semicolon = tokens[*current] {
                    *current += 1;
                    return AstNode::VariableDeclaration {
                        name: name.clone(),
                        value: Box::new(value),
                    };
                }
            }
        }
    }
    AstNode::Number(0) // Fallback
}

fn parse_function(tokens: &[Token], current: &mut usize) -> AstNode {
    if let Token::Fn = tokens[*current] {
        *current += 1;
        if let Token::Identifier(name) = &tokens[*current] {
            *current += 1;
            if let Token::LeftParen = tokens[*current] {
                *current += 1;
                let mut params = Vec::new();
                while let Some(token) = tokens.get(*current) {
                    if let Token::RightParen = token {
                        break;
                    }
                    if let Token::Comma = token {
                        *current += 1;
                        continue;
                    }
                    if let Token::Identifier(param) = token {
                        params.push(param.clone());
                        *current += 1;
                    }
                }
                *current += 1; // Consume the RightParen
                if let Token::LeftBrace = tokens[*current] {
                    *current += 1;
                    let body = parse_block(tokens, current);
                    return AstNode::Function {
                        name: name.clone(),
                        params,
                        body: Box::new(body),
                    };
                }
            }
        }
    }
    AstNode::Number(0) // Fallback
}

fn parse_if(tokens: &[Token], current: &mut usize) -> AstNode {
    if let Token::If = tokens[*current] {
        *current += 1;
        let condition = parse_expression(tokens, current);
        if let Token::LeftBrace = tokens[*current] {
            *current += 1;
            let then_branch = parse_block(tokens, current);
            let else_branch = if let Some(Token::Else) = tokens.get(*current) {
                *current += 1;
                if let Token::LeftBrace = tokens[*current] {
                    *current += 1;
                    Some(parse_block(tokens, current))
                } else {
                    None
                }
            } else {
                None
            };
            return AstNode::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: else_branch.map(Box::new),
            };
        }
    }
    AstNode::Number(0) // Fallback
}

fn parse_while(tokens: &[Token], current: &mut usize) -> AstNode {
    if let Token::While = tokens[*current] {
        *current += 1;
        let condition = parse_expression(tokens, current);
        if let Token::LeftBrace = tokens[*current] {
            *current += 1;
            let body = parse_block(tokens, current);
            return AstNode::While {
                condition: Box::new(condition),
                body: Box::new(body),
            };
        }
    }
    AstNode::Number(0) // Fallback
}

fn parse_block(tokens: &[Token], current: &mut usize) -> AstNode {
    let mut nodes = Vec::new();
    while let Some(token) = tokens.get(*current) {
        if let Token::RightBrace = token {
            break;
        }
        nodes.push(parse_statement(tokens, current));
    }
    *current += 1; // Consume the RightBrace
    AstNode::Block(nodes)
}