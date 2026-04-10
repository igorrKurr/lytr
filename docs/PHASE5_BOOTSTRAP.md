# Phase 5 implementation — bootstrap

The **Phase 5 exit criterion** — *tiny LYTR programs parse, check, run* — is met for a **minimal** subset by the **`lytr`** binary and `lir::lytr` module.

## Done (this repo)

- **`lytr` CLI:** `lytr check <file.lytr>`, `lytr run <file.lytr>` (see `cargo run --bin lytr -- --help`).
- **Language:** first line `lytr/0.1`, then `fn main() -> i32 { … }` with:
  - **`let`** (`let x: i32 = …;` or inferred `let x = …;`), types `i32`, `bool`, `Result<i32, i32>`;
  - **Arithmetic / compare:** `+ - * / %`, `== != < > <= >=`, unary `-` on literals;
  - **`if`** expression: `if cond { e1 } else { e2 }` (condition must be `bool`);
  - **`Ok` / `Err`** (i32 payloads only) and **`match`** with required `Ok(x) => …` and `Err(y) => …` arms;
  - **`return`** must be the **last** statement; only **`let`** may appear before it.
- **Library API:** `parse_lytr_program`, `check_lytr_program`, `run_lytr_program` ([`src/lytr/`](../src/lytr/)).
- **Examples:** [`examples/minimal.lytr`](../examples/minimal.lytr), [`examples/let_if.lytr`](../examples/let_if.lytr), [`examples/match.lytr`](../examples/match.lytr); tests in [`tests/lytr_bootstrap.rs`](../tests/lytr_bootstrap.rs).

## Next (expand LYTR 0.1)

1. `let`, `if`, blocks with multiple statements.
2. `Result` / `match` per [LYTR_EFFECTS_AND_FFI_DRAFT.md](LYTR_EFFECTS_AND_FFI_DRAFT.md).
3. LIR embed surface and LYIR lowering per [LYTR_LOWERING_SKETCH.md](LYTR_LOWERING_SKETCH.md).

**Paper track:** [LYTR_CHARTER_DRAFT.md](LYTR_CHARTER_DRAFT.md), [LYTR_CORE_CALCULUS_DRAFT.md](LYTR_CORE_CALCULUS_DRAFT.md), [LYTR_EFFECTS_AND_FFI_DRAFT.md](LYTR_EFFECTS_AND_FFI_DRAFT.md), [LYTR_LOWERING_SKETCH.md](LYTR_LOWERING_SKETCH.md).
