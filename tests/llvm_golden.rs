//! Compare LLVM emission + `clang` execution against the reference interpreter.
//! Skips silently when `clang` is not on `PATH`.
//!
//! Replaces `target triple = "unknown-unknown-unknown"` with `clang -print-target-triple`
//! when available so native codegen matches the host.
//!
//! Trap programs: the interpreter must return the expected `LirError::Runtime` `code`; the
//! native binary linked from emitted IR must exit non-success (`llvm.trap`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use lir::interp::{RunOutcome, Val};
use lir::{check_program, emit_llvm_ir, parse_program, run_program, LirError};

fn clang_available() -> bool {
    Command::new("clang")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn clang_target_triple() -> Option<String> {
    let o = Command::new("clang")
        .arg("-print-target-triple")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !o.status.success() {
        return None;
    }
    let s = String::from_utf8(o.stdout).ok()?;
    let t = s.trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

fn ir_with_host_triple(ir: &str) -> String {
    let Some(triple) = clang_target_triple() else {
        return ir.to_string();
    };
    ir.lines()
        .map(|line| {
            if line.trim_start().starts_with("target triple =") {
                format!("target triple = \"{triple}\"")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

enum Case {
    Void(&'static str),
    Input32(&'static str, &'static [i32]),
    Input64(&'static str, &'static [i64]),
    InputBool(&'static str, &'static [bool]),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RetWidth {
    I32,
    I64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ElemWidth {
    I32,
    I64,
    I8,
}

struct MainSig {
    ret: RetWidth,
    /// `None` for `lir_main()`; else buffer element type + values.
    input: Option<(ElemWidth, Vec<i64>)>,
}

fn detect_main_sig(ir: &str, case: &Case) -> MainSig {
    for line in ir.lines() {
        let t = line.trim_start();
        if !t.starts_with("define ") || !t.contains("@lir_main(") {
            continue;
        }
        let ret = if t.starts_with("define i64 @lir_main") {
            RetWidth::I64
        } else if t.starts_with("define i32 @lir_main") {
            RetWidth::I32
        } else {
            panic!("unexpected @lir_main return type in: {t}");
        };

        if t.contains("@lir_main()") {
            return MainSig { ret, input: None };
        }

        let el = if t.contains("(i64*") {
            ElemWidth::I64
        } else if t.contains("(i32*") {
            ElemWidth::I32
        } else if t.contains("(i8*") {
            ElemWidth::I8
        } else {
            panic!("unexpected @lir_main parameters in: {t}");
        };

        let values = match case {
            Case::Void(_) => panic!("void case but IR has input main"),
            Case::Input32(_, v) => v.iter().map(|&x| x as i64).collect(),
            Case::Input64(_, v) => v.to_vec(),
            Case::InputBool(_, v) => v.iter().map(|&b| if b { 1i64 } else { 0 }).collect(),
        };
        return MainSig {
            ret,
            input: Some((el, values)),
        };
    }
    panic!("could not find @lir_main in IR");
}

fn harness_c(sig: &MainSig) -> String {
    match (&sig.ret, &sig.input) {
        (RetWidth::I32, None) => r#"#include <stdint.h>
#include <stdio.h>
int32_t lir_main(void);
int main(void) {
  int32_t r = lir_main();
  if (fwrite(&r, sizeof r, 1, stdout) != 1) return 1;
  return 0;
}
"#
        .to_string(),
        (RetWidth::I64, None) => r#"#include <stdint.h>
#include <stdio.h>
int64_t lir_main(void);
int main(void) {
  int64_t r = lir_main();
  if (fwrite(&r, sizeof r, 1, stdout) != 1) return 1;
  return 0;
}
"#
        .to_string(),
        (RetWidth::I32, Some((ElemWidth::I32, values))) => {
            let mut s = String::from(
                r#"#include <stdint.h>
#include <stdio.h>
int32_t lir_main(int32_t* in, int32_t len);
int main(void) {
  int32_t data[] = { "#,
            );
            for (i, v) in values.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&v.to_string());
            }
            s.push_str(
                r#" };
  int32_t r = lir_main(data, (int32_t)(sizeof data / sizeof data[0]));
  if (fwrite(&r, sizeof r, 1, stdout) != 1) return 1;
  return 0;
}
"#,
            );
            s
        }
        (RetWidth::I64, Some((ElemWidth::I64, values))) => {
            let mut s = String::from(
                r#"#include <stdint.h>
#include <stdio.h>
int64_t lir_main(int64_t* in, int32_t len);
int main(void) {
  int64_t data[] = { "#,
            );
            for (i, v) in values.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&v.to_string());
            }
            s.push_str(
                r#" };
  int64_t r = lir_main(data, (int32_t)(sizeof data / sizeof data[0]));
  if (fwrite(&r, sizeof r, 1, stdout) != 1) return 1;
  return 0;
}
"#,
            );
            s
        }
        (RetWidth::I32, Some((ElemWidth::I64, values))) => {
            let mut s = String::from(
                r#"#include <stdint.h>
#include <stdio.h>
int32_t lir_main(int64_t* in, int32_t len);
int main(void) {
  int64_t data[] = { "#,
            );
            for (i, v) in values.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&v.to_string());
            }
            s.push_str(
                r#" };
  int32_t r = lir_main(data, (int32_t)(sizeof data / sizeof data[0]));
  if (fwrite(&r, sizeof r, 1, stdout) != 1) return 1;
  return 0;
}
"#,
            );
            s
        }
        (RetWidth::I64, Some((ElemWidth::I32, _))) => {
            panic!("unexpected i64 return with i32* buffer in harness")
        }
        (RetWidth::I32, Some((ElemWidth::I8, values))) => {
            let mut s = String::from(
                r#"#include <stdint.h>
#include <stdio.h>
int32_t lir_main(int8_t* in, int32_t len);
int main(void) {
  int8_t data[] = { "#,
            );
            for (i, v) in values.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&v.to_string());
            }
            s.push_str(
                r#" };
  int32_t r = lir_main(data, (int32_t)(sizeof data / sizeof data[0]));
  if (fwrite(&r, sizeof r, 1, stdout) != 1) return 1;
  return 0;
}
"#,
            );
            s
        }
        (RetWidth::I64, Some((ElemWidth::I8, _))) => {
            panic!("unexpected i64 return with i8* buffer in harness")
        }
    }
}

fn interp_as_i128(out: RunOutcome) -> i128 {
    match out {
        RunOutcome::Scalar(Val::I32(x)) => x as i128,
        RunOutcome::Scalar(Val::I64(x)) => x as i128,
        o => panic!("expected scalar, got {o:?}"),
    }
}

fn vals_for_case(case: &Case) -> (/* src */ &'static str, Vec<Val>) {
    match case {
        Case::Void(s) => (*s, Vec::new()),
        Case::Input32(s, v) => (*s, v.iter().copied().map(Val::I32).collect()),
        Case::Input64(s, v) => (*s, v.iter().copied().map(Val::I64).collect()),
        Case::InputBool(s, v) => (*s, v.iter().copied().map(Val::Bool).collect()),
    }
}

fn compile_native(ir: &str, sig: &MainSig, dir: &Path) -> PathBuf {
    let ll = dir.join("m.ll");
    let c = dir.join("main.c");
    let exe = dir.join("prg");
    fs::write(&ll, ir).unwrap();
    fs::write(&c, harness_c(sig)).unwrap();
    let o = Command::new("clang")
        .arg("-O0")
        .arg("-o")
        .arg(&exe)
        .arg(&c)
        .arg(&ll)
        .output()
        .expect("spawn clang");
    assert!(
        o.status.success(),
        "clang failed: {}\n{}",
        String::from_utf8_lossy(&o.stderr),
        String::from_utf8_lossy(&o.stdout)
    );
    exe
}

fn run_clang(ir: &str, sig: &MainSig, dir: &Path) -> i128 {
    let exe = compile_native(ir, sig, dir);
    let run = Command::new(&exe).output().expect("spawn exe");
    assert!(
        run.status.success(),
        "native run failed: stderr={}",
        String::from_utf8_lossy(&run.stderr)
    );
    let b = run.stdout.as_slice();
    match sig.ret {
        RetWidth::I32 => {
            assert_eq!(b.len(), 4, "expected 4-byte stdout, got {b:?}");
            i32::from_le_bytes(b[..4].try_into().unwrap()) as i128
        }
        RetWidth::I64 => {
            assert_eq!(b.len(), 8, "expected 8-byte stdout, got {b:?}");
            i64::from_le_bytes(b[..8].try_into().unwrap()) as i128
        }
    }
}

fn run_case(case: Case) {
    let (src, input) = vals_for_case(&case);
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let ir_raw = emit_llvm_ir(&p).unwrap();
    let ir = ir_with_host_triple(&ir_raw);
    let interp = interp_as_i128(run_program(&p, &input).unwrap());

    let dir = tempfile::tempdir().unwrap();
    let sig = detect_main_sig(&ir, &case);
    let native = run_clang(&ir, &sig, dir.path());
    assert_eq!(
        interp, native,
        "interpreter vs clang mismatch for:\n{src}\nIR snippet (first 800 chars):\n{}",
        &ir.chars().take(800).collect::<String>()
    );
}

/// Interpreter must surface a runtime `code`; native `clang` binary must not exit cleanly
/// (`llvm.trap` / abort).
fn run_trap_case(case: Case, expect_code: &'static str) {
    let (src, input) = vals_for_case(&case);
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let ir_raw = emit_llvm_ir(&p).expect("LLVM emit should succeed for trap golden");
    let ir = ir_with_host_triple(&ir_raw);

    let err = run_program(&p, &input).unwrap_err();
    match &err {
        LirError::Runtime { code, .. } => assert_eq!(
            *code, expect_code,
            "wrong interpreter code for:\n{src}\n{err}"
        ),
        _ => panic!("expected runtime [{expect_code}], got {err:?} for:\n{src}"),
    }

    let dir = tempfile::tempdir().unwrap();
    let sig = detect_main_sig(&ir, &case);
    let exe = compile_native(&ir, &sig, dir.path());
    let out = Command::new(&exe).output().expect("spawn trapped exe");
    assert!(
        !out.status.success(),
        "expected native non-success (trap) for:\n{src}\nstdout={:?} stderr={:?}",
        out.stdout,
        out.stderr
    );
}

#[test]
fn golden_void_programs_match_clang() {
    if !clang_available() {
        return;
    }
    for src in [
        "lir/1\nrange(0,5) | reduce sum",
        "lir/1\nlit(1) | take 0 | reduce sum",
        "lir/1\nrange(1,4) | reduce prod",
        "lir/1\nrange(0,5) | reduce count",
        "lir/1\nrange(1,4) | reduce min",
        "lir/1\nrange(1,4) | reduce max",
        "lir/1\nrange(0,3) | scan 0, add | reduce sum",
        "lir/1\nrange(0,3) | scan 0, add | map square | reduce sum",
        "lir/1\nrange(0,10) | filter even | map . add 1 | reduce sum",
        "lir/1\nrange(0,2) | scan 0, add | filter even | reduce sum",
        "lir/1\nlit(true, false, true) | filter eq true | reduce count",
    ] {
        run_case(Case::Void(src));
    }
}

#[test]
fn golden_i64_void_matches_clang() {
    if !clang_available() {
        return;
    }
    run_case(Case::Void(
        "lir/1\nlit(3000000000, 3000000000) | reduce sum",
    ));
}

#[test]
fn golden_input_i32_matches_clang() {
    if !clang_available() {
        return;
    }
    run_case(Case::Input32(
        "lir/1\ninput:i32 | reduce sum",
        &[10, 20, 30],
    ));
}

#[test]
fn golden_input_i64_matches_clang() {
    if !clang_available() {
        return;
    }
    run_case(Case::Input64(
        "lir/1\ninput:i64 | reduce sum",
        &[1_000_000_000_000, 1_000_000_000_000],
    ));
    run_case(Case::Input64(
        "lir/1\ninput:i64 | reduce count",
        &[9, 8, 7],
    ));
    run_case(Case::Input64(
        "lir/1\ninput:i64 | scan 0, add | map . add 1 | reduce sum",
        &[10, 20, 30],
    ));
    run_case(Case::Input64(
        "lir/1\ninput:i64 | map . add 100 | reduce prod",
        &[2, 3],
    ));
}

#[test]
fn golden_input_bool_matches_clang() {
    if !clang_available() {
        return;
    }
    run_case(Case::InputBool(
        "lir/1\ninput:bool | filter eq true | reduce count",
        &[true, false, true, true],
    ));
}

#[test]
fn golden_traps_interpreter_vs_native() {
    if !clang_available() {
        return;
    }
    run_trap_case(
        Case::Void("lir/1\nrange(0,0) | reduce min"),
        "R_REDUCE_EMPTY_MINMAX",
    );
    run_trap_case(
        Case::Void("lir/1\nlit(2000000000, 2000000000) | reduce sum"),
        "R_INTEGER_OVERFLOW",
    );
    run_trap_case(
        Case::Void("lir/1\nlit(9223372036854775807, 1) | reduce sum"),
        "R_INTEGER_OVERFLOW",
    );
    run_trap_case(
        Case::Void("lir/1\nlit(1) | map . div 0 | reduce sum"),
        "R_DIV_BY_ZERO",
    );
    run_trap_case(
        Case::Void("lir/1\nlit(1) | map . mod 0 | reduce sum"),
        "R_DIV_BY_ZERO",
    );
    run_trap_case(
        Case::Void("lir/1\nlit(-2147483648) | map . div -1 | reduce sum"),
        "R_INTEGER_OVERFLOW",
    );
    run_trap_case(
        Case::Void("lir/1\nlit(-2147483648) | map . mod -1 | reduce sum"),
        "R_INTEGER_OVERFLOW",
    );
    run_trap_case(
        Case::Void("lir/1\nlit(-9223372036854775808) | map . mod -1 | reduce sum"),
        "R_INTEGER_OVERFLOW",
    );
    run_trap_case(
        Case::Input64(
            "lir/1\ninput:i64 | reduce sum",
            &[i64::MAX, 1],
        ),
        "R_INTEGER_OVERFLOW",
    );
}
