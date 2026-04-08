use lir::{format_program, parse_program};

#[test]
fn bare_input_becomes_explicit_i32() {
    let src = "lir/1\ninput | reduce count\n";
    let p = parse_program(src).unwrap();
    let out = format_program(&p);
    assert_eq!(out, "lir/1\ninput:i32 | reduce count\n");
}

#[test]
fn format_roundtrip_stable() {
    let src = "lir/1\ninput:i64 | filter gt 0 & even | map . mul 2 | reduce sum\n";
    let p = parse_program(src).unwrap();
    let once = format_program(&p);
    let p2 = parse_program(&once).unwrap();
    let twice = format_program(&p2);
    assert_eq!(once, twice);
}

#[test]
fn bool_lit_filter_count() {
    let src = "lir/1\nlit(true,false,true)|filter eq true|reduce count";
    let p = parse_program(src).unwrap();
    let out = format_program(&p);
    assert_eq!(
        out,
        "lir/1\nlit ( true, false, true ) | filter eq true | reduce count\n"
    );
}

#[test]
fn range_two_arg_default_step() {
    let src = "lir/1\nrange(0,3)|reduce count";
    let p = parse_program(src).unwrap();
    let out = format_program(&p);
    assert!(out.contains("range ( 0 , 3 )"));
}
