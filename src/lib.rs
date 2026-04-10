//! **LIR** — fast data-processing language (`lir/1`): lexer, parser, typecheck, interpreter,
//! LLVM IR, WASM. **LYTR** is the general-purpose language built on LIR (see `docs/NAMING.md`).
//! Normative semantics: `docs/LIR_V1_SPEC.md`.

pub mod ast;
pub mod ast_json;
pub mod error;
pub mod format;
pub mod input_parse;
pub mod interp;
pub mod lex;
pub mod llvm_ir;
pub mod lytr;
pub mod parse;
pub mod types;
pub mod wasm;

pub use ast::Program;
pub use ast_json::{
    deserialize_lir_ast_document, serialize_lir_ast_document, AstJsonError, LirAstDocument,
    AST_JSON_SCHEMA_VERSION,
};
pub use error::{cli_json_line, LirError, Span};
pub use format::{format_program, program_is_canonical_text};
pub use input_parse::parse_input_array;
pub use interp::{run_program, RunOutcome, Val};
pub use parse::parse_program;
pub use types::{check_program, source_stream_ty};
pub use llvm_ir::{codegen_supported, emit_llvm_ir};
pub use lytr::{
    check_lytr_program, parse_lytr_program, run_lytr_program, LytrError, LytrRun, MainTail,
};
pub use wasm::{adapt_llvm_ir_for_wasm, emit_wasm, wasm_clang_target_ok};
