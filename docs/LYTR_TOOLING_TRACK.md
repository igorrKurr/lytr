# LYTR tooling track: LSP, debugger, formatter (draft)

**GA gates G5, G12** — production developer experience beyond CLI.

---

## 1. Dependency on Phase 2 (AST)

The [global plan](LYTR_GLOBAL_IMPLEMENTATION_PLAN.md) **Phase 2** (JSON Schema + `dump-ast` / `apply-ast`) is the **integration spine** for:

- **LSP** (incremental parse, structured edits)
- **Formatter** (already `lir fmt`; extend to LYTR)
- **Refactor** tooling (rename, later)

**Rule:** LSP consumes the **same** AST model as round-trip tests.

---

## 2. LSP phases

| Phase | Capabilities |
|-------|----------------|
| **LSP-0** | Diagnostics from `check`; file sync; `textDocument/formatting` using `fmt` |
| **LSP-1** | Go-to-definition (local); hover for types |
| **LSP-2** | Workspace symbols; find references (same crate) |
| **LSP-3** | Cross-package refs (needs package index from P9) |

---

## 3. Debugger (G12)

**Options:**

- **AOT + lldb/gdb:** Emit **debug info** (DWARF) from LLVM; document source paths and line table.
- **DAP server:** Thin adapter that drives lldb for `lytr`-built binaries.
- **Interpreter-only debug:** For Tier A preview, optional stepping in LYTR interpreter.

**GA minimum:** Document **one** supported path (e.g. “debug LYTR-compiled binaries with lldb using …”).

---

## 4. CLI alignment

- Stable **`--message-format=json`** (or equivalent) for **all** subcommands if LSP spawns CLI.
- Exit codes documented in [AGENTS.md](../AGENTS.md) and expanded reference.

---

## 5. LIR in the IDE

- `.lir` files: same LSP server in **LIR mode** or **embedded** in LYTR documents; `codegen-check` as **code action** or diagnostic hint.
