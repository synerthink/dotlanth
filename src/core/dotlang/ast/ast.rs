#[derive(Debug, PartialEq)]
pub enum AstNode {
    Number(i64),
    Identifier(String),
    BinaryOp {
        left: Box<AstNode>,
        op: String,
        right: Box<AstNode>,
    },
    VariableDeclaration {
        name: String,
        value: Box<AstNode>,
    },
    Function {
        name: String,
        params: Vec<String>,
        body: Box<AstNode>,
    },
    FunctionCall {
        name: String,
        args: Vec<AstNode>,
    },
    If {
        condition: Box<AstNode>,
        then_branch: Box<AstNode>,
        else_branch: Option<Box<AstNode>>,
    },
    While {
        condition: Box<AstNode>,
        body: Box<AstNode>,
    },
    Block(Vec<AstNode>),
}
