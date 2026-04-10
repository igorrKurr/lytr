# Phase 5 implementation — bootstrap

The **Phase 5 exit criterion** — *tiny LYTR programs parse, check, run* — is met for a **minimal** subset by the **`lytr`** binary and `lir::lytr` module.

## Bootstrap surface vs LYTR 0.1 (product)

The current **`lytr`** subset uses **familiar tokens** (`let`, `if`/`else`, `Result`, `Ok`/`Err`, `match`, …) mainly for **implementation velocity** and a straight path from the [core calculus draft](LYTR_CORE_CALCULUS_DRAFT.md) to a working interpreter. That resemblance to Rust (or to common ML-family syntax) is **not** a commitment to Rust parity or to “whatever reads familiar” as the final design.

**What *is* committed for LYTR 0.1** is spelled out in the charter and global plan: precise semantics, LIR embed, and **LLM-first tooling** — not a particular brace-and-keyword aesthetic.

**Claims that LYTR is “more LLM-efficient” than other languages** must rest on **normative spec + toolchain behavior + empirical measurement**, not on the bootstrap alone. Until those are in place, treat this repo’s surface syntax as **provisional scaffolding**.

## Plan: LLM-efficiency as an engineering outcome

“Top LLM-efficient” means the full chain **user → model → program → run** scores better than baselines on **defined metrics**, not that the grammar looks novel. A concrete plan:

1. **Freeze metrics** (see [LYTR_GLOBAL_IMPLEMENTATION_PLAN.md](LYTR_GLOBAL_IMPLEMENTATION_PLAN.md) §goals): primarily **pipeline cost** — prompt + completion tokens, including **retries and repair turns**, until a task passes; secondary: wall time, parse/check failure rate, silent-wrong rate where measurable.

2. **Maintain frozen task sets and baselines** — the repo already runs Tier A, Python baselines, and pilot-style A/B harnesses under [`eval/`](../eval/README.md). LYTR must be added to the **same kind** of protocol: same tasks, same grading, comparable prompts.

3. **Design rules for the *language*** that reduce model error *by construction*: small stable grammar; **unambiguous** parsing; **canonical** formatting (`fmt` / `fmt --check`); **explicit** control and effects where implicit state would cause hallucinated APIs; bounded iteration and clear `Result`/error story so agents do not invent semantics.

4. **Design rules for the *toolchain***: machine-readable diagnostics (JSON lines or stable IDs); stable “verbs” for agents; optional structured IR for tools — the charter’s **LLM-first** row.

5. **Iteration loop:** any change to surface syntax or stdlib surface → **re-run** the harness against frozen baselines → accept only if metrics move the right way (or tradeoffs are documented). Surface changes that help humans but hurt token-to-success should fail this bar unless compensated.

The bootstrap in this file is **step 0** of that loop: a runnable core so semantics and tooling can grow **with** measurement, not after.

## Done (this repo)

- **`lytr` CLI:** `lytr check <file.lytr>`, `lytr run <file.lytr>` (see `cargo run --bin lytr -- --help`).
- **Language:** first line `lytr/0.1`, then `fn main() -> i32` or `-> i64 { … }` with:
  - **`let`** (`let x: i32 = …;` or inferred `let x = …;`), types `i32`, `i64`, `bool`, `Result<i32, i32>` / `Result<i64, i64>` (integer width matches `main`);
  - **Arithmetic / compare:** `+ - * / %`, `== != < > <= >=`, unary `-` on literals;
  - **`if`** expression: `if cond { e1 } else { e2 }` (condition must be `bool`);
  - **`Ok` / `Err`** (i32 payloads only) and **`match`** with required `Ok(x) => …` and `Err(y) => …` arms;
  - **`return`** must be the **last** statement; only **`let`** may appear before it.
- **Library API:** `parse_lytr_program`, `check_lytr_program`, `run_lytr_program` ([`src/lytr/`](../src/lytr/)).
- **Examples:** [`examples/minimal.lytr`](../examples/minimal.lytr), [`examples/let_if.lytr`](../examples/let_if.lytr), [`examples/match.lytr`](../examples/match.lytr); tests in [`tests/lytr_bootstrap.rs`](../tests/lytr_bootstrap.rs).

## Next (expand LYTR 0.1)

1. Blocks with **multiple** statements before `return`; richer `match` / exhaustiveness as in calculus drafts.
2. Align `Result` and effects with [LYTR_EFFECTS_AND_FFI_DRAFT.md](LYTR_EFFECTS_AND_FFI_DRAFT.md) (payload types, not only `i32`).
3. LIR embed surface and LYIR lowering per [LYTR_LOWERING_SKETCH.md](LYTR_LOWERING_SKETCH.md).
4. **Eval:** **Regression:** [`eval/run_lytr_tier.py`](../eval/run_lytr_tier.py) + [`eval/lytr_manifest.json`](../eval/lytr_manifest.json) (stdout parity with Tier A numeric baselines). **LLM:** [`eval/run_llm_lytr_eval.py`](../eval/run_llm_lytr_eval.py) (same manifest; logs `results_llm_lytr.ndjson`). **Aggregate:** [`eval/summarize_llm_tracks.py`](../eval/summarize_llm_tracks.py) compares `results_llm.ndjson` vs `results_llm_lytr.ndjson` (last run per task, shared task ids, token ratios). **Next:** optional frozen baseline JSON for LLM tracks (like pilot); widen LYTR to full 16-task LLM runs for stable A/B.

**Paper track:** [LYTR_CHARTER_DRAFT.md](LYTR_CHARTER_DRAFT.md), [LYTR_CORE_CALCULUS_DRAFT.md](LYTR_CORE_CALCULUS_DRAFT.md), [LYTR_EFFECTS_AND_FFI_DRAFT.md](LYTR_EFFECTS_AND_FFI_DRAFT.md), [LYTR_LOWERING_SKETCH.md](LYTR_LOWERING_SKETCH.md).
