//! Parser: edition `lytr/0.1` + `fn main() -> i32` or `-> i64` `{ … }`.

use crate::Span;

use super::ast::{
    BinOp, Block, CmpOp, Expr, FnItem, MainRetTy, Program, Stmt, Ty,
};
use super::error::LytrError;
use super::lex::{tokenize, Token, TokenKind};

const EDITION: &str = "lytr/0.1";

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
    let mut p = Parser::new(&tokens, full_src);
    p.parse_program()
}

struct Parser<'a> {
    tokens: &'a [Token],
    i: usize,
    src: &'a str,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token], full_src: &'a str) -> Self {
        Self {
            tokens,
            i: 0,
            src: full_src,
        }
    }

    fn ident_text(&self, span: Span) -> String {
        self.src[span.start..span.end].to_string()
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
                fix_hint: "see docs/LYTR_CORE_CALCULUS_DRAFT.md".into(),
            });
        }
        Ok(self.bump())
    }

    fn expect_ident(&mut self) -> Result<(String, Span), LytrError> {
        let t = self.cur();
        if t.kind != TokenKind::Ident {
            return Err(LytrError::Syntax {
                code: "E_LYTR_PARSE",
                span: t.span,
                message: "expected identifier".into(),
                fix_hint: "use a name like `x` or `count`".into(),
            });
        }
        let span = t.span;
        let s = self.ident_text(span);
        self.bump();
        Ok((s, span))
    }

    fn parse_program(&mut self) -> Result<Program, LytrError> {
        let prog_start = self.cur().span.start;
        self.expect(TokenKind::Fn)?;
        let main_tok = self.expect(TokenKind::Main)?;
        self.expect(TokenKind::LParen)?;
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::Arrow)?;
        let ret = match self.cur().kind {
            TokenKind::I32 => {
                self.bump();
                MainRetTy::I32
            }
            TokenKind::I64 => {
                self.bump();
                MainRetTy::I64
            }
            _ => {
                return Err(LytrError::Syntax {
                    code: "E_LYTR_PARSE",
                    span: self.cur().span,
                    message: "expected `i32` or `i64` after `->`".into(),
                    fix_hint: "use `fn main() -> i32 { ... }` or `-> i64`".into(),
                });
            }
        };
        let body = self.parse_block()?;
        self.expect(TokenKind::Eof)?;
        let prog_end = body.span.end;
        Ok(Program {
            span: Span::new(prog_start, prog_end),
            main: FnItem {
                name_span: main_tok.span,
                ret,
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
        match self.cur().kind {
            TokenKind::Let => self.parse_let_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            _ => Err(LytrError::Syntax {
                code: "E_LYTR_PARSE",
                span: self.cur().span,
                message: "expected `let` or `return`".into(),
                fix_hint: "each statement is `let …;` or `return …;`".into(),
            }),
        }
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt, LytrError> {
        let let_tok = self.expect(TokenKind::Let)?;
        let (name, name_span) = self.expect_ident()?;
        let (ty, init) = if self.cur().kind == TokenKind::Colon {
            self.bump();
            let t = self.parse_ty()?;
            self.expect(TokenKind::Assign)?;
            let e = self.parse_expr()?;
            (Some(t), e)
        } else {
            self.expect(TokenKind::Assign)?;
            let e = self.parse_expr()?;
            (None, e)
        };
        let semi = self.expect(TokenKind::Semi)?;
        Ok(Stmt::Let {
            name,
            name_span,
            ty,
            init,
            span: Span::new(let_tok.span.start, semi.span.end),
        })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, LytrError> {
        let ret = self.expect(TokenKind::Return)?;
        let expr = self.parse_expr()?;
        let semi = self.expect(TokenKind::Semi)?;
        Ok(Stmt::Return {
            expr,
            span: Span::new(ret.span.start, semi.span.end),
        })
    }

    fn parse_ty(&mut self) -> Result<Ty, LytrError> {
        match self.cur().kind {
            TokenKind::I32 => {
                self.bump();
                Ok(Ty::I32)
            }
            TokenKind::I64 => {
                self.bump();
                Ok(Ty::I64)
            }
            TokenKind::Bool => {
                self.bump();
                Ok(Ty::Bool)
            }
            TokenKind::Result => {
                self.bump();
                self.expect(TokenKind::Lt)?;
                let t1 = match self.cur().kind {
                    TokenKind::I32 => {
                        self.bump();
                        Ty::I32
                    }
                    TokenKind::I64 => {
                        self.bump();
                        Ty::I64
                    }
                    _ => {
                        return Err(LytrError::Syntax {
                            code: "E_LYTR_PARSE",
                            span: self.cur().span,
                            message: "expected `i32` or `i64` in `Result<…>`".into(),
                            fix_hint: "use `Result<i32, i32>` or `Result<i64, i64>`".into(),
                        });
                    }
                };
                self.expect(TokenKind::Comma)?;
                let t2 = match self.cur().kind {
                    TokenKind::I32 => {
                        self.bump();
                        Ty::I32
                    }
                    TokenKind::I64 => {
                        self.bump();
                        Ty::I64
                    }
                    _ => {
                        return Err(LytrError::Syntax {
                            code: "E_LYTR_PARSE",
                            span: self.cur().span,
                            message: "expected `i32` or `i64` after `,` in `Result`".into(),
                            fix_hint: "both type arguments must match (bootstrap)".into(),
                        });
                    }
                };
                self.expect(TokenKind::Gt)?;
                if t1 != t2 {
                    return Err(LytrError::Syntax {
                        code: "E_LYTR_PARSE",
                        span: self.cur().span,
                        message: "`Result<T, U>` requires `T` and `U` equal in bootstrap".into(),
                        fix_hint: "use `Result<i32, i32>` or `Result<i64, i64>`".into(),
                    });
                }
                match t1 {
                    Ty::I32 => Ok(Ty::ResultI32),
                    Ty::I64 => Ok(Ty::ResultI64),
                    _ => unreachable!(),
                }
            }
            _ => Err(LytrError::Syntax {
                code: "E_LYTR_PARSE",
                span: self.cur().span,
                message: "expected `i32`, `i64`, `bool`, or `Result<…>`".into(),
                fix_hint: "bootstrap types only".into(),
            }),
        }
    }

    /// Comparisons (loosest), then `+` `-`, then `*` `/` `%`.
    fn parse_expr(&mut self) -> Result<Expr, LytrError> {
        self.parse_cmp()
    }

    fn parse_cmp(&mut self) -> Result<Expr, LytrError> {
        let mut lhs = self.parse_add()?;
        loop {
            let op = match self.cur().kind {
                TokenKind::EqEq => Some(CmpOp::Eq),
                TokenKind::Ne => Some(CmpOp::Ne),
                TokenKind::Lt => Some(CmpOp::Lt),
                TokenKind::Gt => Some(CmpOp::Gt),
                TokenKind::Le => Some(CmpOp::Le),
                TokenKind::Ge => Some(CmpOp::Ge),
                _ => None,
            };
            let Some(op) = op else { break };
            self.bump();
            let rhs = self.parse_add()?;
            let span = Span::merge(expr_span(&lhs), expr_span(&rhs));
            lhs = Expr::Cmp {
                op,
                left: Box::new(lhs),
                right: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<Expr, LytrError> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.cur().kind {
                TokenKind::Plus => Some(BinOp::Add),
                TokenKind::Minus => Some(BinOp::Sub),
                _ => None,
            };
            let Some(op) = op else { break };
            self.bump();
            let rhs = self.parse_mul()?;
            let span = Span::merge(expr_span(&lhs), expr_span(&rhs));
            lhs = Expr::Binary {
                op,
                left: Box::new(lhs),
                right: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Expr, LytrError> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.cur().kind {
                TokenKind::Star => Some(BinOp::Mul),
                TokenKind::Slash => Some(BinOp::Div),
                TokenKind::Percent => Some(BinOp::Mod),
                _ => None,
            };
            let Some(op) = op else { break };
            self.bump();
            let rhs = self.parse_unary()?;
            let span = Span::merge(expr_span(&lhs), expr_span(&rhs));
            lhs = Expr::Binary {
                op,
                left: Box::new(lhs),
                right: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, LytrError> {
        // unary `-` for negative literals: `-` INT
        if self.cur().kind == TokenKind::Minus {
            let m = self.bump().span;
            let t = self.cur();
            if let TokenKind::Int(v) = t.kind {
                self.bump();
                let span = Span::new(m.start, t.span.end);
                return Ok(Expr::Int {
                    value: v.checked_neg().ok_or_else(|| LytrError::Syntax {
                        code: "E_LYTR_INT",
                        span,
                        message: "integer overflow".into(),
                        fix_hint: "literal too large".into(),
                    })?,
                    span,
                });
            }
            return Err(LytrError::Syntax {
                code: "E_LYTR_PARSE",
                span: t.span,
                message: "expected integer after `-`".into(),
                fix_hint: "use `-42` as a single literal form".into(),
            });
        }
        self.parse_primary()
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
            TokenKind::True => {
                self.bump();
                Ok(Expr::BoolLit {
                    value: true,
                    span: t.span,
                })
            }
            TokenKind::False => {
                self.bump();
                Ok(Expr::BoolLit {
                    value: false,
                    span: t.span,
                })
            }
            TokenKind::Ident => {
                let span = t.span;
                let name = self.ident_text(span);
                self.bump();
                Ok(Expr::Var { name, span })
            }
            TokenKind::LParen => {
                self.bump();
                let e = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(e)
            }
            TokenKind::If => self.parse_if_expr(),
            TokenKind::Ok => self.parse_ok_expr(),
            TokenKind::Err => self.parse_err_expr(),
            TokenKind::Match => self.parse_match_expr(),
            _ => Err(LytrError::Syntax {
                code: "E_LYTR_PARSE",
                span: t.span,
                message: "expected expression".into(),
                fix_hint: "literal, variable, `(`, `if`, `Ok`, `Err`, or `match`".into(),
            }),
        }
    }

    fn parse_if_expr(&mut self) -> Result<Expr, LytrError> {
        let if_tok = self.expect(TokenKind::If)?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::LBrace)?;
        let then_b = self.parse_expr()?;
        self.expect(TokenKind::RBrace)?;
        self.expect(TokenKind::Else)?;
        self.expect(TokenKind::LBrace)?;
        let else_b = self.parse_expr()?;
        let rb = self.expect(TokenKind::RBrace)?;
        let span = Span::new(if_tok.span.start, rb.span.end);
        Ok(Expr::If {
            cond: Box::new(cond),
            then_b: Box::new(then_b),
            else_b: Box::new(else_b),
            span,
        })
    }

    fn parse_ok_expr(&mut self) -> Result<Expr, LytrError> {
        self.expect(TokenKind::Ok)?;
        self.expect(TokenKind::LParen)?;
        let inner = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        Ok(Expr::Ok(Box::new(inner)))
    }

    fn parse_err_expr(&mut self) -> Result<Expr, LytrError> {
        self.expect(TokenKind::Err)?;
        self.expect(TokenKind::LParen)?;
        let inner = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        Ok(Expr::Err(Box::new(inner)))
    }

    fn parse_match_expr(&mut self) -> Result<Expr, LytrError> {
        let match_tok = self.expect(TokenKind::Match)?;
        let scrutinee = self.parse_expr()?;
        self.expect(TokenKind::LBrace)?;
        self.expect(TokenKind::Ok)?;
        self.expect(TokenKind::LParen)?;
        let (ok_name, ok_name_span) = self.expect_ident()?;
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::FatArrow)?;
        let ok_arm = self.parse_expr()?;
        if self.cur().kind == TokenKind::Comma {
            self.bump();
        }
        self.expect(TokenKind::Err)?;
        self.expect(TokenKind::LParen)?;
        let (err_name, err_name_span) = self.expect_ident()?;
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::FatArrow)?;
        let err_arm = self.parse_expr()?;
        if self.cur().kind == TokenKind::Comma {
            self.bump();
        }
        let rb = self.expect(TokenKind::RBrace)?;
        let span = Span::new(match_tok.span.start, rb.span.end);
        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            ok_name,
            ok_name_span,
            ok_arm: Box::new(ok_arm),
            err_name,
            err_name_span,
            err_arm: Box::new(err_arm),
            span,
        })
    }
}

fn expr_span(e: &Expr) -> Span {
    match e {
        Expr::Int { span, .. }
        | Expr::BoolLit { span, .. }
        | Expr::Var { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Cmp { span, .. }
        | Expr::If { span, .. }
        | Expr::Match { span, .. } => *span,
        Expr::Ok(inner) | Expr::Err(inner) => expr_span(inner),
    }
}
