//! LYTR 0.1 bootstrap CLI (`lytr check` / `lytr run`).

use std::env;
use std::fs;
use std::process::ExitCode;

use lir::{
    check_lytr_program, cli_json_line, parse_lytr_program, run_lytr_program, LytrRun,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(cmd) = args.first().map(String::as_str) else {
        print_usage();
        eprintln!(
            "{}",
            cli_json_line("E_CLI_ARGS", "missing command; use `lytr help`")
        );
        return ExitCode::from(2);
    };

    if matches!(cmd, "help" | "-h" | "--help") {
        print_usage();
        return ExitCode::SUCCESS;
    }

    match cmd {
        "check" | "run" => {
            let Some(path) = args.get(1) else {
                eprintln!("usage: lytr {cmd} <file.lytr>");
                eprintln!("{}", cli_json_line("E_CLI_ARGS", "missing file path"));
                return ExitCode::from(2);
            };
            let src = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("read {path}: {e}");
                    eprintln!(
                        "{}",
                        cli_json_line("E_IO", &format!("read {path}: {e}"))
                    );
                    return ExitCode::from(1);
                }
            };
            let prog = match parse_lytr_program(&src) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("{}", e.to_json_line());
                    return ExitCode::from(1);
                }
            };
            if let Err(e) = check_lytr_program(&prog) {
                eprintln!("{e}");
                eprintln!("{}", e.to_json_line());
                return ExitCode::from(1);
            }
            if cmd == "check" {
                println!("ok");
                return ExitCode::SUCCESS;
            }
            match run_lytr_program(&prog) {
                Ok(LytrRun::I32(v)) => {
                    println!("{v}");
                    ExitCode::SUCCESS
                }
                Ok(LytrRun::I64(v)) => {
                    println!("{v}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("{}", e.to_json_line());
                    ExitCode::from(1)
                }
            }
        }
        _ => {
            eprintln!("unknown command `{cmd}` (try `lytr help`)");
            eprintln!(
                "{}",
                cli_json_line(
                    "E_CLI_ARGS",
                    &format!("unknown command `{cmd}`"),
                )
            );
            ExitCode::from(2)
        }
    }
}

fn print_usage() {
    eprintln!(
        "\
lytr — LYTR 0.1 bootstrap (lytr/0.1 + fn main() -> i32 | i64 {{ … }})

Commands:
  check <file.lytr>   Parse, type-check
  run <file.lytr>     Parse, check, print integer result (stdout)
  help                This text

See docs/LYTR_CORE_CALCULUS_DRAFT.md and docs/PHASE5_BOOTSTRAP.md."
    );
}
