use lir::{check_lytr_program, parse_lytr_program, run_lytr_program};

#[test]
fn minimal_example_runs() {
    let src = include_str!("../examples/minimal.lytr");
    let p = parse_lytr_program(src).unwrap();
    check_lytr_program(&p).unwrap();
    assert_eq!(run_lytr_program(&p).unwrap(), 7);
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
    assert_eq!(run_lytr_program(&p).unwrap(), 9);
}

#[test]
fn reject_bad_header() {
    let src = "lir/1\nfn main() -> i32 { return 0; }\n";
    assert!(parse_lytr_program(src).is_err());
}
