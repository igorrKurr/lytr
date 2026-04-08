# LIR product strategy: full language vs LLVM/WASM subset

## Current state

| Surface | Semantics | Native codegen |
|---------|-----------|----------------|
| **Full LIR v1** (`lir/1`) | [LIR_V1_SPEC.md](LIR_V1_SPEC.md), reference interpreter | **Subset only** ([LLVM_ABI.md](LLVM_ABI.md), [codegen_subset.json](codegen_subset.json)) |

Well-typed programs **outside** the subset **run** in the interpreter but **`lir compile` / `lir wasm`** fail with `T_CODEGEN_UNSUPPORTED` (or size limits).

This is a **product and teaching** risk: users assume “if it checks, it compiles.”

---

## Strategic options (choose explicitly; record in decision log)

1. **Widen codegen** — Gradually expand `compile_plan` / lowering until **all** well-typed LIR v1 programs emit LLVM IR (and WASM where applicable). *Cost:* engineering; *benefit:* one semantic story for LIR.
2. **Versioned SKUs** — Keep v1 as-is; introduce **`lir/2`** (or profiles) that align syntax with what codegen supports; v1 remains interp-only legacy. *Cost:* migration; *benefit:* clear naming.
3. **Product naming** — Market **“LIR Runtime”** (full) vs **“LIR Native”** (subset) with `codegen-check` in CI/docs as the gate. *Cost:* communication; *benefit:* no immediate compiler change.
4. **LYTR as primary compiled surface** — Encourage heavy logic in LYTR; LIR stays **embedded** for streams with **defined** compiled bridge. *Cost:* LYTR must ship; *benefit:* clear layering.

---

## GA implication ([LYTR_PRODUCTION_READINESS.md](LYTR_PRODUCTION_READINESS.md) gate G10)

Before **LYTR GA**, publish:

- A **single paragraph** user-facing statement (README + diagnostics): what compiles and what does not.
- **CI recommendation:** `lir codegen-check` on any package claiming “compiled LIR.”

---

## Recommendation placeholder

Decision **open** until eval + LYTR embed path matures. Default **interim** posture: **option 3 + 4** (clear messaging + LYTR as compiled host) while **option 1** proceeds opportunistically.
