use lir::parse_program;

#[test]
fn header_requires_newline_after_lir_slash_1() {
    assert!(
        parse_program("lir/1").is_err(),
        "§1: `lir/1` must be followed by a newline"
    );
    assert!(parse_program("lir/1\ninput:i32 | reduce count").is_ok());
    assert!(parse_program("lir/1\r\ninput:i32 | reduce count").is_ok());
}
