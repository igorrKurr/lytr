//! Golden: text → parse → JSON → parse → fmt matches fmt ∘ parse (Phase 2).

use lir::{
    check_program, deserialize_lir_ast_document, format_program, parse_program,
    serialize_lir_ast_document,
};

fn assert_json_roundtrip(src: &str) {
    let p = parse_program(src).unwrap();
    check_program(&p).expect("fixture must type-check");
    let canon = format_program(&p);
    let json = serialize_lir_ast_document(&p).unwrap();
    let p2 = deserialize_lir_ast_document(&json).unwrap();
    check_program(&p2).unwrap();
    let out = format_program(&p2);
    assert_eq!(out, canon, "JSON round-trip changed canonical text");
}

#[test]
fn roundtrip_range_sum_task() {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/eval/tasks/001_range_sum/starter.lir"
    ));
    assert_json_roundtrip(src);
}

#[test]
fn roundtrip_filter_map_reduce() {
    let src = "lir/1\ninput:i64 | filter gt 0 & even | map . mul 2 | reduce sum\n";
    assert_json_roundtrip(src);
}

#[test]
fn roundtrip_bool_lit() {
    let src = "lir/1\nlit ( true, false, true ) | filter eq true | reduce count\n";
    assert_json_roundtrip(src);
}

#[test]
fn reject_wrong_schema_version() {
    let src = "lir/1\ninput:i32 | reduce count\n";
    let p = parse_program(src).unwrap();
    let good = serialize_lir_ast_document(&p).unwrap();
    let bad = good.replace("\"schema_version\": 1", "\"schema_version\": 999");
    assert!(deserialize_lir_ast_document(&bad).is_err());
}
