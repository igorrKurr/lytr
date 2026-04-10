//! **LYTR 0.1** bootstrap: parse, type-check, and interpret a tiny subset.
//!
//! Grammar: edition line `lytr/0.1`, then `fn main() -> i32 { return <expr>; }`
//! where `<expr>` is integer literals with `+ - * / %` and parentheses.
//!
//! Normative language design: `docs/LYTR_CORE_CALCULUS_DRAFT.md`. This module proves
//! the Phase 5 "tiny program parse/check/run" milestone.

pub mod ast;
pub mod error;
pub mod interp;
pub mod lex;
pub mod parse;
pub mod types;

pub use error::LytrError;
pub use interp::run_lytr_program;
pub use parse::parse_lytr_program;
pub use types::check_lytr_program;
