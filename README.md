# LYTR monorepo

**LYTR** is the planned **general-purpose language** built on **LIR**, a **fast data-processing** DSL (`lir/1`). See **[docs/NAMING.md](docs/NAMING.md)** for terminology.

## Current implementation (LIR tier)

- **Spec:** [docs/LIR_V1_SPEC.md](docs/LIR_V1_SPEC.md)
- **CLI:** `cargo run --bin lir -- --help` — `check`, `run`, `fmt`, `fmt --check`, `codegen-check`, `compile`, `wasm`
- **Tests:** `cargo test` (LLVM/WASM goldens need suitable `clang` on Linux CI)

## Roadmap & production

| Doc | Purpose |
|-----|---------|
| [docs/LYTR_GLOBAL_IMPLEMENTATION_PLAN.md](docs/LYTR_GLOBAL_IMPLEMENTATION_PLAN.md) | Full phased plan (0–10 + **§11 P11–P16** + **§13 ordered backlog**) |
| [docs/LYTR_PRODUCTION_READINESS.md](docs/LYTR_PRODUCTION_READINESS.md) | **GA gates G1–G12** and release checklist |
| [docs/LYTR_GOALS_AND_TIERS.md](docs/LYTR_GOALS_AND_TIERS.md) | Agent-optimal vs production-ecosystem products |

**Agents:** [AGENTS.md](AGENTS.md)

## Eval skeleton

[eval/README.md](eval/README.md) — local smoke + future LLM runner.
