# LYTR charter (draft)

**LYTR** is the general-purpose language layered on **LIR** (see [NAMING.md](NAMING.md)). This charter is a working draft for Phase 5 of [LYTR_GLOBAL_IMPLEMENTATION_PLAN.md](LYTR_GLOBAL_IMPLEMENTATION_PLAN.md).

## Goals (v0.1 target)

- Replace **Python-class** workloads for **LLM+agent** authoring: scripts, CLI tools, glue, small services — with **precise** semantics and **fast** compiled execution where possible.
- **Embed LIR** for stream/dataflow fragments so numeric pipelines reuse existing fusion and LLVM/WASM paths.
- **LLM-first:** boring syntax, canonical formatting, machine-readable errors, short agent docs ([`AGENTS.md`](../AGENTS.md)).

## Non-goals (initial editions)

- Full compatibility with Python or Rust syntax or stdlibs.
- Self-hosting the compiler before the semantics and eval metrics are stable.
- Implicit global mutable state without an explicit effect or capability story.

## Open decisions

- Memory model: see [LYTR_MEMORY_OPTIONS_DRAFT.md](LYTR_MEMORY_OPTIONS_DRAFT.md).
- Concurrency v1: async I/O vs threads + channels (pick one first).
- Exact module / packaging story (files vs content-addressed units).

## Success criteria

- Eval harness ([`eval/`](../eval/README.md)) shows **lower pipeline cost** than baseline for a frozen task set.
- Reference tests: LYTR ↔ LIR embed paths preserve **numeric and trap** behavior vs the LIR oracle.
- **Production bar:** [LYTR_PRODUCTION_READINESS.md](LYTR_PRODUCTION_READINESS.md) GA gates (Tier B).

---

## Appendix A — v0.1 **minimal** language surface (planning)

*Normative grammar comes later; this bounds scope for semantics work.*

- Functions with value parameters; **no** user generics in v0.1 unless explicitly promoted.
- Records (named fields); tagged unions **or** small closed `enum` set.
- `if`, bounded `while`/`loop` with explicit fuel or analyzer guard (TBD).
- `match` on tags only (no deep pattern exhaustiveness proofs in v0.1).
- Single error model (`Result` *or* exceptions — global plan B2).
- **LIR embed:** one syntactic form (block, macro, or intrinsic) TBD.

---

## Appendix B — **Explicitly deferred** (not v0.1)

- User-defined **generics** / type-parameterized functions (may appear v0.2+).
- **Async** + **threads** together (global plan: one v1; second model is **edition 2+** — see [LYTR_GLOBAL_IMPLEMENTATION_PLAN.md](LYTR_GLOBAL_IMPLEMENTATION_PLAN.md) §11).
- **Operator overloading** beyond fixed set.
- **Macros** / compile-time reflection.
- Full **HKT** / advanced type-level programming.
