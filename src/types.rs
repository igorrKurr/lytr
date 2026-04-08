use crate::ast::{
    CmpOp, CmpRhs, ElemTy, Expr, LitElem, Predicate, Program, Source, Stage,
};
use crate::error::{LirError, Span};

pub fn check_program(p: &Program) -> Result<(), LirError> {
    let mut cur = source_elem_ty(&p.source)?;
    let mut scalar = false;
    for (idx, st) in p.stages.iter().enumerate() {
        if scalar {
            return Err(LirError::Type {
                code: "T_STAGE_AFTER_SCALAR",
                span: stage_span(st),
                message: "no stages allowed after reduce".into(),
                fix_hint: "Remove trailing stages after reduce.".into(),
                stage_index: Some(idx),
            });
        }
        match st {
            Stage::Filter { pred, span } => {
                check_predicate(pred, cur, *span, idx)?;
            }
            Stage::Map { expr, span } => {
                cur = infer_map_ty(cur, expr, *span, idx)?;
            }
            Stage::Scan { init, op: _, span } => {
                check_scan_init(*init, cur, *span, idx)?;
                // Scan preserves the stream element type.
            }
            Stage::Reduce { op, span: _ } => {
                check_reduce_ok(cur, *op, idx, stage_span(st))?;
                scalar = true;
            }
            Stage::Take { .. } | Stage::Drop { .. } | Stage::Id { .. } => {}
        }
    }
    Ok(())
}

fn stage_span(st: &Stage) -> Span {
    match st {
        Stage::Filter { span, .. }
        | Stage::Map { span, .. }
        | Stage::Scan { span, .. }
        | Stage::Reduce { span, .. }
        | Stage::Take { span, .. }
        | Stage::Drop { span, .. }
        | Stage::Id { span } => *span,
    }
}

/// Element type of the stream produced by `source` (before any stages).
pub fn source_stream_ty(src: &Source) -> Result<ElemTy, LirError> {
    source_elem_ty(src)
}

fn source_elem_ty(src: &Source) -> Result<ElemTy, LirError> {
    match src {
        Source::Input { ty, .. } => Ok(*ty),
        Source::Range { .. } => Ok(ElemTy::I32),
        Source::Lit { elems, span } => lit_homogeneous_ty(elems, *span),
    }
}

fn lit_homogeneous_ty(elems: &[LitElem], span: Span) -> Result<ElemTy, LirError> {
    if elems.is_empty() {
        // Empty `lit()` → empty stream, default element type i32 (no values materialized).
        return Ok(ElemTy::I32);
    }
    match &elems[0] {
        LitElem::Bool(_) => {
            for e in elems {
                if !matches!(e, LitElem::Bool(_)) {
                    return Err(LirError::Type {
                        code: "T_LIT_MIXED",
                        span,
                        message: "lit() elements must all have the same type".into(),
                        fix_hint: "Do not mix bool and integer literals.".into(),
                        stage_index: None,
                    });
                }
            }
            Ok(ElemTy::Bool)
        }
        LitElem::I32(_) | LitElem::I64(_) => {
            for e in elems {
                match e {
                    LitElem::I32(_) | LitElem::I64(_) => {}
                    _ => {
                        return Err(LirError::Type {
                            code: "T_LIT_MIXED",
                            span,
                            message: "lit() elements must all have the same type".into(),
                            fix_hint: "Do not mix bool and integer literals.".into(),
                            stage_index: None,
                        });
                    }
                }
            }
            if elems.iter().any(|e| matches!(e, LitElem::I64(_))) {
                Ok(ElemTy::I64)
            } else {
                Ok(ElemTy::I32)
            }
        }
    }
}

fn check_predicate(
    pred: &Predicate,
    ty: ElemTy,
    span: Span,
    stage_index: usize,
) -> Result<(), LirError> {
    match pred {
        Predicate::Or { left, right, .. } | Predicate::And { left, right, .. } => {
            check_predicate(left, ty, span, stage_index)?;
            check_predicate(right, ty, span, stage_index)?;
        }
        Predicate::Not { inner, .. } => check_predicate(inner, ty, span, stage_index)?,
        Predicate::Even { .. } | Predicate::Odd { .. } => {
            if ty == ElemTy::Bool {
                return Err(LirError::Type {
                    code: "T_FILTER_EVEN_BOOL",
                    span,
                    message: "even/odd require integer stream".into(),
                    fix_hint: "Use eq true / eq false on bool streams.".into(),
                    stage_index: Some(stage_index),
                });
            }
        }
        Predicate::Cmp { op, rhs, span: cspan } => {
            if let CmpRhs::Int(v) = rhs {
                if ty == ElemTy::I32 && (*v < i32::MIN as i64 || *v > i32::MAX as i64) {
                    return Err(LirError::Type {
                        code: "T_CMP_RHS_RANGE",
                        span: *cspan,
                        message: "comparison literal does not fit i32 stream".into(),
                        fix_hint: "Use a smaller literal or input:i64.".into(),
                        stage_index: Some(stage_index),
                    });
                }
            }
            if ty == ElemTy::Bool {
                if *op != CmpOp::Eq {
                    return Err(LirError::Type {
                        code: "T_BOOL_CMP_OP",
                        span,
                        message: "bool streams only support `eq` comparisons".into(),
                        fix_hint: "Use `eq true` or `eq false`.".into(),
                        stage_index: Some(stage_index),
                    });
                }
                if !matches!(rhs, CmpRhs::Bool(_)) {
                    return Err(LirError::Type {
                        code: "T_BOOL_CMP_RHS",
                        span,
                        message: "bool comparison requires `true` or `false`".into(),
                        fix_hint: "Use `eq true` or `eq false`.".into(),
                        stage_index: Some(stage_index),
                    });
                }
            } else if matches!(rhs, CmpRhs::Bool(_)) {
                return Err(LirError::Type {
                    code: "T_INT_CMP_BOOL",
                    span,
                    message: "integer stream comparison cannot use bool literal".into(),
                    fix_hint: "Use integer right-hand side, e.g. `gt 10`.".into(),
                    stage_index: Some(stage_index),
                });
            }
        }
    }
    Ok(())
}

fn infer_map_ty(
    cur: ElemTy,
    expr: &Expr,
    span: Span,
    stage_index: usize,
) -> Result<ElemTy, LirError> {
    if cur == ElemTy::Bool {
        return Err(LirError::Type {
            code: "T_MAP_BOOL",
            span,
            message: "map is not defined for bool streams in v1".into(),
            fix_hint: "Filter bools then reduce count, or use integer streams.".into(),
            stage_index: Some(stage_index),
        });
    }
    check_literal_ranges(cur, expr, span, stage_index)?;
    let out = expr_result_ty(cur, expr)?;
    if out == ElemTy::Bool {
        return Err(LirError::Type {
            code: "T_MAP_TO_BOOL",
            span,
            message: "map expression must produce integer type in v1".into(),
            fix_hint: "Avoid comparisons in map; use filter for predicates.".into(),
            stage_index: Some(stage_index),
        });
    }
    Ok(out)
}

fn check_literal_ranges(
    cur: ElemTy,
    e: &Expr,
    stage_span: Span,
    stage_index: usize,
) -> Result<(), LirError> {
    match e {
        Expr::Lit { v, span, .. } => {
            if cur == ElemTy::I32 && (*v < i32::MIN as i64 || *v > i32::MAX as i64) {
                return Err(LirError::Type {
                    code: "T_LIT_RANGE",
                    span: *span,
                    message: "integer literal does not fit i32 stream".into(),
                    fix_hint: "Use input:i64 or a smaller literal.".into(),
                    stage_index: Some(stage_index),
                });
            }
            Ok(())
        }
        Expr::Add { left, right, .. }
        | Expr::Sub { left, right, .. }
        | Expr::Mul { left, right, .. }
        | Expr::Div { left, right, .. }
        | Expr::Mod { left, right, .. } => {
            check_literal_ranges(cur, left, stage_span, stage_index)?;
            check_literal_ranges(cur, right, stage_span, stage_index)
        }
        Expr::Neg { inner, .. } => check_literal_ranges(cur, inner, stage_span, stage_index),
        Expr::Dot { .. } => Ok(()),
    }
}

fn expr_result_ty(cur: ElemTy, e: &Expr) -> Result<ElemTy, LirError> {
    match e {
        Expr::Add { left, right, .. } | Expr::Sub { left, right, .. } => Ok(widen(
            expr_result_ty(cur, left)?,
            expr_result_ty(cur, right)?,
        )),
        Expr::Mul { left, right, .. } | Expr::Div { left, right, .. } | Expr::Mod { left, right, .. } => {
            Ok(widen(
                expr_result_ty(cur, left)?,
                expr_result_ty(cur, right)?,
            ))
        }
        Expr::Neg { inner, .. } => expr_result_ty(cur, inner),
        Expr::Dot { .. } => Ok(cur),
        Expr::Lit { .. } => Ok(match cur {
            ElemTy::I32 => ElemTy::I32,
            ElemTy::I64 => ElemTy::I64,
            ElemTy::Bool => unreachable!(),
        }),
    }
}

fn widen(a: ElemTy, b: ElemTy) -> ElemTy {
    if a == ElemTy::I64 || b == ElemTy::I64 {
        ElemTy::I64
    } else {
        ElemTy::I32
    }
}

fn check_scan_init(
    init: i64,
    cur: ElemTy,
    span: Span,
    stage_index: usize,
) -> Result<(), LirError> {
    match cur {
        ElemTy::I32 => {
            if init < i32::MIN as i64 || init > i32::MAX as i64 {
                return Err(LirError::Type {
                    code: "T_SCAN_INIT_RANGE",
                    span,
                    message: "scan initializer does not fit i32 stream type".into(),
                    fix_hint: "Use a smaller initializer or switch to input:i64.".into(),
                    stage_index: Some(stage_index),
                });
            }
        }
        ElemTy::I64 => {}
        ElemTy::Bool => {
            return Err(LirError::Type {
                code: "T_SCAN_BOOL",
                span,
                message: "scan requires integer stream".into(),
                fix_hint: "Use integer streams with scan.".into(),
                stage_index: Some(stage_index),
            });
        }
    }
    Ok(())
}

fn check_reduce_ok(
    cur: ElemTy,
    op: crate::ast::ReduceOp,
    stage_index: usize,
    span: Span,
) -> Result<(), LirError> {
    match op {
        crate::ast::ReduceOp::Count => Ok(()),
        _ if cur == ElemTy::Bool => Err(LirError::Type {
            code: "T_REDUCE_BOOL",
            span,
            message: "this reducer is not supported on bool streams in v1".into(),
            fix_hint: "Use reduce count, or map to integers first (not available for bool in v1).".into(),
            stage_index: Some(stage_index),
        }),
        _ => Ok(()),
    }
}
