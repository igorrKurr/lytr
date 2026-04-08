//! Bracket-array parsing for `lir run --input`, typed by the program's `input:*` source.

use crate::ast::ElemTy;
use crate::interp::Val;

/// Parse `[...]` elements to [`Val`]s matching `ty` (`input:i32` / `input:i64` / `input:bool`).
pub fn parse_input_array(s: &str, ty: ElemTy) -> Result<Vec<Val>, String> {
    let s = s.trim();
    if !s.starts_with('[') {
        return Err("input must be a bracketed array, e.g. '[1, 2]'".into());
    }
    let inner = s.trim_start_matches('[').trim_end_matches(']');
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for part in inner.split(',') {
        let p = part.trim();
        match ty {
            ElemTy::Bool => {
                if p == "true" {
                    out.push(Val::Bool(true));
                } else if p == "false" {
                    out.push(Val::Bool(false));
                } else {
                    return Err(format!(
                        "input:bool expects true or false in the array, got `{p}`"
                    ));
                }
            }
            ElemTy::I64 => {
                if p == "true" || p == "false" {
                    return Err(format!(
                        "input:i64 expects numeric elements, got `{p}`"
                    ));
                }
                let n: i64 = p
                    .parse()
                    .map_err(|_| format!("invalid i64 array element `{p}`"))?;
                out.push(Val::I64(n));
            }
            ElemTy::I32 => {
                if p == "true" || p == "false" {
                    return Err(format!(
                        "input:i32 expects integer elements, got `{p}`"
                    ));
                }
                let n: i64 = p
                    .parse()
                    .map_err(|_| format!("invalid integer array element `{p}`"))?;
                if n < i32::MIN as i64 || n > i32::MAX as i64 {
                    return Err(format!(
                        "value `{p}` does not fit i32 (use input:i64 for larger values)"
                    ));
                }
                out.push(Val::I32(n as i32));
            }
        }
    }
    Ok(out)
}
