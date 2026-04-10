# Agent notes (LYTR monorepo)

**Naming:** **LIR** = fast data-processing language (`lir/1`). **LYTR** = planned general-purpose layer on top. See `docs/NAMING.md`.

## After editing `.lir` files

1. Run **`cargo run --bin lir -- check <file.lir>`** (or `lir check` if on `PATH`).
2. If the program should compile, run **`lir codegen-check <file.lir>`** — only a **subset** of valid LIR lowers to LLVM/WASM (`docs/codegen_subset.json`, `docs/LLVM_ABI.md`).
3. Run **`lir fmt --check <file.lir>`** before commit when touching sources (canonical §11 form).
4. Optional: **`lir dump-ast <file.lir>`** prints JSON AST; **`lir apply-ast <file.json>`** round-trips to canonical text (see `docs/LIR_AST_JSON.md`).
5. **LYTR 0.1 bootstrap:** **`lytr check`** / **`lytr run`** on `lytr/0.1` + `fn main() -> i32 { return expr; }` — see `docs/PHASE5_BOOTSTRAP.md` and `examples/minimal.lytr`.

## Errors

- Language diagnostics include a **JSON line** after the human message (`kind`: `syntax` | `type` | `runtime`).
- CLI usage errors use **`kind`: `cli`** (`cli_json_line` in Rust).

## Tests

- `cargo test` — includes interpreter, LLVM golden (needs `clang`), WASM golden (needs wasm `clang`, skips otherwise).
- Do not assume macOS Xcode `clang` can link wasm; CI uses Ubuntu + `clang` + `lld`.

## Roadmap

- Global plan: `docs/LYTR_GLOBAL_IMPLEMENTATION_PLAN.md`.
- **Production / GA criteria:** `docs/LYTR_PRODUCTION_READINESS.md` (gates G1–G12).
- Product tiers (agent vs ecosystem): `docs/LYTR_GOALS_AND_TIERS.md`.
- LIR interp vs native subset: `docs/LIR_PRODUCT_STRATEGY.md`.
- **Tier A eval (run after LIR changes):** `python3 eval/run_tier_a.py` (see `eval/manifest.json`; optional `eval/tasks/<id>/hidden/assertions.json` merged after manifest). Optional: `python3 eval/baseline/python/run_all.py` (Python outcomes 001–008, 012–020); `python3 eval/run_llm_eval.py --dry-run` or with `OPENAI_API_KEY` for live LLM grading (`eval/README.md`). For a **single local comparison** (Tier A + baseline ± LLM, timings): `python3 eval/run_comparison.py` (`eval/BASELINE.md`). **A/B pilot** (LLM→LIR vs LLM→Python on four tasks): `eval/pilot/README.md`, `python3 eval/run_pilot_ab.py`.
