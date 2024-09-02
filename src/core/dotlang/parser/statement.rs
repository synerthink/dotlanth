use crate::core::dotlang::{ast::ast::AstNode, lexer::lexer::Token};

use super::{errors::parser_error::ParserError, expression::parse_expression};

pub fn parse_statement(tokens: &[Token], current: &mut usize) -> Result<AstNode, ParserError> {
    match tokens.get(*current) {
        Some(Token::Let) => parse_variable_declaration(tokens, current),
        Some(Token::If) => parse_if(tokens, current),
        Some(Token::While) => parse_while(tokens, current),
        Some(Token::Fn) => parse_function(tokens, current),
        Some(Token::Identifier(_)) => {
            // Check if it's a function call or assignment
            if tokens.get(*current + 1) == Some(&Token::LeftParen)
                || tokens.get(*current + 1) == Some(&Token::Equal)
            {
                parse_expression_statement(tokens, current)
            } else {
                Err(ParserError::UnknownStatementType)
            }
        }
        Some(Token::Number(_)) => parse_expression_statement(tokens, current),
        Some(_) => Err(ParserError::UnknownStatementType),
        None => Err(ParserError::UnexpectedEndOfInput),
    }
}

fn parse_expression_statement(
    tokens: &[Token],
    current: &mut usize,
) -> Result<AstNode, ParserError> {
    let expr = parse_expression(tokens, current)?;
    match tokens.get(*current) {
        Some(Token::Semicolon) => {
            *current += 1;
            Ok(expr)
        }
        _ => Err(ParserError::ExpectedSemicolon),
    }
}

fn parse_variable_declaration(
    tokens: &[Token],
    current: &mut usize,
) -> Result<AstNode, ParserError> {
    *current += 1; // Consume 'let'
    if let Some(Token::Identifier(name)) = tokens.get(*current) {
        *current += 1;
        expect_token(tokens, current, Token::Equal)?;

        // Check if there's an expression after the equals sign
        if tokens.get(*current) == Some(&Token::Semicolon) {
            return Err(ParserError::ExpectedExpression);
        }

        let value = parse_expression(tokens, current)?;
        expect_token(tokens, current, Token::Semicolon)?;
        Ok(AstNode::VariableDeclaration {
            name: name.clone(),
            value: Box::new(value),
        })
    } else {
        Err(ParserError::ExpectedVariableName)
    }
}

fn expect_token(tokens: &[Token], current: &mut usize, expected: Token) -> Result<(), ParserError> {
    if let Some(token) = tokens.get(*current) {
        if *token == expected {
            *current += 1;
            Ok(())
        } else {
            Err(ParserError::UnexpectedToken)
        }
    } else {
        Err(ParserError::UnexpectedEndOfInput)
    }
}

fn parse_function(tokens: &[Token], current: &mut usize) -> Result<AstNode, ParserError> {
    *current += 1; // Consume 'fn'
    if let Some(Token::Identifier(name)) = tokens.get(*current) {
        *current += 1;
        expect_token(tokens, current, Token::LeftParen)?;
        let mut params = Vec::new();
        while let Some(token) = tokens.get(*current) {
            match token {
                Token::RightParen => break,
                Token::Comma => *current += 1,
                Token::Identifier(param) => {
                    params.push(param.clone());
                    *current += 1;
                }
                _ => return Err(ParserError::ExpectedParameterName),
            }
        }
        expect_token(tokens, current, Token::RightParen)?;
        let body = parse_block(tokens, current)?;
        Ok(AstNode::Function {
            name: name.clone(),
            params,
            body: Box::new(body),
        })
    } else {
        Err(ParserError::ExpectedFunctionName)
    }
}

fn parse_if(tokens: &[Token], current: &mut usize) -> Result<AstNode, ParserError> {
    *current += 1; // Consume 'if'
    expect_token(tokens, current, Token::LeftParen)?;
    let condition = parse_expression(tokens, current)?;
    expect_token(tokens, current, Token::RightParen)?;
    let then_branch = parse_block(tokens, current)?;
    let else_branch = if let Some(Token::Else) = tokens.get(*current) {
        *current += 1;
        Some(Box::new(parse_block(tokens, current)?))
    } else {
        None
    };
    Ok(AstNode::If {
        condition: Box::new(condition),
        then_branch: Box::new(then_branch),
        else_branch,
    })
}

fn parse_while(tokens: &[Token], current: &mut usize) -> Result<AstNode, ParserError> {
    *current += 1; // Consume 'while'
    expect_token(tokens, current, Token::LeftParen)?;
    let condition = parse_expression(tokens, current)?;
    expect_token(tokens, current, Token::RightParen)?;
    let body = parse_block(tokens, current)?;
    Ok(AstNode::While {
        condition: Box::new(condition),
        body: Box::new(body),
    })
}

fn parse_block(tokens: &[Token], current: &mut usize) -> Result<AstNode, ParserError> {
    expect_token(tokens, current, Token::LeftBrace)?;
    let mut nodes = Vec::new();
    while let Some(token) = tokens.get(*current) {
        if let Token::RightBrace = token {
            *current += 1;
            break;
        }
        match parse_statement(tokens, current) {
            Ok(node) => nodes.push(node),
            Err(ParserError::UnknownStatementType) => {
                // Skip the unknown statement and continue parsing
                *current += 1;
                while let Some(token) = tokens.get(*current) {
                    if let Token::Semicolon = token {
                        *current += 1;
                        break;
                    }
                    *current += 1;
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(AstNode::Block(nodes))
}
