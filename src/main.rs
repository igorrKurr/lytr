use std::env;
use std::fs;
use std::process::ExitCode;

use lir::interp::{RunOutcome, Val};
use lir::ast::ElemTy;
use lir::{
    check_program, cli_json_line, codegen_supported, deserialize_lir_ast_document, emit_llvm_ir,
    emit_wasm, format_program, parse_input_array, parse_program, program_is_canonical_text,
    run_program, serialize_lir_ast_document, source_stream_ty,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(cmd) = args.first().map(String::as_str) else {
        print_usage_overview();
        eprintln!(
            "{}",
            cli_json_line("E_CLI_ARGS", "missing command; use `lir help` for usage")
        );
        return ExitCode::from(2);
    };

    if matches!(cmd, "help" | "-h" | "--help") {
        match args.get(1).map(String::as_str) {
            None => print_usage_overview(),
            Some("compile") => print_compile_help(),
            Some("wasm") => print_wasm_help(),
            Some("run") => eprintln!(
                "lir run <file.lir> [--input '[...]']\n\n\
Runs with the reference interpreter. Omit --input for an empty stream.\n\
Elements are parsed to match the source: input:i32 (integers in i32 range), \
input:i64 (decimal integers as i64), input:bool (true/false)."
            ),
            Some("check") => eprintln!(
                "lir check <file.lir>\n\nParse and type-check only; prints `ok` on success."
            ),
            Some("fmt") => eprintln!(
                "lir fmt [--check] <file.lir>\n\nPrint canonical formatting (§11) to stdout, or with --check verify file is already canonical (exit 1 if not)."
            ),
            Some("codegen-check") => eprintln!(
                "lir codegen-check <file.lir>\n\nParse, type-check, and verify the program is in the LLVM/WASM codegen subset (see docs/codegen_subset.json)."
            ),
            Some("dump-ast") => eprintln!(
                "lir dump-ast <file.lir>\n\nParse LIR text and print JSON AST (schema_version + program). See docs/LIR_AST_JSON.md."
            ),
            Some("apply-ast") => eprintln!(
                "lir apply-ast <file.json>\n\nLoad JSON AST, type-check, print canonical §11 source to stdout."
            ),
            Some(topic) => eprintln!("No detailed help for `{topic}`. Try `lir help` or `lir help compile`."),
        }
        return ExitCode::SUCCESS;
    }

    match cmd {
        "fmt" => {
            let (check_only, path) = match args.get(1).map(String::as_str) {
                Some("--check") => (
                    true,
                    match args.get(2) {
                        Some(p) => p,
                        None => {
                            eprintln!("usage: lir fmt [--check] <file.lir>");
                            eprintln!("{}", cli_json_line("E_CLI_ARGS", "missing file path"));
                            return ExitCode::from(2);
                        }
                    },
                ),
                Some(_) => (false, args.get(1).unwrap()),
                None => {
                    eprintln!("usage: lir fmt [--check] <file.lir>");
                    eprintln!("{}", cli_json_line("E_CLI_ARGS", "missing file path"));
                    return ExitCode::from(2);
                }
            };
            let src = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("read {}: {e}", path);
                    eprintln!(
                        "{}",
                        cli_json_line("E_IO", &format!("read {path}: {e}"))
                    );
                    return ExitCode::from(1);
                }
            };
            let prog = match parse_program(&src) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("{}", e.to_json_line());
                    return ExitCode::from(1);
                }
            };
            if check_only {
                if program_is_canonical_text(&src, &prog) {
                    println!("ok");
                    ExitCode::SUCCESS
                } else {
                    eprintln!("not formatted (run `lir fmt` without --check to print canonical source)");
                    eprintln!(
                        "{}",
                        cli_json_line(
                            "E_FMT_CHECK",
                            "source differs from canonical §11 formatting",
                        )
                    );
                    return ExitCode::from(1);
                }
            } else {
                print!("{}", format_program(&prog));
                ExitCode::SUCCESS
            }
        }
        "dump-ast" => {
            let Some(path) = args.get(1) else {
                eprintln!("usage: lir dump-ast <file.lir>");
                eprintln!("{}", cli_json_line("E_CLI_ARGS", "missing file path"));
                return ExitCode::from(2);
            };
            let src = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("read {}: {e}", path);
                    eprintln!(
                        "{}",
                        cli_json_line("E_IO", &format!("read {path}: {e}"))
                    );
                    return ExitCode::from(1);
                }
            };
            let prog = match parse_program(&src) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("{}", e.to_json_line());
                    return ExitCode::from(1);
                }
            };
            let json = match serialize_lir_ast_document(&prog) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("failed to serialize AST: {e}");
                    eprintln!(
                        "{}",
                        cli_json_line("E_AST_JSON", &format!("json serialize: {e}"))
                    );
                    return ExitCode::from(1);
                }
            };
            println!("{}", json);
            ExitCode::SUCCESS
        }
        "apply-ast" => {
            let Some(path) = args.get(1) else {
                eprintln!("usage: lir apply-ast <file.json>");
                eprintln!("{}", cli_json_line("E_CLI_ARGS", "missing file path"));
                return ExitCode::from(2);
            };
            let raw = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("read {}: {e}", path);
                    eprintln!(
                        "{}",
                        cli_json_line("E_IO", &format!("read {path}: {e}"))
                    );
                    return ExitCode::from(1);
                }
            };
            let prog = match deserialize_lir_ast_document(&raw) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("{}", cli_json_line("E_AST_JSON", &e.to_string()));
                    return ExitCode::from(1);
                }
            };
            if let Err(e) = check_program(&prog) {
                eprintln!("{e}");
                eprintln!("{}", e.to_json_line());
                return ExitCode::from(1);
            }
            print!("{}", format_program(&prog));
            ExitCode::SUCCESS
        }
        "codegen-check" => {
            let Some(path) = args.get(1) else {
                eprintln!("usage: lir codegen-check <file.lir>");
                eprintln!("{}", cli_json_line("E_CLI_ARGS", "missing file path"));
                return ExitCode::from(2);
            };
            let src = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("read {}: {e}", path);
                    eprintln!(
                        "{}",
                        cli_json_line("E_IO", &format!("read {path}: {e}"))
                    );
                    return ExitCode::from(1);
                }
            };
            let prog = match parse_program(&src) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("{}", e.to_json_line());
                    return ExitCode::from(1);
                }
            };
            if let Err(e) = check_program(&prog) {
                eprintln!("{e}");
                eprintln!("{}", e.to_json_line());
                return ExitCode::from(1);
            }
            if let Err(e) = codegen_supported(&prog) {
                eprintln!("{e}");
                eprintln!("{}", e.to_json_line());
                return ExitCode::from(1);
            }
            println!("ok");
            ExitCode::SUCCESS
        }
        "wasm" => {
            let rest = &args[1..];
            let (path, out_path) = match parse_wasm_rest(rest) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("{}", cli_json_line("E_CLI_ARGS", &e));
                    return ExitCode::from(2);
                }
            };
            let src = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("read {}: {e}", path);
                    eprintln!(
                        "{}",
                        cli_json_line("E_IO", &format!("read {path}: {e}"))
                    );
                    return ExitCode::from(1);
                }
            };
            let prog = match parse_program(&src) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("{}", e.to_json_line());
                    return ExitCode::from(1);
                }
            };
            if let Err(e) = check_program(&prog) {
                eprintln!("{e}");
                eprintln!("{}", e.to_json_line());
                return ExitCode::from(1);
            }
            let bytes = match emit_wasm(&prog) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("{}", e.to_json_line());
                    return ExitCode::from(1);
                }
            };
            if let Err(e) = fs::write(&out_path, bytes) {
                eprintln!("write {out_path}: {e}");
                eprintln!(
                    "{}",
                    cli_json_line("E_IO", &format!("write {out_path}: {e}"))
                );
                return ExitCode::from(1);
            }
            println!("wrote {}", out_path);
            ExitCode::SUCCESS
        }
        "compile" => {
            let rest = &args[1..];
            let (path, out_path) = match parse_compile_rest(rest) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("{}", cli_json_line("E_CLI_ARGS", &e));
                    return ExitCode::from(2);
                }
            };
            let src = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("read {}: {e}", path);
                    eprintln!(
                        "{}",
                        cli_json_line("E_IO", &format!("read {path}: {e}"))
                    );
                    return ExitCode::from(1);
                }
            };
            let prog = match parse_program(&src) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("{}", e.to_json_line());
                    return ExitCode::from(1);
                }
            };
            if let Err(e) = check_program(&prog) {
                eprintln!("{e}");
                eprintln!("{}", e.to_json_line());
                return ExitCode::from(1);
            }
            let ir = match emit_llvm_ir(&prog) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("{}", e.to_json_line());
                    return ExitCode::from(1);
                }
            };
            if let Err(e) = fs::write(&out_path, ir) {
                eprintln!("write {out_path}: {e}");
                eprintln!(
                    "{}",
                    cli_json_line("E_IO", &format!("write {out_path}: {e}"))
                );
                return ExitCode::from(1);
            }
            println!("wrote {}", out_path);
            ExitCode::SUCCESS
        }
        "check" | "run" => {
            let Some(path) = args.get(1) else {
                eprintln!("missing file path");
                eprintln!("{}", cli_json_line("E_CLI_ARGS", "missing file path"));
                return ExitCode::from(2);
            };
            let src = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("read {}: {e}", path);
                    eprintln!(
                        "{}",
                        cli_json_line("E_IO", &format!("read {path}: {e}"))
                    );
                    return ExitCode::from(1);
                }
            };
            let prog = match parse_program(&src) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("{}", e.to_json_line());
                    return ExitCode::from(1);
                }
            };
            match cmd {
                "check" => {
                    if let Err(e) = check_program(&prog) {
                        eprintln!("{e}");
                        eprintln!("{}", e.to_json_line());
                        return ExitCode::from(1);
                    }
                    println!("ok");
                    ExitCode::SUCCESS
                }
                "run" => {
                    if let Err(e) = check_program(&prog) {
                        eprintln!("{e}");
                        eprintln!("{}", e.to_json_line());
                        return ExitCode::from(1);
                    }
                    let input_ty = match source_stream_ty(&prog.source) {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("{e}");
                            eprintln!("{}", e.to_json_line());
                            return ExitCode::from(1);
                        }
                    };
                    let mut run_args = args.iter().skip(2).cloned();
                    let input = match parse_input_args(&mut run_args, input_ty) {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("{e}");
                            eprintln!("{}", cli_json_line("E_CLI_ARGS", &e));
                            return ExitCode::from(2);
                        }
                    };
                    match run_program(&prog, &input) {
                        Ok(RunOutcome::Stream(vs)) => {
                            println!("{}", vals_to_json(&vs));
                            ExitCode::SUCCESS
                        }
                        Ok(RunOutcome::Scalar(v)) => {
                            println!("{}", val_to_json(&v));
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("{e}");
                            eprintln!("{}", e.to_json_line());
                            ExitCode::from(1)
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        _ => {
            eprintln!("unknown command `{cmd}` (try `lir help`)");
            eprintln!(
                "{}",
                cli_json_line("E_CLI_ARGS", &format!("unknown command `{cmd}`"))
            );
            ExitCode::from(2)
        }
    }
}

fn print_usage_overview() {
    eprintln!(
        "\
lir — LIR toolchain (fast data-processing language: interpreter, LLVM IR, WASM)

Commands:
  check <file>           Parse and type-check
  codegen-check <file>   Type-check + in LLVM/WASM codegen subset (see docs/codegen_subset.json)
  fmt [--check] <file>   Canonical source (§11); --check verifies formatting
  dump-ast <file.lir>    Parse and print JSON AST (versioned envelope)
  apply-ast <file.json>  Load JSON AST, check, print canonical source
  run <file> [--input …] Run with the interpreter
  compile -o <out.ll> <file>
                         Emit LLVM IR (text) for supported programs
  wasm -o <out.wasm> <file>
                         Emit WebAssembly (clang wasm32; see docs/WASM_ABI.md)
  help [command]         Show this overview or notes for a command

Examples:
  lir check examples/foo.lir
  lir codegen-check examples/foo.lir
  lir fmt examples/foo.lir
  lir fmt --check examples/foo.lir
  lir run examples/foo.lir --input '[1, 2, 3]'
  lir compile -o out.ll examples/foo.lir
  lir wasm -o out.wasm examples/foo.lir
  lir help compile"
    );
}

fn print_compile_help() {
    eprintln!(
        "\
lir compile — emit LLVM IR

Syntax (order of flags is flexible):
  lir compile -o <out.ll> <file.lir>
  lir compile <file.lir> -o <out.ll>

Requirements:
  - Exactly one input .lir file
  - -o <path> is required (output LLVM assembly)

The emitter supports a fused subset (i32/i64/bool streams, take/drop prefixes,
filter/map/scan stages in supported orders, reduce sum|prod|count|min|max).
Programs outside that subset fail with a typed diagnostic. Use
`lir codegen-check <file.lir>` to test membership without writing IR; see
docs/codegen_subset.json and docs/LLVM_ABI.md#codegen-subset.

Environment:
  LIR_LLVM_TRIPLE   If set, overrides the emitted LLVM `target triple` (default is
                    unknown-unknown-unknown). Use your host triple for easier AOT with clang."
    );
}

fn print_wasm_help() {
    eprintln!(
        "\
lir wasm — emit WebAssembly via clang (wasm32-unknown-unknown)

Syntax:
  lir wasm -o <out.wasm> <file.lir>
  lir wasm <file.lir> -o <out.wasm>

Requires clang with the WebAssembly target (often missing on macOS Xcode clang).
Set LIR_CLANG or WASI_SDK_PATH if needed. See docs/WASM_ABI.md."
    );
}

fn parse_wasm_rest(rest: &[String]) -> Result<(String, String), String> {
    let mut out: Option<String> = None;
    let mut files: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < rest.len() {
        if rest[i] == "-o" {
            let p = rest
                .get(i + 1)
                .ok_or_else(|| "wasm: -o needs a path".to_string())?;
            if p.is_empty() {
                return Err("wasm: empty path after -o".into());
            }
            out = Some(p.clone());
            i += 2;
        } else if rest[i].starts_with('-') {
            return Err(format!("wasm: unknown flag `{}`", rest[i]));
        } else {
            files.push(&rest[i]);
            i += 1;
        }
    }
    if files.len() != 1 {
        return Err("wasm expects exactly one source file (see `lir help wasm`)".into());
    }
    let out_path = out.ok_or_else(|| "wasm requires -o <out.wasm>".to_string())?;
    Ok((files[0].to_string(), out_path))
}

fn parse_compile_rest(rest: &[String]) -> Result<(String, String), String> {
    let mut out: Option<String> = None;
    let mut files: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < rest.len() {
        if rest[i] == "-o" {
            let p = rest
                .get(i + 1)
                .ok_or_else(|| "compile: -o needs a path".to_string())?;
            if p.is_empty() {
                return Err("compile: empty path after -o".into());
            }
            out = Some(p.clone());
            i += 2;
        } else if rest[i].starts_with('-') {
            return Err(format!("compile: unknown flag `{}`", rest[i]));
        } else {
            files.push(&rest[i]);
            i += 1;
        }
    }
    if files.len() != 1 {
        return Err("compile expects exactly one source file (see `lir help compile`)".into());
    }
    let out_path = out.ok_or_else(|| {
        "compile requires -o <out.ll> (see `lir help compile`)".to_string()
    })?;
    Ok((files[0].to_string(), out_path))
}

fn parse_input_args(
    args: &mut impl Iterator<Item = String>,
    input_ty: ElemTy,
) -> Result<Vec<Val>, String> {
    match args.next() {
        Some(flag) if flag == "--input" => {
            let raw = args
                .next()
                .ok_or_else(|| "--input needs an array argument".to_string())?;
            parse_input_array(&raw, input_ty)
        }
        None => Ok(Vec::new()),
        Some(x) => Err(format!("unexpected argument `{x}`; use --input '[...]'")),
    }
}

fn vals_to_json(vs: &[Val]) -> String {
    let mut s = String::from("[");
    for (i, v) in vs.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&val_to_json(v));
    }
    s.push(']');
    s
}

fn val_to_json(v: &Val) -> String {
    match v {
        Val::I32(x) => x.to_string(),
        Val::I64(x) => x.to_string(),
        Val::Bool(b) => b.to_string(),
    }
}
