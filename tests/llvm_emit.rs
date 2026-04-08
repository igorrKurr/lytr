use lir::{check_program, emit_llvm_ir, parse_program};

#[test]
fn llvm_emits_scan_reduce_sum() {
    let src = "lir/1\nrange(0,3) | scan 0, add | reduce sum";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let ir = emit_llvm_ir(&p).unwrap();
    assert!(ir.contains("scan_pre:"));
    assert!(ir.contains("%scan_acc"));
    assert!(ir.contains("%scan_acc1"));
}

#[test]
fn llvm_emits_range_count() {
    let src = "lir/1\nrange(0,3) | reduce count";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let ir = emit_llvm_ir(&p).unwrap();
    assert!(ir.contains("define i32 @lir_main()"));
    assert!(ir.contains("@lir_data"));
    assert!(ir.contains("llvm.sadd.with.overflow.i32"));
    assert!(ir.contains("trap_ov"));
}

#[test]
fn llvm_emits_input_i32_sum() {
    let src = "lir/1\ninput:i32 | reduce sum";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let ir = emit_llvm_ir(&p).unwrap();
    assert!(ir.contains("define i32 @lir_main(i32*"));
    assert!(ir.contains("%in_len"));
}

#[test]
fn llvm_emits_post_scan_filter_pipeline() {
    let src = "lir/1\nrange(0,2) | scan 0, add | filter even | reduce sum";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let ir = emit_llvm_ir(&p).unwrap();
    assert!(ir.contains("scan_pre:"));
    assert!(ir.contains("scan_pre_fail:"));
    assert!(ir.contains("fold_tail:"));
    assert!(
        ir.match_indices("br i1 ").count() >= 2,
        "expected filter checks in scan_pre and in-loop post-scan pipeline"
    );
}

#[test]
fn llvm_emits_scan_map_reduce() {
    let src = "lir/1\nrange(0,3) | scan 0, add | map square | reduce sum";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let ir = emit_llvm_ir(&p).unwrap();
    assert!(ir.contains("scan_pre:"));
    assert!(ir.contains("%scan_el0"));
}

#[test]
fn llvm_emits_nested_map() {
    let src = "lir/1\nrange(0,2) | map . add (1 add 2) | reduce sum";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let ir = emit_llvm_ir(&p).unwrap();
    assert!(
        ir.match_indices("llvm.sadd.with.overflow.i32").count() >= 2,
        "nested add should use multiple checked sadds"
    );
}

#[test]
fn llvm_emits_reduce_min_max() {
    for (src, needle) in [
        ("lir/1\nrange(1,4) | reduce min", "icmp sle"),
        ("lir/1\nrange(1,4) | reduce max", "icmp sge"),
    ] {
        let p = parse_program(src).unwrap();
        check_program(&p).unwrap();
        let ir = emit_llvm_ir(&p).unwrap();
        assert!(ir.contains("phi i1"), "{src}: expected seen phi");
        assert!(ir.contains(needle), "{src}: expected {needle}");
        assert!(ir.contains("trap_mm_empty"), "{src}: empty min/max trap");
    }
}

#[test]
fn llvm_empty_minmax_has_empty_trap() {
    let src = "lir/1\nrange(0,0) | reduce min";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let ir = emit_llvm_ir(&p).unwrap();
    assert!(ir.contains("trap_mm_empty"));
}
