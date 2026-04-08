//! Canonical program formatting (§11): lowercase, single spaces around `|`,
//! spaces after commas in lists, explicit `input:i32` / `input:i64` / `input:bool`.

use crate::ast::{
    CmpOp, CmpRhs, ElemTy, Expr, LitElem, Predicate, Program, ReduceOp, ScanOp, Source, Stage,
};

pub fn format_program(p: &Program) -> String {
    let mut out = String::from("lir/1\n");
    out.push_str(&format_source(&p.source));
    for st in &p.stages {
        out.push_str(" | ");
        out.push_str(&format_stage(st));
    }
    out.push('\n');
    out
}

/// True if `src` equals the canonical §11 formatting of `prog` (after normalizing CRLF → LF).
pub fn program_is_canonical_text(src: &str, prog: &Program) -> bool {
    let normalized = src.replace("\r\n", "\n");
    format_program(prog) == normalized
}

fn format_source(s: &Source) -> String {
    match s {
        Source::Input { ty, .. } => {
            let suf = match ty {
                ElemTy::I32 => "i32",
                ElemTy::I64 => "i64",
                ElemTy::Bool => "bool",
            };
            format!("input:{suf}")
        }
        Source::Range {
            start, stop, step, ..
        } => {
            let def = default_range_step(*start, *stop);
            if *step == def {
                format!("range ( {start} , {stop} )")
            } else {
                format!("range ( {start} , {stop} , {step} )")
            }
        }
        Source::Lit { elems, .. } => {
            if elems.is_empty() {
                "lit ()".into()
            } else {
                let parts: Vec<_> = elems.iter().map(format_lit_elem).collect();
                format!("lit ( {} )", parts.join(", "))
            }
        }
    }
}

fn default_range_step(start: i64, stop: i64) -> i64 {
    if start < stop {
        1
    } else if start > stop {
        -1
    } else {
        1
    }
}

fn format_lit_elem(e: &LitElem) -> String {
    match e {
        LitElem::I32(v) => v.to_string(),
        LitElem::I64(v) => v.to_string(),
        LitElem::Bool(b) => b.to_string(),
    }
}

fn format_stage(s: &Stage) -> String {
    match s {
        Stage::Filter { pred, .. } => format!("filter {}", format_predicate(pred)),
        Stage::Map { expr, .. } => format!("map {}", format_expr(expr)),
        Stage::Scan { init, op, .. } => format!(
            "scan {init}, {}",
            match op {
                ScanOp::Add => "add",
                ScanOp::Sub => "sub",
                ScanOp::Mul => "mul",
            }
        ),
        Stage::Reduce { op, .. } => format!(
            "reduce {}",
            match op {
                ReduceOp::Sum => "sum",
                ReduceOp::Prod => "prod",
                ReduceOp::Count => "count",
                ReduceOp::Min => "min",
                ReduceOp::Max => "max",
            }
        ),
        Stage::Take { n, .. } => format!("take {n}"),
        Stage::Drop { n, .. } => format!("drop {n}"),
        Stage::Id { .. } => "id".into(),
    }
}

fn flatten_or<'a>(p: &'a Predicate, out: &mut Vec<&'a Predicate>) {
    match p {
        Predicate::Or { left, right, .. } => {
            flatten_or(left, out);
            flatten_or(right, out);
        }
        _ => out.push(p),
    }
}

fn flatten_and<'a>(p: &'a Predicate, out: &mut Vec<&'a Predicate>) {
    match p {
        Predicate::And { left, right, .. } => {
            flatten_and(left, out);
            flatten_and(right, out);
        }
        _ => out.push(p),
    }
}

fn format_predicate(p: &Predicate) -> String {
    match p {
        Predicate::Or { .. } => {
            let mut parts = Vec::new();
            flatten_or(p, &mut parts);
            parts
                .iter()
                .map(|x| format_or_operand(x))
                .collect::<Vec<_>>()
                .join(" or ")
        }
        Predicate::And { .. } => {
            let mut parts = Vec::new();
            flatten_and(p, &mut parts);
            parts
                .iter()
                .map(|x| format_and_operand(x))
                .collect::<Vec<_>>()
                .join(" & ")
        }
        Predicate::Not { inner, .. } => format!("not {}", format_not_inner(inner.as_ref())),
        Predicate::Even { .. } => "even".into(),
        Predicate::Odd { .. } => "odd".into(),
        Predicate::Cmp { op, rhs, .. } => {
            let op_s = match op {
                CmpOp::Eq => "eq",
                CmpOp::Lt => "lt",
                CmpOp::Le => "le",
                CmpOp::Gt => "gt",
                CmpOp::Ge => "ge",
            };
            match rhs {
                CmpRhs::Int(v) => format!("{op_s} {v}"),
                CmpRhs::Bool(b) => format!("{op_s} {b}"),
            }
        }
    }
}

fn format_or_operand(p: &Predicate) -> String {
    match p {
        Predicate::And { .. } => format!("({})", format_predicate(p)),
        _ => format_predicate_non_or(p),
    }
}

fn format_and_operand(p: &Predicate) -> String {
    match p {
        Predicate::Or { .. } => format!("({})", format_predicate(p)),
        _ => format_predicate_non_or(p),
    }
}

fn format_predicate_non_or(p: &Predicate) -> String {
    match p {
        Predicate::And { .. } => format_predicate(p),
        Predicate::Not { .. } => format_predicate(p),
        Predicate::Even { .. } | Predicate::Odd { .. } | Predicate::Cmp { .. } => {
            format_predicate(p)
        }
        Predicate::Or { .. } => format_predicate(p),
    }
}

fn format_not_inner(p: &Predicate) -> String {
    match p {
        Predicate::Or { .. } | Predicate::And { .. } => format!("({})", format_predicate(p)),
        _ => format_predicate(p),
    }
}

fn expr_prec(e: &Expr) -> i32 {
    match e {
        Expr::Add { .. } | Expr::Sub { .. } => 1,
        Expr::Mul { .. } | Expr::Div { .. } | Expr::Mod { .. } => 2,
        Expr::Neg { .. } => 3,
        Expr::Dot { .. } | Expr::Lit { .. } => 4,
    }
}

fn format_expr(e: &Expr) -> String {
    format_expr_inner(e, 0)
}

fn format_expr_inner(e: &Expr, parent_prec: i32) -> String {
    let my = expr_prec(e);
    let s = match e {
        Expr::Add { left, right, .. } => format!(
            "{} add {}",
            format_expr_inner(left, my),
            format_expr_inner(right, my)
        ),
        Expr::Sub { left, right, .. } => format!(
            "{} sub {}",
            format_expr_inner(left, my),
            format_expr_inner(right, my)
        ),
        Expr::Mul { left, right, .. } => format!(
            "{} mul {}",
            format_expr_inner(left, my),
            format_expr_inner(right, my)
        ),
        Expr::Div { left, right, .. } => format!(
            "{} div {}",
            format_expr_inner(left, my),
            format_expr_inner(right, my)
        ),
        Expr::Mod { left, right, .. } => format!(
            "{} mod {}",
            format_expr_inner(left, my),
            format_expr_inner(right, my)
        ),
        Expr::Neg { inner, .. } => format!("neg {}", format_expr_inner(inner.as_ref(), my)),
        Expr::Dot { .. } => ".".into(),
        Expr::Lit { v, .. } => v.to_string(),
    };
    if my < parent_prec {
        format!("({s})")
    } else {
        s
    }
}
