#[derive(Debug)]
pub struct Program {
    pub imports: Vec<Import>,
    pub functions: Vec<Function>,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone)]
pub struct Path {
    pub segments: Vec<PathSegment>,
}

#[derive(Debug, Clone)]
pub struct PathSegment {
    pub ident: String,
    pub generic_args: Option<Vec<Type>>,
}

#[derive(Debug)]
pub struct Import {
    pub path: Path,
    pub alias: Option<String>,
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

#[derive(Debug, Clone)]
pub enum Type {
    I8, I16, I32, I64, I128,
    U8, U16, U32, U64, U128,
    F32, F64,
    Bool,
    Char,
    Buffer(Box<Type>, Option<usize>),
    Named(String),
    Tuple(Vec<Type>),
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
    ParallelFor {
        var: String,
        range: (Expression, Expression),
        body: Block,
    },
    For {
        var: String,
        range: (Expression, Expression),
        body: Block,
    },
    If {
        condition: Expression,
        then_branch: Block,
        else_branch: Option<Block>,
    },
    While {
        condition: Expression,
        body: Block,
    },
    Loop(Block),
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,
    Identifier(String),
    Path(Path),
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
    Index {
        obj: Box<Expression>,
        index: Box<Expression>,
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
    Array(Vec<Expression>),
    Spawn {
        device: Option<Box<Expression>>,
        task: Box<Expression>,
        await_: bool,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add, Sub, Mul, Div, Eq, Ne, Lt, Le, Gt, Ge, And, Or,
}