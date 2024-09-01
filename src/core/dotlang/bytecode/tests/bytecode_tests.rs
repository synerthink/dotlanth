#[cfg(test)]
mod tests {
    use crate::core::{dotlang::{
        ast::ast::AstNode,
        bytecode::bytecode::{generate_bytecode, hash_name_to_address},
        compiler::errors::CompilerError,
    }, execution_engine::opcodes::opcode::Opcode};

    #[test]
    fn test_generate_bytecode_number() {
        let ast = AstNode::Number(42);
        let bytecode = generate_bytecode(&ast).unwrap();
        assert_eq!(
            bytecode,
            vec![Opcode::LoadNumber.into(), 42, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_generate_bytecode_binary_op() {
        let ast = AstNode::BinaryOp {
            left: Box::new(AstNode::Number(10)),
            op: "+".to_string(),
            right: Box::new(AstNode::Number(5)),
        };
        let bytecode = generate_bytecode(&ast).unwrap();
        assert_eq!(
            bytecode,
            vec![
                Opcode::LoadNumber.into(),
                10,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                Opcode::LoadNumber.into(),
                5,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                Opcode::Add.into()
            ]
        );
    }

    #[test]
    fn test_generate_bytecode_variable_declaration() {
        let ast = AstNode::VariableDeclaration {
            name: "x".to_string(),
            value: Box::new(AstNode::Number(100)),
        };
        let bytecode = generate_bytecode(&ast).unwrap();
        let expected_address = hash_name_to_address("x");
        assert_eq!(
            bytecode,
            vec![
                Opcode::LoadNumber.into(),
                100,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                Opcode::StoreToMemory.into()
            ]
            .into_iter()
            .chain(expected_address.to_le_bytes().into_iter())
            .collect::<Vec<u8>>()
        );
    }

    #[test]
    fn test_generate_bytecode_identifier() {
        let ast = AstNode::Identifier("y".to_string());
        let bytecode = generate_bytecode(&ast).unwrap();
        let expected_address = hash_name_to_address("y");
        assert_eq!(
            bytecode,
            vec![Opcode::LoadFromMemory.into()]
                .into_iter()
                .chain(expected_address.to_le_bytes().into_iter())
                .collect::<Vec<u8>>()
        );
    }

    #[test]
    fn test_generate_bytecode_function_call() {
        let ast = AstNode::FunctionCall {
            name: "print".to_string(),
            args: vec![AstNode::Number(42)],
        };
        let bytecode = generate_bytecode(&ast).unwrap();
        assert_eq!(
            bytecode,
            vec![
                Opcode::LoadNumber.into(),
                42,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                Opcode::CallFunction.into(),
                5,
                b'p',
                b'r',
                b'i',
                b'n',
                b't'
            ]
        );
    }

    #[test]
    fn test_generate_bytecode_if_statement() {
        let ast = AstNode::If {
            condition: Box::new(AstNode::Number(1)),
            then_branch: Box::new(AstNode::Number(10)),
            else_branch: None,
        };
        let bytecode = generate_bytecode(&ast).unwrap();
        assert_eq!(
            bytecode,
            vec![
                Opcode::LoadNumber.into(),
                1,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                Opcode::JumpIf.into(),
                9,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                Opcode::LoadNumber.into(),
                10,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ]
        );
    }

    #[test]
    fn test_generate_bytecode_while_loop() {
        let ast = AstNode::While {
            condition: Box::new(AstNode::Number(1)),
            body: Box::new(AstNode::Number(42)),
        };
        let bytecode = generate_bytecode(&ast).unwrap();
        assert_eq!(
            bytecode,
            vec![
                Opcode::LoadNumber.into(),
                1,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                Opcode::JumpIf.into(),
                9,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                Opcode::LoadNumber.into(),
                42,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                Opcode::Jump.into(),
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ]
        );
    }

    #[test]
    fn test_hash_name_to_address() {
        assert_ne!(hash_name_to_address("x"), hash_name_to_address("y"));
        assert_eq!(hash_name_to_address("x"), hash_name_to_address("x"));
    }

    #[test]
    fn test_generate_bytecode_unsupported_node() {
        let ast = AstNode::Function {
            name: "test".to_string(),
            params: vec![],
            body: Box::new(AstNode::Number(0)),
        };
        let result = generate_bytecode(&ast);
        assert!(result.is_err());
        if let Err(CompilerError::BytecodeError(msg)) = result {
            assert_eq!(msg, "Unsupported AST node");
        } else {
            panic!("Expected BytecodeError");
        }
    }
}
