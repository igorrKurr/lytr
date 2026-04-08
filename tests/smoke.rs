use lir::interp::{RunOutcome, Val};
use lir::{check_program, parse_program, run_program};

#[test]
fn filter_map_reduce_example() {
    let src = r#"lir/1
input:i32 | filter even & gt 10 | map square | reduce sum"#;
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let input: Vec<Val> = vec![Val::I32(4), Val::I32(12), Val::I32(11), Val::I32(20)];
    let out = run_program(&p, &input).unwrap();
    match out {
        RunOutcome::Scalar(Val::I32(n)) => assert_eq!(n, 144 + 400),
        _ => panic!("expected scalar i32"),
    }
}

#[test]
fn range_reduce_count() {
    let src = "lir/1\nrange(0,5) | reduce count";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let out = run_program(&p, &[]).unwrap();
    match out {
        RunOutcome::Scalar(Val::I32(n)) => assert_eq!(n, 5),
        _ => panic!("expected scalar"),
    }
}

#[test]
fn empty_sum_is_zero() {
    let src = "lir/1\nlit(1) | take 0 | reduce sum";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let out = run_program(&p, &[]).unwrap();
    match out {
        RunOutcome::Scalar(Val::I32(0)) => {}
        _ => panic!("expected 0"),
    }
}

#[test]
fn scan_add_then_reduce_sum() {
    let src = "lir/1\nrange(0,3) | scan 0, add | reduce sum";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let out = run_program(&p, &[]).unwrap();
    match out {
        RunOutcome::Scalar(Val::I32(n)) => assert_eq!(n, 4),
        _ => panic!("expected scalar"),
    }
}

#[test]
fn scan_on_empty_stream() {
    let src = "lir/1\nrange(0,0) | scan 5, add | reduce sum";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let out = run_program(&p, &[]).unwrap();
    match out {
        RunOutcome::Scalar(Val::I32(n)) => assert_eq!(n, 5),
        _ => panic!("expected scalar"),
    }
}

#[test]
fn nested_map_add_literal_sum() {
    let src = "lir/1\nrange(0,2) | map . add (1 add 2) | reduce sum";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let out = run_program(&p, &[]).unwrap();
    match out {
        RunOutcome::Scalar(Val::I32(n)) => assert_eq!(n, 7),
        _ => panic!("expected scalar"),
    }
}

#[test]
fn scan_then_map_square_then_sum() {
    let src = "lir/1\nrange(0,3) | scan 0, add | map square | reduce sum";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let out = run_program(&p, &[]).unwrap();
    match out {
        RunOutcome::Scalar(Val::I32(n)) => assert_eq!(n, 10),
        _ => panic!("expected scalar"),
    }
}

#[test]
fn range_min_max() {
    let min_src = "lir/1\nrange(1,4) | reduce min";
    let p = parse_program(min_src).unwrap();
    check_program(&p).unwrap();
    let out = run_program(&p, &[]).unwrap();
    match out {
        RunOutcome::Scalar(Val::I32(n)) => assert_eq!(n, 1),
        _ => panic!("expected scalar"),
    }
    let max_src = "lir/1\nrange(1,4) | reduce max";
    let p = parse_program(max_src).unwrap();
    check_program(&p).unwrap();
    let out = run_program(&p, &[]).unwrap();
    match out {
        RunOutcome::Scalar(Val::I32(n)) => assert_eq!(n, 3),
        _ => panic!("expected scalar"),
    }
}

#[test]
fn min_empty_errors() {
    let src = "lir/1\nlit(1) | take 0 | reduce min";
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let e = run_program(&p, &[]).unwrap_err();
    assert!(e.to_string().contains("R_REDUCE_EMPTY_MINMAX"));
}
