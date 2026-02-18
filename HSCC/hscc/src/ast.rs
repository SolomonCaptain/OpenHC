#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
    pub tasks: Vec<Task>,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Block,
}

#[derive(Debug)]
pub struct Task {
    pub name: String,
    pub pattern: Option<Pattern>,
    pub policy: Option<Policy>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Block,
}

#[derive(Debug)]
pub struct Pattern {
    pub kind: String,
    pub fields: Vec<(String, Expression)>,
}

#[derive(Debug)]
pub struct Policy {
    pub kind: String,
    pub fields: Vec<(String, Expression)>
}

#[derive(Debug)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug)]
pub enum Type {
    I32, F32, Bool,
    Buffer(Box<Type>, Option<usize>),
    Named(String),
}

#[derive(Debug)]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub enum Statement {
    Let {
        mutable: bool,
        name: String,
        ty: Option<Type>,
        init: Option<Expression>,
    },
    Return(Option<Expression>),
    Expr(Expression),
    Spawn {
        device: Option<Expression>,
        task: Expression,
        await_: bool,
    },
    ParallelFor {
        var: String,
        range: (Expression, Expression),
        body: Block,
    },
}

#[derive(Debug, Clone)]
pub enum Expression {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,
    Identifier(String),
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    Call {
        func: Box<Expression>,
        args: Vec<Expression>,
    },
    FieldAccess {
        obj: Box<Expression>,
        field: String,
    },
    MethodCall {
        obj: Box<Expression>,
        method: String,
        args: Vec<Expression>,
    },
    PlaceOn {
        expr: Box<Expression>,
        device: Box<Expression>,
    },
    MoveTo {
        expr: Box<Expression>,
        device: Box<Expression>,
    },
    Await(Box<Expression>),
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add, Sub, Mul, Div, Eq, Ne, Lt, Le, Gt, Ge, And, Or,
}