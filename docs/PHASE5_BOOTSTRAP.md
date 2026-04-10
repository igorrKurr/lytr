# Phase 5 implementation — bootstrap (after paper B0–B2)

The **Phase 5 exit criterion** in the global plan — *tiny LYTR programs parse, check, run* — requires a **bootstrap interpreter** (or compiled stub), not only charter/calculus drafts.

**Paper track (complete in repo):** [LYTR_CHARTER_DRAFT.md](LYTR_CHARTER_DRAFT.md) (B0), [LYTR_CORE_CALCULUS_DRAFT.md](LYTR_CORE_CALCULUS_DRAFT.md) (B1), [LYTR_EFFECTS_AND_FFI_DRAFT.md](LYTR_EFFECTS_AND_FFI_DRAFT.md) (B2), [LYTR_LOWERING_SKETCH.md](LYTR_LOWERING_SKETCH.md).

**Suggested bootstrap order (implementation):**

1. **`lytr/0.1` header** + lexer/parser for a **minimal** expression subset (literals + `+` + parens).
2. **Single `fn main() -> i32`** (or `()`) entry; interpreter returns exit code or prints.
3. **`check`** only for that subset; expand to `let`, `if`, then `Result` + `match`.

Until step 1 lands, **`cargo test`** remains LIR-only; LYTR lives under `docs/` + plan backlog §13.
