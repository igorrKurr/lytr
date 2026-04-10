use lir::{check_lytr_program, parse_lytr_program, run_lytr_program, LytrRun};

#[test]
fn minimal_example_runs() {
    let src = include_str!("../examples/minimal.lytr");
    let p = parse_lytr_program(src).unwrap();
    check_lytr_program(&p).unwrap();
    assert_eq!(run_lytr_program(&p).unwrap(), LytrRun::I32(7));
}

#[test]
fn parens_and_precedence() {
    let src = r"lytr/0.1

fn main() -> i32 {
  return (1 + 2) * 3;
}
";
    let p = parse_lytr_program(src).unwrap();
    check_lytr_program(&p).unwrap();
    assert_eq!(run_lytr_program(&p).unwrap(), LytrRun::I32(9));
}

#[test]
fn reject_bad_header() {
    let src = "lir/1\nfn main() -> i32 { return 0; }\n";
    assert!(parse_lytr_program(src).is_err());
}

#[test]
fn let_and_if() {
    let src = include_str!("../examples/let_if.lytr");
    let p = parse_lytr_program(src).unwrap();
    check_lytr_program(&p).unwrap();
    assert_eq!(run_lytr_program(&p).unwrap(), LytrRun::I32(10));
}

#[test]
fn result_match_ok_arm() {
    let src = include_str!("../examples/match.lytr");
    let p = parse_lytr_program(src).unwrap();
    check_lytr_program(&p).unwrap();
    assert_eq!(run_lytr_program(&p).unwrap(), LytrRun::I32(42));
}

#[test]
fn result_match_err_arm() {
    let src = r"lytr/0.1

fn main() -> i32 {
  let r = Err(7);
  return match r {
    Ok(v) => v,
    Err(e) => e + 1
  };
}
";
    let p = parse_lytr_program(src).unwrap();
    check_lytr_program(&p).unwrap();
    assert_eq!(run_lytr_program(&p).unwrap(), LytrRun::I32(8));
}

#[test]
fn i64_main_large_literal() {
    let src = r"lytr/0.1

fn main() -> i64 {
  return 3000000000 + 3000000000;
}
";
    let p = parse_lytr_program(src).unwrap();
    check_lytr_program(&p).unwrap();
    assert_eq!(
        run_lytr_program(&p).unwrap(),
        LytrRun::I64(6_000_000_000)
    );
}
