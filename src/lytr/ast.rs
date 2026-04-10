//! Minimal LYTR 0.1 bootstrap AST (integer expressions + single `main`).

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
    Return { expr: Expr, span: Span },
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Return { span, .. } => *span,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Int { value: i32, span: Span },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
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
