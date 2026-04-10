# LYTR monorepo

**LYTR** is the planned **general-purpose language** built on **LIR**, a **fast data-processing** DSL (`lir/1`). See **[docs/NAMING.md](docs/NAMING.md)** for terminology.

## Current implementation (LIR tier)

- **Spec:** [docs/LIR_V1_SPEC.md](docs/LIR_V1_SPEC.md)
- **CLI (LIR):** `cargo run --bin lir -- --help` — `check`, `run`, `fmt`, `fmt --check`, `codegen-check`, `dump-ast`, `apply-ast`, `compile`, `wasm` (JSON AST: [docs/LIR_AST_JSON.md](docs/LIR_AST_JSON.md))
- **CLI (LYTR 0.1 bootstrap):** `cargo run --bin lytr -- --help` — `check`, `run` on `let` / `if` / `Result` / `match` ([`docs/PHASE5_BOOTSTRAP.md`](docs/PHASE5_BOOTSTRAP.md); [`examples/minimal.lytr`](examples/minimal.lytr), [`examples/let_if.lytr`](examples/let_if.lytr), [`examples/match.lytr`](examples/match.lytr))
- **Tests:** `cargo test` (LLVM/WASM goldens need suitable `clang` on Linux CI)

## Roadmap & production

| Doc | Purpose |
|-----|---------|
| [docs/LYTR_GLOBAL_IMPLEMENTATION_PLAN.md](docs/LYTR_GLOBAL_IMPLEMENTATION_PLAN.md) | Full phased plan (0–10 + **§11 P11–P16** + **§13 ordered backlog**) |
| [docs/LYTR_PRODUCTION_READINESS.md](docs/LYTR_PRODUCTION_READINESS.md) | **GA gates G1–G12** and release checklist |
| [docs/LYTR_GOALS_AND_TIERS.md](docs/LYTR_GOALS_AND_TIERS.md) | Agent-optimal vs production-ecosystem products |

**Agents:** [AGENTS.md](AGENTS.md)

## Tier A eval (LIR)

[eval/README.md](eval/README.md) — `python3 eval/run_tier_a.py` (LIR manifest + optional `tasks/*/hidden/assertions.json`); **`python3 eval/run_lytr_tier.py`** (LYTR numeric parity vs the same task ids); CI also runs Python baselines and `run_llm_eval.py --dry-run`. **`python3 eval/run_comparison.py`** runs Tier A + Python baseline + LYTR tier together (optional `--llm`); see [eval/BASELINE.md](eval/BASELINE.md). Measures agent-relevant surface **before** full LYTR GP.
