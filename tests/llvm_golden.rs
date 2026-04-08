//! Compare LLVM emission + `clang` execution against the reference interpreter.
//! Skips silently when `clang` is not on `PATH`.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use lir::interp::{RunOutcome, Val};
use lir::{check_program, emit_llvm_ir, parse_program, run_program};

fn clang_available() -> bool {
    Command::new("clang")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

enum Case {
    Void(&'static str),
    Input(&'static str, &'static [i32]),
}

enum MainKind {
    I32Void,
    I64Void,
    I32Input { values: Vec<i32> },
}

fn detect_main_kind(ir: &str, input_vals: &[i32]) -> MainKind {
    for line in ir.lines() {
        let t = line.trim_start();
        if t.starts_with("define i32 @lir_main()") {
            return MainKind::I32Void;
        }
        if t.starts_with("define i64 @lir_main()") {
            return MainKind::I64Void;
        }
        if t.starts_with("define i32 @lir_main(i32*") {
            return MainKind::I32Input {
                values: input_vals.to_vec(),
            };
        }
        if t.starts_with("define i64 @lir_main(i64*") {
            panic!("golden harness does not yet cover i64 input mains");
        }
    }
    panic!("could not find @lir_main in IR");
}

fn harness_c(kind: &MainKind) -> String {
    match kind {
        MainKind::I32Void => r#"#include <stdint.h>
#include <stdio.h>
int32_t lir_main(void);
int main(void) {
  int32_t r = lir_main();
  if (fwrite(&r, sizeof r, 1, stdout) != 1) return 1;
  return 0;
}
"#
        .to_string(),
        MainKind::I64Void => r#"#include <stdint.h>
#include <stdio.h>
int64_t lir_main(void);
int main(void) {
  int64_t r = lir_main();
  if (fwrite(&r, sizeof r, 1, stdout) != 1) return 1;
  return 0;
}
"#
        .to_string(),
        MainKind::I32Input { values } => {
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
    }
}

fn interp_as_i128(out: RunOutcome) -> i128 {
    match out {
        RunOutcome::Scalar(Val::I32(x)) => x as i128,
        RunOutcome::Scalar(Val::I64(x)) => x as i128,
        o => panic!("expected scalar, got {o:?}"),
    }
}

fn run_clang(ir: &str, kind: &MainKind, dir: &Path) -> i128 {
    let ll = dir.join("m.ll");
    let c = dir.join("main.c");
    let exe = dir.join("prg");
    fs::write(&ll, ir).unwrap();
    fs::write(&c, harness_c(kind)).unwrap();
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
    let run = Command::new(&exe).output().expect("spawn exe");
    assert!(
        run.status.success(),
        "native run failed: stderr={}",
        String::from_utf8_lossy(&run.stderr)
    );
    let b = run.stdout.as_slice();
    match kind {
        MainKind::I32Void | MainKind::I32Input { .. } => {
            assert_eq!(b.len(), 4, "expected 4-byte stdout, got {b:?}");
            i32::from_le_bytes(b[..4].try_into().unwrap()) as i128
        }
        MainKind::I64Void => {
            assert_eq!(b.len(), 8, "expected 8-byte stdout, got {b:?}");
            i64::from_le_bytes(b[..8].try_into().unwrap()) as i128
        }
    }
}

fn run_case(case: Case) {
    let (src, input_i32) = match case {
        Case::Void(s) => (s, &[][..]),
        Case::Input(s, v) => (s, v),
    };
    let input: Vec<Val> = input_i32.iter().copied().map(Val::I32).collect();
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let ir = emit_llvm_ir(&p).unwrap();
    let interp = interp_as_i128(run_program(&p, &input).unwrap());

    let dir = tempfile::tempdir().unwrap();
    let kind = detect_main_kind(&ir, input_i32);
    let native = run_clang(&ir, &kind, dir.path());
    assert_eq!(
        interp, native,
        "interpreter vs clang mismatch for:\n{src}\nIR snippet (first 800 chars):\n{}",
        &ir.chars().take(800).collect::<String>()
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
        "lir/1\nrange(0,3) | scan 0, add | reduce sum",
        "lir/1\nrange(0,3) | scan 0, add | map square | reduce sum",
        "lir/1\nrange(0,10) | filter even | map . add 1 | reduce sum",
        "lir/1\nrange(0,2) | scan 0, add | filter even | reduce sum",
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
fn golden_input_main_matches_clang() {
    if !clang_available() {
        return;
    }
    run_case(Case::Input(
        "lir/1\ninput:i32 | reduce sum",
        &[10, 20, 30],
    ));
}
