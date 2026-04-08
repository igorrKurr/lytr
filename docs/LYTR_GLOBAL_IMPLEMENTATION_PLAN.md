# LYTR global implementation plan

**Status:** planning document for the **LYTR** project — pipeline, **LIR** (data-processing language), **LYTR** general-purpose language, performance platform, and LLM-first tooling.

**Naming:** **LIR** = fast data-processing language (`lir/1`, this repo today). **LYTR** = general-purpose language built on LIR. See [NAMING.md](NAMING.md).

**Goal:** Maximize effectiveness of the full chain **user → LLM → program → computation → hardware**: **precision** (correct programs, few silent errors), **pipeline cost** (tokens and retries until success), and **hardware performance** (predictable, fast execution). The final *surface syntax* is secondary; **sugar/desugar** are justified only when they improve this chain.

**Non-goals:** Redesigning general-purpose programming for novelty alone; maximizing brevity of symbols at the expense of model success rate.

---

## 1. North-star metrics

Measure everything against the same yardstick. Log runs as JSON lines (eval + CI smoke).

| Metric | Definition | Used for |
|--------|------------|----------|
| **Precision** | Parser + typecheck pass rate; share of programs that pass **hidden tests** on first or N-th attempt | Correctness of the pipeline |
| **Pipeline cost** | Total **prompt + completion tokens** until task success (include **retries** and repair turns) | LLM effectiveness (not raw character count) |
| **Time to green** | Wall clock from task start to passing `check` + tests | Agent + human productivity |
| **Hardware perf** | Wall time of **compiled** tier vs problem size (throughput, latency) | Performance tier |
| **Semantic agreement** | Same results: interpreter vs LLVM vs WASM on shared fixtures | **One semantic truth** |

**Effective tokens:** optimize **expected total cost** (tokens × success probability + retry penalty), not minimal punctuation.

---

## 2. Design principles

1. **One semantic truth** — Parse, typecheck, interpreter, and compiled backends must agree on defined behavior; divergences are bugs or explicitly documented unsupported corners.
2. **Optimize the pipeline, not the logo** — Choose surface form, IR, and tooling by **empirical** fit to metrics (eval harness), not aesthetics alone.
3. **LLM-first tooling** — Small stable verbs, canonical formatting, machine-readable errors, optional structured IR for tools; long normative spec is **reference**, not primary onboarding.
4. **Layered language product** — **LIR** remains the **fast data-processing** DSL with strong oracles; **LYTR** **embeds** LIR for dataflow; both lower through a **shared performance platform**.
5. **LYTR = capability, not Python cosplay** — Turing-complete computation, data composition, reuse, effects, and scale — without requiring identical constructs to Python (modules-as-files, user recursion, etc.) unless they win on metrics.
6. **Performance is explicit** — Predictable lowering (LLVM AOT first); GC vs explicit memory, concurrency model, JIT, SIMD are **versioned** choices with **gates** before large investment.

---

## 3. Target architecture (end state)

```
┌─────────────────────────────────────────────────────────────────┐
│  Authoring layer (what user / agent produces)                    │
│  • Canonical text (LIR, LYTR)  • Optional JSON AST / DAG (tools) │
└────────────────────────────┬────────────────────────────────────┘
                             │ parse · validate · fmt
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│  Semantic layer                                                  │
│  • Typecheck · interp (reference) · error codes + spans          │
└────────────────────────────┬────────────────────────────────────┘
                             │ lower / fuse
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│  Internal IR (LYTR + lowered LIR fragments)                     │
│  • Single spine to optimizers                                    │
└────────────────────────────┬────────────────────────────────────┘
                             │ LLVM IR (+ wasm triple variant)
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│  Hardware / runtime                                              │
│  • Native AOT · WASM · (optional JIT) · runtime (GC / arenas)  │
│  • Concurrency executor · FFI                                    │
└─────────────────────────────────────────────────────────────────┘
```

**LIR** today sits mostly in the top and middle boxes with a **codegen subset** documented in [LLVM_ABI.md](LLVM_ABI.md) and [LIR_V1_SPEC.md §13](LIR_V1_SPEC.md). **LYTR** adds the full general-purpose semantic layer and IR for non-pipeline programs; **LIR embeds into LYTR** without making LYTR streams-only.

---

## 4. Unified phase plan

Phases **interleave** foundation (**LIR** + tooling + eval), **LYTR**, and **performance platform**. Dependencies are noted in §5.

### Phase 0 — Baseline truth and visibility (weeks 1–3)

**Objectives**

- Make **one semantic truth** observable and CI-enforced where codegen exists.
- Make **codegen subset** machine- and human-obvious for agents.

**Work items**

- Extend golden coverage: **interpreter vs LLVM vs WASM** on every program in the **codegen subset** (Linux CI with `clang`+wasm as today).
- Optional CLI: report whether a program is **codegen-supported** without emitting full IR (e.g. `lir compile --supported-only` or dedicated subcommand).
- Machine-readable **subset descriptor** (e.g. JSON alongside prose in `docs/`) for agents and eval.

**Exit criteria**

- CI green; no silent drift between interp and native/wasm on subset fixtures.
- Subset rules available in one structured artifact + existing spec links.

---

### Phase 1 — LLM-first toolchain surface (weeks 2–6, overlaps Phase 0)

**Objectives**

- Minimize agent friction: **predictable I/O**, **fast feedback**, **stable errors**.

**Work items**

- Audit **all** CLI paths: consistent **JSON line** (or equivalent) on errors; stable **exit codes**.
- `lir fmt --check` (fail if not canonical) for CI and agents.
- **`AGENTS.md`** (or `.cursor/rules`): short patterns (run `check` after edits, link to codegen subset, pointer to eval tasks).
- Single **agent cheat sheet** (≈30–50 lines): valid patterns + anti-patterns; spec remains long-form reference.

**Exit criteria**

- New contributor/agent can run **check · fmt · run · compile · wasm** with documented behavior.
- No command that prints only human prose on failure without a machine line.

---

### Phase 2 — Canonical interchange: AST / DAG (weeks 4–12)

**Objectives**

- **Round-trip** structure ↔ text for **tooling, diff, merge**, optional constrained decoding later.
- **One schema family** for LIR (and future LYTR nodes) to avoid a second ad-hoc JSON dialect later.

**Work items**

- Define **JSON Schema** (or equivalent) for LIR AST nodes + spans + stable ordering.
- Implement `lir dump-ast` / `lir apply-ast` (or pipe) with **golden tests**: `text → AST → fmt(text)` invariants.
- Version the schema (`schema_version` field).

**Exit criteria**

- Round-trip tests pass; schema published in `docs/` or `schemas/`.

**Note:** Does not change LIR v1 **semantics**; adds a **view**.

---

### Phase 3 — End-to-end eval harness (weeks 6–16, continuous)

**Objectives**

- Measure **user → LLM → program → hardware** empirically; drive later choices.

**Work items**

- **`eval/`** directory: 20–50 versioned **tasks** (generate, fix, extend) with **hidden tests**.
- Runner: invokes model API (or records traces), runs `check` / `compile` / `run`, logs **tokens, retries, outcome, latency**.
- Baseline: **LIR text + current tools**; document how to reproduce.

**Exit criteria**

- One command produces a **report** (CSV/JSON) for regression comparison.
- Baseline numbers stored for at least one **frozen** task set version.

---

### Phase 4 — Evidence-driven surface tuning (ongoing from Phase 3)

**Objectives**

- Improve **expected pipeline cost** using data, not taste.

**Work items**

- A/B experiments (documented): e.g. keyword verbosity, optional structured authoring for a **subset** of tasks.
- Promote winning defaults into **AGENTS.md**, formatter, and schema examples.

**Exit criteria**

- Each experiment has a **one-page** conclusion: metrics before/after, decision.

---

### Phase 5 — LYTR charter and core semantics (weeks 8–20)

**Objectives**

- **General-purpose** capability for **Python-class** workloads (scope explicit), **without** requiring Python-shaped syntax.

**Work items**

- **B0 — Charter:** 1–2 pages — target workloads, **non-goals**, first **edition** name (e.g. **LYTR 0.1**).
- **B1 — Core calculus:** functions, records, tagged unions (or minimal enum), `if`, bounded loops, `match`; **type rules** on paper then implementation.
- **B2 — Effects v1:** single error model (`Result`-style *or* exceptions — **one**); IO surface; **FFI** to C with explicit **unsafe** boundary.
- **Lowering:** LYTR → **internal LYTR IR** (distinct from LIR text; may share AST schema **namespace** with Phase 2).

**Exit criteria**

- Tiny LYTR programs **parse, check, run** (interpreter or compiled stub).
- Charter approved; non-goals prevent unbounded scope creep.

---

### Phase 6 — Performance platform: memory model (weeks 12–28, gated by B1 sketch)

**Objectives**

- Choose and implement **memory** so LYTR performance and agent safety are **predictable**.

**Work items**

- **C1 — Decision memo:** GC vs **explicit + arenas/regions** vs **hybrid** — one paragraph “why” + risk list.
- Prototype allocator + **microbench** + **small agent-written** programs to observe failure modes (leaks, UB, pauses).
- **LLVM mapping:** layouts, safe points / stack maps if GC; **documented** ABI.

**Exit criteria**

- Documented semantics users and codegen rely on; prototype runs **representative** LYTR programs.

**Gate:** Do **not** build full stdlib before **C1** direction is chosen (layout depends on it).

---

### Phase 7 — Performance platform: concurrency v1 (weeks 18–32, gated by B2)

**Objectives**

- **One** clear concurrency story for **LYTR** edition 1 (not two full models at once).

**Work items**

- **Pick one primary:** **async/await** (I/O-bound scripts) **or** **threads + channels** (CPU + simple sharing model).
- Runtime: executor / thread pool; interaction with **C1** (GC safe points, or explicit `Send`-style rules).
- LLVM lowering: state machines for async, or pthreads + atomics for threads model.

**Exit criteria**

- Spec section + tests + **eval tasks** that use concurrency v1.

---

### Phase 8 — LYTR compiler backend + LIR embed (weeks 22–40)

**Objectives**

- **Shared spine:** LYTR lowers to **LLVM** (and **WASM** where applicable); **LIR embedded** in LYTR lowers to **existing** fusion / LLVM path.

**Work items**

- LYTR IR → LLVM IR (reuse driver patterns from current `llvm_ir`).
- **B4 — LIR embed:** syntax or API bridging **LYTR ↔ LIR** fragment; tests: **same numeric/trap behavior** as standalone LIR.
- **WASM** tier for LYTR subset mirroring today’s wasm story (clang target + optional wasmi tests).

**Exit criteria**

- Non-trivial LYTR program compiles and beats **interp** on benchmark; LIR-in-LYTR matches standalone LIR oracle tests.

---

### Phase 9 — Incremental compilation + scale (weeks 30–48)

**Objectives**

- **Module** graph (or chosen packaging alternative), **caching**, faster edit-compile-run.

**Work items**

- **C6:** dependency graph, incremental artifacts; stable **ABI** between units.
- Align with **LYTR module** story from charter (files vs content-addressed units, etc.).

**Exit criteria**

- Second compile of large project **materially** faster than cold; documented limits.

---

### Phase 10 — Optional advanced performance (gated by eval)

**Objectives**

- Add only what **metrics** justify.

**Work items**

- **C4 — JIT:** LLVM ORC or Cranelift for **dev/REPL** after AOT covers most needs.
- **C5 — SIMD / GPU:** intrinsics or small kernel DSL for **hot loops** identified in eval.
- **Deeper optimizer:** fusion beyond current LIR subset where IR allows proof.

**Gates**

- JIT: REPL or plugin **latency** in eval exceeds threshold.
- SIMD/GPU: **user-written** numeric hot paths in benchmarks.

---

## 5. Dependency overview

```
Phase 0 (truth) ────────────────────────────────┐
Phase 1 (tools) ────────────────────────────────┤
Phase 2 (AST) ──────► Phase 5 (LYTR) schema align │
Phase 3 (eval) ─────► Phase 4 (tuning)           │
                 └──► Phase 5–8 (LYTR choices)    │
Phase 5 B1 sketch ─► Phase 6 (C1 memory)         │
Phase 6 C1 ────────► Phase 7 (C2 concurrency)    │
Phase 5–7 ─────────► Phase 8 (LLVM LYTR + LIR)  │
Phase 8 ───────────► Phase 9 (incremental)         │
Phase 8 + eval ────► Phase 10 (JIT/SIMD)         │
```

**LIR v1** spec remains **frozen** per project policy unless you explicitly **version** (e.g. `lir/2`); **LYTR** is a **separate edition** with its own versioning (e.g. `lytr/1` header when introduced).

---

## 6. LLM and agent integration (cross-cutting)

| Mechanism | Phase |
|-----------|--------|
| Stable JSON errors, exit codes | 1 |
| `fmt` / `fmt --check` | 1 |
| `AGENTS.md` + short cheat sheet | 1 |
| Machine-readable codegen subset | 0 |
| JSON AST + schema | 2 |
| Eval harness + regression metrics | 3–4 |
| Optional constrained decoding on AST | 4+ (after schema proves value) |
| Custom tokenizer / fine-tune | **Optional product track** — only if inference-only ceiling hit |

**Principle:** Agents always need **task** instructions; the goal is **no extra language tutorial** beyond short repo docs + **tool feedback**.

---

## 7. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| LYTR and LIR semantics drift | Shared tests; **B4** embed path; one numeric/trap story |
| Spec and impl explode | **Charter non-goals**; **editioned** language versions |
| GC pauses vs “top notch” | **Incremental** GC later; **explicit** sublanguage for hot paths if hybrid |
| Two IRs confuse tools | **One schema namespace**; LIR as `kind: "lir"` (or equivalent) inside unified AST |
| Eval not run | CI job runs **subset** of eval nightly; block regressions on critical tasks |

---

## 8. Decision log (fill as you go)

Record **date**, **decision**, **alternatives rejected**, **metric or principle** that justified it.

| # | Topic | Owner | Status |
|---|--------|-------|--------|
| D1 | LYTR memory: GC / explicit / hybrid | TBD | Open |
| D2 | LYTR concurrency v1: async vs threads+channels | TBD | Open |
| D3 | LYTR v1 workload scope (scripts only vs +services) | TBD | Open |
| D4 | Model strategy: inference-only vs fine-tune | TBD | Open |
| D5 | Bootstrap: Rust host vs self-host timeline | TBD | Open |

---

## 9. Immediate next actions (first 30 days)

1. **Phase 0:** CI cross-checks interp / LLVM / WASM on all codegen-subset tests; optional `--supported` style reporting; subset JSON.
2. **Phase 1:** `fmt --check`; error JSON audit; `AGENTS.md` + cheat sheet.
3. **Phase 3 (skeleton):** `eval/` with ≥10 tasks + runner + log format.
4. **Phase 5 (paper):** LYTR charter + B1 grammar/types draft.
5. **Phase 6 (paper):** C1 decision memo draft (options matrix, no commitment until B1 types sketched).

---

## 10. Document maintenance

- Update this plan when **phases complete** or **gates** change.
- Link new ADRs or decision records from §8.
- Keep [LIR_V1_SPEC.md](LIR_V1_SPEC.md) and [LLVM_ABI.md](LLVM_ABI.md) as **normative** for LIR; LYTR should gain **`docs/LYTR_*.md`** as it materializes.

---

*End of global plan.*
