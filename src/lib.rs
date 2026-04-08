//! LIR v1 — reference implementation (lexer, parser, typecheck, interpreter).
//! See `docs/LIR_V1_SPEC.md` for normative semantics.

pub mod ast;
pub mod error;
pub mod interp;
pub mod lex;
pub mod llvm_ir;
pub mod parse;
pub mod types;

pub use ast::Program;
pub use error::{LirError, Span};
pub use interp::{run_program, RunOutcome, Val};
pub use parse::parse_program;
pub use types::{check_program, source_stream_ty};
pub use llvm_ir::emit_llvm_ir;
