//! Interpreter.

use std::collections::HashMap;

use crate::Span;

use super::ast::{BinOp, CmpOp, Expr, Program, Stmt};
use super::error::LytrError;

#[derive(Clone, Debug)]
pub enum Val {
    I32(i32),
    Bool(bool),
    ResOk(i32),
    ResErr(i32),
}

pub fn run_lytr_program(prog: &Program) -> Result<i32, LytrError> {
    let mut env: HashMap<String, Val> = HashMap::new();
    let stmts = &prog.main.body.stmts;
    for st in &stmts[..stmts.len().saturating_sub(1)] {
        if let Stmt::Let { name, init, .. } = st {
            let v = eval_expr(init, &mut env)?;
            env.insert(name.clone(), v);
        }
    }
    let Some(Stmt::Return { expr, .. }) = stmts.last() else {
        unreachable!()
    };
    let v = eval_expr(expr, &mut env)?;
    match v {
        Val::I32(i) => Ok(i),
        other => Err(LytrError::Runtime {
            code: "E_LYTR_RUNTIME",
            span: Span::new(0, 0),
            message: format!("`main` returned non-i32: {other:?}"),
            fix_hint: "return an i32".into(),
        }),
    }
}

fn eval_expr(e: &Expr, env: &mut HashMap<String, Val>) -> Result<Val, LytrError> {
    match e {
        Expr::Int { value, .. } => Ok(Val::I32(*value)),
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
        } => {
            let a = eval_i32(left, env, *span)?;
            let b = eval_i32(right, env, *span)?;
            let r = match op {
                BinOp::Add => a.checked_add(b),
                BinOp::Sub => a.checked_sub(b),
                BinOp::Mul => a.checked_mul(b),
                BinOp::Div => {
                    if b == 0 {
                        return Err(LytrError::Runtime {
                            code: "E_LYTR_DIV0",
                            span: *span,
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
                            span: *span,
                            message: "modulo by zero".into(),
                            fix_hint: "".into(),
                        });
                    }
                    Some(a % b)
                }
            };
            Ok(Val::I32(r.ok_or_else(|| overflow(*span))?))
        }
        Expr::Cmp {
            op,
            left,
            right,
            span,
        } => {
            let a = eval_i32(left, env, *span)?;
            let b = eval_i32(right, env, *span)?;
            let out = match op {
                CmpOp::Eq => a == b,
                CmpOp::Ne => a != b,
                CmpOp::Lt => a < b,
                CmpOp::Gt => a > b,
                CmpOp::Le => a <= b,
                CmpOp::Ge => a >= b,
            };
            Ok(Val::Bool(out))
        }
        Expr::If {
            cond,
            then_b,
            else_b,
            span,
        } => {
            match eval_expr(cond, env)? {
                Val::Bool(true) => eval_expr(then_b, env),
                Val::Bool(false) => eval_expr(else_b, env),
                _ => Err(LytrError::Runtime {
                    code: "E_LYTR_RUNTIME",
                    span: *span,
                    message: "if condition not bool".into(),
                    fix_hint: "".into(),
                }),
            }
        }
        Expr::Ok(inner) => {
            let i = eval_i32(inner, env, expr_span(inner))?;
            Ok(Val::ResOk(i))
        }
        Expr::Err(inner) => {
            let i = eval_i32(inner, env, expr_span(inner))?;
            Ok(Val::ResErr(i))
        }
        Expr::Match {
            scrutinee,
            ok_name,
            ok_arm,
            err_name,
            err_arm,
            span,
            ..
        } => {
            let v = eval_expr(scrutinee, env)?;
            match v {
                Val::ResOk(payload) => {
                    env.insert(ok_name.clone(), Val::I32(payload));
                    let out = eval_expr(ok_arm, env)?;
                    env.remove(ok_name);
                    Ok(out)
                }
                Val::ResErr(payload) => {
                    env.insert(err_name.clone(), Val::I32(payload));
                    let out = eval_expr(err_arm, env)?;
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

fn eval_i32(e: &Expr, env: &mut HashMap<String, Val>, span: Span) -> Result<i32, LytrError> {
    match eval_expr(e, env)? {
        Val::I32(i) => Ok(i),
        _ => Err(LytrError::Runtime {
            code: "E_LYTR_RUNTIME",
            span,
            message: "expected i32".into(),
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
        | Expr::Match { span, .. } => *span,
        Expr::Ok(inner) | Expr::Err(inner) => expr_span(inner),
    }
}
