use crate::ast::{
    CmpOp, CmpRhs, ElemTy, Expr, Predicate, Program, ReduceOp, ScanOp, Source, Stage,
};
use crate::error::{LirError, Span};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Val {
    I32(i32),
    I64(i64),
    Bool(bool),
}

#[derive(Debug)]
pub enum RunOutcome {
    Stream(Vec<Val>),
    Scalar(Val),
}

pub fn run_program(p: &Program, input: &[Val]) -> Result<RunOutcome, LirError> {
    let mut flow = RunOutcome::Stream(source_values(&p.source, input)?);
    for (idx, st) in p.stages.iter().enumerate() {
        flow = run_stage(flow, st, idx)?;
    }
    Ok(flow)
}

fn source_values(src: &Source, input: &[Val]) -> Result<Vec<Val>, LirError> {
    match src {
        Source::Input { ty, span, .. } => {
            validate_input(*ty, input, *span)?;
            Ok(input.to_vec())
        }
        Source::Range {
            start,
            stop,
            step,
            span,
        } => Ok(range_values(*start, *stop, *step, *span)?),
        Source::Lit { elems, span } => lit_values(elems, *span),
    }
}

fn validate_input(ty: ElemTy, input: &[Val], span: Span) -> Result<(), LirError> {
    for (i, v) in input.iter().enumerate() {
        let ok = match (ty, v) {
            (ElemTy::I32, Val::I32(_)) => true,
            (ElemTy::I64, Val::I64(_)) => true,
            (ElemTy::Bool, Val::Bool(_)) => true,
            _ => false,
        };
        if !ok {
            return Err(LirError::Runtime {
                code: "R_INPUT_TY",
                span,
                message: format!("input value at index {i} does not match declared input type"),
                fix_hint: "Pass a JSON array matching input:i32, input:i64, or input:bool.".into(),
                stage_index: 0,
                element_index: Some(i),
            });
        }
    }
    Ok(())
}

fn range_values(start: i64, stop: i64, step: i64, span: Span) -> Result<Vec<Val>, LirError> {
    let mut out = Vec::new();
    let mut x = start;
    loop {
        if step > 0 {
            if x >= stop {
                break;
            }
        } else if step < 0 {
            if x <= stop {
                break;
            }
        } else {
            return Err(LirError::Runtime {
                code: "R_RANGE_STEP",
                span,
                message: "internal: zero step".into(),
                fix_hint: "Report a compiler bug.".into(),
                stage_index: 0,
                element_index: None,
            });
        }
        if x < i32::MIN as i64 || x > i32::MAX as i64 {
            return Err(LirError::Runtime {
                code: "R_RANGE_ELEM_RANGE",
                span,
                message: "range element does not fit i32".into(),
                fix_hint: "Use smaller bounds or extend language to i64 range (future).".into(),
                stage_index: 0,
                element_index: None,
            });
        }
        out.push(Val::I32(x as i32));
        x = x
            .checked_add(step)
            .ok_or_else(|| runtime_overflow(span, 0))?;
    }
    Ok(out)
}

fn lit_values(
    elems: &[crate::ast::LitElem],
    _span: Span,
) -> Result<Vec<Val>, LirError> {
    let mut out = Vec::new();
    for e in elems {
        out.push(match e {
            crate::ast::LitElem::I32(v) => Val::I32(*v),
            crate::ast::LitElem::I64(v) => Val::I64(*v),
            crate::ast::LitElem::Bool(b) => Val::Bool(*b),
        });
    }
    Ok(out)
}

fn run_stage(flow: RunOutcome, st: &Stage, stage_index: usize) -> Result<RunOutcome, LirError> {
    let span = stage_span(st);
    match flow {
        RunOutcome::Scalar(_) => Err(LirError::Runtime {
            code: "R_INDEX",
            span,
            message: "internal: scalar passed to stage".into(),
            fix_hint: "Report a compiler bug.".into(),
            stage_index,
            element_index: None,
        }),
        RunOutcome::Stream(mut vs) => match st {
            Stage::Filter { pred, .. } => {
                let ty = elem_ty_of_stream(&vs)?;
                let mut out = Vec::new();
                for (i, v) in vs.into_iter().enumerate() {
                    if eval_pred(pred, &v, ty, span, stage_index, i)? {
                        out.push(v);
                    }
                }
                Ok(RunOutcome::Stream(out))
            }
            Stage::Map { expr, .. } => {
                let mut out = Vec::new();
                for (i, v) in vs.into_iter().enumerate() {
                    out.push(eval_expr(expr, &v, span, stage_index, i)?);
                }
                Ok(RunOutcome::Stream(out))
            }
            Stage::Scan { init, op, .. } => {
                let ty = elem_ty_of_stream(&vs)?;
                let mut out = Vec::new();
                let mut acc = scan_init_val(*init, ty, span, stage_index)?;
                out.push(acc.clone());
                for (i, v) in vs.into_iter().enumerate() {
                    acc = scan_step(*op, &acc, &v, ty, span, stage_index, i)?;
                    out.push(acc.clone());
                }
                Ok(RunOutcome::Stream(out))
            }
            Stage::Reduce { op, .. } => {
                let ty = elem_ty_of_stream(&vs)?;
                Ok(RunOutcome::Scalar(reduce_op(
                    *op, &vs, ty, span, stage_index,
                )?))
            }
            Stage::Take { n, .. } => {
                vs.truncate(*n as usize);
                Ok(RunOutcome::Stream(vs))
            }
            Stage::Drop { n, .. } => {
                let n = *n as usize;
                if n > vs.len() {
                    vs.clear();
                } else {
                    vs.drain(0..n);
                }
                Ok(RunOutcome::Stream(vs))
            }
            Stage::Id { .. } => Ok(RunOutcome::Stream(vs)),
        },
    }
}

fn elem_ty_of_stream(vs: &[Val]) -> Result<ElemTy, LirError> {
    if let Some(v) = vs.first() {
        Ok(match v {
            Val::I32(_) => ElemTy::I32,
            Val::I64(_) => ElemTy::I64,
            Val::Bool(_) => ElemTy::Bool,
        })
    } else {
        // Empty stream: treat as i32 container (matches empty lit() typing).
        Ok(ElemTy::I32)
    }
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

fn eval_pred(
    p: &Predicate,
    v: &Val,
    ty: ElemTy,
    stage_span: Span,
    stage_index: usize,
    element_index: usize,
) -> Result<bool, LirError> {
    match p {
        Predicate::Or { left, right, .. } => {
            if eval_pred(left, v, ty, stage_span, stage_index, element_index)? {
                Ok(true)
            } else {
                eval_pred(right, v, ty, stage_span, stage_index, element_index)
            }
        }
        Predicate::And { left, right, .. } => {
            if !eval_pred(left, v, ty, stage_span, stage_index, element_index)? {
                Ok(false)
            } else {
                eval_pred(right, v, ty, stage_span, stage_index, element_index)
            }
        }
        Predicate::Not { inner, .. } => Ok(!eval_pred(
            inner,
            v,
            ty,
            stage_span,
            stage_index,
            element_index,
        )?),
        Predicate::Even { .. } => {
            let n = as_int(v, ty, stage_span, stage_index, element_index)?;
            Ok(n % 2 == 0)
        }
        Predicate::Odd { .. } => {
            let n = as_int(v, ty, stage_span, stage_index, element_index)?;
            Ok(n % 2 != 0)
        }
        Predicate::Cmp { op, rhs, .. } => {
            cmp_val(*op, v, ty, rhs, stage_span, stage_index, element_index)
        }
    }
}

fn cmp_val(
    op: CmpOp,
    v: &Val,
    ty: ElemTy,
    rhs: &CmpRhs,
    stage_span: Span,
    stage_index: usize,
    element_index: usize,
) -> Result<bool, LirError> {
    match (ty, v, rhs) {
        (ElemTy::Bool, Val::Bool(b), CmpRhs::Bool(r)) => Ok(match op {
            CmpOp::Eq => b == r,
            _ => {
                return Err(LirError::Runtime {
                    code: "R_INDEX",
                    span: stage_span,
                    message: "internal: non-eq bool cmp".into(),
                    fix_hint: "Report a compiler bug.".into(),
                    stage_index,
                    element_index: Some(element_index),
                });
            }
        }),
        (_, _, CmpRhs::Bool(_)) => Err(LirError::Runtime {
            code: "R_INDEX",
            span: stage_span,
            message: "internal: bool rhs on integer stream".into(),
            fix_hint: "Report a compiler bug.".into(),
            stage_index,
            element_index: Some(element_index),
        }),
        (ElemTy::I32, Val::I32(l), CmpRhs::Int(r)) => {
            if *r < i32::MIN as i64 || *r > i32::MAX as i64 {
                return Err(LirError::Runtime {
                    code: "R_INDEX",
                    span: stage_span,
                    message: "cmp rhs out of range".into(),
                    fix_hint: "Report a compiler bug.".into(),
                    stage_index,
                    element_index: Some(element_index),
                });
            }
            Ok(cmp_i32(*l, *r as i32, op))
        }
        (ElemTy::I64, Val::I64(l), CmpRhs::Int(r)) => Ok(cmp_i64(*l, *r, op)),
        _ => Err(LirError::Runtime {
            code: "R_INDEX",
            span: stage_span,
            message: "internal cmp mismatch".into(),
            fix_hint: "Report a compiler bug.".into(),
            stage_index,
            element_index: Some(element_index),
        }),
    }
}

fn cmp_i32(l: i32, r: i32, op: CmpOp) -> bool {
    match op {
        CmpOp::Eq => l == r,
        CmpOp::Lt => l < r,
        CmpOp::Le => l <= r,
        CmpOp::Gt => l > r,
        CmpOp::Ge => l >= r,
    }
}

fn cmp_i64(l: i64, r: i64, op: CmpOp) -> bool {
    match op {
        CmpOp::Eq => l == r,
        CmpOp::Lt => l < r,
        CmpOp::Le => l <= r,
        CmpOp::Gt => l > r,
        CmpOp::Ge => l >= r,
    }
}

fn as_int(
    v: &Val,
    ty: ElemTy,
    stage_span: Span,
    stage_index: usize,
    element_index: usize,
) -> Result<i64, LirError> {
    match (ty, v) {
        (ElemTy::I32, Val::I32(x)) => Ok(*x as i64),
        (ElemTy::I64, Val::I64(x)) => Ok(*x),
        _ => Err(LirError::Runtime {
            code: "R_INDEX",
            span: stage_span,
            message: "expected integer value".into(),
            fix_hint: "Report a compiler bug.".into(),
            stage_index,
            element_index: Some(element_index),
        }),
    }
}

fn eval_expr(
    e: &Expr,
    elem: &Val,
    stage_span: Span,
    stage_index: usize,
    element_index: usize,
) -> Result<Val, LirError> {
    match e {
        Expr::Add { left, right, .. } => {
            let a = eval_expr(left, elem, stage_span, stage_index, element_index)?;
            let b = eval_expr(right, elem, stage_span, stage_index, element_index)?;
            val_add(&a, &b, stage_span, stage_index, element_index)
        }
        Expr::Sub { left, right, .. } => {
            let a = eval_expr(left, elem, stage_span, stage_index, element_index)?;
            let b = eval_expr(right, elem, stage_span, stage_index, element_index)?;
            val_sub(&a, &b, stage_span, stage_index, element_index)
        }
        Expr::Mul { left, right, .. } => {
            let a = eval_expr(left, elem, stage_span, stage_index, element_index)?;
            let b = eval_expr(right, elem, stage_span, stage_index, element_index)?;
            val_mul(&a, &b, stage_span, stage_index, element_index)
        }
        Expr::Div { left, right, .. } => {
            let a = eval_expr(left, elem, stage_span, stage_index, element_index)?;
            let b = eval_expr(right, elem, stage_span, stage_index, element_index)?;
            val_div(&a, &b, stage_span, stage_index, element_index)
        }
        Expr::Mod { left, right, .. } => {
            let a = eval_expr(left, elem, stage_span, stage_index, element_index)?;
            let b = eval_expr(right, elem, stage_span, stage_index, element_index)?;
            val_mod(&a, &b, stage_span, stage_index, element_index)
        }
        Expr::Neg { inner, .. } => {
            let v = eval_expr(inner, elem, stage_span, stage_index, element_index)?;
            val_neg(&v, stage_span, stage_index, element_index)
        }
        Expr::Dot { .. } => Ok(elem.clone()),
        Expr::Lit { v, .. } => Ok(if *v >= i32::MIN as i64 && *v <= i32::MAX as i64 {
            Val::I32(*v as i32)
        } else {
            Val::I64(*v)
        }),
    }
}

fn val_add(
    a: &Val,
    b: &Val,
    stage_span: Span,
    stage_index: usize,
    element_index: usize,
) -> Result<Val, LirError> {
    match (a, b) {
        (Val::I32(x), Val::I32(y)) => x
            .checked_add(*y)
            .map(Val::I32)
            .ok_or_else(|| runtime_overflow(stage_span, stage_index)),
        (Val::I64(x), Val::I64(y)) => x
            .checked_add(*y)
            .map(Val::I64)
            .ok_or_else(|| runtime_overflow(stage_span, stage_index)),
        (Val::I32(x), Val::I64(y)) => (*x as i64)
            .checked_add(*y)
            .map(Val::I64)
            .ok_or_else(|| runtime_overflow(stage_span, stage_index)),
        (Val::I64(x), Val::I32(y)) => x
            .checked_add(*y as i64)
            .map(Val::I64)
            .ok_or_else(|| runtime_overflow(stage_span, stage_index)),
        _ => Err(LirError::Runtime {
            code: "R_INDEX",
            span: stage_span,
            message: "add type mismatch".into(),
            fix_hint: "Report a compiler bug.".into(),
            stage_index,
            element_index: Some(element_index),
        }),
    }
}

fn val_sub(
    a: &Val,
    b: &Val,
    stage_span: Span,
    stage_index: usize,
    element_index: usize,
) -> Result<Val, LirError> {
    match (a, b) {
        (Val::I32(x), Val::I32(y)) => x
            .checked_sub(*y)
            .map(Val::I32)
            .ok_or_else(|| runtime_overflow(stage_span, stage_index)),
        (Val::I64(x), Val::I64(y)) => x
            .checked_sub(*y)
            .map(Val::I64)
            .ok_or_else(|| runtime_overflow(stage_span, stage_index)),
        (Val::I32(x), Val::I64(y)) => (*x as i64)
            .checked_sub(*y)
            .map(Val::I64)
            .ok_or_else(|| runtime_overflow(stage_span, stage_index)),
        (Val::I64(x), Val::I32(y)) => x
            .checked_sub(*y as i64)
            .map(Val::I64)
            .ok_or_else(|| runtime_overflow(stage_span, stage_index)),
        _ => Err(LirError::Runtime {
            code: "R_INDEX",
            span: stage_span,
            message: "sub type mismatch".into(),
            fix_hint: "Report a compiler bug.".into(),
            stage_index,
            element_index: Some(element_index),
        }),
    }
}

fn val_mul(
    a: &Val,
    b: &Val,
    stage_span: Span,
    stage_index: usize,
    element_index: usize,
) -> Result<Val, LirError> {
    match (a, b) {
        (Val::I32(x), Val::I32(y)) => x
            .checked_mul(*y)
            .map(Val::I32)
            .ok_or_else(|| runtime_overflow(stage_span, stage_index)),
        (Val::I64(x), Val::I64(y)) => x
            .checked_mul(*y)
            .map(Val::I64)
            .ok_or_else(|| runtime_overflow(stage_span, stage_index)),
        (Val::I32(x), Val::I64(y)) => (*x as i64)
            .checked_mul(*y)
            .map(Val::I64)
            .ok_or_else(|| runtime_overflow(stage_span, stage_index)),
        (Val::I64(x), Val::I32(y)) => x
            .checked_mul(*y as i64)
            .map(Val::I64)
            .ok_or_else(|| runtime_overflow(stage_span, stage_index)),
        _ => Err(LirError::Runtime {
            code: "R_INDEX",
            span: stage_span,
            message: "mul type mismatch".into(),
            fix_hint: "Report a compiler bug.".into(),
            stage_index,
            element_index: Some(element_index),
        }),
    }
}

fn val_div(
    a: &Val,
    b: &Val,
    stage_span: Span,
    stage_index: usize,
    element_index: usize,
) -> Result<Val, LirError> {
    match (a, b) {
        (Val::I32(x), Val::I32(y)) => {
            if *y == 0 {
                return Err(div_zero(stage_span, stage_index, element_index));
            }
            if *x == i32::MIN && *y == -1 {
                return Err(runtime_overflow(stage_span, stage_index));
            }
            Ok(Val::I32(x / y))
        }
        (Val::I64(x), Val::I64(y)) => {
            if *y == 0 {
                return Err(div_zero(stage_span, stage_index, element_index));
            }
            if *x == i64::MIN && *y == -1 {
                return Err(runtime_overflow(stage_span, stage_index));
            }
            Ok(Val::I64(x / y))
        }
        (Val::I32(x), Val::I64(y)) => {
            let x = *x as i64;
            if *y == 0 {
                return Err(div_zero(stage_span, stage_index, element_index));
            }
            if x == i64::MIN && *y == -1 {
                return Err(runtime_overflow(stage_span, stage_index));
            }
            Ok(Val::I64(x / y))
        }
        (Val::I64(x), Val::I32(y)) => {
            let y = *y as i64;
            if y == 0 {
                return Err(div_zero(stage_span, stage_index, element_index));
            }
            if *x == i64::MIN && y == -1 {
                return Err(runtime_overflow(stage_span, stage_index));
            }
            Ok(Val::I64(x / y))
        }
        _ => Err(LirError::Runtime {
            code: "R_INDEX",
            span: stage_span,
            message: "div type mismatch".into(),
            fix_hint: "Report a compiler bug.".into(),
            stage_index,
            element_index: Some(element_index),
        }),
    }
}

fn val_mod(
    a: &Val,
    b: &Val,
    stage_span: Span,
    stage_index: usize,
    element_index: usize,
) -> Result<Val, LirError> {
    match (a, b) {
        (Val::I32(x), Val::I32(y)) => {
            if *y == 0 {
                return Err(div_zero(stage_span, stage_index, element_index));
            }
            if *x == i32::MIN && *y == -1 {
                return Err(runtime_overflow(stage_span, stage_index));
            }
            Ok(Val::I32(x % y))
        }
        (Val::I64(x), Val::I64(y)) => {
            if *y == 0 {
                return Err(div_zero(stage_span, stage_index, element_index));
            }
            if *x == i64::MIN && *y == -1 {
                return Err(runtime_overflow(stage_span, stage_index));
            }
            Ok(Val::I64(x % y))
        }
        (Val::I32(x), Val::I64(y)) => {
            let x = *x as i64;
            if *y == 0 {
                return Err(div_zero(stage_span, stage_index, element_index));
            }
            if x == i64::MIN && *y == -1 {
                return Err(runtime_overflow(stage_span, stage_index));
            }
            Ok(Val::I64(x % *y))
        }
        (Val::I64(x), Val::I32(y)) => {
            let y = *y as i64;
            if y == 0 {
                return Err(div_zero(stage_span, stage_index, element_index));
            }
            if *x == i64::MIN && y == -1 {
                return Err(runtime_overflow(stage_span, stage_index));
            }
            Ok(Val::I64(x % y))
        }
        _ => Err(LirError::Runtime {
            code: "R_INDEX",
            span: stage_span,
            message: "mod type mismatch".into(),
            fix_hint: "Report a compiler bug.".into(),
            stage_index,
            element_index: Some(element_index),
        }),
    }
}

fn val_neg(
    v: &Val,
    stage_span: Span,
    stage_index: usize,
    element_index: usize,
) -> Result<Val, LirError> {
    match v {
        Val::I32(x) => x
            .checked_neg()
            .map(Val::I32)
            .ok_or_else(|| runtime_overflow(stage_span, stage_index)),
        Val::I64(x) => x
            .checked_neg()
            .map(Val::I64)
            .ok_or_else(|| runtime_overflow(stage_span, stage_index)),
        _ => Err(LirError::Runtime {
            code: "R_INDEX",
            span: stage_span,
            message: "neg type mismatch".into(),
            fix_hint: "Report a compiler bug.".into(),
            stage_index,
            element_index: Some(element_index),
        }),
    }
}

fn scan_init_val(
    init: i64,
    ty: ElemTy,
    stage_span: Span,
    stage_index: usize,
) -> Result<Val, LirError> {
    match ty {
        ElemTy::I32 => {
            if init < i32::MIN as i64 || init > i32::MAX as i64 {
                return Err(LirError::Runtime {
                    code: "R_SCAN_INIT_RANGE",
                    span: stage_span,
                    message: "scan init does not fit i32".into(),
                    fix_hint: "Fix initializer or use i64 streams.".into(),
                    stage_index,
                    element_index: None,
                });
            }
            Ok(Val::I32(init as i32))
        }
        ElemTy::I64 => Ok(Val::I64(init)),
        ElemTy::Bool => Err(LirError::Runtime {
            code: "R_INDEX",
            span: stage_span,
            message: "internal: scan on bool stream".into(),
            fix_hint: "Report a compiler bug.".into(),
            stage_index,
            element_index: None,
        }),
    }
}

fn scan_step(
    op: ScanOp,
    acc: &Val,
    v: &Val,
    _ty: ElemTy,
    stage_span: Span,
    stage_index: usize,
    element_index: usize,
) -> Result<Val, LirError> {
    match op {
        ScanOp::Add => val_add(acc, v, stage_span, stage_index, element_index),
        ScanOp::Sub => val_sub(acc, v, stage_span, stage_index, element_index),
        ScanOp::Mul => val_mul(acc, v, stage_span, stage_index, element_index),
    }
}

fn reduce_op(
    op: ReduceOp,
    vs: &[Val],
    ty: ElemTy,
    stage_span: Span,
    stage_index: usize,
) -> Result<Val, LirError> {
    match op {
        ReduceOp::Count => Ok(Val::I32(vs.len() as i32)),
        ReduceOp::Sum => {
            if vs.is_empty() {
                return match ty {
                    ElemTy::I32 => Ok(Val::I32(0)),
                    ElemTy::I64 => Ok(Val::I64(0)),
                    ElemTy::Bool => Err(LirError::Runtime {
                        code: "R_INDEX",
                        span: stage_span,
                        message: "internal: sum on bool stream".into(),
                        fix_hint: "Report a compiler bug.".into(),
                        stage_index,
                        element_index: None,
                    }),
                };
            }
            let mut acc = vs[0].clone();
            for (i, v) in vs.iter().enumerate().skip(1) {
                acc = val_add(&acc, v, stage_span, stage_index, i)?;
            }
            Ok(acc)
        }
        ReduceOp::Prod => {
            if vs.is_empty() {
                return match ty {
                    ElemTy::I32 => Ok(Val::I32(1)),
                    ElemTy::I64 => Ok(Val::I64(1)),
                    ElemTy::Bool => Err(LirError::Runtime {
                        code: "R_INDEX",
                        span: stage_span,
                        message: "internal: prod on bool stream".into(),
                        fix_hint: "Report a compiler bug.".into(),
                        stage_index,
                        element_index: None,
                    }),
                };
            }
            let mut acc = vs[0].clone();
            for (i, v) in vs.iter().enumerate().skip(1) {
                acc = val_mul(&acc, v, stage_span, stage_index, i)?;
            }
            Ok(acc)
        }
        ReduceOp::Min | ReduceOp::Max => {
            if vs.is_empty() {
                return Err(LirError::Runtime {
                    code: "R_REDUCE_EMPTY_MINMAX",
                    span: stage_span,
                    message: "min/max on empty stream".into(),
                    fix_hint: "Guard with take/drop or handle empty input in host.".into(),
                    stage_index,
                    element_index: None,
                });
            }
            let mut acc = vs[0].clone();
            for v in vs.iter().skip(1) {
                acc = if op == ReduceOp::Min {
                    val_min(&acc, v)?
                } else {
                    val_max(&acc, v)?
                };
            }
            Ok(acc)
        }
    }
}

fn val_min(a: &Val, b: &Val) -> Result<Val, LirError> {
    match (a, b) {
        (Val::I32(x), Val::I32(y)) => Ok(Val::I32(*x.min(y))),
        (Val::I64(x), Val::I64(y)) => Ok(Val::I64(*x.min(y))),
        (Val::I32(x), Val::I64(y)) => Ok(Val::I64((*x as i64).min(*y))),
        (Val::I64(x), Val::I32(y)) => Ok(Val::I64((*x).min(*y as i64))),
        _ => Err(LirError::Runtime {
            code: "R_INDEX",
            span: Span::new(0, 0),
            message: "min mismatch".into(),
            fix_hint: "Report a compiler bug.".into(),
            stage_index: 0,
            element_index: None,
        }),
    }
}

fn val_max(a: &Val, b: &Val) -> Result<Val, LirError> {
    match (a, b) {
        (Val::I32(x), Val::I32(y)) => Ok(Val::I32(*x.max(y))),
        (Val::I64(x), Val::I64(y)) => Ok(Val::I64(*x.max(y))),
        (Val::I32(x), Val::I64(y)) => Ok(Val::I64((*x as i64).max(*y))),
        (Val::I64(x), Val::I32(y)) => Ok(Val::I64((*x).max(*y as i64))),
        _ => Err(LirError::Runtime {
            code: "R_INDEX",
            span: Span::new(0, 0),
            message: "max mismatch".into(),
            fix_hint: "Report a compiler bug.".into(),
            stage_index: 0,
            element_index: None,
        }),
    }
}

fn runtime_overflow(span: Span, stage_index: usize) -> LirError {
    LirError::Runtime {
        code: "R_INTEGER_OVERFLOW",
        span,
        message: "integer overflow".into(),
        fix_hint: "Use a smaller intermediate range or i64 streams.".into(),
        stage_index,
        element_index: None,
    }
}

fn div_zero(span: Span, stage_index: usize, element_index: usize) -> LirError {
    LirError::Runtime {
        code: "R_DIV_BY_ZERO",
        span,
        message: "division or remainder by zero".into(),
        fix_hint: "Avoid zero divisors in map/mod/div.".into(),
        stage_index,
        element_index: Some(element_index),
    }
}
