use crate::error::Span;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    pub span: Span,
    pub source: Source,
    pub stages: Vec<Stage>,
}

/// Element type of a stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElemTy {
    I32,
    I64,
    Bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Source {
    Input {
        ty: ElemTy,
        explicit: bool,
        span: Span,
    },
    Range {
        start: i64,
        stop: i64,
        step: i64,
        span: Span,
    },
    Lit {
        elems: Vec<LitElem>,
        span: Span,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LitElem {
    I32(i32),
    I64(i64),
    Bool(bool),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Stage {
    Filter {
        pred: Predicate,
        span: Span,
    },
    Map {
        expr: Expr,
        span: Span,
    },
    Scan {
        init: i64,
        op: ScanOp,
        span: Span,
    },
    Reduce {
        op: ReduceOp,
        span: Span,
    },
    Take {
        n: u32,
        span: Span,
    },
    Drop {
        n: u32,
        span: Span,
    },
    Id {
        span: Span,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanOp {
    Add,
    Sub,
    Mul,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReduceOp {
    Sum,
    Prod,
    Count,
    Min,
    Max,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Predicate {
    Or {
        left: Box<Predicate>,
        right: Box<Predicate>,
        span: Span,
    },
    And {
        left: Box<Predicate>,
        right: Box<Predicate>,
        span: Span,
    },
    Not {
        inner: Box<Predicate>,
        span: Span,
    },
    Even {
        span: Span,
    },
    Odd {
        span: Span,
    },
    Cmp {
        op: CmpOp,
        rhs: CmpRhs,
        span: Span,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CmpRhs {
    Int(i64),
    Bool(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CmpOp {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expr {
    Add {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Sub {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Mul {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Div {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Mod {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Neg {
        inner: Box<Expr>,
        span: Span,
    },
    Dot {
        span: Span,
    },
    Lit {
        v: i64,
        span: Span,
    },
}
