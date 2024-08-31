#[cfg(test)]
mod tests {
    use crate::core::dotlang::{ast::ast::AstNode, lexer::lexer::tokenize, parser::parser::parse};

    #[test]
    fn test_parse_variable_declaration() {
        let source = "let x = 5;";
        let tokens = tokenize(source);
        let ast = parse(&tokens);

        assert_eq!(
            ast,
            AstNode::Block(vec![AstNode::VariableDeclaration {
                name: "x".to_string(),
                value: Box::new(AstNode::Number(5)),
            }])
        );
    }

    #[test]
    fn test_parse_binary_op() {
        let source = "let x = 5 + 3;";
        let tokens = tokenize(source);
        let ast = parse(&tokens);

        assert_eq!(
            ast,
            AstNode::Block(vec![AstNode::VariableDeclaration {
                name: "x".to_string(),
                value: Box::new(AstNode::BinaryOp {
                    left: Box::new(AstNode::Number(5)),
                    op: "+".to_string(),
                    right: Box::new(AstNode::Number(3)),
                }),
            }])
        );
    }

    #[test]
    fn test_parse_function() {
        let source = "fn add(a, b) { let c = a + b; }";
        let tokens = tokenize(source);
        let ast = parse(&tokens);

        assert_eq!(
            ast,
            AstNode::Block(vec![AstNode::Function {
                name: "add".to_string(),
                params: vec!["a".to_string(), "b".to_string()],
                body: Box::new(AstNode::Block(vec![AstNode::VariableDeclaration {
                    name: "c".to_string(),
                    value: Box::new(AstNode::BinaryOp {
                        left: Box::new(AstNode::Identifier("a".to_string())),
                        op: "+".to_string(),
                        right: Box::new(AstNode::Identifier("b".to_string())),
                    }),
                }])),
            }])
        );
    }

    #[test]
    fn test_parse_if() {
        let source = "if (x) { let y = 1; } else { let z = 2; }";
        let tokens = tokenize(source);
        let ast = parse(&tokens);

        assert_eq!(
            ast,
            AstNode::Block(vec![AstNode::If {
                condition: Box::new(AstNode::Identifier("x".to_string())),
                then_branch: Box::new(AstNode::Block(vec![AstNode::VariableDeclaration {
                    name: "y".to_string(),
                    value: Box::new(AstNode::Number(1)),
                }])),
                else_branch: Some(Box::new(AstNode::Block(vec![
                    AstNode::VariableDeclaration {
                        name: "z".to_string(),
                        value: Box::new(AstNode::Number(2)),
                    }
                ]))),
            }])
        );
    }

    #[test]
    fn test_parse_while() {
        let source = "while (x) { let y = 1; }";
        let tokens = tokenize(source);
        let ast = parse(&tokens);

        assert_eq!(
            ast,
            AstNode::Block(vec![AstNode::While {
                condition: Box::new(AstNode::Identifier("x".to_string())),
                body: Box::new(AstNode::Block(vec![AstNode::VariableDeclaration {
                    name: "y".to_string(),
                    value: Box::new(AstNode::Number(1)),
                }])),
            }])
        );
    }
}
