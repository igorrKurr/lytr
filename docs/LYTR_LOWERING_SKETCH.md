# LYTR lowering sketch — internal IR (LYIR)

**Purpose:** Satisfy Phase 5 plan item “LYTR → internal LYTR IR (distinct from LIR text).” This is **design** only; the repo still implements **LIR** end-to-end.

## 1. Name and layering

- **LYIR** — **LYTR internal IR**: typed, desugared program after LYTR name resolution, `match`/`if`/`Result` elaboration, and **LIR embed** lowering.
- **LIR text** (`lir/1`) — unchanged normative surface for pipeline tiers ([LIR_V1_SPEC.md](LIR_V1_SPEC.md)); embedded fragments may be **materialized** as LIR source strings for **reuse** of the existing `lir` pipeline or carried as structured nodes until codegen.

## 2. LIR embed

- **Surface:** one syntactic form (block, macro, or intrinsic — **TBD** in grammar) denotes a **stream pipeline** typed against LIR rules.
- **Lowering:** LIR-shaped subtree → validate against LIR AST → either emit **canonical LIR text** for `lir check` / fusion, or feed a **shared** structured representation aligned with [LIR_AST_JSON.md](LIR_AST_JSON.md) `program` objects (same **schema family**, discriminant `kind: "lir_program"` under a LYTR wrapper node).

## 3. After LYIR

LYIR → LLVM IR for general control flow + **calls** into existing **LIR codegen** for embedded pipelines (or merge modules in one LLVM module). Details belong in a later **LYTR backend** spec once B1 is fixed.

## 4. Schema versioning

LYIR JSON (future) should carry its own `schema_version` in the same spirit as LIR AST JSON; **namespace** in a unified tool AST can use `language: "lytr"` vs `language: "lir"` at the root.
