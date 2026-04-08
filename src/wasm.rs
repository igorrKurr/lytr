//! WebAssembly backend: compile the same LLVM IR as [`crate::emit_llvm_ir`] to `.wasm`
//! using **clang** with `--target=wasm32-unknown-unknown`.
//!
//! Requires a clang build that can **link** wasm (typical on Linux packages;
//! Apple’s Xcode clang often prints a wasm triple but cannot link — install LLVM or set `LIR_CLANG`).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::ast::Program;
use crate::error::LirError;
use crate::llvm_ir::emit_llvm_ir;

/// LLVM IR `target datalayout` suitable for `wasm32-unknown-unknown` when lowering this crate’s IR.
const WASM_DATALAYOUT: &str =
    "e-m:e-p:32:32-p10:8:8-p20:8:8-i64:64-n32:64-S128";
const WASM_TRIPLE: &str = "wasm32-unknown-unknown";

/// Rewrite module header so clang’s WebAssembly backend accepts the IR.
pub fn adapt_llvm_ir_for_wasm(ir: &str) -> String {
    ir.lines()
        .map(|line| {
            let t = line.trim_start();
            if t.starts_with("target datalayout =") {
                format!("target datalayout = \"{WASM_DATALAYOUT}\"")
            } else if t.starts_with("target triple =") {
                format!("target triple = \"{WASM_TRIPLE}\"")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn clang_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("LIR_CLANG") {
        let p = p.trim();
        if !p.is_empty() {
            out.push(PathBuf::from(p));
        }
    }
    if let Ok(sdk) = std::env::var("WASI_SDK_PATH") {
        out.push(Path::new(&sdk).join("bin").join("clang"));
    }
    out.push(PathBuf::from("clang"));
    out
}

fn try_clang_wasm(clang: &Path, ir: &str) -> Result<Vec<u8>, String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let ll = dir.path().join("module.ll");
    let wasm = dir.path().join("module.wasm");
    std::fs::write(&ll, ir).map_err(|e| e.to_string())?;
    let out = Command::new(clang)
        .arg("--target=wasm32-unknown-unknown")
        .arg("-nostdlib")
        .arg("-fno-builtin")
        .arg("-O2")
        .arg("-Wl,--no-entry")
        .arg("-Wl,--export=lir_main")
        .arg("-o")
        .arg(&wasm)
        .arg(&ll)
        .output()
        .map_err(|e| format!("{}: {e}", clang.display()))?;
    if !out.status.success() {
        return Err(format!(
            "{} failed:\n{}",
            clang.display(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    std::fs::read(&wasm).map_err(|e| e.to_string())
}

/// Emit a WebAssembly module for the same fused subset as LLVM IR (`emit_llvm_ir`).
///
/// On failure (e.g. no wasm-capable clang), returns [`LirError::Type`] with code `T_WASM_COMPILE`.
pub fn emit_wasm(prog: &Program) -> Result<Vec<u8>, LirError> {
    let ir = emit_llvm_ir(prog)?;
    let ir = adapt_llvm_ir_for_wasm(&ir);
    let mut last = String::new();
    for clang in clang_candidates() {
        match try_clang_wasm(&clang, &ir) {
            Ok(bytes) => return Ok(bytes),
            Err(e) => last = e,
        }
    }
    Err(LirError::Type {
        code: "T_WASM_COMPILE",
        span: prog.span,
        message: format!("cannot compile LLVM IR to WebAssembly: {last}"),
        fix_hint: "Install clang with the WebAssembly target (Ubuntu: apt install clang lld), set LIR_CLANG to that binary, or use WASI_SDK_PATH. macOS Xcode clang often lacks wasm32.".into(),
        stage_index: None,
    })
}

fn probe_wasm_link_works() -> bool {
    let ir = format!(
        "target datalayout = \"{WASM_DATALAYOUT}\"\n\
         target triple = \"{WASM_TRIPLE}\"\n\
         define i32 @lir_main() local_unnamed_addr #0 {{\n  ret i32 0\n}}\n\
         attributes #0 = {{ nounwind }}\n"
    );
    let Ok(dir) = tempfile::tempdir() else {
        return false;
    };
    let ll = dir.path().join("probe.ll");
    let wasm = dir.path().join("probe.wasm");
    if std::fs::write(&ll, ir.as_bytes()).is_err() {
        return false;
    }
    for clang in clang_candidates() {
        let Ok(out) = Command::new(&clang)
            .arg("--target=wasm32-unknown-unknown")
            .arg("-nostdlib")
            .arg("-fno-builtin")
            .arg("-Wl,--no-entry")
            .arg("-Wl,--export=lir_main")
            .arg("-o")
            .arg(&wasm)
            .arg(&ll)
            .output()
        else {
            continue;
        };
        if out.status.success() && std::fs::read(&wasm).map(|b| !b.is_empty()).unwrap_or(false) {
            return true;
        }
    }
    false
}

static WASM_LINK_OK: OnceLock<bool> = OnceLock::new();

/// True if a probe `clang` invocation can link a trivial wasm32 module (cached).
pub fn wasm_clang_target_ok() -> bool {
    *WASM_LINK_OK.get_or_init(probe_wasm_link_works)
}
