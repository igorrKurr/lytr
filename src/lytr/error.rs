//! Errors for the LYTR 0.1 bootstrap parser / interpreter.

use std::fmt;

use crate::error::serde_json_escape;
use crate::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LytrError {
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
    },
    Runtime {
        code: &'static str,
        span: Span,
        message: String,
        fix_hint: String,
    },
}

impl fmt::Display for LytrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LytrError::Syntax {
                code,
                span,
                message,
                fix_hint,
            } => write!(
                f,
                "[{code}] {} (bytes {}..{}) — {fix_hint}",
                message, span.start, span.end
            ),
            LytrError::Type {
                code,
                span,
                message,
                fix_hint,
            } => write!(
                f,
                "[{code}] {} (bytes {}..{}) — {fix_hint}",
                message, span.start, span.end
            ),
            LytrError::Runtime {
                code,
                span,
                message,
                fix_hint,
            } => write!(
                f,
                "[{code}] {} (bytes {}..{}) — {fix_hint}",
                message, span.start, span.end
            ),
        }
    }
}

impl LytrError {
    pub fn to_json_line(&self) -> String {
        match self {
            LytrError::Syntax {
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
            LytrError::Type {
                code,
                span,
                message,
                fix_hint,
            } => format!(
                r#"{{"kind":"type","code":"{code}","span":{{"start":{},"end":{}}},"message":{},"fix_hint":{}}}"#,
                span.start,
                span.end,
                serde_json_escape(message),
                serde_json_escape(fix_hint),
            ),
            LytrError::Runtime {
                code,
                span,
                message,
                fix_hint,
            } => format!(
                r#"{{"kind":"runtime","code":"{code}","span":{{"start":{},"end":{}}},"message":{},"fix_hint":{}}}"#,
                span.start,
                span.end,
                serde_json_escape(message),
                serde_json_escape(fix_hint),
            ),
        }
    }
}
