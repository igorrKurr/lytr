//! Interpreter for LYTR 0.1 bootstrap (i32 expressions).

use crate::Span;

use super::ast::{BinOp, Expr, Program, Stmt};
use super::error::LytrError;

pub fn run_lytr_program(prog: &Program) -> Result<i32, LytrError> {
    let stmt = prog.main.body.stmts.first().ok_or_else(|| LytrError::Runtime {
        code: "E_LYTR_RUNTIME",
        span: prog.main.body.span,
        message: "empty body".into(),
        fix_hint: "internal: check_lytr_program should run first".into(),
    })?;
    let Stmt::Return { ref expr, .. } = stmt;
    eval_expr(expr)
}

fn eval_expr(e: &Expr) -> Result<i32, LytrError> {
    match e {
        Expr::Int { value, .. } => Ok(*value),
        Expr::Binary {
            op,
            left,
            right,
            span,
        } => {
            let a = eval_expr(left)?;
            let b = eval_expr(right)?;
            match op {
                BinOp::Add => a.checked_add(b).ok_or(overflow(*span)),
                BinOp::Sub => a.checked_sub(b).ok_or(overflow(*span)),
                BinOp::Mul => a.checked_mul(b).ok_or(overflow(*span)),
                BinOp::Div => {
                    if b == 0 {
                        Err(LytrError::Runtime {
                            code: "E_LYTR_DIV0",
                            span: *span,
                            message: "division by zero".into(),
                            fix_hint: "avoid `/` with zero divisor".into(),
                        })
                    } else {
                        Ok(a / b)
                    }
                }
                BinOp::Mod => {
                    if b == 0 {
                        Err(LytrError::Runtime {
                            code: "E_LYTR_DIV0",
                            span: *span,
                            message: "modulo by zero".into(),
                            fix_hint: "avoid `%` with zero divisor".into(),
                        })
                    } else {
                        Ok(a % b)
                    }
                }
            }
        }
    }
}

fn overflow(span: Span) -> LytrError {
    LytrError::Runtime {
        code: "E_LYTR_OVERFLOW",
        span,
        message: "integer overflow".into(),
        fix_hint: "expression exceeds i32 range".into(),
    }
}
