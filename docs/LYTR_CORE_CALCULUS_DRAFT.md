# LYTR core calculus — draft (B1)

**Edition target:** **LYTR 0.1** (see [LYTR_CHARTER_DRAFT.md](LYTR_CHARTER_DRAFT.md)).  
**Normative for implementation later:** this doc is a **paper** sketch; LIR remains specified by [LIR_V1_SPEC.md](LIR_V1_SPEC.md).

## 1. Syntax sketch (EBNF-style)

- **Program:** optional module header `lytr/0.1` (edition TBD to match charter), then a list of **items** (functions, types, `const`).
- **Types:** `i32`, `i64`, `bool`, `()`, **record** `{ f1: T1, … }`, **enum** `E { A | B(T) | … }` (closed, tagged), **function** `fn(T1,…) -> U`, **`Result<T, E>`** (see [LYTR_EFFECTS_AND_FFI_DRAFT.md](LYTR_EFFECTS_AND_FFI_DRAFT.md)).
- **Expressions:** literals; variables; field access `e.f`; enum constructors `E.A`, `E.B(e)`; block `{ s*; e }` value of last expression; `if e { e1 } else { e2 }`; **`match e { E.A => … }`** (patterns **at most** one level deep on enums in v0.1); calls `f(a,…)`; **bounded** `while` / `for` with explicit **fuel** or static bound (analyzer proves iteration cap — exact rule TBD).
- **Statements:** `let x: T = e;`, assignment to `mut` locals only (no hidden globals in v0.1), `return e;`.

**Out of scope for v0.1:** user generics, HKT, deep pattern matching exhaustiveness proofs, async/threads (see charter appendix B).

## 2. Typing judgment (informal)

We write `Γ ⊢ e : τ` under environment `Γ` mapping variables to types.

- **Literals:** standard rules for `i32`/`i64`/`bool`.
- **Records:** typing ensures field names and types match the record definition; projection `e.f` requires `e : { …, f: τ, … }`.
- **Enums:** constructors carry payload types per variant; `match` branches must cover **all** variants **or** include a catch-all `_` (v0.1 allows `_` to keep agents productive).
- **Functions:** `fn(x1:T1,…) -> R { body }` — body `Γ, x1:T1,… ⊢ body : R` with structured control flow (no fall-through past `return`).
- **If:** both branches must agree on type `τ`; `if` without `else` only in statement position or when type is `()` (TBD: unify with expression `if`).

## 3. Bounded loops

v0.1 requires **no unbounded** general `while` without proof. Options (pick one at implementation):

1. **Static bound:** loop variable ranges over `0..n` where `n` is known at compile time; or  
2. **Fuel parameter:** `while fuel(n) { … }` consuming an `i32`/`i64` counter; or  
3. **Analyzer:** prove decreasing measure (harder).

The implementation plan gates full **LYTR** compiler work; this choice is recorded when the first interpreter lands.

## 4. Relation to Phase 2 (LIR AST JSON)

LYTR surface syntax lowers to an internal IR (see [LYTR_LOWERING_SKETCH.md](LYTR_LOWERING_SKETCH.md)). **LIR** pipeline fragments may reuse the **same JSON schema family** with a discriminant `kind: "lir_embed"` vs `kind: "lytr_stmt"` for tooling that already speaks [LIR_AST_JSON.md](LIR_AST_JSON.md)-style envelopes.
