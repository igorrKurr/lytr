//! Interpreter vs WebAssembly (clang wasm32 + wasmi), §12 differential tests.
//! Skips when `wasm_clang_target_ok()` is false (common on macOS Xcode clang).

use lir::interp::{RunOutcome, Val};
use lir::{
    check_program, emit_llvm_ir, emit_wasm, parse_program, run_program, wasm_clang_target_ok,
};
use wasmi::{Engine, Linker, Module, Store};

#[derive(Clone, Copy)]
enum RetTy {
    I32,
    I64,
}

#[derive(Clone, Copy)]
enum PtrElem {
    I32,
    I64,
    I8,
}

fn classify_ir(ir: &str) -> (RetTy, Option<PtrElem>) {
    for line in ir.lines() {
        let t = line.trim_start();
        if !t.starts_with("define ") || !t.contains("@lir_main(") {
            continue;
        }
        let ret = if t.starts_with("define i64 ") {
            RetTy::I64
        } else {
            RetTy::I32
        };
        if t.contains("@lir_main()") {
            return (ret, None);
        }
        let el = if t.contains("(i64*") {
            PtrElem::I64
        } else if t.contains("(i32*") {
            PtrElem::I32
        } else if t.contains("(i8*") {
            PtrElem::I8
        } else {
            panic!("unexpected @lir_main params: {t}");
        };
        return (ret, Some(el));
    }
    panic!("no @lir_main in IR");
}

fn interp_scalar_i128(out: RunOutcome) -> i128 {
    match out {
        RunOutcome::Scalar(Val::I32(x)) => x as i128,
        RunOutcome::Scalar(Val::I64(x)) => x as i128,
        o => panic!("expected scalar int, got {o:?}"),
    }
}

fn run_wasm(ir: &str, wasm: &[u8], input: &[Val]) -> i128 {
    let (ret_ty, ptr_el) = classify_ir(ir);
    let engine = Engine::default();
    let module = Module::new(&engine, wasm).expect("wasm module");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .expect("instantiate");

    match ptr_el {
        None => match ret_ty {
            RetTy::I32 => {
                let f = instance
                    .get_typed_func::<(), i32>(&store, "lir_main")
                    .expect("lir_main");
                f.call(&mut store, ()).expect("call") as i128
            }
            RetTy::I64 => {
                let f = instance
                    .get_typed_func::<(), i64>(&store, "lir_main")
                    .expect("lir_main");
                f.call(&mut store, ()).expect("call") as i128
            }
        },
        Some(PtrElem::I32) => {
            let data: Vec<i32> = input
                .iter()
                .map(|v| match v {
                    Val::I32(x) => *x,
                    _ => panic!("expected i32 vals"),
                })
                .collect();
            let mem = instance
                .get_memory(&store, "memory")
                .expect("exported memory");
            let offset = 1024usize;
            let need = offset + 4 * data.len();
            while mem.data_size(&store) < need {
                mem.grow(&mut store, 1).expect("grow memory");
            }
            {
                let m = mem.data_mut(&mut store);
                for (i, v) in data.iter().enumerate() {
                    m[offset + i * 4..offset + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
                }
            }
            match ret_ty {
                RetTy::I32 => {
                    let f = instance
                        .get_typed_func::<(i32, i32), i32>(&store, "lir_main")
                        .expect("lir_main");
                    f.call(&mut store, (offset as i32, data.len() as i32))
                        .expect("call") as i128
                }
                RetTy::I64 => {
                    let f = instance
                        .get_typed_func::<(i32, i32), i64>(&store, "lir_main")
                        .expect("lir_main");
                    f.call(&mut store, (offset as i32, data.len() as i32))
                        .expect("call") as i128
                }
            }
        }
        Some(PtrElem::I64) => {
            let data: Vec<i64> = input
                .iter()
                .map(|v| match v {
                    Val::I64(x) => *x,
                    _ => panic!("expected i64 vals"),
                })
                .collect();
            let mem = instance
                .get_memory(&store, "memory")
                .expect("exported memory");
            let offset = 1024usize;
            let need = offset + 8 * data.len();
            while mem.data_size(&store) < need {
                mem.grow(&mut store, 1).expect("grow memory");
            }
            {
                let m = mem.data_mut(&mut store);
                for (i, v) in data.iter().enumerate() {
                    m[offset + i * 8..offset + i * 8 + 8].copy_from_slice(&v.to_le_bytes());
                }
            }
            match ret_ty {
                RetTy::I32 => {
                    let f = instance
                        .get_typed_func::<(i32, i32), i32>(&store, "lir_main")
                        .expect("lir_main");
                    f.call(&mut store, (offset as i32, data.len() as i32))
                        .expect("call") as i128
                }
                RetTy::I64 => {
                    let f = instance
                        .get_typed_func::<(i32, i32), i64>(&store, "lir_main")
                        .expect("lir_main");
                    f.call(&mut store, (offset as i32, data.len() as i32))
                        .expect("call") as i128
                }
            }
        }
        Some(PtrElem::I8) => {
            let data: Vec<u8> = input
                .iter()
                .map(|v| match v {
                    Val::Bool(b) => *b as u8,
                    _ => panic!("expected bool vals"),
                })
                .collect();
            let mem = instance
                .get_memory(&store, "memory")
                .expect("exported memory");
            let offset = 1024usize;
            let need = offset + data.len();
            while mem.data_size(&store) < need {
                mem.grow(&mut store, 1).expect("grow memory");
            }
            mem.data_mut(&mut store)[offset..offset + data.len()].copy_from_slice(&data);
            let f = instance
                .get_typed_func::<(i32, i32), i32>(&store, "lir_main")
                .expect("lir_main");
            f.call(&mut store, (offset as i32, data.len() as i32))
                .expect("call") as i128
        }
    }
}

fn run_case(src: &str, input: &[Val]) {
    if !wasm_clang_target_ok() {
        return;
    }
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let ir = emit_llvm_ir(&p).unwrap();
    let wasm = match emit_wasm(&p) {
        Ok(w) => w,
        Err(e) => {
            panic!("emit_wasm failed (install clang with wasm32): {e}");
        }
    };
    let i = interp_scalar_i128(run_program(&p, input).unwrap());
    let w = run_wasm(&ir, &wasm, input);
    assert_eq!(i, w, "interp vs wasm mismatch for:\n{src}");
}

fn run_trap_case(src: &str, input: &[Val], expect_code: &str) {
    if !wasm_clang_target_ok() {
        return;
    }
    let p = parse_program(src).unwrap();
    check_program(&p).unwrap();
    let ir = emit_llvm_ir(&p).expect("llvm emit");
    let wasm = emit_wasm(&p).expect("wasm emit");
    let err = run_program(&p, input).unwrap_err();
    match &err {
        lir::LirError::Runtime { code, .. } => assert_eq!(*code, expect_code),
        _ => panic!("expected runtime {expect_code}, got {err:?}"),
    }
    let (ret_ty, ptr_el) = classify_ir(&ir);
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm).unwrap();
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker.instantiate_and_start(&mut store, &module).unwrap();
    let trapped = match (ptr_el, ret_ty) {
        (None, RetTy::I32) => instance
            .get_typed_func::<(), i32>(&store, "lir_main")
            .unwrap()
            .call(&mut store, ())
            .is_err(),
        (None, RetTy::I64) => instance
            .get_typed_func::<(), i64>(&store, "lir_main")
            .unwrap()
            .call(&mut store, ())
            .is_err(),
        (Some(PtrElem::I32), RetTy::I32) => {
            let data: Vec<i32> = input.iter().map(|v| match v {
                Val::I32(x) => *x,
                _ => panic!(),
            }).collect();
            let mem = instance.get_memory(&store, "memory").unwrap();
            let offset = 1024usize;
            let need = offset + 4 * data.len().max(1);
            while mem.data_size(&store) < need {
                mem.grow(&mut store, 1).unwrap();
            }
            if !data.is_empty() {
                let m = mem.data_mut(&mut store);
                for (i, v) in data.iter().enumerate() {
                    m[offset + i * 4..offset + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
                }
            }
            instance
                .get_typed_func::<(i32, i32), i32>(&store, "lir_main")
                .unwrap()
                .call(&mut store, (offset as i32, data.len() as i32))
                .is_err()
        }
        (Some(PtrElem::I32), RetTy::I64) => {
            let data: Vec<i32> = input.iter().map(|v| match v {
                Val::I32(x) => *x,
                _ => panic!(),
            }).collect();
            let mem = instance.get_memory(&store, "memory").unwrap();
            let offset = 1024usize;
            let need = offset + 4 * data.len().max(1);
            while mem.data_size(&store) < need {
                mem.grow(&mut store, 1).unwrap();
            }
            if !data.is_empty() {
                let m = mem.data_mut(&mut store);
                for (i, v) in data.iter().enumerate() {
                    m[offset + i * 4..offset + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
                }
            }
            instance
                .get_typed_func::<(i32, i32), i64>(&store, "lir_main")
                .unwrap()
                .call(&mut store, (offset as i32, data.len() as i32))
                .is_err()
        }
        (Some(PtrElem::I64), RetTy::I32) => {
            let data: Vec<i64> = input.iter().map(|v| match v {
                Val::I64(x) => *x,
                _ => panic!(),
            }).collect();
            let mem = instance.get_memory(&store, "memory").unwrap();
            let offset = 1024usize;
            let need = offset + 8 * data.len().max(1);
            while mem.data_size(&store) < need {
                mem.grow(&mut store, 1).unwrap();
            }
            if !data.is_empty() {
                let m = mem.data_mut(&mut store);
                for (i, v) in data.iter().enumerate() {
                    m[offset + i * 8..offset + i * 8 + 8].copy_from_slice(&v.to_le_bytes());
                }
            }
            instance
                .get_typed_func::<(i32, i32), i32>(&store, "lir_main")
                .unwrap()
                .call(&mut store, (offset as i32, data.len() as i32))
                .is_err()
        }
        (Some(PtrElem::I64), RetTy::I64) => {
            let data: Vec<i64> = input.iter().map(|v| match v {
                Val::I64(x) => *x,
                _ => panic!(),
            }).collect();
            let mem = instance.get_memory(&store, "memory").unwrap();
            let offset = 1024usize;
            let need = offset + 8 * data.len().max(1);
            while mem.data_size(&store) < need {
                mem.grow(&mut store, 1).unwrap();
            }
            if !data.is_empty() {
                let m = mem.data_mut(&mut store);
                for (i, v) in data.iter().enumerate() {
                    m[offset + i * 8..offset + i * 8 + 8].copy_from_slice(&v.to_le_bytes());
                }
            }
            instance
                .get_typed_func::<(i32, i32), i64>(&store, "lir_main")
                .unwrap()
                .call(&mut store, (offset as i32, data.len() as i32))
                .is_err()
        }
        (Some(PtrElem::I8), _) => {
            let data: Vec<u8> = input.iter().map(|v| match v {
                Val::Bool(b) => *b as u8,
                _ => panic!(),
            }).collect();
            let mem = instance.get_memory(&store, "memory").unwrap();
            let offset = 1024usize;
            let need = offset + data.len().max(1);
            while mem.data_size(&store) < need {
                mem.grow(&mut store, 1).unwrap();
            }
            if !data.is_empty() {
                mem.data_mut(&mut store)[offset..offset + data.len()].copy_from_slice(&data);
            }
            instance
                .get_typed_func::<(i32, i32), i32>(&store, "lir_main")
                .unwrap()
                .call(&mut store, (offset as i32, data.len() as i32))
                .is_err()
        }
    };
    assert!(trapped, "expected wasm trap for:\n{src}");
}

#[test]
fn wasm_void_matches_interp_batch() {
    for src in [
        "lir/1\nrange(0,5) | reduce sum",
        "lir/1\nrange(0,5) | reduce count",
        "lir/1\nrange(1,4) | reduce prod",
        "lir/1\nrange(1,4) | reduce min",
        "lir/1\nrange(1,4) | reduce max",
        "lir/1\nrange(0,3) | scan 0, add | reduce sum",
        "lir/1\nlit(true, false, true) | filter eq true | reduce count",
    ] {
        run_case(src, &[]);
    }
}

#[test]
fn wasm_i64_lit_matches_interp() {
    run_case(
        "lir/1\nlit(3000000000, 3000000000) | reduce sum",
        &[],
    );
}

#[test]
fn wasm_input_i32_matches_interp() {
    run_case(
        "lir/1\ninput:i32 | reduce sum",
        &[Val::I32(10), Val::I32(20), Val::I32(30)],
    );
}

#[test]
fn wasm_input_i64_matches_interp() {
    run_case(
        "lir/1\ninput:i64 | reduce count",
        &[Val::I64(9), Val::I64(8), Val::I64(7)],
    );
    run_case(
        "lir/1\ninput:i64 | map . add 100 | reduce prod",
        &[Val::I64(2), Val::I64(3)],
    );
}

#[test]
fn wasm_input_bool_matches_interp() {
    run_case(
        "lir/1\ninput:bool | filter eq true | reduce count",
        &[Val::Bool(true), Val::Bool(false), Val::Bool(true), Val::Bool(true)],
    );
}

#[test]
fn wasm_traps_match_interp_codes() {
    run_trap_case("lir/1\nrange(0,0) | reduce min", &[], "R_REDUCE_EMPTY_MINMAX");
    run_trap_case(
        "lir/1\nlit(2000000000, 2000000000) | reduce sum",
        &[],
        "R_INTEGER_OVERFLOW",
    );
    run_trap_case(
        "lir/1\nlit(1) | map . div 0 | reduce sum",
        &[],
        "R_DIV_BY_ZERO",
    );
    run_trap_case(
        "lir/1\ninput:i64 | reduce sum",
        &[Val::I64(i64::MAX), Val::I64(1)],
        "R_INTEGER_OVERFLOW",
    );
}
