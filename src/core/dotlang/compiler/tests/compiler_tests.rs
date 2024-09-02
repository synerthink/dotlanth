#[cfg(test)]
mod tests {
    use crate::core::dotlang::bytecode::bytecode::hash_name_to_address;
    use crate::core::dotlang::compiler::compiler::DotLangCompiler;
    use crate::core::dotlang::compiler::errors::CompilerError;
    use crate::core::execution_engine::opcodes::opcode::Opcode;

    fn setup_compiler() -> DotLangCompiler {
        DotLangCompiler
    }

    #[test]
    fn test_compile_number() {
        let compiler = setup_compiler();
        let source = "42;";
        let bytecode = compiler.compile(source).unwrap();
        assert_eq!(
            bytecode,
            vec![Opcode::LoadNumber.into(), 42, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_compile_binary_op() {
        let compiler = setup_compiler();
        let source = "10 + 5;";
        let bytecode = compiler.compile(source).unwrap();
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
    fn test_compile_variable_declaration() {
        let compiler = setup_compiler();
        let source = "let x = 100;";
        let bytecode = compiler.compile(source).unwrap();
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
    fn test_compile_identifier() {
        let compiler = setup_compiler();
        let source = "y;";
        let bytecode = compiler.compile(source).unwrap();
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
    fn test_compile_function_call() {
        let compiler = setup_compiler();
        let source = "print(42);";
        let bytecode = compiler.compile(source).unwrap();
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
    fn test_compile_if_statement() {
        let compiler = setup_compiler();
        let source = "if (1) { 10; }";
        let bytecode = compiler.compile(source).unwrap();
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
    fn test_compile_while_loop() {
        let compiler = setup_compiler();
        let source = "while (1) { 42; }";
        let bytecode = compiler.compile(source).unwrap();
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
    fn test_compile_invalid_syntax() {
        let compiler = setup_compiler();
        let source = "let x = ;";
        let result = compiler.compile(source);
        assert!(result.is_err());
        if let Err(CompilerError::ParserError(msg)) = result {
            assert_eq!(msg, "Expected expression after '='");
        } else {
            panic!("Expected ParseError");
        }
    }

    #[test]
    fn test_compile_unsupported_node() {
        let compiler = setup_compiler();
        let source = "fn test() { return 0; }";
        let result = compiler.compile(source);
        assert!(result.is_err());
        if let Err(CompilerError::BytecodeError(msg)) = result {
            assert_eq!(msg, "Unsupported AST node");
        } else {
            panic!("Expected BytecodeError");
        }
    }
}
