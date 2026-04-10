# Implementation status (vs [LYTR_GLOBAL_IMPLEMENTATION_PLAN.md](LYTR_GLOBAL_IMPLEMENTATION_PLAN.md))

This file is a **snapshot** of what is implemented in the repository versus what remains on the multi-quarter roadmap. It is **not** a promise that all phases will be “finished” in a single release.

| Phase | Scope | Status |
|-------|--------|--------|
| **0** | LIR semantic truth, codegen subset JSON, goldens | **Done** (LIR) |
| **1** | CLI JSON lines, `fmt --check`, agent docs | **Done** |
| **2** | LIR AST JSON, `dump-ast` / `apply-ast` | **Done** |
| **3** | Tier A eval, LLM harness, pilot, regression | **Done** (continuous) |
| **4** | Evidence / experiments | **Ongoing**; pilot conclusion: [EXPERIMENT_PILOT_LLM_AB.md](EXPERIMENT_PILOT_LLM_AB.md) |
| **5** | LYTR charter + calculus + **bootstrap `lytr`** | **Papers + minimal `lytr` done**; full language **not** done |
| **6** | Memory model (C1), prototypes | **Not implemented** (draft only) |
| **7** | Concurrency v1 | **Not implemented** |
| **8** | LYTR LLVM backend + LIR embed | **Not implemented** |
| **9** | Incremental compilation | **Not implemented** |
| **10** | JIT / SIMD / advanced opt | **Not implemented** |
| **GA** | [LYTR_PRODUCTION_READINESS.md](LYTR_PRODUCTION_READINESS.md) gates G1–G12 | **Open** |

**Bottom line:** LIR tooling and Tier A eval are strong; **LYTR** has a **bootstrap** interpreter only. Everything from **memory model through GA** is still ahead.
