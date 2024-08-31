#[cfg(test)]
mod tests {
    use crate::core::dotlang::ast::ast::AstNode;

    #[test]
    fn test_number() {
        let node = AstNode::Number(42);
        assert_eq!(node, AstNode::Number(42));
    }

    #[test]
    fn test_identifier() {
        let node = AstNode::Identifier("x".to_string());
        assert_eq!(node, AstNode::Identifier("x".to_string()));
    }

    #[test]
    fn test_binary_op() {
        let node = AstNode::BinaryOp {
            left: Box::new(AstNode::Number(5)),
            op: "+".to_string(),
            right: Box::new(AstNode::Number(3)),
        };
        assert_eq!(
            node,
            AstNode::BinaryOp {
                left: Box::new(AstNode::Number(5)),
                op: "+".to_string(),
                right: Box::new(AstNode::Number(3)),
            }
        );
    }

    #[test]
    fn test_variable_declaration() {
        let node = AstNode::VariableDeclaration {
            name: "x".to_string(),
            value: Box::new(AstNode::Number(10)),
        };
        assert_eq!(
            node,
            AstNode::VariableDeclaration {
                name: "x".to_string(),
                value: Box::new(AstNode::Number(10)),
            }
        );
    }

    #[test]
    fn test_function() {
        let node = AstNode::Function {
            name: "add".to_string(),
            params: vec!["a".to_string(), "b".to_string()],
            body: Box::new(AstNode::BinaryOp {
                left: Box::new(AstNode::Identifier("a".to_string())),
                op: "+".to_string(),
                right: Box::new(AstNode::Identifier("b".to_string())),
            }),
        };
        assert_eq!(
            node,
            AstNode::Function {
                name: "add".to_string(),
                params: vec!["a".to_string(), "b".to_string()],
                body: Box::new(AstNode::BinaryOp {
                    left: Box::new(AstNode::Identifier("a".to_string())),
                    op: "+".to_string(),
                    right: Box::new(AstNode::Identifier("b".to_string())),
                }),
            }
        );
    }

    #[test]
    fn test_function_call() {
        let node = AstNode::FunctionCall {
            name: "add".to_string(),
            args: vec![AstNode::Number(5), AstNode::Number(3)],
        };
        assert_eq!(
            node,
            AstNode::FunctionCall {
                name: "add".to_string(),
                args: vec![AstNode::Number(5), AstNode::Number(3)],
            }
        );
    }

    #[test]
    fn test_if() {
        let node = AstNode::If {
            condition: Box::new(AstNode::BinaryOp {
                left: Box::new(AstNode::Identifier("x".to_string())),
                op: ">".to_string(),
                right: Box::new(AstNode::Number(0)),
            }),
            then_branch: Box::new(AstNode::Number(1)),
            else_branch: Some(Box::new(AstNode::Number(0))),
        };
        assert_eq!(
            node,
            AstNode::If {
                condition: Box::new(AstNode::BinaryOp {
                    left: Box::new(AstNode::Identifier("x".to_string())),
                    op: ">".to_string(),
                    right: Box::new(AstNode::Number(0)),
                }),
                then_branch: Box::new(AstNode::Number(1)),
                else_branch: Some(Box::new(AstNode::Number(0))),
            }
        );
    }

    #[test]
    fn test_while() {
        let node = AstNode::While {
            condition: Box::new(AstNode::BinaryOp {
                left: Box::new(AstNode::Identifier("x".to_string())),
                op: "<".to_string(),
                right: Box::new(AstNode::Number(10)),
            }),
            body: Box::new(AstNode::Block(vec![AstNode::BinaryOp {
                left: Box::new(AstNode::Identifier("x".to_string())),
                op: "+=".to_string(),
                right: Box::new(AstNode::Number(1)),
            }])),
        };
        assert_eq!(
            node,
            AstNode::While {
                condition: Box::new(AstNode::BinaryOp {
                    left: Box::new(AstNode::Identifier("x".to_string())),
                    op: "<".to_string(),
                    right: Box::new(AstNode::Number(10)),
                }),
                body: Box::new(AstNode::Block(vec![AstNode::BinaryOp {
                    left: Box::new(AstNode::Identifier("x".to_string())),
                    op: "+=".to_string(),
                    right: Box::new(AstNode::Number(1)),
                },])),
            }
        );
    }

    #[test]
    fn test_block() {
        let node = AstNode::Block(vec![
            AstNode::VariableDeclaration {
                name: "x".to_string(),
                value: Box::new(AstNode::Number(5)),
            },
            AstNode::VariableDeclaration {
                name: "y".to_string(),
                value: Box::new(AstNode::Number(10)),
            },
        ]);
        assert_eq!(
            node,
            AstNode::Block(vec![
                AstNode::VariableDeclaration {
                    name: "x".to_string(),
                    value: Box::new(AstNode::Number(5)),
                },
                AstNode::VariableDeclaration {
                    name: "y".to_string(),
                    value: Box::new(AstNode::Number(10)),
                },
            ])
        );
    }
}
