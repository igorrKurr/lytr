//! **LIR** — fast data-processing language (`lir/1`): lexer, parser, typecheck, interpreter,
//! LLVM IR, WASM. **LYTR** is the general-purpose language built on LIR (see `docs/NAMING.md`).
//! Normative semantics: `docs/LIR_V1_SPEC.md`.

pub mod ast;
pub mod error;
pub mod format;
pub mod input_parse;
pub mod interp;
pub mod lex;
pub mod llvm_ir;
pub mod parse;
pub mod types;
pub mod wasm;

pub use ast::Program;
pub use error::{LirError, Span};
pub use format::format_program;
pub use input_parse::parse_input_array;
pub use interp::{run_program, RunOutcome, Val};
pub use parse::parse_program;
pub use types::{check_program, source_stream_ty};
pub use llvm_ir::emit_llvm_ir;
pub use wasm::{adapt_llvm_ir_for_wasm, emit_wasm, wasm_clang_target_ok};
