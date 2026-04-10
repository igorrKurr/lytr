//! Type-checking (bootstrap).

use std::collections::HashMap;

use crate::Span;

use super::ast::{Expr, MainRetTy, MainTail, Program, Stmt, Ty};
use super::error::LytrError;

pub fn check_lytr_program(prog: &Program) -> Result<(), LytrError> {
    let int_ty = match prog.main.ret {
        MainRetTy::I32 => Ty::I32,
        MainRetTy::I64 => Ty::I64,
    };
    let res_ty = match int_ty {
        Ty::I32 => Ty::ResultI32,
        Ty::I64 => Ty::ResultI64,
        _ => unreachable!(),
    };

    let mut env: HashMap<String, Ty> = HashMap::new();
    for st in &prog.main.body.stmts {
        let Stmt::Let {
            name,
            ty,
            init,
            name_span,
            ..
        } = st;
        let got = type_expr(init, &env, int_ty, res_ty)?;
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

    let out = match &prog.main.body.tail {
        MainTail::Return { expr, span } => {
            let t = type_expr(expr, &env, int_ty, res_ty)?;
            if t != int_ty {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: *span,
                    message: format!("`main` must return {int_ty:?}, got {t:?}"),
                    fix_hint: "match the function's `-> i32` or `-> i64`".into(),
                });
            }
            return Ok(());
        }
        MainTail::Expr(e) => e,
    };
    let t = type_expr(out, &env, int_ty, res_ty)?;
    if t != int_ty {
        return Err(LytrError::Type {
            code: "E_LYTR_TYPE",
            span: expr_span(out),
            message: format!("`main` body must produce {int_ty:?}, got {t:?}"),
            fix_hint: "end with an expression of the declared return type".into(),
        });
    }
    Ok(())
}

fn ty_compatible(decl: &Ty, got: &Ty) -> bool {
    decl == got
}

fn type_expr(
    e: &Expr,
    env: &HashMap<String, Ty>,
    int_ty: Ty,
    res_ty: Ty,
) -> Result<Ty, LytrError> {
    debug_assert!(matches!(int_ty, Ty::I32 | Ty::I64));
    debug_assert!(matches!(res_ty, Ty::ResultI32 | Ty::ResultI64));

    match e {
        Expr::Int { value, span } => {
            match int_ty {
                Ty::I32 => {
                    if *value < i32::MIN as i64 || *value > i32::MAX as i64 {
                        return Err(LytrError::Type {
                            code: "E_LYTR_TYPE",
                            span: *span,
                            message: format!("literal `{value}` is out of range for i32"),
                            fix_hint: "use a smaller literal or `fn main() -> i64`".into(),
                        });
                    }
                    Ok(Ty::I32)
                }
                Ty::I64 => Ok(Ty::I64),
                _ => unreachable!(),
            }
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
            let a = type_expr(left, env, int_ty, res_ty)?;
            let b = type_expr(right, env, int_ty, res_ty)?;
            if a != int_ty || b != int_ty {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: *span,
                    message: format!("arithmetic expects two `{int_ty:?}` operands"),
                    fix_hint: "use the same integer type as `main`".into(),
                });
            }
            Ok(int_ty)
        }
        Expr::Cmp { left, right, span, .. } => {
            let a = type_expr(left, env, int_ty, res_ty)?;
            let b = type_expr(right, env, int_ty, res_ty)?;
            if a != int_ty || b != int_ty {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: *span,
                    message: format!("comparison expects two `{int_ty:?}` operands"),
                    fix_hint: "compare values of the same integer type as `main`".into(),
                });
            }
            Ok(Ty::Bool)
        }
        Expr::Block { stmts, tail, span: _ } => {
            let mut env2 = env.clone();
            for st in stmts {
                let Stmt::Let {
                    name,
                    ty,
                    init,
                    name_span,
                    ..
                } = st;
                let got = type_expr(init, &env2, int_ty, res_ty)?;
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
                env2.insert(name.clone(), got);
            }
            type_expr(tail, &env2, int_ty, res_ty)
        }
        Expr::If {
            cond,
            then_b,
            else_b,
            span,
        } => {
            let tc = type_expr(cond, env, int_ty, res_ty)?;
            if tc != Ty::Bool {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: *span,
                    message: "`if` condition must be bool".into(),
                    fix_hint: "use a comparison or `true`/`false`".into(),
                });
            }
            let t1 = type_expr(then_b, env, int_ty, res_ty)?;
            let t2 = type_expr(else_b, env, int_ty, res_ty)?;
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
            let t = type_expr(inner, env, int_ty, res_ty)?;
            if t != int_ty {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: expr_span(inner),
                    message: format!("`Ok` payload must be `{int_ty:?}`"),
                    fix_hint: "match `Result<…>` to main's integer type".into(),
                });
            }
            Ok(res_ty)
        }
        Expr::Err(inner) => {
            let t = type_expr(inner, env, int_ty, res_ty)?;
            if t != int_ty {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: expr_span(inner),
                    message: format!("`Err` payload must be `{int_ty:?}`"),
                    fix_hint: "match `Result<…>` to main's integer type".into(),
                });
            }
            Ok(res_ty)
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
            let ts = type_expr(scrutinee, env, int_ty, res_ty)?;
            if ts != res_ty {
                return Err(LytrError::Type {
                    code: "E_LYTR_TYPE",
                    span: *span,
                    message: format!("`match` scrutinee must be `{res_ty:?}`"),
                    fix_hint: "use `Ok(…)` or `Err(…)` with matching `Result` type".into(),
                });
            }
            let mut env_ok = env.clone();
            env_ok.insert(ok_name.clone(), int_ty);
            let mut env_err = env.clone();
            env_err.insert(err_name.clone(), int_ty);
            let t_ok = type_expr(ok_arm, &env_ok, int_ty, res_ty)?;
            let t_err = type_expr(err_arm, &env_err, int_ty, res_ty)?;
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
        | Expr::Block { span, .. }
        | Expr::Match { span, .. } => *span,
        Expr::Ok(inner) | Expr::Err(inner) => expr_span(inner),
    }
}
