//! Type-checking (bootstrap).

use std::collections::HashMap;

use crate::Span;

use super::ast::{Expr, Program, Stmt, Ty};
use super::error::LytrError;

pub fn check_lytr_program(prog: &Program) -> Result<(), LytrError> {
    if prog.main.body.stmts.is_empty() {
        return Err(LytrError::Type {
            code: "E_LYTR_TYPE",
            span: prog.main.body.span,
            message: "`main` body cannot be empty".into(),
            fix_hint: "add `return …;`".into(),
        });
    }
    let last = prog.main.body.stmts.last().unwrap();
    if !matches!(last, Stmt::Return { .. }) {
        return Err(LytrError::Type {
            code: "E_LYTR_TYPE",
            span: last.span(),
            message: "last statement in `main` must be `return`".into(),
            fix_hint: "end with `return <expr>;`".into(),
        });
    }
    let mid = &prog.main.body.stmts[..prog.main.body.stmts.len() - 1];
    for st in mid {
        if !matches!(st, Stmt::Let { .. }) {
            return Err(LytrError::Type {
                code: "E_LYTR_TYPE",
                span: st.span(),
                message: "only `let` is allowed before the final `return`".into(),
                fix_hint: "use `let` bindings then a single `return`".into(),
            });
        }
    }

    let mut env: HashMap<String, Ty> = HashMap::new();
    for st in &prog.main.body.stmts {
        match st {
            Stmt::Let {
                name,
                ty,
                init,
                name_span,
                ..
            } => {
                let got = type_expr(init, &env)?;
                if let Some(decl) = ty {
                    if !ty_compatible(decl, &got) {
                        return Err(LytrError::Type {
                            code: "E_LYTR_TYPE",
                            span: *name_span,
                            message: format!(
                                "initializer type {got:?} does not match `{name}: {decl:?}`"
                            ),
                            fix_hint: "fix type ascription or expression".into(),
                        });
                    }
                }
                env.insert(name.clone(), got);
            }
            Stmt::Return { expr, span } => {
                let t = type_expr(expr, &env)?;
                if t != Ty::I32 {
                    return Err(LytrError::Type {
                        code: "E_LYTR_TYPE",
                        span: *span,
                        message: format!("`main` must return i32, got {t:?}"),
                        fix_hint: "return an i32 expression".into(),
                    });
                }
            }
        }
    }
    Ok(())
}

fn ty_compatible(decl: &Ty, got: &Ty) -> bool {
    decl == got
}

fn type_expr(e: &Expr, env: &HashMap<String, Ty>) -> Result<Ty, LytrError> {
    match e {
        Expr::Int { span, .. } => {
            let _ = span;
            Ok(Ty::I32)
        }
        Expr::BoolLit { span, .. } => {
            let _ = span;
            Ok(Ty::Bool)
        }
        Expr::Var { name, span } => env.get(name).cloned().ok_or_else(|| LytrError::Type {
            code: "E_LYTR_TYPE",
            span: *span,
            message: format!("unknown variable `{name}`"),
            fix_hint: "define with `let` first".into(),
        }),
        Expr::Binary { left, right, span, .. } => {
            let a = type_expr(left, env)?;
            let b = type_expr(right, env)?;
            if a != Ty::I32 || b != Ty::I32 {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: *span,
                    message: "arithmetic expects i32 operands".into(),
                    fix_hint: "use i32 expressions".into(),
                });
            }
            Ok(Ty::I32)
        }
        Expr::Cmp { left, right, span, .. } => {
            let a = type_expr(left, env)?;
            let b = type_expr(right, env)?;
            if a != Ty::I32 || b != Ty::I32 {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: *span,
                    message: "comparison expects i32 operands".into(),
                    fix_hint: "compare i32 values".into(),
                });
            }
            Ok(Ty::Bool)
        }
        Expr::If {
            cond,
            then_b,
            else_b,
            span,
        } => {
            let tc = type_expr(cond, env)?;
            if tc != Ty::Bool {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: *span,
                    message: "`if` condition must be bool".into(),
                    fix_hint: "use a comparison or `true`/`false`".into(),
                });
            }
            let t1 = type_expr(then_b, env)?;
            let t2 = type_expr(else_b, env)?;
            if t1 != t2 {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: *span,
                    message: format!("`if` branches disagree: {t1:?} vs {t2:?}"),
                    fix_hint: "both branches must have the same type".into(),
                });
            }
            Ok(t1)
        }
        Expr::Ok(inner) => {
            let t = type_expr(inner, env)?;
            if t != Ty::I32 {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: expr_span(inner),
                    message: "`Ok` payload must be i32 in this bootstrap".into(),
                    fix_hint: "use `Ok(0)` style i32 payload".into(),
                });
            }
            Ok(Ty::ResultI32)
        }
        Expr::Err(inner) => {
            let t = type_expr(inner, env)?;
            if t != Ty::I32 {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: expr_span(inner),
                    message: "`Err` payload must be i32 in this bootstrap".into(),
                    fix_hint: "use `Err(0)` style i32 payload".into(),
                });
            }
            Ok(Ty::ResultI32)
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
            let ts = type_expr(scrutinee, env)?;
            if ts != Ty::ResultI32 {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: *span,
                    message: "`match` scrutinee must be Result<i32,i32>".into(),
                    fix_hint: "use `Ok(…)` or `Err(…)`".into(),
                });
            }
            let mut env_ok = env.clone();
            env_ok.insert(ok_name.clone(), Ty::I32);
            let mut env_err = env.clone();
            env_err.insert(err_name.clone(), Ty::I32);
            let t_ok = type_expr(ok_arm, &env_ok)?;
            let t_err = type_expr(err_arm, &env_err)?;
            if t_ok != t_err {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: *span,
                    message: format!("`match` arms disagree: {t_ok:?} vs {t_err:?}"),
                    fix_hint: "both arms must have the same type".into(),
                });
            }
            Ok(t_ok)
        }
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
