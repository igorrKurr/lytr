//! LLVM IR (text) for `i32` / `i64` streams and `bool` streams (`i8` 0/1).
//! `map` expressions are lowered recursively with checked overflow intrinsics.

use crate::ast::{
    CmpOp, CmpRhs, ElemTy, Expr, LitElem, Predicate, Program, ReduceOp, ScanOp, Source, Stage,
};
use crate::error::{LirError, Span};
use crate::types::source_stream_ty;

const MAX_MATERIALIZED_LEN: usize = 1 << 20;

#[derive(Clone, Copy, PartialEq, Eq)]
enum IntWidth {
    /// `i8` in LLVM (`true` → 1, `false` → 0).
    W8,
    W32,
    W64,
}

impl IntWidth {
    fn llvm_ty(self) -> &'static str {
        match self {
            IntWidth::W8 => "i8",
            IntWidth::W32 => "i32",
            IntWidth::W64 => "i64",
        }
    }

    fn pair_ty(self) -> &'static str {
        match self {
            IntWidth::W8 => "{ i8, i1 }",
            IntWidth::W32 => "{ i32, i1 }",
            IntWidth::W64 => "{ i64, i1 }",
        }
    }

    fn sadd_intrinsic(self) -> &'static str {
        match self {
            IntWidth::W8 => "llvm.sadd.with.overflow.i8",
            IntWidth::W32 => "llvm.sadd.with.overflow.i32",
            IntWidth::W64 => "llvm.sadd.with.overflow.i64",
        }
    }

    fn ssub_intrinsic(self) -> &'static str {
        match self {
            IntWidth::W8 => "llvm.ssub.with.overflow.i8",
            IntWidth::W32 => "llvm.ssub.with.overflow.i32",
            IntWidth::W64 => "llvm.ssub.with.overflow.i64",
        }
    }

    fn smul_intrinsic(self) -> &'static str {
        match self {
            IntWidth::W8 => "llvm.smul.with.overflow.i8",
            IntWidth::W32 => "llvm.smul.with.overflow.i32",
            IntWidth::W64 => "llvm.smul.with.overflow.i64",
        }
    }

    fn align(self) -> u32 {
        match self {
            IntWidth::W8 => 1,
            IntWidth::W32 => 4,
            IntWidth::W64 => 8,
        }
    }

    fn int_min_div(self) -> &'static str {
        match self {
            IntWidth::W8 => "-128",
            IntWidth::W32 => "-2147483648",
            IntWidth::W64 => "-9223372036854775808",
        }
    }
}

fn llvm_fn_ret_ty(width: IntWidth, reduce: ReduceOp) -> &'static str {
    match reduce {
        ReduceOp::Count => "i32",
        _ => match width {
            IntWidth::W8 => "i8",
            IntWidth::W32 => "i32",
            IntWidth::W64 => "i64",
        },
    }
}

#[derive(Clone, Copy)]
enum PipelineOp<'a> {
    Filter(&'a Predicate),
    Map(&'a Expr),
}

pub fn emit_llvm_ir(p: &Program) -> Result<String, LirError> {
    let plan = compile_plan(p)?;

    let mut out = String::new();
    emit_prelude(&mut out);

    match &plan.kind {
        PlanKind::Input { drop, take } => {
            emit_input_main(
                &mut out,
                plan.width,
                plan.reduce,
                *drop,
                *take,
                &plan.pre_scan,
                &plan.post_scan,
                plan.scan,
            );
        }
        PlanKind::ArrayI32 { elems } => {
            emit_rodata_i32(&mut out, elems);
            emit_array_main(
                &mut out,
                plan.width,
                plan.reduce,
                elems.len(),
                &plan.pre_scan,
                &plan.post_scan,
                plan.scan,
            );
        }
        PlanKind::ArrayI64 { elems } => {
            emit_rodata_i64(&mut out, elems);
            emit_array_main(
                &mut out,
                plan.width,
                plan.reduce,
                elems.len(),
                &plan.pre_scan,
                &plan.post_scan,
                plan.scan,
            );
        }
        PlanKind::ArrayI8 { elems } => {
            emit_rodata_i8(&mut out, elems);
            emit_array_main(
                &mut out,
                plan.width,
                plan.reduce,
                elems.len(),
                &plan.pre_scan,
                &plan.post_scan,
                plan.scan,
            );
        }
    }

    out.push_str("\nattributes #0 = { nounwind }\n");
    Ok(out)
}

fn codegen_err(span: Span, msg: &str) -> LirError {
    LirError::Type {
        code: "T_CODEGEN_UNSUPPORTED",
        span,
        message: msg.into(),
        fix_hint: "Use input:i32|i64|bool or materialized range/lit, prefix take/drop, then filter/map/id in order, optional scan, then filter/map/id, reduce sum|prod|count|min|max.".into(),
        stage_index: None,
    }
}

struct Plan<'a> {
    width: IntWidth,
    kind: PlanKind,
    pre_scan: Vec<PipelineOp<'a>>,
    post_scan: Vec<PipelineOp<'a>>,
    scan: Option<(i64, ScanOp)>,
    reduce: ReduceOp,
}

enum PlanKind {
    Input { drop: u32, take: Option<u32> },
    ArrayI32 { elems: Vec<i32> },
    ArrayI64 { elems: Vec<i64> },
    /// Boolean stream as `i8` 0/1.
    ArrayI8 { elems: Vec<u8> },
}

fn compile_plan<'a>(p: &'a Program) -> Result<Plan<'a>, LirError> {
    let stages = &p.stages;
    let Some(Stage::Reduce { op: reduce, .. }) = stages.last() else {
        return Err(codegen_err(p.span, "last stage must be reduce"));
    };
    let reduce = *reduce;
    let mid = &stages[..stages.len() - 1];

    let mut drop_acc: u32 = 0;
    let mut take_acc: Option<u32> = None;
    let mut prefix_end = 0usize;
    for (i, s) in mid.iter().enumerate() {
        match s {
            Stage::Drop { n, .. } => {
                drop_acc = drop_acc.saturating_add(*n);
                prefix_end = i + 1;
            }
            Stage::Take { n, .. } => {
                take_acc = Some(match take_acc {
                    Some(t) => t.min(*n),
                    None => *n,
                });
                prefix_end = i + 1;
            }
            Stage::Id { .. } => prefix_end = i + 1,
            _ => break,
        }
    }

    let src_ty = source_stream_ty(&p.source)?;
    let width = match src_ty {
        ElemTy::Bool => IntWidth::W8,
        ElemTy::I32 => IntWidth::W32,
        ElemTy::I64 => IntWidth::W64,
    };

    let tail = &mid[prefix_end..];
    let mut pre_scan = Vec::new();
    let mut post_scan = Vec::new();
    let mut scan: Option<(i64, ScanOp)> = None;
    let mut ti = 0usize;
    while ti < tail.len() {
        match &tail[ti] {
            Stage::Filter { pred, .. } => {
                pre_scan.push(PipelineOp::Filter(pred));
                ti += 1;
            }
            Stage::Map { expr, .. } => {
                pre_scan.push(PipelineOp::Map(expr));
                ti += 1;
            }
            Stage::Id { .. } => {
                ti += 1;
            }
            Stage::Scan { init, op, span } => {
                if scan.is_some() {
                    return Err(codegen_err(
                        *span,
                        "LLVM path supports at most one scan stage",
                    ));
                }
                if width == IntWidth::W8 {
                    return Err(codegen_err(
                        *span,
                        "LLVM path does not support scan on bool streams",
                    ));
                }
                if width == IntWidth::W32
                    && (*init < i32::MIN as i64 || *init > i32::MAX as i64)
                {
                    return Err(LirError::Runtime {
                        code: "R_SCAN_INIT_RANGE",
                        span: *span,
                        message: "scan init does not fit i32 stream for LLVM".into(),
                        fix_hint: "Use an i32-sized initializer or input:i64.".into(),
                        stage_index: ti + prefix_end,
                        element_index: None,
                    });
                }
                scan = Some((*init, *op));
                ti += 1;
                break;
            }
            Stage::Take { span, .. } | Stage::Drop { span, .. } | Stage::Reduce { span, .. } => {
                return Err(codegen_err(
                    *span,
                    "take/drop must prefix the pipeline (before filter/map/scan)",
                ));
            }
        }
    }
    while ti < tail.len() {
        match &tail[ti] {
            Stage::Filter { pred, .. } => {
                post_scan.push(PipelineOp::Filter(pred));
                ti += 1;
            }
            Stage::Map { expr, .. } => {
                post_scan.push(PipelineOp::Map(expr));
                ti += 1;
            }
            Stage::Id { .. } => {
                ti += 1;
            }
            Stage::Scan { span, .. }
            | Stage::Take { span, .. }
            | Stage::Drop { span, .. }
            | Stage::Reduce { span, .. } => {
                return Err(codegen_err(
                    *span,
                    "LLVM path allows only filter, map, or id after scan",
                ));
            }
        }
    }

    let kind = match &p.source {
        Source::Input { .. } => PlanKind::Input {
            drop: drop_acc,
            take: take_acc,
        },
        Source::Range {
            start,
            stop,
            step,
            span,
        } => {
            let mut vals = materialize_range(*start, *stop, *step, *span)?;
            apply_drop_take(&mut vals, drop_acc, take_acc);
            if vals.len() > MAX_MATERIALIZED_LEN {
                return Err(LirError::Type {
                    code: "T_CODEGEN_TOO_LARGE",
                    span: *span,
                    message: "range too large to embed".into(),
                    fix_hint: "Use a smaller range.".into(),
                    stage_index: None,
                });
            }
            PlanKind::ArrayI32 { elems: vals }
        }
        Source::Lit { elems, span } => {
            match width {
                IntWidth::W8 => {
                    let mut vals: Vec<u8> = elems
                        .iter()
                        .map(|e| match e {
                            LitElem::Bool(b) => *b as u8,
                            _ => unreachable!("typecheck: homogeneous bool lit"),
                        })
                        .collect();
                    apply_drop_take_u8(&mut vals, drop_acc, take_acc);
                    if vals.len() > MAX_MATERIALIZED_LEN {
                        return Err(LirError::Type {
                            code: "T_CODEGEN_TOO_LARGE",
                            span: *span,
                            message: "lit too large".into(),
                            fix_hint: "Use a smaller lit().".into(),
                            stage_index: None,
                        });
                    }
                    PlanKind::ArrayI8 { elems: vals }
                }
                IntWidth::W32 => {
                    let mut vals = Vec::new();
                    for e in elems {
                        match e {
                            LitElem::I32(v) => vals.push(*v),
                            LitElem::I64(v) => {
                                if *v < i32::MIN as i64 || *v > i32::MAX as i64 {
                                    return Err(codegen_err(
                                        *span,
                                        "i64 literal out of i32 range for LLVM",
                                    ));
                                }
                                vals.push(*v as i32);
                            }
                            LitElem::Bool(_) => unreachable!(),
                        }
                    }
                    apply_drop_take(&mut vals, drop_acc, take_acc);
                    if vals.len() > MAX_MATERIALIZED_LEN {
                        return Err(LirError::Type {
                            code: "T_CODEGEN_TOO_LARGE",
                            span: *span,
                            message: "lit too large".into(),
                            fix_hint: "Use a smaller lit().".into(),
                            stage_index: None,
                        });
                    }
                    PlanKind::ArrayI32 { elems: vals }
                }
                IntWidth::W64 => {
                    let mut vals = Vec::new();
                    for e in elems {
                        match e {
                            LitElem::I32(v) => vals.push(*v as i64),
                            LitElem::I64(v) => vals.push(*v),
                            LitElem::Bool(_) => unreachable!(),
                        }
                    }
                    apply_drop_take_i64(&mut vals, drop_acc, take_acc);
                    if vals.len() > MAX_MATERIALIZED_LEN {
                        return Err(LirError::Type {
                            code: "T_CODEGEN_TOO_LARGE",
                            span: *span,
                            message: "lit too large".into(),
                            fix_hint: "Use a smaller lit().".into(),
                            stage_index: None,
                        });
                    }
                    PlanKind::ArrayI64 { elems: vals }
                }
            }
        }
    };

    Ok(Plan {
        width,
        kind,
        pre_scan,
        post_scan,
        scan,
        reduce,
    })
}

fn materialize_range(start: i64, stop: i64, step: i64, span: Span) -> Result<Vec<i32>, LirError> {
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
                message: "zero step".into(),
                fix_hint: "".into(),
                stage_index: 0,
                element_index: None,
            });
        }
        if x < i32::MIN as i64 || x > i32::MAX as i64 {
            return Err(LirError::Runtime {
                code: "R_RANGE_ELEM_RANGE",
                span,
                message: "range value does not fit i32".into(),
                fix_hint: "".into(),
                stage_index: 0,
                element_index: None,
            });
        }
        out.push(x as i32);
        x = x
            .checked_add(step)
            .ok_or_else(|| LirError::Runtime {
                code: "R_INTEGER_OVERFLOW",
                span,
                message: "range step overflow".into(),
                fix_hint: "".into(),
                stage_index: 0,
                element_index: None,
            })?;
    }
    Ok(out)
}

fn apply_drop_take(vals: &mut Vec<i32>, drop: u32, take: Option<u32>) {
    let d = drop as usize;
    if d >= vals.len() {
        vals.clear();
    } else {
        vals.drain(0..d);
    }
    if let Some(t) = take {
        let t = t as usize;
        if vals.len() > t {
            vals.truncate(t);
        }
    }
}

fn apply_drop_take_i64(vals: &mut Vec<i64>, drop: u32, take: Option<u32>) {
    let d = drop as usize;
    if d >= vals.len() {
        vals.clear();
    } else {
        vals.drain(0..d);
    }
    if let Some(t) = take {
        let t = t as usize;
        if vals.len() > t {
            vals.truncate(t);
        }
    }
}

fn apply_drop_take_u8(vals: &mut Vec<u8>, drop: u32, take: Option<u32>) {
    let d = drop as usize;
    if d >= vals.len() {
        vals.clear();
    } else {
        vals.drain(0..d);
    }
    if let Some(t) = take {
        let t = t as usize;
        if vals.len() > t {
            vals.truncate(t);
        }
    }
}

/// Host-controlled triple must not break IR quoting or inject structure; only conservative chars.
fn llvm_triple_from_env() -> String {
    match std::env::var("LIR_LLVM_TRIPLE") {
        Ok(s) if llvm_triple_is_safe(&s) => s,
        Ok(_) | Err(_) => "unknown-unknown-unknown".into(),
    }
}

fn llvm_triple_is_safe(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.is_ascii()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_'))
}

fn emit_prelude(out: &mut String) {
    use std::fmt::Write;
    let triple = llvm_triple_from_env();
    let _ = writeln!(out, "; LIR v1 — generated LLVM IR");
    let _ = writeln!(out, "target datalayout = \"e-m:e-i64:64-f80:128-n8:16:32:64-S128\"");
    let _ = writeln!(out, "target triple = \"{triple}\"");
    let _ = writeln!(out);
    let _ = writeln!(out, "declare void @llvm.trap() cold noreturn nounwind");
    let _ = writeln!(out, "declare {{ i32, i1 }} @llvm.sadd.with.overflow.i32(i32, i32) nounwind readnone speculatable");
    let _ = writeln!(out, "declare {{ i32, i1 }} @llvm.ssub.with.overflow.i32(i32, i32) nounwind readnone speculatable");
    let _ = writeln!(out, "declare {{ i32, i1 }} @llvm.smul.with.overflow.i32(i32, i32) nounwind readnone speculatable");
    let _ = writeln!(out, "declare {{ i64, i1 }} @llvm.sadd.with.overflow.i64(i64, i64) nounwind readnone speculatable");
    let _ = writeln!(out, "declare {{ i64, i1 }} @llvm.ssub.with.overflow.i64(i64, i64) nounwind readnone speculatable");
    let _ = writeln!(out, "declare {{ i64, i1 }} @llvm.smul.with.overflow.i64(i64, i64) nounwind readnone speculatable");
    let _ = writeln!(out, "declare {{ i8, i1 }} @llvm.sadd.with.overflow.i8(i8, i8) nounwind readnone speculatable");
    let _ = writeln!(out, "declare {{ i8, i1 }} @llvm.ssub.with.overflow.i8(i8, i8) nounwind readnone speculatable");
    let _ = writeln!(out, "declare {{ i8, i1 }} @llvm.smul.with.overflow.i8(i8, i8) nounwind readnone speculatable");
    let _ = writeln!(out);
}

fn emit_rodata_i32(out: &mut String, elems: &[i32]) {
    use std::fmt::Write;
    let n = elems.len();
    let _ = writeln!(out, "@lir_data = private unnamed_addr constant [{n} x i32] [", n = n);
    for (i, v) in elems.iter().enumerate() {
        let _ = writeln!(
            out,
            "  i32 {}{}",
            v,
            if i + 1 < n { "," } else { "" }
        );
    }
    let _ = writeln!(out, "], align 4");
    let _ = writeln!(out);
}

fn emit_rodata_i64(out: &mut String, elems: &[i64]) {
    use std::fmt::Write;
    let n = elems.len();
    let _ = writeln!(out, "@lir_data = private unnamed_addr constant [{n} x i64] [", n = n);
    for (i, v) in elems.iter().enumerate() {
        let _ = writeln!(
            out,
            "  i64 {}{}",
            v,
            if i + 1 < n { "," } else { "" }
        );
    }
    let _ = writeln!(out, "], align 8");
    let _ = writeln!(out);
}

fn emit_rodata_i8(out: &mut String, elems: &[u8]) {
    use std::fmt::Write;
    let n = elems.len();
    let _ = writeln!(out, "@lir_data = private unnamed_addr constant [{n} x i8] [", n = n);
    for (i, v) in elems.iter().enumerate() {
        let _ = writeln!(
            out,
            "  i8 {}{}",
            v,
            if i + 1 < n { "," } else { "" }
        );
    }
    let _ = writeln!(out, "], align 1");
    let _ = writeln!(out);
}

fn emit_input_main(
    out: &mut String,
    width: IntWidth,
    reduce: ReduceOp,
    drop: u32,
    take: Option<u32>,
    pre_scan: &[PipelineOp<'_>],
    post_scan: &[PipelineOp<'_>],
    scan: Option<(i64, ScanOp)>,
) {
    use std::fmt::Write;
    let ret_ty = llvm_fn_ret_ty(width, reduce);
    let et = width.llvm_ty();
    let _ = writeln!(
        out,
        "define {ret_ty} @lir_main({et}* nocapture readonly %in, i32 %in_len) local_unnamed_addr #0 {{",
        ret_ty = ret_ty,
        et = et
    );
    let _ = writeln!(out, "entry:");
    let _ = writeln!(out, "  %dropped = icmp ult i32 %in_len, {drop}", drop = drop);
    let _ = writeln!(
        out,
        "  %base = select i1 %dropped, i32 %in_len, i32 {drop}",
        drop = drop
    );
    let _ = writeln!(out, "  %avail = sub i32 %in_len, %base");
    match take {
        Some(tk) => {
            let _ = writeln!(out, "  %small = icmp ult i32 %avail, {tk}", tk = tk);
            let _ = writeln!(
                out,
                "  %count = select i1 %small, i32 %avail, i32 {tk}",
                tk = tk
            );
        }
        None => {
            let _ = writeln!(out, "  %count = add i32 %avail, 0");
        }
    }
    emit_loop_core(
        out,
        width,
        reduce,
        LoopMode::Input,
        pre_scan,
        post_scan,
        scan,
    );
    let _ = writeln!(out, "}}\n");
}

fn emit_array_main(
    out: &mut String,
    width: IntWidth,
    reduce: ReduceOp,
    n: usize,
    pre_scan: &[PipelineOp<'_>],
    post_scan: &[PipelineOp<'_>],
    scan: Option<(i64, ScanOp)>,
) {
    use std::fmt::Write;
    let ret_ty = llvm_fn_ret_ty(width, reduce);
    let _ = writeln!(
        out,
        "define {ret_ty} @lir_main() local_unnamed_addr #0 {{",
        ret_ty = ret_ty
    );
    let _ = writeln!(out, "entry:");
    emit_loop_core(
        out,
        width,
        reduce,
        LoopMode::Array { n },
        pre_scan,
        post_scan,
        scan,
    );
    let _ = writeln!(out, "}}\n");
}

enum LoopMode {
    Input,
    Array { n: usize },
}

fn emit_scan_pre_seed_at(
    out: &mut String,
    width: IntWidth,
    reduce: ReduceOp,
    mapped: &str,
    dst: &str,
) {
    use std::fmt::Write;
    let acc_ty = llvm_fn_ret_ty(width, reduce);
    match reduce {
        ReduceOp::Sum => {
            let _ = writeln!(
                out,
                "  {dst} = add nsw {acc_ty} 0, {m}",
                dst = dst,
                acc_ty = acc_ty,
                m = mapped
            );
        }
        ReduceOp::Prod => {
            let _ = writeln!(
                out,
                "  {dst} = mul nsw {acc_ty} 1, {m}",
                dst = dst,
                acc_ty = acc_ty,
                m = mapped
            );
        }
        ReduceOp::Count => {
            let _ = writeln!(out, "  {dst} = add nsw i32 0, 1", dst = dst);
        }
        ReduceOp::Min | ReduceOp::Max => {
            let _ = writeln!(
                out,
                "  {dst} = add nsw {acc_ty} {m}, 0",
                dst = dst,
                acc_ty = acc_ty,
                m = mapped
            );
        }
    }
}

fn emit_scan_pre_identity_at(out: &mut String, width: IntWidth, reduce: ReduceOp, dst: &str) {
    use std::fmt::Write;
    let acc_ty = llvm_fn_ret_ty(width, reduce);
    match reduce {
        ReduceOp::Sum | ReduceOp::Min | ReduceOp::Max => {
            let _ = writeln!(
                out,
                "  {dst} = add nsw {acc_ty} 0, 0",
                dst = dst,
                acc_ty = acc_ty
            );
        }
        ReduceOp::Prod => {
            let _ = writeln!(
                out,
                "  {dst} = add nsw {acc_ty} 0, 1",
                dst = dst,
                acc_ty = acc_ty
            );
        }
        ReduceOp::Count => {
            let _ = writeln!(out, "  {dst} = add nsw i32 0, 0", dst = dst);
        }
    }
}

/// Seeds `%acc_seed` (and optionally `%seen_seed` via join) for the reduce, then branches to `%loop`.
fn emit_scan_pre(
    out: &mut String,
    width: IntWidth,
    reduce: ReduceOp,
    init: i64,
    post_scan: &[PipelineOp<'_>],
    tmp: &mut u32,
    minmax: bool,
) -> String {
    use std::fmt::Write;
    let ty = width.llvm_ty();
    let init_lit = match width {
        IntWidth::W8 => format!("{}", init as i8),
        IntWidth::W32 => format!("{}", init as i32),
        IntWidth::W64 => format!("{}", init),
    };
    let _ = writeln!(out, "scan_pre:");
    let _ = writeln!(
        out,
        "  %scan_el0 = add nsw {ty} {lit}, 0",
        ty = ty,
        lit = init_lit
    );

    if post_scan.is_empty() {
        emit_scan_pre_seed_at(out, width, reduce, "%scan_el0", "%acc_seed");
        let _ = writeln!(out, "  br label %loop");
        return "scan_pre".into();
    }

    let mut cg = IrCg {
        out,
        tmp: *tmp,
        w: width,
    };
    let first = format!("pl_{}", cg.fresh());
    let _ = writeln!(cg.out, "  br label %{first}", first = first);
    let (fin, _skips) =
        cg.emit_pipeline_ops(post_scan, "%scan_el0", &first, "scan_pre_fail", "scan_pre_ok");
    *tmp = cg.tmp;

    let _ = writeln!(out, "scan_pre_fail:");
    emit_scan_pre_identity_at(out, width, reduce, "%acc_fail");
    let _ = writeln!(out, "  br label %scan_pre_join");

    let _ = writeln!(out, "scan_pre_ok:");
    emit_scan_pre_seed_at(out, width, reduce, &fin, "%acc_ok");
    let _ = writeln!(out, "  br label %scan_pre_join");

    let _ = writeln!(out, "scan_pre_join:");
    let acc_ty = llvm_fn_ret_ty(width, reduce);
    let _ = writeln!(
        out,
        "  %acc_seed = phi {acc_ty} [ %acc_ok, %scan_pre_ok ], [ %acc_fail, %scan_pre_fail ]",
        acc_ty = acc_ty
    );
    if minmax {
        let _ = writeln!(
            out,
            "  %seen_seed = phi i1 [ true, %scan_pre_ok ], [ false, %scan_pre_fail ]"
        );
    }
    let _ = writeln!(out, "  br label %loop");
    "scan_pre_join".into()
}

fn emit_loop_core(
    out: &mut String,
    width: IntWidth,
    reduce: ReduceOp,
    mode: LoopMode,
    pre_scan: &[PipelineOp<'_>],
    post_scan: &[PipelineOp<'_>],
    scan: Option<(i64, ScanOp)>,
) {
    use std::fmt::Write;

    let has_scan = scan.is_some();
    let minmax = matches!(reduce, ReduceOp::Min | ReduceOp::Max);
    let minmax_need_empty_trap = minmax && !has_scan;
    let elem_ty = width.llvm_ty();
    let acc_ty = llvm_fn_ret_ty(width, reduce);
    let al = width.align();
    let scan_init_lit = scan.map(|(i, _)| match width {
        IntWidth::W8 => format!("{}", i as i8),
        IntWidth::W32 => format!("{}", i as i32),
        IntWidth::W64 => format!("{}", i),
    });

    let mut tmp_counter = 0u32;
    let loop_pred = if let Some((init, _)) = scan {
        let _ = writeln!(out, "  br label %scan_pre");
        emit_scan_pre(
            out,
            width,
            reduce,
            init,
            post_scan,
            &mut tmp_counter,
            minmax,
        )
    } else {
        let _ = writeln!(out, "  br label %loop");
        "entry".into()
    };

    let _ = writeln!(out, "loop:");
    let _ = writeln!(
        out,
        "  %i = phi i32 [ 0, %{lp} ], [ %i1, %inc ]",
        lp = loop_pred
    );

    if let Some(ref sil) = scan_init_lit {
        let _ = writeln!(
            out,
            "  %scan_acc = phi {et} [ {lit}, %{lp} ], [ %scan_acc1, %inc ]",
            et = elem_ty,
            lit = sil,
            lp = loop_pred
        );
    }

    if minmax {
        let seen_entry = if has_scan {
            if post_scan.is_empty() {
                "true"
            } else {
                "%seen_seed"
            }
        } else {
            "false"
        };
        let _ = writeln!(
            out,
            "  %seen = phi i1 [ {se}, %{lp} ], [ %seen1, %inc ]",
            se = seen_entry,
            lp = loop_pred
        );
    }

    if has_scan {
        let _ = writeln!(
            out,
            "  %acc = phi {acc_ty} [ %acc_seed, %{lp} ], [ %acc1, %inc ]",
            acc_ty = acc_ty,
            lp = loop_pred
        );
    } else {
        let acc0 = match reduce {
            ReduceOp::Sum | ReduceOp::Count => match acc_ty {
                "i32" => "0",
                "i64" => "0",
                _ => "0",
            },
            ReduceOp::Prod => match acc_ty {
                "i32" => "1",
                "i64" => "1",
                _ => "1",
            },
            ReduceOp::Min | ReduceOp::Max => match acc_ty {
                "i32" => "0",
                "i64" => "0",
                _ => "0",
            },
        };
        let _ = writeln!(
            out,
            "  %acc = phi {acc_ty} [ {acc0}, %{lp} ], [ %acc1, %inc ]",
            acc_ty = acc_ty,
            acc0 = acc0,
            lp = loop_pred
        );
    }

    let count_op = match &mode {
        LoopMode::Input => "%count".to_string(),
        LoopMode::Array { n } => n.to_string(),
    };
    let _ = writeln!(
        out,
        "  %done = icmp uge i32 %i, {cnt}",
        cnt = count_op
    );
    let _ = writeln!(out, "  br i1 %done, label %exit, label %body");

    let _ = writeln!(out, "body:");
    match mode {
        LoopMode::Input => {
            let _ = writeln!(out, "  %idx = add i32 %base, %i");
            let _ = writeln!(
                out,
                "  %p = getelementptr inbounds {et}, {et}* %in, i32 %idx",
                et = elem_ty
            );
            let _ = writeln!(
                out,
                "  %x = load {et}, {et}* %p, align {al}",
                et = elem_ty,
                al = al
            );
        }
        LoopMode::Array { n } => {
            let _ = writeln!(
                out,
                "  %p = getelementptr inbounds [{n} x {et}], [{n} x {et}]* @lir_data, i64 0, i32 %i",
                n = n,
                et = elem_ty
            );
            let _ = writeln!(
                out,
                "  %x = load {et}, {et}* %p, align {al}",
                et = elem_ty,
                al = al
            );
        }
    }

    let mut cg = IrCg {
        out,
        tmp: tmp_counter,
        w: width,
    };

    let mut inc_skip: Vec<String> = Vec::new();

    let y_after_pre = if pre_scan.is_empty() {
        let _ = writeln!(cg.out, "  br label %scan_hit");
        "%x".to_string()
    } else {
        let first = format!("pl_{}", cg.fresh());
        let _ = writeln!(cg.out, "  br label %{first}", first = first);
        let (fin, skips) = cg.emit_pipeline_ops(pre_scan, "%x", &first, "inc", "scan_hit");
        inc_skip.extend(skips);
        fin
    };

    let _ = writeln!(cg.out, "scan_hit:");
    let sn = if let Some((_, sop)) = scan {
        cg.emit_scan_step(sop, "%scan_acc", &y_after_pre)
    } else {
        y_after_pre.clone()
    };

    let outer_fold_bb = if post_scan.is_empty() {
        "scan_hit"
    } else {
        "fold_tail"
    };
    if post_scan.is_empty() {
        let fold_v = sn.clone();
        emit_reduce_block(&mut cg, reduce, &fold_v);
    } else {
        let first = format!("pl_{}", cg.fresh());
        let _ = writeln!(cg.out, "  br label %{first}", first = first);
        let (fin, skips) = cg.emit_pipeline_ops(post_scan, &sn, &first, "inc", "fold_tail");
        inc_skip.extend(skips);
        let _ = writeln!(cg.out, "fold_tail:");
        emit_reduce_block(&mut cg, reduce, &fin);
    }

    let fold_edge = match reduce {
        ReduceOp::Sum => "sum_ok",
        ReduceOp::Prod => "prod_ok",
        ReduceOp::Count => "cnt_ok",
        ReduceOp::Min | ReduceOp::Max => outer_fold_bb,
    };

    let _ = writeln!(cg.out, "  br label %inc");

    let _ = writeln!(cg.out, "inc:");
    if minmax {
        let mut parts: Vec<String> = inc_skip
            .iter()
            .map(|b| format!("[ %seen, %{b} ]"))
            .collect();
        parts.push(format!("[ true, %{fe} ]", fe = fold_edge));
        let _ = writeln!(cg.out, "  %seen1 = phi i1 {}", parts.join(", "));
    }
    if scan.is_some() {
        let mut parts: Vec<String> = inc_skip
            .iter()
            .map(|b| format!("[ %scan_acc, %{b} ]"))
            .collect();
        parts.push(format!("[ {sn}, %{fe} ]", sn = sn, fe = fold_edge));
        let scan_phi = format!("  %scan_acc1 = phi {} {}", elem_ty, parts.join(", "));
        let _ = writeln!(cg.out, "{scan_phi}");
    }
    let mut acc_parts: Vec<String> = inc_skip
        .iter()
        .map(|b| format!("[ %acc, %{b} ]"))
        .collect();
    acc_parts.push(format!("[ %acch, %{fe} ]", fe = fold_edge));
    let acc_phi = format!("  %acc1 = phi {} {}", acc_ty, acc_parts.join(", "));
    let _ = writeln!(cg.out, "{acc_phi}");
    let _ = writeln!(cg.out, "  %i1 = add nuw nsw i32 %i, 1");
    let _ = writeln!(cg.out, "  br label %loop");

    let _ = writeln!(cg.out, "exit:");
    if minmax_need_empty_trap {
        let _ = writeln!(cg.out, "  br i1 %seen, label %mm_ok, label %trap_mm_empty");
        let _ = writeln!(cg.out, "mm_ok:");
        let _ = writeln!(cg.out, "  ret {acc_ty} %acc", acc_ty = acc_ty);
        let _ = writeln!(cg.out, "trap_mm_empty:");
        let _ = writeln!(cg.out, "  call void @llvm.trap()");
        let _ = writeln!(cg.out, "  unreachable");
    } else {
        let _ = writeln!(cg.out, "  ret {acc_ty} %acc", acc_ty = acc_ty);
    }

    let _ = writeln!(cg.out, "trap_ov:");
    let _ = writeln!(cg.out, "  call void @llvm.trap()");
    let _ = writeln!(cg.out, "  unreachable");

    let _ = writeln!(cg.out, "trap_div:");
    let _ = writeln!(cg.out, "  call void @llvm.trap()");
    let _ = writeln!(cg.out, "  unreachable");
}

fn emit_reduce_block(cg: &mut IrCg<'_>, reduce: ReduceOp, fold_v: &str) {
    use std::fmt::Write;
    let width = cg.w;
    let elem_ty = width.llvm_ty();
    let pair = width.pair_ty();
    let sadd = width.sadd_intrinsic();
    let smul = width.smul_intrinsic();

    match reduce {
        ReduceOp::Sum => {
            let _ = writeln!(
                cg.out,
                "  %t = call {pair} @{sadd}({et} %acc, {et} {fv})",
                pair = pair,
                sadd = sadd,
                et = elem_ty,
                fv = fold_v
            );
            let _ = writeln!(cg.out, "  %ov = extractvalue {pair} %t, 1", pair = pair);
            let _ = writeln!(cg.out, "  br i1 %ov, label %trap_ov, label %sum_ok");
            let _ = writeln!(cg.out, "sum_ok:");
            let _ = writeln!(cg.out, "  %acch = extractvalue {pair} %t, 0", pair = pair);
        }
        ReduceOp::Prod => {
            let _ = writeln!(
                cg.out,
                "  %t = call {pair} @{smul}({et} %acc, {et} {fv})",
                pair = pair,
                smul = smul,
                et = elem_ty,
                fv = fold_v
            );
            let _ = writeln!(cg.out, "  %ov = extractvalue {pair} %t, 1", pair = pair);
            let _ = writeln!(cg.out, "  br i1 %ov, label %trap_ov, label %prod_ok");
            let _ = writeln!(cg.out, "prod_ok:");
            let _ = writeln!(cg.out, "  %acch = extractvalue {pair} %t, 0", pair = pair);
        }
        ReduceOp::Count => {
            let _ = writeln!(
                cg.out,
                "  %t = call {{ i32, i1 }} @llvm.sadd.with.overflow.i32(i32 %acc, i32 1)"
            );
            let _ = writeln!(cg.out, "  %ov = extractvalue {{ i32, i1 }} %t, 1");
            let _ = writeln!(cg.out, "  br i1 %ov, label %trap_ov, label %cnt_ok");
            let _ = writeln!(cg.out, "cnt_ok:");
            let _ = writeln!(cg.out, "  %acch = extractvalue {{ i32, i1 }} %t, 0");
        }
        ReduceOp::Min => {
            let _ = writeln!(
                cg.out,
                "  %mm_le = icmp sle {et} %acc, {fv}",
                et = elem_ty,
                fv = fold_v
            );
            let _ = writeln!(
                cg.out,
                "  %mm_pick = select i1 %mm_le, {et} %acc, {et} {fv}",
                et = elem_ty,
                fv = fold_v
            );
            let _ = writeln!(
                cg.out,
                "  %acch = select i1 %seen, {et} %mm_pick, {et} {fv}",
                et = elem_ty,
                fv = fold_v
            );
        }
        ReduceOp::Max => {
            let _ = writeln!(
                cg.out,
                "  %mm_ge = icmp sge {et} %acc, {fv}",
                et = elem_ty,
                fv = fold_v
            );
            let _ = writeln!(
                cg.out,
                "  %mm_pick = select i1 %mm_ge, {et} %acc, {et} {fv}",
                et = elem_ty,
                fv = fold_v
            );
            let _ = writeln!(
                cg.out,
                "  %acch = select i1 %seen, {et} %mm_pick, {et} {fv}",
                et = elem_ty,
                fv = fold_v
            );
        }
    }
}

struct IrCg<'a> {
    out: &'a mut String,
    tmp: u32,
    w: IntWidth,
}

impl<'a> IrCg<'a> {
    fn fresh(&mut self) -> String {
        let t = self.tmp;
        self.tmp += 1;
        format!("t{t}")
    }

    /// Lowers `ops` starting at basic block `first_label` (caller emits the branch into it).
    /// On filter failure, branches to `fail_label`. On success through all ops, branches to `succ_label`.
    /// Returns the SSA name of the final element value and basic block names that skip the fold
    /// (filter false edges) for phi construction at `%inc`.
    fn emit_pipeline_ops(
        &mut self,
        ops: &[PipelineOp<'_>],
        start_reg: &str,
        first_label: &str,
        fail_label: &str,
        succ_label: &str,
    ) -> (String, Vec<String>) {
        use std::fmt::Write;
        if ops.is_empty() {
            return (start_reg.to_string(), Vec::new());
        }
        let mut skip_blocks = Vec::new();
        let mut lbl = first_label.to_string();
        let mut cur = start_reg.to_string();
        for (i, op) in ops.iter().enumerate() {
            let _ = writeln!(self.out, "{}:", lbl);
            match op {
                PipelineOp::Filter(p) => {
                    skip_blocks.push(lbl.clone());
                    let ok_lbl = format!("pl_{}", self.fresh());
                    self.emit_predicate_branch(p, &cur, &ok_lbl, fail_label);
                    if i + 1 == ops.len() {
                        let _ = writeln!(self.out, "{}:", ok_lbl);
                        let _ = writeln!(self.out, "  br label %{succ}", succ = succ_label);
                    } else {
                        lbl = ok_lbl;
                    }
                }
                PipelineOp::Map(expr) => {
                    cur = self.emit_map_expr(expr, &cur);
                    if i + 1 == ops.len() {
                        let _ = writeln!(self.out, "  br label %{succ}", succ = succ_label);
                    } else {
                        let next = format!("pl_{}", self.fresh());
                        let _ = writeln!(self.out, "  br label %{n}", n = next);
                        lbl = next;
                    }
                }
            }
        }
        (cur, skip_blocks)
    }

    fn emit_scan_step(&mut self, op: ScanOp, acc: &str, y: &str) -> String {
        let intr = match (self.w, op) {
            (IntWidth::W8, ScanOp::Add) => "llvm.sadd.with.overflow.i8",
            (IntWidth::W8, ScanOp::Sub) => "llvm.ssub.with.overflow.i8",
            (IntWidth::W8, ScanOp::Mul) => "llvm.smul.with.overflow.i8",
            (IntWidth::W32, ScanOp::Add) => "llvm.sadd.with.overflow.i32",
            (IntWidth::W32, ScanOp::Sub) => "llvm.ssub.with.overflow.i32",
            (IntWidth::W32, ScanOp::Mul) => "llvm.smul.with.overflow.i32",
            (IntWidth::W64, ScanOp::Add) => "llvm.sadd.with.overflow.i64",
            (IntWidth::W64, ScanOp::Sub) => "llvm.ssub.with.overflow.i64",
            (IntWidth::W64, ScanOp::Mul) => "llvm.smul.with.overflow.i64",
        };
        self.checked_binop(intr, acc, y)
    }

    /// §8: `filter` lowers `&` and `or` with left-to-right short-circuit (LLVM control flow).
    fn emit_predicate_branch(&mut self, p: &Predicate, x: &str, ok_lbl: &str, fail_lbl: &str) {
        use std::fmt::Write;
        match p {
            Predicate::And { left, right, .. } => {
                let mid = format!("pl_{}", self.fresh());
                self.emit_predicate_branch(left, x, &mid, fail_lbl);
                let _ = writeln!(self.out, "{}:", mid);
                self.emit_predicate_branch(right, x, ok_lbl, fail_lbl);
            }
            Predicate::Or { left, right, .. } => {
                let mid = format!("pl_{}", self.fresh());
                self.emit_predicate_branch(left, x, ok_lbl, &mid);
                let _ = writeln!(self.out, "{}:", mid);
                self.emit_predicate_branch(right, x, ok_lbl, fail_lbl);
            }
            Predicate::Not { inner, .. } => {
                self.emit_predicate_branch(inner, x, fail_lbl, ok_lbl);
            }
            _ => {
                let reg = self.predicate_atom_i1(p, x);
                let _ = writeln!(
                    self.out,
                    "  br i1 {reg}, label %{ok}, label %{fail}",
                    reg = reg,
                    ok = ok_lbl,
                    fail = fail_lbl
                );
            }
        }
    }

    /// Leaf predicate as a single `i1` SSA value (no terminator).
    fn predicate_atom_i1(&mut self, p: &Predicate, x: &str) -> String {
        use std::fmt::Write;
        let et = self.w.llvm_ty();
        match p {
            Predicate::Even { .. } => {
                let a = self.fresh();
                let b = self.fresh();
                let _ = writeln!(self.out, "  %{a} = and {et} {x}, 1", et = et, x = x);
                let _ = writeln!(self.out, "  %{b} = icmp eq {et} %{a}, 0", et = et);
                format!("%{}", b)
            }
            Predicate::Odd { .. } => {
                let a = self.fresh();
                let b = self.fresh();
                let _ = writeln!(self.out, "  %{a} = and {et} {x}, 1", et = et, x = x);
                let _ = writeln!(self.out, "  %{b} = icmp ne {et} %{a}, 0", et = et);
                format!("%{}", b)
            }
            Predicate::Cmp { op, rhs, .. } => match (self.w, rhs) {
                (IntWidth::W8, CmpRhs::Bool(want)) => {
                    if !matches!(op, CmpOp::Eq) {
                        let t = self.fresh();
                        let _ = writeln!(self.out, "  %{t} = icmp eq i32 0, 1");
                        return format!("%{}", t);
                    }
                    let rhs_lit = if *want { 1 } else { 0 };
                    let t = self.fresh();
                    let _ = writeln!(
                        self.out,
                        "  %{t} = icmp eq {et} {x}, {rhs_lit}",
                        et = et,
                        x = x,
                        rhs_lit = rhs_lit
                    );
                    format!("%{}", t)
                }
                (IntWidth::W8, CmpRhs::Int(_)) => {
                    let t = self.fresh();
                    let _ = writeln!(self.out, "  %{t} = icmp eq i32 0, 1");
                    format!("%{}", t)
                }
                (_, CmpRhs::Bool(_)) => {
                    let t = self.fresh();
                    let _ = writeln!(self.out, "  %{t} = icmp eq i32 0, 1");
                    format!("%{}", t)
                }
                (_, CmpRhs::Int(v)) => {
                    let (ok, rhs_lit) = match self.w {
                        IntWidth::W8 => (false, String::new()),
                        IntWidth::W32 => {
                            if *v < i32::MIN as i64 || *v > i32::MAX as i64 {
                                (false, String::new())
                            } else {
                                (true, format!("{}", *v as i32))
                            }
                        }
                        IntWidth::W64 => (true, format!("{}", *v)),
                    };
                    if !ok {
                        let t = self.fresh();
                        let _ = writeln!(self.out, "  %{t} = icmp eq i32 0, 1");
                        return format!("%{}", t);
                    }
                    let icmp = match op {
                        CmpOp::Eq => "eq",
                        CmpOp::Lt => "slt",
                        CmpOp::Le => "sle",
                        CmpOp::Gt => "sgt",
                        CmpOp::Ge => "sge",
                    };
                    let t = self.fresh();
                    let _ = writeln!(
                        self.out,
                        "  %{t} = icmp {icmp} {et} {x}, {rhs}",
                        icmp = icmp,
                        et = et,
                        x = x,
                        rhs = rhs_lit
                    );
                    format!("%{}", t)
                }
            },
            Predicate::And { .. } | Predicate::Or { .. } | Predicate::Not { .. } => {
                unreachable!("emit_predicate_branch handles compound predicates")
            }
        }
    }

    /// Lower a `map` expression; `dot` is the SSA name (or bare name) bound to `.`.
    fn emit_map_expr(&mut self, e: &Expr, dot: &str) -> String {
        use std::fmt::Write;
        match e {
            Expr::Dot { .. } => {
                if dot.starts_with('%') {
                    dot.to_string()
                } else {
                    format!("%{}", dot)
                }
            }
            Expr::Lit { v, .. } => match self.w {
                IntWidth::W8 => {
                    if *v < i8::MIN as i64 || *v > i8::MAX as i64 {
                        "0".into()
                    } else {
                        format!("{}", *v as i8)
                    }
                }
                IntWidth::W32 => {
                    if *v < i32::MIN as i64 || *v > i32::MAX as i64 {
                        "0".into()
                    } else {
                        format!("{}", *v as i32)
                    }
                }
                IntWidth::W64 => format!("{}", *v),
            },
            Expr::Neg { inner, .. } => {
                let v = self.emit_map_expr(inner.as_ref(), dot);
                let intr = self.w.ssub_intrinsic();
                let t = self.fresh();
                let o = self.fresh();
                let lb = self.fresh();
                let s = self.fresh();
                let pair = self.w.pair_ty();
                let et = self.w.llvm_ty();
                let _ = writeln!(
                    self.out,
                    "  %{t} = call {pair} @{intr}({et} 0, {et} {v})",
                    pair = pair,
                    intr = intr,
                    et = et,
                    v = v
                );
                let _ = writeln!(
                    self.out,
                    "  %{o} = extractvalue {pair} %{t}, 1",
                    pair = pair,
                    t = t
                );
                let _ = writeln!(
                    self.out,
                    "  br i1 %{o}, label %trap_ov, label %neg_{lb}",
                    o = o,
                    lb = lb
                );
                let _ = writeln!(self.out, "neg_{lb}:", lb = lb);
                let _ = writeln!(
                    self.out,
                    "  %{s} = extractvalue {pair} %{t}, 0",
                    s = s,
                    pair = pair,
                    t = t
                );
                format!("%{}", s)
            }
            Expr::Add { left, right, .. } => {
                let a = self.emit_map_expr(left, dot);
                let b = self.emit_map_expr(right, dot);
                self.checked_binop(self.w.sadd_intrinsic(), &a, &b)
            }
            Expr::Sub { left, right, .. } => {
                let a = self.emit_map_expr(left, dot);
                let b = self.emit_map_expr(right, dot);
                self.checked_binop(self.w.ssub_intrinsic(), &a, &b)
            }
            Expr::Mul { left, right, .. } => {
                let a = self.emit_map_expr(left, dot);
                let b = self.emit_map_expr(right, dot);
                self.checked_binop(self.w.smul_intrinsic(), &a, &b)
            }
            Expr::Div { left, right, .. } => {
                let a = self.emit_map_expr(left, dot);
                let b = self.emit_map_expr(right, dot);
                self.emit_div(&a, &b)
            }
            Expr::Mod { left, right, .. } => {
                let a = self.emit_map_expr(left, dot);
                let b = self.emit_map_expr(right, dot);
                self.emit_mod(&a, &b)
            }
        }
    }

    fn checked_binop(&mut self, intr: &str, a: &str, b: &str) -> String {
        use std::fmt::Write;
        let pair = self.w.pair_ty();
        let et = self.w.llvm_ty();
        let t = self.fresh();
        let o = self.fresh();
        let s = self.fresh();
        let lbl = self.fresh();
        let _ = writeln!(
            self.out,
            "  %{t} = call {pair} @{intr}({et} {a}, {et} {b})",
            pair = pair,
            intr = intr,
            et = et,
            a = a,
            b = b
        );
        let _ = writeln!(
            self.out,
            "  %{o} = extractvalue {pair} %{t}, 1",
            pair = pair,
            t = t
        );
        let _ = writeln!(self.out, "  br i1 %{o}, label %trap_ov, label %cb_{lbl}", lbl = lbl);
        let _ = writeln!(self.out, "cb_{lbl}:", lbl = lbl);
        let _ = writeln!(
            self.out,
            "  %{s} = extractvalue {pair} %{t}, 0",
            s = s,
            pair = pair,
            t = t
        );
        format!("%{}", s)
    }

    fn emit_div(&mut self, a: &str, b: &str) -> String {
        use std::fmt::Write;
        let et = self.w.llvm_ty();
        let z = self.fresh();
        let ok = self.fresh();
        let _ = writeln!(self.out, "  %{z} = icmp eq {et} {b}, 0", et = et, b = b);
        let _ = writeln!(
            self.out,
            "  br i1 %{z}, label %trap_div, label %dv_{ok}",
            z = z,
            ok = ok
        );
        let _ = writeln!(self.out, "dv_{ok}:", ok = ok);
        let bad = self.fresh();
        let bad2 = self.fresh();
        let bad3 = self.fresh();
        let ok2 = self.fresh();
        let min = self.w.int_min_div();
        let _ = writeln!(
            self.out,
            "  %{bad} = icmp eq {et} {a}, {min}",
            et = et,
            a = a,
            min = min
        );
        let _ = writeln!(self.out, "  %{bad2} = icmp eq {et} {b}, -1", et = et, b = b);
        let _ = writeln!(self.out, "  %{bad3} = and i1 %{bad}, %{bad2}");
        let _ = writeln!(
            self.out,
            "  br i1 %{bad3}, label %trap_ov, label %dv2_{ok2}",
            bad3 = bad3,
            ok2 = ok2
        );
        let _ = writeln!(self.out, "dv2_{ok2}:", ok2 = ok2);
        let q = self.fresh();
        let _ = writeln!(
            self.out,
            "  %{q} = sdiv {et} {a}, {b}",
            q = q,
            et = et,
            a = a,
            b = b
        );
        format!("%{}", q)
    }

    fn emit_mod(&mut self, a: &str, b: &str) -> String {
        use std::fmt::Write;
        let et = self.w.llvm_ty();
        let z = self.fresh();
        let ok = self.fresh();
        let _ = writeln!(self.out, "  %{z} = icmp eq {et} {b}, 0", et = et, b = b);
        let _ = writeln!(
            self.out,
            "  br i1 %{z}, label %trap_div, label %md_{ok}",
            z = z,
            ok = ok
        );
        let _ = writeln!(self.out, "md_{ok}:", ok = ok);
        let bad = self.fresh();
        let bad2 = self.fresh();
        let bad3 = self.fresh();
        let ok2 = self.fresh();
        let min = self.w.int_min_div();
        let _ = writeln!(
            self.out,
            "  %{bad} = icmp eq {et} {a}, {min}",
            et = et,
            a = a,
            min = min
        );
        let _ = writeln!(self.out, "  %{bad2} = icmp eq {et} {b}, -1", et = et, b = b);
        let _ = writeln!(self.out, "  %{bad3} = and i1 %{bad}, %{bad2}");
        let _ = writeln!(
            self.out,
            "  br i1 %{bad3}, label %trap_ov, label %md2_{ok2}",
            bad3 = bad3,
            ok2 = ok2
        );
        let _ = writeln!(self.out, "md2_{ok2}:", ok2 = ok2);
        let r = self.fresh();
        let _ = writeln!(
            self.out,
            "  %{r} = srem {et} {a}, {b}",
            r = r,
            et = et,
            a = a,
            b = b
        );
        format!("%{}", r)
    }
}
