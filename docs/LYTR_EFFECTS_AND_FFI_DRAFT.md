# LYTR effects and FFI — draft (B2)

**Edition:** LYTR 0.1 target. **Pairs with:** [LYTR_CORE_CALCULUS_DRAFT.md](LYTR_CORE_CALCULUS_DRAFT.md), [LYTR_MEMORY_OPTIONS_DRAFT.md](LYTR_MEMORY_OPTIONS_DRAFT.md) (heap ownership).

## 1. Single error model (decision for v0.1 paper)

**Chosen representation:** **`Result<T, E>`** as a real algebraic type in the surface language, with **structured** error payloads `E` (records or enums), **not** exceptions for control flow in v0.1.

- **Rationale:** Maps cleanly to LLVM lowering, keeps LLM-visible **explicit** handling (`match`, `?`-style sugar TBD), and matches agent-friendly “check errors at boundaries” style.
- **Syntactic sugar:** A trailing `?` operator (Rust-like) may **desugar** to `match` + early `return` — exact grammar deferred to the first parser.

**Exceptions** (stack unwinding) are **out of scope** for v0.1 unless a later edition proves they reduce **pipeline cost** on eval tasks (revisit with data).

## 2. IO and ambient authority

- **Goal:** No **hidden** global `print` that implies ambient network or filesystem unless the **capability** is in scope.
- **v0.1 sketch:** A small **`std::io`**-shaped module (names TBD) exposing **`stdin` / `stdout` / `stderr`** as **values** passed explicitly **or** imported once per module with a **capability token** checked at link time (exact model follows memory choice D1).
- **Network / process:** **Not** in Tier 0 stdlib charter ([LYTR_STDLIB_CHARTER_DRAFT.md](LYTR_STDLIB_CHARTER_DRAFT.md)); any preview behind **`unsafe` + edition flag** only.

## 3. FFI to C (`unsafe` boundary)

- **`unsafe` block or `unsafe fn`:** only sites that may call **foreign** code or reinterpret bits with **layout** assumptions.
- **`extern "C"`** declarations: name, Arg/return types as fixed layout (see future **ABI** doc); pointers/references require **explicit** `unsafe` to dereference.
- **Interaction with memory model:** GC vs explicit ([LYTR_MEMORY_OPTIONS_DRAFT.md](LYTR_MEMORY_OPTIONS_DRAFT.md)) determines whether foreign pointers are **opaque handles** or **owned** values; v0.1 should **minimize** FFI surface to **POD** structs and scalar calls.

## 4. Lowering note

Effects and calls lower to **LYIR** (see [LYTR_LOWERING_SKETCH.md](LYTR_LOWERING_SKETCH.md)); `Result` becomes tagged representation in IR, then LLVM `noundef` / landing pads only if exceptions are ever added (not v0.1).
