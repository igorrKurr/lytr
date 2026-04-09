use std::fmt;

use serde::{Deserialize, Serialize};

/// Byte offsets into the UTF-8 source (inclusive start, exclusive end).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn merge(a: Span, b: Span) -> Span {
        Span {
            start: a.start.min(b.start),
            end: a.end.max(b.end),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LirError {
    Syntax {
        code: &'static str,
        span: Span,
        message: String,
        fix_hint: String,
    },
    Type {
        code: &'static str,
        span: Span,
        message: String,
        fix_hint: String,
        stage_index: Option<usize>,
    },
    Runtime {
        code: &'static str,
        span: Span,
        message: String,
        fix_hint: String,
        stage_index: usize,
        element_index: Option<usize>,
    },
}

impl fmt::Display for LirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LirError::Syntax {
                code,
                span,
                message,
                fix_hint,
            } => write!(
                f,
                "[{code}] {} (bytes {}..{}) — {fix_hint}",
                message, span.start, span.end
            ),
            LirError::Type {
                code,
                span,
                message,
                fix_hint,
                stage_index,
            } => write!(
                f,
                "[{code}] {} (bytes {}..{}) stage={:?} — {fix_hint}",
                message, span.start, span.end, stage_index
            ),
            LirError::Runtime {
                code,
                span,
                message,
                fix_hint,
                stage_index,
                element_index,
            } => write!(
                f,
                "[{code}] {} (bytes {}..{}) stage={} elem={:?} — {fix_hint}",
                message, span.start, span.end, stage_index, element_index
            ),
        }
    }
}

/// JSON line for CLI / agent misuse (not a [`LirError`] from the language).
pub fn cli_json_line(code: &'static str, message: &str) -> String {
    format!(
        r#"{{"kind":"cli","code":"{code}","message":{}}}"#,
        serde_json_escape(message)
    )
}

impl LirError {
    pub fn to_json_line(&self) -> String {
        // Minimal JSON envelope for tooling / LLM loops.
        match self {
            LirError::Syntax {
                code,
                span,
                message,
                fix_hint,
            } => format!(
                r#"{{"kind":"syntax","code":"{code}","span":{{"start":{},"end":{}}},"message":{},"fix_hint":{}}}"#,
                span.start,
                span.end,
                serde_json_escape(message),
                serde_json_escape(fix_hint),
            ),
            LirError::Type {
                code,
                span,
                message,
                fix_hint,
                stage_index,
            } => format!(
                r#"{{"kind":"type","code":"{code}","span":{{"start":{},"end":{}}},"stage_index":{},"message":{},"fix_hint":{}}}"#,
                span.start,
                span.end,
                serde_json_stage(stage_index),
                serde_json_escape(message),
                serde_json_escape(fix_hint),
            ),
            LirError::Runtime {
                code,
                span,
                message,
                fix_hint,
                stage_index,
                element_index,
            } => format!(
                r#"{{"kind":"runtime","code":"{code}","span":{{"start":{},"end":{}}},"stage_index":{},"element_index":{},"message":{},"fix_hint":{}}}"#,
                span.start,
                span.end,
                stage_index,
                serde_json_elem(element_index),
                serde_json_escape(message),
                serde_json_escape(fix_hint),
            ),
        }
    }
}

fn serde_json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                use fmt::Write;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn serde_json_stage(s: &Option<usize>) -> String {
    match s {
        Some(i) => i.to_string(),
        None => "null".into(),
    }
}

fn serde_json_elem(s: &Option<usize>) -> String {
    match s {
        Some(i) => i.to_string(),
        None => "null".into(),
    }
}
