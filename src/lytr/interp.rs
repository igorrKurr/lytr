//! Interpreter.

use std::collections::HashMap;

use crate::Span;

use super::ast::{BinOp, CmpOp, Expr, MainRetTy, Program, Stmt};
use super::error::LytrError;

/// Result of running a LYTR program (`main` is always `i32` or `i64`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LytrRun {
    I32(i32),
    I64(i64),
}

#[derive(Clone, Debug)]
pub enum Val {
    I32(i32),
    I64(i64),
    Bool(bool),
    ResOkI32(i32),
    ResErrI32(i32),
    ResOkI64(i64),
    ResErrI64(i64),
}

pub fn run_lytr_program(prog: &Program) -> Result<LytrRun, LytrError> {
    let ret = prog.main.ret;
    let mut env: HashMap<String, Val> = HashMap::new();
    let stmts = &prog.main.body.stmts;
    for st in &stmts[..stmts.len().saturating_sub(1)] {
        if let Stmt::Let { name, init, .. } = st {
            let v = eval_expr(init, &mut env, ret)?;
            env.insert(name.clone(), v);
        }
    }
    let Some(Stmt::Return { expr, .. }) = stmts.last() else {
        unreachable!()
    };
    let v = eval_expr(expr, &mut env, ret)?;
    match (ret, v) {
        (MainRetTy::I32, Val::I32(i)) => Ok(LytrRun::I32(i)),
        (MainRetTy::I64, Val::I64(i)) => Ok(LytrRun::I64(i)),
        (r, other) => Err(LytrError::Runtime {
            code: "E_LYTR_RUNTIME",
            span: Span::new(0, 0),
            message: format!("`main` return mismatch: expected {r:?}, got {other:?}"),
            fix_hint: "internal: typecheck should catch".into(),
        }),
    }
}

fn eval_expr(
    e: &Expr,
    env: &mut HashMap<String, Val>,
    ret: MainRetTy,
) -> Result<Val, LytrError> {
    match e {
        Expr::Int { value, .. } => match ret {
            MainRetTy::I32 => Ok(Val::I32(*value as i32)),
            MainRetTy::I64 => Ok(Val::I64(*value)),
        },
        Expr::BoolLit { value, .. } => Ok(Val::Bool(*value)),
        Expr::Var { name, span } => env.get(name).cloned().ok_or_else(|| LytrError::Runtime {
            code: "E_LYTR_RUNTIME",
            span: *span,
            message: format!("unknown variable `{name}`"),
            fix_hint: "internal: typecheck should catch".into(),
        }),
        Expr::Binary {
            op,
            left,
            right,
            span,
        } => match ret {
            MainRetTy::I32 => {
                let a = eval_i32(left, env, ret, *span)?;
                let b = eval_i32(right, env, ret, *span)?;
                let r = binop_i32(*op, a, b, *span)?;
                Ok(Val::I32(r))
            }
            MainRetTy::I64 => {
                let a = eval_i64(left, env, ret, *span)?;
                let b = eval_i64(right, env, ret, *span)?;
                let r = binop_i64(*op, a, b, *span)?;
                Ok(Val::I64(r))
            }
        },
        Expr::Cmp {
            op,
            left,
            right,
            span,
        } => match ret {
            MainRetTy::I32 => {
                let a = eval_i32(left, env, ret, *span)?;
                let b = eval_i32(right, env, ret, *span)?;
                Ok(Val::Bool(cmp_i32(*op, a, b)))
            }
            MainRetTy::I64 => {
                let a = eval_i64(left, env, ret, *span)?;
                let b = eval_i64(right, env, ret, *span)?;
                Ok(Val::Bool(cmp_i64(*op, a, b)))
            }
        },
        Expr::Block { stmts, tail, .. } => {
            let mut inner = env.clone();
            for st in stmts {
                if let Stmt::Let { name, init, .. } = st {
                    let v = eval_expr(init, &mut inner, ret)?;
                    inner.insert(name.clone(), v);
                }
            }
            eval_expr(tail, &mut inner, ret)
        }
        Expr::If {
            cond,
            then_b,
            else_b,
            span,
        } => {
            match eval_expr(cond, env, ret)? {
                Val::Bool(true) => eval_expr(then_b, env, ret),
                Val::Bool(false) => eval_expr(else_b, env, ret),
                _ => Err(LytrError::Runtime {
                    code: "E_LYTR_RUNTIME",
                    span: *span,
                    message: "if condition not bool".into(),
                    fix_hint: "".into(),
                }),
            }
        }
        Expr::Ok(inner) => match ret {
            MainRetTy::I32 => {
                let i = eval_i32(inner, env, ret, expr_span(inner))?;
                Ok(Val::ResOkI32(i))
            }
            MainRetTy::I64 => {
                let i = eval_i64(inner, env, ret, expr_span(inner))?;
                Ok(Val::ResOkI64(i))
            }
        },
        Expr::Err(inner) => match ret {
            MainRetTy::I32 => {
                let i = eval_i32(inner, env, ret, expr_span(inner))?;
                Ok(Val::ResErrI32(i))
            }
            MainRetTy::I64 => {
                let i = eval_i64(inner, env, ret, expr_span(inner))?;
                Ok(Val::ResErrI64(i))
            }
        },
        Expr::Match {
            scrutinee,
            ok_name,
            ok_arm,
            err_name,
            err_arm,
            span,
            ..
        } => {
            let v = eval_expr(scrutinee, env, ret)?;
            match (ret, v) {
                (MainRetTy::I32, Val::ResOkI32(payload)) => {
                    env.insert(ok_name.clone(), Val::I32(payload));
                    let out = eval_expr(ok_arm, env, ret)?;
                    env.remove(ok_name);
                    Ok(out)
                }
                (MainRetTy::I32, Val::ResErrI32(payload)) => {
                    env.insert(err_name.clone(), Val::I32(payload));
                    let out = eval_expr(err_arm, env, ret)?;
                    env.remove(err_name);
                    Ok(out)
                }
                (MainRetTy::I64, Val::ResOkI64(payload)) => {
                    env.insert(ok_name.clone(), Val::I64(payload));
                    let out = eval_expr(ok_arm, env, ret)?;
                    env.remove(ok_name);
                    Ok(out)
                }
                (MainRetTy::I64, Val::ResErrI64(payload)) => {
                    env.insert(err_name.clone(), Val::I64(payload));
                    let out = eval_expr(err_arm, env, ret)?;
                    env.remove(err_name);
                    Ok(out)
                }
                _ => Err(LytrError::Runtime {
                    code: "E_LYTR_RUNTIME",
                    span: *span,
                    message: "match on non-Result".into(),
                    fix_hint: "".into(),
                }),
            }
        }
    }
}

fn binop_i32(op: BinOp, a: i32, b: i32, span: Span) -> Result<i32, LytrError> {
    let r = match op {
        BinOp::Add => a.checked_add(b),
        BinOp::Sub => a.checked_sub(b),
        BinOp::Mul => a.checked_mul(b),
        BinOp::Div => {
            if b == 0 {
                return Err(LytrError::Runtime {
                    code: "E_LYTR_DIV0",
                    span,
                    message: "division by zero".into(),
                    fix_hint: "".into(),
                });
            }
            Some(a / b)
        }
        BinOp::Mod => {
            if b == 0 {
                return Err(LytrError::Runtime {
                    code: "E_LYTR_DIV0",
                    span,
                    message: "modulo by zero".into(),
                    fix_hint: "".into(),
                });
            }
            Some(a % b)
        }
    };
    r.ok_or_else(|| overflow(span))
}

fn binop_i64(op: BinOp, a: i64, b: i64, span: Span) -> Result<i64, LytrError> {
    let r = match op {
        BinOp::Add => a.checked_add(b),
        BinOp::Sub => a.checked_sub(b),
        BinOp::Mul => a.checked_mul(b),
        BinOp::Div => {
            if b == 0 {
                return Err(LytrError::Runtime {
                    code: "E_LYTR_DIV0",
                    span,
                    message: "division by zero".into(),
                    fix_hint: "".into(),
                });
            }
            Some(a / b)
        }
        BinOp::Mod => {
            if b == 0 {
                return Err(LytrError::Runtime {
                    code: "E_LYTR_DIV0",
                    span,
                    message: "modulo by zero".into(),
                    fix_hint: "".into(),
                });
            }
            Some(a % b)
        }
    };
    r.ok_or_else(|| overflow(span))
}

fn cmp_i32(op: CmpOp, a: i32, b: i32) -> bool {
    match op {
        CmpOp::Eq => a == b,
        CmpOp::Ne => a != b,
        CmpOp::Lt => a < b,
        CmpOp::Gt => a > b,
        CmpOp::Le => a <= b,
        CmpOp::Ge => a >= b,
    }
}

fn cmp_i64(op: CmpOp, a: i64, b: i64) -> bool {
    match op {
        CmpOp::Eq => a == b,
        CmpOp::Ne => a != b,
        CmpOp::Lt => a < b,
        CmpOp::Gt => a > b,
        CmpOp::Le => a <= b,
        CmpOp::Ge => a >= b,
    }
}

fn eval_i32(
    e: &Expr,
    env: &mut HashMap<String, Val>,
    ret: MainRetTy,
    span: Span,
) -> Result<i32, LytrError> {
    match eval_expr(e, env, ret)? {
        Val::I32(i) => Ok(i),
        _ => Err(LytrError::Runtime {
            code: "E_LYTR_RUNTIME",
            span,
            message: "expected i32".into(),
            fix_hint: "".into(),
        }),
    }
}

fn eval_i64(
    e: &Expr,
    env: &mut HashMap<String, Val>,
    ret: MainRetTy,
    span: Span,
) -> Result<i64, LytrError> {
    match eval_expr(e, env, ret)? {
        Val::I64(i) => Ok(i),
        _ => Err(LytrError::Runtime {
            code: "E_LYTR_RUNTIME",
            span,
            message: "expected i64".into(),
            fix_hint: "".into(),
        }),
    }
}

fn overflow(span: Span) -> LytrError {
    LytrError::Runtime {
        code: "E_LYTR_OVERFLOW",
        span,
        message: "integer overflow".into(),
        fix_hint: "".into(),
    }
}

fn expr_span(e: &Expr) -> Span {
    match e {
        Expr::Int { span, .. }
        | Expr::BoolLit { span, .. }
        | Expr::Var { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Cmp { span, .. }
        | Expr::If { span, .. }
        | Expr::Block { span, .. }
        | Expr::Match { span, .. } => *span,
        Expr::Ok(inner) | Expr::Err(inner) => expr_span(inner),
    }
}
