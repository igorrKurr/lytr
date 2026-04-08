use crate::error::{LirError, Span};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Ident(String),
    Int(i64),
    Pipe,
    LParen,
    RParen,
    Comma,
    Dot,
    Amp,
    Colon,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Lexeme {
    pub tok: Token,
    pub span: Span,
}

pub fn lex(body: &str) -> Result<Vec<Lexeme>, LirError> {
    let mut out = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b' ' || b == b'\t' || b == b'\r' {
            i += 1;
            continue;
        }
        if b == b'\n' {
            i += 1;
            continue;
        }
        let start = i;
        match b {
            b'|' => {
                i += 1;
                out.push(Lexeme {
                    tok: Token::Pipe,
                    span: Span::new(start, i),
                });
            }
            b'(' => {
                i += 1;
                out.push(Lexeme {
                    tok: Token::LParen,
                    span: Span::new(start, i),
                });
            }
            b')' => {
                i += 1;
                out.push(Lexeme {
                    tok: Token::RParen,
                    span: Span::new(start, i),
                });
            }
            b',' => {
                i += 1;
                out.push(Lexeme {
                    tok: Token::Comma,
                    span: Span::new(start, i),
                });
            }
            b'.' => {
                i += 1;
                out.push(Lexeme {
                    tok: Token::Dot,
                    span: Span::new(start, i),
                });
            }
            b'&' => {
                i += 1;
                out.push(Lexeme {
                    tok: Token::Amp,
                    span: Span::new(start, i),
                });
            }
            b':' => {
                i += 1;
                out.push(Lexeme {
                    tok: Token::Colon,
                    span: Span::new(start, i),
                });
            }
            b'0'..=b'9' | b'-' => {
                let neg = if b == b'-' {
                    i += 1;
                    if i >= bytes.len() || !matches!(bytes[i], b'0'..=b'9') {
                        return Err(LirError::Syntax {
                            code: "E_BAD_MINUS",
                            span: Span::new(start, start + 1),
                            message: "`-` must be followed by digits here".into(),
                            fix_hint: "Use a unary `neg` inside map expressions; in range/lit use `-123`.".into(),
                        });
                    }
                    true
                } else {
                    false
                };
                let num_start = i;
                i += 1;
                while i < bytes.len() && matches!(bytes[i], b'0'..=b'9') {
                    i += 1;
                }
                let s = std::str::from_utf8(&bytes[num_start..i]).unwrap();
                let v128: i128 = s.parse().map_err(|_| LirError::Syntax {
                    code: "E_INT_PARSE",
                    span: Span::new(start, i),
                    message: format!("invalid integer literal `{s}`"),
                    fix_hint: "Use a decimal integer literal.".into(),
                })?;
                let v128 = if neg { -v128 } else { v128 };
                if v128 < i64::MIN as i128 || v128 > i64::MAX as i128 {
                    return Err(LirError::Syntax {
                        code: "E_INT_RANGE",
                        span: Span::new(start, i),
                        message: format!("integer literal `{}{s}` is out of i64 range", if neg { "-" } else { "" }),
                        fix_hint: "Use a value that fits in signed 64 bits.".into(),
                    });
                }
                let v = v128 as i64;
                out.push(Lexeme {
                    tok: Token::Int(v),
                    span: Span::new(start, i),
                });
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                i += 1;
                while i < bytes.len()
                    && (matches!(bytes[i], b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_'))
                {
                    i += 1;
                }
                let s = std::str::from_utf8(&bytes[start..i]).unwrap().to_string();
                out.push(Lexeme {
                    tok: Token::Ident(s),
                    span: Span::new(start, i),
                });
            }
            _ => {
                return Err(LirError::Syntax {
                    code: "E_UNEXPECTED_CHAR",
                    span: Span::new(start, start + 1),
                    message: format!("unexpected character `{}`", body[start..].chars().next().unwrap()),
                    fix_hint: "LIR v1 allows only letters, digits, _, and | ( ) , . & :.".into(),
                });
            }
        }
    }
    out.push(Lexeme {
        tok: Token::Eof,
        span: Span::new(bytes.len(), bytes.len()),
    });
    Ok(out)
}
