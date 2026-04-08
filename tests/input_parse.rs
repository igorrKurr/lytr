use lir::ast::ElemTy;
use lir::interp::Val;
use lir::parse_input_array;

#[test]
fn i64_source_accepts_small_integers_as_i64() {
    let v = parse_input_array("[9, 8, 7]", ElemTy::I64).unwrap();
    assert_eq!(
        v,
        vec![Val::I64(9), Val::I64(8), Val::I64(7)]
    );
}

#[test]
fn i32_source_rejects_out_of_range() {
    let e = parse_input_array("[3000000000]", ElemTy::I32).unwrap_err();
    assert!(e.contains("i32") || e.contains("fit"));
}

#[test]
fn bool_source_accepts_only_bools() {
    let v = parse_input_array("[true, false]", ElemTy::Bool).unwrap();
    assert_eq!(v, vec![Val::Bool(true), Val::Bool(false)]);
    assert!(parse_input_array("[1]", ElemTy::Bool).is_err());
}
