//! Type-checked programs may still be outside the LLVM/WASM codegen subset (§13).

use lir::{
    check_program, codegen_supported, emit_llvm_ir, parse_program, run_program, LirError,
};
use lir::interp::{RunOutcome, Val};

fn assert_codegen_unsupported(src: &str) {
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    assert!(
        codegen_supported(&p).is_err(),
        "codegen_supported should fail for:\n{src}"
    );
    let err = emit_llvm_ir(&p).expect_err("expected codegen to reject");
    match &err {
        LirError::Type { code: "T_CODEGEN_UNSUPPORTED", .. } => {}
        e => panic!("expected T_CODEGEN_UNSUPPORTED, got {e:?}"),
    }
}

#[test]
fn drop_after_map_typechecks_but_not_llvm() {
    assert_codegen_unsupported("lir/1\nrange(0,5) | map . add 1 | drop 2 | reduce sum");
}

#[test]
fn two_scans_typechecks_but_not_llvm() {
    assert_codegen_unsupported("lir/1\nrange(0,3) | scan 0, add | scan 0, mul | reduce sum");
}

#[test]
fn take_after_filter_typechecks_but_not_llvm() {
    assert_codegen_unsupported("lir/1\nrange(0,10) | filter even | take 3 | reduce sum");
}

/// Representative programs in the LLVM codegen subset (see docs/codegen_subset.json).
const SUBSET_SOURCES: &[&str] = &[
    "lir/1\nrange(0,5) | reduce sum",
    "lir/1\nrange(0,5) | reduce count",
    "lir/1\nlit(3000000000, 3000000000) | reduce sum",
    "lir/1\ninput:i32 | reduce sum",
    "lir/1\ninput:i64 | reduce count",
    "lir/1\ninput:bool | filter eq true | reduce count",
    "lir/1\nrange(0,3) | scan 0, add | map square | reduce sum",
    "lir/1\nrange(0,10) | drop 1 | take 4 | filter even | map . add 1 | reduce sum",
];

#[test]
fn codegen_supported_agrees_with_emit_llvm_ir() {
    for src in SUBSET_SOURCES {
        let p = parse_program(src).unwrap();
        check_program(&p).unwrap();
        let sup = codegen_supported(&p);
        let emit = emit_llvm_ir(&p).map(|_| ());
        assert_eq!(
            sup.is_ok(),
            emit.is_ok(),
            "codegen_supported vs emit mismatch for:\n{src}\n sup={sup:?} emit={emit:?}"
        );
    }
}

#[test]
fn subset_pipeline_runs_same_interp_and_llvm() {
    let src = "lir/1\nrange(0,10) | drop 1 | take 4 | filter even | map . add 1 | reduce sum";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    codegen_supported(&p).expect("subset should be codegen_supported");
    let ir = emit_llvm_ir(&p).expect("subset should compile");
    assert!(ir.contains("@lir_main"));
    let i = run_program(&p, &[]).unwrap();
    let RunOutcome::Scalar(Val::I32(n)) = i else {
        panic!("expected i32 scalar");
    };
    assert_eq!(n, 8);
}
