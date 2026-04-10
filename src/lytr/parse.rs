//! Parser: `lytr/0.1` + `fn main() -> i32 { return <expr>; }`

use crate::Span;

use super::ast::{BinOp, Block, Expr, FnItem, Program, Stmt};
use super::error::LytrError;
use super::lex::{tokenize, Token, TokenKind};

const EDITION: &str = "lytr/0.1";

/// Strip `lytr/0.1` and following newline; return body slice and byte offset of body start.
fn strip_edition(full_src: &str) -> Result<(&str, usize), LytrError> {
    let after_edition = full_src.strip_prefix(EDITION).ok_or_else(|| LytrError::Syntax {
        code: "E_LYTR_HEADER",
        span: Span::new(0, full_src.len().min(EDITION.len().saturating_add(4))),
        message: format!("expected `{EDITION}` at start of file"),
        fix_hint: "first line must be lytr/0.1".into(),
    })?;
    let rest = if let Some(r) = after_edition.strip_prefix("\r\n") {
        r
    } else if let Some(r) = after_edition.strip_prefix('\n') {
        r
    } else {
        return Err(LytrError::Syntax {
            code: "E_LYTR_HEADER",
            span: Span::new(0, EDITION.len()),
            message: "newline required after lytr/0.1".into(),
            fix_hint: "use a newline after the edition line".into(),
        });
    };
    let nl_len = after_edition.len() - rest.len();
    let body_start = EDITION.len() + nl_len;
    debug_assert_eq!(&full_src[body_start..], rest);
    Ok((rest, body_start))
}

pub fn parse_lytr_program(full_src: &str) -> Result<Program, LytrError> {
    let (body, body_start) = strip_edition(full_src)?;
    let tokens = tokenize(body, body_start)?;
    let mut p = Parser::new(&tokens);
    p.parse_program()
}

struct Parser<'a> {
    tokens: &'a [Token],
    i: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, i: 0 }
    }

    fn cur(&self) -> Token {
        self.tokens[self.i]
    }

    fn bump(&mut self) -> Token {
        let t = self.cur();
        if self.i + 1 < self.tokens.len() {
            self.i += 1;
        }
        t
    }

    fn expect(&mut self, want: TokenKind) -> Result<Token, LytrError> {
        let t = self.cur();
        if t.kind != want {
            return Err(LytrError::Syntax {
                code: "E_LYTR_PARSE",
                span: t.span,
                message: format!("unexpected token (expected {want:?}, got {:?})", t.kind),
                fix_hint: "check fn main() -> i32 { return … }".into(),
            });
        }
        Ok(self.bump())
    }

    fn parse_program(&mut self) -> Result<Program, LytrError> {
        let prog_start = self.cur().span.start;
        self.expect(TokenKind::Fn)?;
        let main_tok = self.expect(TokenKind::Main)?;
        self.expect(TokenKind::LParen)?;
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::Arrow)?;
        self.expect(TokenKind::I32)?;
        let body = self.parse_block()?;
        self.expect(TokenKind::Eof)?;
        let prog_end = body.span.end;
        Ok(Program {
            span: Span::new(prog_start, prog_end),
            main: FnItem {
                name_span: main_tok.span,
                body,
            },
        })
    }

    fn parse_block(&mut self) -> Result<Block, LytrError> {
        let lb = self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while self.cur().kind != TokenKind::RBrace {
            stmts.push(self.parse_stmt()?);
        }
        let rb = self.expect(TokenKind::RBrace)?;
        Ok(Block {
            span: Span::new(lb.span.start, rb.span.end),
            stmts,
        })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, LytrError> {
        let ret = self.expect(TokenKind::Return)?;
        let expr = self.parse_expr(0)?;
        let semi = self.expect(TokenKind::Semi)?;
        Ok(Stmt::Return {
            expr,
            span: Span::new(ret.span.start, semi.span.end),
        })
    }

    /// Pratt: `min_prec` 0 = `+` `-`, 1 = `*` `/` `%`.
    fn parse_expr(&mut self, min_prec: u8) -> Result<Expr, LytrError> {
        let mut lhs = self.parse_primary()?;
        loop {
            let op = match self.cur().kind {
                TokenKind::Plus if min_prec == 0 => Some(BinOp::Add),
                TokenKind::Minus if min_prec == 0 => Some(BinOp::Sub),
                TokenKind::Star if min_prec <= 1 => Some(BinOp::Mul),
                TokenKind::Slash if min_prec <= 1 => Some(BinOp::Div),
                TokenKind::Percent if min_prec <= 1 => Some(BinOp::Mod),
                _ => None,
            };
            let Some(op) = op else { break };
            let prec = match op {
                BinOp::Add | BinOp::Sub => 0u8,
                BinOp::Mul | BinOp::Div | BinOp::Mod => 1u8,
            };
            self.bump();
            let rhs = self.parse_expr(prec + 1)?;
            let span = Span::merge(lhs_span(&lhs), lhs_span(&rhs));
            lhs = Expr::Binary {
                op,
                left: Box::new(lhs),
                right: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_primary(&mut self) -> Result<Expr, LytrError> {
        let t = self.cur();
        match t.kind {
            TokenKind::Int(v) => {
                self.bump();
                Ok(Expr::Int {
                    value: v,
                    span: t.span,
                })
            }
            TokenKind::LParen => {
                self.bump();
                let e = self.parse_expr(0)?;
                self.expect(TokenKind::RParen)?;
                Ok(e)
            }
            _ => Err(LytrError::Syntax {
                code: "E_LYTR_PARSE",
                span: t.span,
                message: "expected integer or `(`".into(),
                fix_hint: "expression uses literals, + - * / %, and parentheses".into(),
            }),
        }
    }
}

fn lhs_span(e: &Expr) -> Span {
    match e {
        Expr::Int { span, .. } => *span,
        Expr::Binary { span, .. } => *span,
    }
}
