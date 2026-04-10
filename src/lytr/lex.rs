//! Lexer for LYTR 0.1 bootstrap subset.

use crate::Span;

use super::error::LytrError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Fn,
    Main,
    LParen,
    RParen,
    Arrow,
    I32,
    LBrace,
    RBrace,
    Return,
    Semi,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Int(i32),
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub fn tokenize(src: &str, base: usize) -> Result<Vec<Token>, LytrError> {
    let mut out = Vec::new();
    let mut i = 0usize;
    let bytes = src.as_bytes();
    let n = bytes.len();

    while i < n {
        let b = bytes[i];
        if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
            i += 1;
            continue;
        }
        if i + 1 < n && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            i += 2;
            while i < n && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        let start = base + i;

        if b == b'-' && i + 1 < n && bytes[i + 1] == b'>' {
            i += 2;
            let end = base + i;
            out.push(Token {
                kind: TokenKind::Arrow,
                span: Span::new(start, end),
            });
            continue;
        }

        if (b as char).is_ascii_alphabetic() || b == b'_' {
            let j = i;
            i += 1;
            while i < n {
                let c = bytes[i];
                if (c as char).is_ascii_alphanumeric() || c == b'_' {
                    i += 1;
                } else {
                    break;
                }
            }
            let word = &src[j..i];
            let end = base + i;
            let span = Span::new(start, end);
            let kind = match word {
                "fn" => TokenKind::Fn,
                "main" => TokenKind::Main,
                "return" => TokenKind::Return,
                "i32" => TokenKind::I32,
                _ => {
                    return Err(LytrError::Syntax {
                        code: "E_LYTR_LEX",
                        span,
                        message: format!("unexpected word `{word}`"),
                        fix_hint: "bootstrap only allows fn main() -> i32 { … }".into(),
                    });
                }
            };
            out.push(Token { kind, span });
            continue;
        }

        if (b as char).is_ascii_digit() {
            let j = i;
            i += 1;
            while i < n && (bytes[i] as char).is_ascii_digit() {
                i += 1;
            }
            let slice = &src[j..i];
            let end = base + i;
            let span = Span::new(start, end);
            let v: i32 = slice.parse().map_err(|_| LytrError::Syntax {
                code: "E_LYTR_INT",
                span,
                message: format!("invalid integer literal `{slice}`"),
                fix_hint: "use a decimal i32 literal".into(),
            })?;
            out.push(Token {
                kind: TokenKind::Int(v),
                span,
            });
            continue;
        }

        i += 1;
        let end = base + i;
        let span = Span::new(start, end);
        let kind = match b {
            b'(' => TokenKind::LParen,
            b')' => TokenKind::RParen,
            b'{' => TokenKind::LBrace,
            b'}' => TokenKind::RBrace,
            b';' => TokenKind::Semi,
            b'+' => TokenKind::Plus,
            b'-' => TokenKind::Minus,
            b'*' => TokenKind::Star,
            b'/' => TokenKind::Slash,
            b'%' => TokenKind::Percent,
            _ => {
                return Err(LytrError::Syntax {
                    code: "E_LYTR_LEX",
                    span,
                    message: format!("unexpected character `{}`", b as char),
                    fix_hint: "bootstrap syntax: fn main() -> i32 {{ return expr; }}".into(),
                });
            }
        };
        out.push(Token { kind, span });
    }

    out.push(Token {
        kind: TokenKind::Eof,
        span: Span::new(base + i, base + i),
    });
    Ok(out)
}
