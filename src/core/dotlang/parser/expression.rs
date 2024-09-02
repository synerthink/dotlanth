use crate::core::dotlang::{ast::ast::AstNode, lexer::lexer::Token};

use super::errors::parser_error::ParserError;

pub fn parse_expression(tokens: &[Token], current: &mut usize) -> Result<AstNode, ParserError> {
    match tokens.get(*current) {
        Some(Token::Number(_)) | Some(Token::Identifier(_)) | Some(Token::LeftParen) => {
            parse_binary_op(tokens, current, 0)
        }
        Some(_) => Err(ParserError::UnexpectedToken),
        None => Err(ParserError::ExpectedExpression),
    }
}

fn parse_binary_op(
    tokens: &[Token],
    current: &mut usize,
    precedence: u8,
) -> Result<AstNode, ParserError> {
    let mut left = parse_primary(tokens, current)?;
    while let Some(op_prec) = get_precedence(tokens.get(*current)) {
        if op_prec < precedence {
            break;
        }
        if let Some(op) = get_binary_op(tokens.get(*current)) {
            *current += 1;
            let right = parse_binary_op(tokens, current, op_prec + 1)?;
            left = AstNode::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
    }
    Ok(left)
}

fn parse_primary(tokens: &[Token], current: &mut usize) -> Result<AstNode, ParserError> {
    match tokens.get(*current) {
        Some(Token::Number(value)) => {
            *current += 1;
            Ok(AstNode::Number(*value))
        }
        Some(Token::Identifier(name)) => {
            *current += 1;
            if let Some(Token::LeftParen) = tokens.get(*current) {
                parse_function_call(tokens, current, name.clone())
            } else {
                Ok(AstNode::Identifier(name.clone()))
            }
        }
        Some(Token::LeftParen) => {
            *current += 1;
            let expr = parse_expression(tokens, current)?;
            if let Some(Token::RightParen) = tokens.get(*current) {
                *current += 1;
                Ok(expr)
            } else {
                Err(ParserError::ExpectedRightParen)
            }
        }
        Some(_) => Err(ParserError::UnexpectedToken),
        None => Err(ParserError::ExpectedExpression),
    }
}

fn parse_function_call(
    tokens: &[Token],
    current: &mut usize,
    name: String,
) -> Result<AstNode, ParserError> {
    *current += 1; // Consume the LeftParen
    let mut args = Vec::new();
    while let Some(token) = tokens.get(*current) {
        if let Token::RightParen = token {
            break;
        }
        if let Token::Comma = token {
            *current += 1;
            continue;
        }
        args.push(parse_expression(tokens, current)?);
    }
    if let Some(Token::RightParen) = tokens.get(*current) {
        *current += 1;
        Ok(AstNode::FunctionCall { name, args })
    } else {
        Err(ParserError::ExpectedRightParen)
    }
}

fn get_precedence(token: Option<&Token>) -> Option<u8> {
    match token {
        Some(Token::Plus) | Some(Token::Minus) => Some(1),
        Some(Token::Asterisk) | Some(Token::Slash) => Some(2),
        _ => None,
    }
}

fn get_binary_op(token: Option<&Token>) -> Option<String> {
    match token {
        Some(Token::Plus) => Some("+".to_string()),
        Some(Token::Minus) => Some("-".to_string()),
        Some(Token::Asterisk) => Some("*".to_string()),
        Some(Token::Slash) => Some("/".to_string()),
        _ => None,
    }
}
