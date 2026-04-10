//! LYTR 0.1 bootstrap AST — `let`, comparisons, `if`, `Result`, `match`.

use crate::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub span: Span,
    pub main: FnItem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnItem {
    pub name_span: Span,
    pub body: Block,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub span: Span,
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stmt {
    Let {
        name: String,
        name_span: Span,
        /// `None` = infer from initializer
        ty: Option<Ty>,
        init: Expr,
        span: Span,
    },
    Return { expr: Expr, span: Span },
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } | Stmt::Return { span, .. } => *span,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    I32,
    Bool,
    ResultI32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Int { value: i32, span: Span },
    BoolLit { value: bool, span: Span },
    Var { name: String, span: Span },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Cmp {
        op: CmpOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    If {
        cond: Box<Expr>,
        then_b: Box<Expr>,
        else_b: Box<Expr>,
        span: Span,
    },
    Ok(Box<Expr>),
    Err(Box<Expr>),
    Match {
        scrutinee: Box<Expr>,
        ok_name: String,
        ok_name_span: Span,
        ok_arm: Box<Expr>,
        err_name: String,
        err_name_span: Span,
        err_arm: Box<Expr>,
        span: Span,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}
