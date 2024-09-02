use crate::core::{
    dotlang::{ast::ast::AstNode, compiler::errors::CompilerError},
    execution_engine::opcodes::opcode::Opcode,
};

pub fn generate_bytecode(ast: &AstNode) -> Result<Vec<u8>, CompilerError> {
    let mut bytecode = Vec::new();
    compile_node(ast, &mut bytecode)?;
    Ok(bytecode)
}

pub fn compile_node(node: &AstNode, bytecode: &mut Vec<u8>) -> Result<(), CompilerError> {
    match node {
        AstNode::Number(value) => compile_number(*value, bytecode),
        AstNode::BinaryOp { left, op, right } => compile_binary_op(left, op, right, bytecode),
        AstNode::VariableDeclaration { name, value } => {
            compile_variable_declaration(name, value, bytecode)
        }
        AstNode::Identifier(name) => compile_identifier(name, bytecode),
        AstNode::FunctionCall { name, args } => compile_function_call(name, args, bytecode),
        AstNode::If {
            condition,
            then_branch,
            else_branch,
        } => compile_if(condition, then_branch, else_branch, bytecode),
        AstNode::While { condition, body } => compile_while(condition, body, bytecode),
        AstNode::Block(nodes) => compile_block(nodes, bytecode),
        _ => Err(CompilerError::BytecodeError(
            "Unsupported AST node".to_string(),
        )),
    }
}

pub fn compile_number(value: i64, bytecode: &mut Vec<u8>) -> Result<(), CompilerError> {
    bytecode.push(Opcode::LoadNumber.into());
    bytecode.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

pub fn compile_binary_op(
    left: &AstNode,
    op: &str,
    right: &AstNode,
    bytecode: &mut Vec<u8>,
) -> Result<(), CompilerError> {
    compile_node(left, bytecode)?;
    compile_node(right, bytecode)?;

    let opcode = match op {
        "+" => Opcode::Add,
        "-" => Opcode::Sub,
        "*" => Opcode::Mul,
        "/" => Opcode::Div,
        _ => {
            return Err(CompilerError::BytecodeError(format!(
                "Unknown binary operator: {}",
                op
            )))
        }
    };
    bytecode.push(opcode.into());
    Ok(())
}

pub fn compile_variable_declaration(
    name: &str,
    value: &AstNode,
    bytecode: &mut Vec<u8>,
) -> Result<(), CompilerError> {
    compile_node(value, bytecode)?;
    let address = hash_name_to_address(name);
    bytecode.push(Opcode::StoreToMemory.into());
    bytecode.extend_from_slice(&address.to_le_bytes());
    Ok(())
}

pub fn compile_identifier(name: &str, bytecode: &mut Vec<u8>) -> Result<(), CompilerError> {
    let address = hash_name_to_address(name);
    bytecode.push(Opcode::LoadFromMemory.into());
    bytecode.extend_from_slice(&address.to_le_bytes());
    Ok(())
}

pub fn compile_function_call(
    name: &str,
    args: &[AstNode],
    bytecode: &mut Vec<u8>,
) -> Result<(), CompilerError> {
    for arg in args {
        compile_node(arg, bytecode)?;
    }
    bytecode.push(Opcode::CallFunction.into());
    let name_bytes = name.as_bytes();
    bytecode.push(name_bytes.len() as u8);
    bytecode.extend_from_slice(name_bytes);
    Ok(())
}

pub fn compile_if(
    condition: &AstNode,
    then_branch: &AstNode,
    else_branch: &Option<Box<AstNode>>,
    bytecode: &mut Vec<u8>,
) -> Result<(), CompilerError> {
    compile_node(condition, bytecode)?;
    bytecode.push(Opcode::JumpIf.into());
    let then_bytecode = generate_bytecode(then_branch)?;
    bytecode.extend_from_slice(&(then_bytecode.len() as u64).to_le_bytes());
    bytecode.extend(then_bytecode);
    if let Some(else_branch) = else_branch {
        let else_bytecode = generate_bytecode(else_branch)?;
        bytecode.extend(else_bytecode);
    }
    Ok(())
}

pub fn compile_while(
    condition: &AstNode,
    body: &AstNode,
    bytecode: &mut Vec<u8>,
) -> Result<(), CompilerError> {
    let start_pos = bytecode.len();
    compile_node(condition, bytecode)?;
    bytecode.push(Opcode::JumpIf.into());
    let body_bytecode = generate_bytecode(body)?;
    bytecode.extend_from_slice(&(body_bytecode.len() as u64).to_le_bytes());
    bytecode.extend(body_bytecode);
    bytecode.push(Opcode::Jump.into());
    bytecode.extend_from_slice(&(start_pos as u64).to_le_bytes());
    Ok(())
}

pub fn compile_block(nodes: &[AstNode], bytecode: &mut Vec<u8>) -> Result<(), CompilerError> {
    for node in nodes {
        compile_node(node, bytecode)?;
    }
    Ok(())
}

pub fn hash_name_to_address(name: &str) -> u64 {
    let mut hash = 0u64;
    for byte in name.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    hash
}
