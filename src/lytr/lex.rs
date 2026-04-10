//! Lexer for LYTR 0.1 bootstrap.

use crate::Span;

use super::error::LytrError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Fn,
    Main,
    Let,
    If,
    Else,
    Return,
    True,
    False,
    Bool,
    I32,
    Result,
    Ok,
    Err,
    Match,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Arrow,
    Semi,
    Comma,
    Colon,
    FatArrow,
    Lt,
    Gt,
    Assign,
    EqEq,
    Ne,
    Le,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Ident, // name = src[span]
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

        // Two-char operators (before single-char)
        if i + 1 < n {
            let two = [bytes[i], bytes[i + 1]];
            let kind = match two {
                [b'-', b'>'] => Some((TokenKind::Arrow, 2)),
                [b'=', b'='] => Some((TokenKind::EqEq, 2)),
                [b'!', b'='] => Some((TokenKind::Ne, 2)),
                [b'<', b'='] => Some((TokenKind::Le, 2)),
                [b'>', b'='] => Some((TokenKind::Ge, 2)),
                [b'=', b'>'] => Some((TokenKind::FatArrow, 2)),
                _ => None,
            };
            if let Some((kind, len)) = kind {
                i += len;
                out.push(Token {
                    kind,
                    span: Span::new(start, base + i),
                });
                continue;
            }
        }

        if b == b'=' {
            i += 1;
            out.push(Token {
                kind: TokenKind::Assign,
                span: Span::new(start, base + i),
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
                "let" => TokenKind::Let,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "return" => TokenKind::Return,
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                "bool" => TokenKind::Bool,
                "i32" => TokenKind::I32,
                "Result" => TokenKind::Result,
                "Ok" => TokenKind::Ok,
                "Err" => TokenKind::Err,
                "match" => TokenKind::Match,
                _ => TokenKind::Ident,
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
            b',' => TokenKind::Comma,
            b':' => TokenKind::Colon,
            b'<' => TokenKind::Lt,
            b'>' => TokenKind::Gt,
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
                    fix_hint: "see docs/LYTR_CORE_CALCULUS_DRAFT.md".into(),
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
