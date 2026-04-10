//! Type-check (bootstrap: `main` returns `i32`, expression is well-formed).

use crate::Span;

use super::ast::Program;
use super::error::LytrError;

pub fn check_lytr_program(prog: &Program) -> Result<(), LytrError> {
    if prog.main.body.stmts.is_empty() {
        return Err(LytrError::Type {
            code: "E_LYTR_TYPE",
            span: prog.main.body.span,
            message: "`main` body must contain `return <expr>;`".into(),
            fix_hint: "add a return statement".into(),
        });
    }
    if prog.main.body.stmts.len() > 1 {
        return Err(LytrError::Type {
            code: "E_LYTR_TYPE",
            span: Span::new(
                prog.main.body.stmts[1].span().start,
                prog.main.body.stmts.last().unwrap().span().end,
            ),
            message: "bootstrap allows only one `return` in `main`".into(),
            fix_hint: "use a single return statement".into(),
        });
    }
    Ok(())
}
