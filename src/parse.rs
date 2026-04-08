use crate::ast::{
    CmpOp, CmpRhs, ElemTy, Expr, LitElem, Predicate, Program, ReduceOp, ScanOp, Source, Stage,
};
use crate::error::{LirError, Span};
use crate::lex::{Lexeme, Token};

pub fn parse_program(src: &str) -> Result<Program, LirError> {
    let trimmed = src.strip_prefix('\u{feff}').unwrap_or(src);
    let mut lines = trimmed.splitn(2, '\n');
    let first = lines.next().unwrap_or("").trim_end_matches('\r');
    if first != "lir/1" {
        return Err(LirError::Syntax {
            code: "E_HEADER",
            span: Span::new(0, first.len().min(trimmed.len()).max(1)),
            message: "first line must be exactly `lir/1`".into(),
            fix_hint: "Start the program with a line containing only lir/1".into(),
        });
    }
    let body = lines.next().unwrap_or("").trim_start_matches('\r');
    let offset = trimmed
        .find('\n')
        .map(|i| i + 1)
        .unwrap_or(trimmed.len());
    let toks = crate::lex::lex(body)?;
    let mut p = Parser {
        src: body,
        global_off: offset,
        toks,
        i: 0,
    };
    let (source, _sspan) = p.parse_source()?;
    let mut stages = Vec::new();
    if !p.match_pipe() {
        let lx = p.cur();
        return Err(LirError::Syntax {
            code: "E_EXPECTED_STAGE",
            span: p.gspan(lx.span),
            message: "expected `|` followed by a stage after the source".into(),
            fix_hint: "Write e.g. `input:i32 | reduce count`.".into(),
        });
    }
    loop {
        let stage = p.parse_stage()?;
        stages.push(stage);
        if !p.match_pipe() {
            break;
        }
    }
    if !p.is_eof() {
        let lx = p.cur();
        return Err(LirError::Syntax {
            code: "E_EXPECTED_EOF",
            span: p.gspan(lx.span),
            message: "unexpected token after pipeline".into(),
            fix_hint: "Remove trailing tokens or insert | before the next stage.".into(),
        });
    }
    let end = offset + body.len();
    Ok(Program {
        span: Span::new(0, end),
        source,
        stages,
    })
}

struct Parser<'a> {
    src: &'a str,
    global_off: usize,
    toks: Vec<Lexeme>,
    i: usize,
}

impl<'a> Parser<'a> {
    fn gspan(&self, s: Span) -> Span {
        Span::new(s.start + self.global_off, s.end + self.global_off)
    }

    fn cur(&self) -> &Lexeme {
        self.toks.get(self.i).unwrap()
    }

    fn bump(&mut self) -> Lexeme {
        let lx = self.cur().clone();
        self.i += 1;
        lx
    }

    fn is_eof(&self) -> bool {
        matches!(self.cur().tok, Token::Eof)
    }

    fn match_pipe(&mut self) -> bool {
        if matches!(self.cur().tok, Token::Pipe) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn expect_ident(&mut self) -> Result<(String, Span), LirError> {
        let lx = self.bump();
        match &lx.tok {
            Token::Ident(s) => Ok((s.clone(), self.gspan(lx.span))),
            _ => Err(LirError::Syntax {
                code: "E_EXPECTED_IDENT",
                span: self.gspan(lx.span),
                message: "expected identifier".into(),
                fix_hint: "Use a keyword or name allowed by the LIR v1 grammar.".into(),
            }),
        }
    }

    fn expect_int(&mut self) -> Result<(i64, Span), LirError> {
        let lx = self.bump();
        match lx.tok {
            Token::Int(v) => Ok((v, self.gspan(lx.span))),
            _ => Err(LirError::Syntax {
                code: "E_EXPECTED_INT",
                span: self.gspan(lx.span),
                message: "expected integer literal".into(),
                fix_hint: "Provide a decimal integer.".into(),
            }),
        }
    }

    fn parse_source(&mut self) -> Result<(Source, Span), LirError> {
        let (name, span) = self.expect_ident()?;
        match name.as_str() {
            "input" => {
                let ty = if matches!(self.cur().tok, Token::Colon) {
                    self.bump();
                    let (tname, _) = self.expect_ident()?;
                    let ty = match tname.as_str() {
                        "i32" => ElemTy::I32,
                        "i64" => ElemTy::I64,
                        "bool" => ElemTy::Bool,
                        _ => {
                            return Err(LirError::Syntax {
                                code: "E_BAD_INPUT_TY",
                                span,
                                message: format!("unknown input type `{tname}`"),
                                fix_hint: "Use i32, i64, or bool.".into(),
                            });
                        }
                    };
                    (ty, true)
                } else {
                    (ElemTy::I32, false)
                };
                Ok((
                    Source::Input {
                        ty: ty.0,
                        explicit: ty.1,
                        span,
                    },
                    span,
                ))
            }
            "range" => self.parse_range(span),
            "lit" => self.parse_lit(span),
            _ => Err(LirError::Syntax {
                code: "E_BAD_SOURCE",
                span,
                message: format!("unknown source `{name}`"),
                fix_hint: "Start with input, range, or lit.".into(),
            }),
        }
    }

    fn parse_range(&mut self, span_start: Span) -> Result<(Source, Span), LirError> {
        self.expect_tok(Token::LParen, "(")?;
        let (a, _) = self.expect_int()?;
        self.expect_tok(Token::Comma, ",")?;
        let (b, _) = self.expect_int()?;
        let (start, stop, step, end_span) = if matches!(self.cur().tok, Token::Comma) {
            self.bump();
            let (s, _) = self.expect_int()?;
            let rparen = self.cur().span;
            self.expect_tok(Token::RParen, ")")?;
            if s == 0 {
                return Err(LirError::Syntax {
                    code: "E_RANGE_STEP_ZERO",
                    span: span_start,
                    message: "range step must not be zero".into(),
                    fix_hint: "Use a non-zero step or omit the third argument.".into(),
                });
            }
            if provably_infinite(a, b, s) {
                return Err(LirError::Syntax {
                    code: "E_RANGE_INFINITE",
                    span: span_start,
                    message: "range never reaches stop (infinite)".into(),
                    fix_hint: "Adjust start/stop/step so the sequence terminates at stop.".into(),
                });
            }
            (a, b, s, rparen)
        } else {
            let rparen = self.cur().span;
            self.expect_tok(Token::RParen, ")")?;
            let step = if a < b {
                1
            } else if a > b {
                -1
            } else {
                1
            };
            (a, b, step, rparen)
        };
        let span = Span::merge(span_start, self.gspan(end_span));
        Ok((
            Source::Range {
                start,
                stop,
                step,
                span,
            },
            span,
        ))
    }

    fn parse_lit(&mut self, span_start: Span) -> Result<(Source, Span), LirError> {
        self.expect_tok(Token::LParen, "(")?;
        let mut elems = Vec::new();
        if matches!(self.cur().tok, Token::RParen) {
            let rparen = self.cur().span;
            self.bump();
            let span = Span::merge(span_start, self.gspan(rparen));
            return Ok((Source::Lit { elems, span }, span));
        }
        loop {
            let el = match &self.cur().tok {
                Token::Int(v) => {
                    let v = *v;
                    let _sp = self.bump().span;
                    if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                        LitElem::I32(v as i32)
                    } else {
                        LitElem::I64(v)
                    }
                }
                Token::Ident(s) => match s.as_str() {
                    "true" => {
                        self.bump();
                        LitElem::Bool(true)
                    }
                    "false" => {
                        self.bump();
                        LitElem::Bool(false)
                    }
                    _ => {
                        return Err(LirError::Syntax {
                            code: "E_LIT_ATOM",
                            span: self.gspan(self.cur().span),
                            message: format!("expected integer or bool in lit(), got `{s}`"),
                            fix_hint: "Use integer literals or true/false.".into(),
                        });
                    }
                },
                _ => {
                    return Err(LirError::Syntax {
                        code: "E_LIT_ATOM",
                        span: self.gspan(self.cur().span),
                        message: "expected integer or bool in lit()".into(),
                        fix_hint: "Use integer literals or true/false.".into(),
                    });
                }
            };
            elems.push(el);
            if matches!(self.cur().tok, Token::Comma) {
                self.bump();
                if matches!(self.cur().tok, Token::RParen) {
                    return Err(LirError::Syntax {
                        code: "E_TRAILING_COMMA",
                        span: self.gspan(self.cur().span),
                        message: "trailing comma before `)`".into(),
                        fix_hint: "Remove the trailing comma.".into(),
                    });
                }
                continue;
            }
            break;
        }
        let rparen = self.cur().span;
        self.expect_tok(Token::RParen, ")")?;
        let span = Span::merge(span_start, self.gspan(rparen));
        Ok((Source::Lit { elems, span }, span))
    }

    fn expect_tok(&mut self, want: Token, human: &str) -> Result<(), LirError> {
        let lx = self.cur();
        let ok = match (&lx.tok, &want) {
            (Token::LParen, Token::LParen) => true,
            (Token::RParen, Token::RParen) => true,
            (Token::Comma, Token::Comma) => true,
            _ => false,
        };
        if ok {
            self.bump();
            Ok(())
        } else {
            Err(LirError::Syntax {
                code: "E_EXPECTED_PUNCT",
                span: self.gspan(lx.span),
                message: format!("expected `{human}`"),
                fix_hint: "Fix punctuation to match the grammar.".into(),
            })
        }
    }

    fn parse_stage(&mut self) -> Result<Stage, LirError> {
        let (name, span) = self.expect_ident()?;
        match name.as_str() {
            "filter" => {
                let pred = self.parse_predicate()?;
                Ok(Stage::Filter { pred, span })
            }
            "map" => {
                let expr = self.parse_expr()?;
                let expr = desugar_map_expr(expr);
                Ok(Stage::Map { expr, span })
            }
            "scan" => {
                let (init, _) = self.expect_int()?;
                self.expect_tok(Token::Comma, ",")?;
                let (opname, _) = self.expect_ident()?;
                let op = match opname.as_str() {
                    "add" => ScanOp::Add,
                    "sub" => ScanOp::Sub,
                    "mul" => ScanOp::Mul,
                    _ => {
                        return Err(LirError::Syntax {
                            code: "E_SCAN_OP",
                            span,
                            message: format!("unknown scan op `{opname}`"),
                            fix_hint: "Use add, sub, or mul.".into(),
                        });
                    }
                };
                Ok(Stage::Scan {
                    init,
                    op,
                    span,
                })
            }
            "reduce" => {
                let (rname, _) = self.expect_ident()?;
                let op = match rname.as_str() {
                    "sum" => ReduceOp::Sum,
                    "prod" => ReduceOp::Prod,
                    "count" => ReduceOp::Count,
                    "min" => ReduceOp::Min,
                    "max" => ReduceOp::Max,
                    _ => {
                        return Err(LirError::Syntax {
                            code: "E_REDUCE_OP",
                            span,
                            message: format!("unknown reducer `{rname}`"),
                            fix_hint: "Use sum, prod, count, min, or max.".into(),
                        });
                    }
                };
                Ok(Stage::Reduce { op, span })
            }
            "take" => {
                let (n, nsp) = self.expect_int()?;
                if n < 0 || n > u32::MAX as i64 {
                    return Err(LirError::Syntax {
                        code: "E_TAKE_RANGE",
                        span: nsp,
                        message: "take requires non-negative i32-sized literal".into(),
                        fix_hint: "Use an integer from 0 to 2^32-1.".into(),
                    });
                }
                Ok(Stage::Take {
                    n: n as u32,
                    span,
                })
            }
            "drop" => {
                let (n, nsp) = self.expect_int()?;
                if n < 0 || n > u32::MAX as i64 {
                    return Err(LirError::Syntax {
                        code: "E_DROP_RANGE",
                        span: nsp,
                        message: "drop requires non-negative i32-sized literal".into(),
                        fix_hint: "Use an integer from 0 to 2^32-1.".into(),
                    });
                }
                Ok(Stage::Drop {
                    n: n as u32,
                    span,
                })
            }
            "id" => Ok(Stage::Id { span }),
            _ => Err(LirError::Syntax {
                code: "E_BAD_STAGE",
                span,
                message: format!("unknown stage `{name}`"),
                fix_hint: "Use filter, map, scan, reduce, take, drop, or id.".into(),
            }),
        }
    }

    fn parse_predicate(&mut self) -> Result<Predicate, LirError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Predicate, LirError> {
        let mut left = self.parse_and()?;
        while self.cur_ident_is("or") {
            let span_left = pred_span(&left);
            self.bump();
            let right = self.parse_and()?;
            let span = Span::merge(span_left, pred_span(&right));
            left = Predicate::Or {
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Predicate, LirError> {
        let mut left = self.parse_not()?;
        while matches!(self.cur().tok, Token::Amp) {
            let span_left = pred_span(&left);
            self.bump();
            let right = self.parse_not()?;
            let span = Span::merge(span_left, pred_span(&right));
            left = Predicate::And {
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Predicate, LirError> {
        if self.cur_ident_is("not") {
            let sp = self.gspan(self.cur().span);
            self.bump();
            let inner = self.parse_not()?;
            let span = Span::merge(sp, pred_span(&inner));
            return Ok(Predicate::Not {
                inner: Box::new(inner),
                span,
            });
        }
        self.parse_primary_pred()
    }

    fn parse_primary_pred(&mut self) -> Result<Predicate, LirError> {
        if matches!(self.cur().tok, Token::LParen) {
            self.bump();
            let p = self.parse_predicate()?;
            self.expect_tok(Token::RParen, ")")?;
            return Ok(p);
        }
        let (name, span) = self.expect_ident()?;
        match name.as_str() {
            "even" => Ok(Predicate::Even { span }),
            "odd" => Ok(Predicate::Odd { span }),
            "eq" | "lt" | "le" | "gt" | "ge" => {
                let op = match name.as_str() {
                    "eq" => CmpOp::Eq,
                    "lt" => CmpOp::Lt,
                    "le" => CmpOp::Le,
                    "gt" => CmpOp::Gt,
                    "ge" => CmpOp::Ge,
                    _ => unreachable!(),
                };
                let rhs = self.parse_cmp_rhs(op, span)?;
                Ok(Predicate::Cmp { op, rhs, span })
            }
            _ => Err(LirError::Syntax {
                code: "E_PRED_ATOM",
                span,
                message: format!("unknown predicate atom `{name}`"),
                fix_hint: "Use even, odd, eq/lt/le/gt/ge <int>, or parentheses.".into(),
            }),
        }
    }

    fn cur_ident_is(&self, s: &str) -> bool {
        matches!(&self.cur().tok, Token::Ident(x) if x == s)
    }

    fn parse_cmp_rhs(&mut self, op: CmpOp, op_span: Span) -> Result<CmpRhs, LirError> {
        match &self.cur().tok {
            Token::Int(v) => {
                let v = *v;
                self.bump();
                Ok(CmpRhs::Int(v))
            }
            Token::Ident(s) => {
                let b = match s.as_str() {
                    "true" => {
                        self.bump();
                        true
                    }
                    "false" => {
                        self.bump();
                        false
                    }
                    _ => {
                        return Err(LirError::Syntax {
                            code: "E_CMP_RHS",
                            span: op_span,
                            message: "expected integer or true/false after comparison op".into(),
                            fix_hint: "Use e.g. `gt 10` or `eq true`.".into(),
                        });
                    }
                };
                if op != CmpOp::Eq {
                    return Err(LirError::Syntax {
                        code: "E_BOOL_CMP",
                        span: op_span,
                        message: "bool literal comparisons require `eq`".into(),
                        fix_hint: "Use `eq true` or `eq false`, or compare integers with lt/le/gt/ge.".into(),
                    });
                }
                Ok(CmpRhs::Bool(b))
            }
            _ => Err(LirError::Syntax {
                code: "E_CMP_RHS",
                span: self.gspan(self.cur().span),
                message: "expected integer or true/false after comparison op".into(),
                fix_hint: "Use e.g. `gt 10` or `eq true`.".into(),
            }),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, LirError> {
        self.parse_add()
    }

    fn parse_add(&mut self) -> Result<Expr, LirError> {
        let mut left = self.parse_mul()?;
        loop {
            if self.cur_ident_is("add") {
                self.bump();
                let right = self.parse_mul()?;
                let span = Span::merge(expr_span(&left), expr_span(&right));
                left = Expr::Add {
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                };
            } else if self.cur_ident_is("sub") {
                self.bump();
                let right = self.parse_mul()?;
                let span = Span::merge(expr_span(&left), expr_span(&right));
                left = Expr::Sub {
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, LirError> {
        let mut left = self.parse_unary()?;
        loop {
            if self.cur_ident_is("mul") {
                self.bump();
                let right = self.parse_unary()?;
                let span = Span::merge(expr_span(&left), expr_span(&right));
                left = Expr::Mul {
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                };
            } else if self.cur_ident_is("div") {
                self.bump();
                let right = self.parse_unary()?;
                let span = Span::merge(expr_span(&left), expr_span(&right));
                left = Expr::Div {
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                };
            } else if self.cur_ident_is("mod") {
                self.bump();
                let right = self.parse_unary()?;
                let span = Span::merge(expr_span(&left), expr_span(&right));
                left = Expr::Mod {
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, LirError> {
        if self.cur_ident_is("neg") {
            let sp = self.gspan(self.cur().span);
            self.bump();
            let inner = self.parse_unary()?;
            let span = Span::merge(sp, expr_span(&inner));
            return Ok(Expr::Neg {
                inner: Box::new(inner),
                span,
            });
        }
        self.parse_primary_expr()
    }

    fn parse_primary_expr(&mut self) -> Result<Expr, LirError> {
        let lx = self.cur();
        match &lx.tok {
            Token::Dot => {
                let sp = self.gspan(lx.span);
                self.bump();
                Ok(Expr::Dot { span: sp })
            }
            Token::Int(v) => {
                let sp = self.gspan(lx.span);
                let v = *v;
                self.bump();
                Ok(Expr::Lit { v, span: sp })
            }
            Token::Ident(s) if s == "square" => {
                let sp = self.gspan(lx.span);
                self.bump();
                let d = Expr::Dot { span: sp };
                Ok(Expr::Mul {
                    left: Box::new(d.clone()),
                    right: Box::new(d),
                    span: sp,
                })
            }
            Token::LParen => {
                self.bump();
                let e = self.parse_expr()?;
                self.expect_tok(Token::RParen, ")")?;
                Ok(e)
            }
            _ => Err(LirError::Syntax {
                code: "E_EXPR_PRIMARY",
                span: self.gspan(lx.span),
                message: "expected `.`, integer, square, or `(`".into(),
                fix_hint: "In map, use `.` for the current element, e.g. `mul . .` or `square`.".into(),
            }),
        }
    }
}

fn pred_span(p: &Predicate) -> Span {
    match p {
        Predicate::Or { span, .. }
        | Predicate::And { span, .. }
        | Predicate::Not { span, .. }
        | Predicate::Even { span }
        | Predicate::Odd { span }
        | Predicate::Cmp { span, .. } => *span,
    }
}

fn expr_span(e: &Expr) -> Span {
    match e {
        Expr::Add { span, .. }
        | Expr::Sub { span, .. }
        | Expr::Mul { span, .. }
        | Expr::Div { span, .. }
        | Expr::Mod { span, .. }
        | Expr::Neg { span, .. }
        | Expr::Dot { span }
        | Expr::Lit { span, .. } => *span,
    }
}

fn desugar_map_expr(e: Expr) -> Expr {
    e
}

fn provably_infinite(start: i64, stop: i64, step: i64) -> bool {
    if step == 0 {
        return true;
    }
    if start < stop && step > 0 {
        return false;
    }
    if start > stop && step < 0 {
        return false;
    }
    if start == stop {
        return false;
    }
    true
}
