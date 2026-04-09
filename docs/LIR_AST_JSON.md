# LIR AST JSON interchange (Phase 2)

This document describes the **versioned JSON envelope** for the LIR abstract syntax tree. It does **not** change LIR v1 semantics; it is a **tooling view** for diff, merge, and future structured authoring.

## Normative representation

- **Rust types:** `src/ast.rs` (`Program`, `Source`, `Stage`, …) with `serde` derives.
- **Envelope:** `src/ast_json.rs` — `schema_version` + `program`.
- **CLI:** `lir dump-ast <file.lir>` prints JSON; `lir apply-ast <file.json>` loads JSON, runs `check`, prints canonical §11 text.

Current **`schema_version`** is **`1`** (`AST_JSON_SCHEMA_VERSION` in `ast_json.rs`). Bump it when the JSON shape changes incompatibly.

## Envelope shape

```json
{
  "schema_version": 1,
  "program": {
    "span": { "start": 0, "end": 42 },
    "source": { "kind": "range", "start": 0, "stop": 5, "step": 1, "span": { "start": 6, "end": 28 } },
    "stages": [ … ]
  }
}
```

Most enums use **`kind`** + snake_case variant names (for example `filter`, `reduce`, `cmp`). Tuple enums such as **`LitElem`** use serde’s default externally tagged form (for example `{"I32": 0}`). **`CmpRhs`** uses `Int` / `Bool` keys for its tuple variants.

## JSON Schema

A **minimal** machine-readable schema for the top-level document lives at [`schemas/lir_ast_v1.schema.json`](../schemas/lir_ast_v1.schema.json). It validates the envelope; the full tree is defined by the Rust types and round-trip tests in `tests/ast_json_roundtrip.rs`.

## Round-trip invariant

For any parseable, type-correct program:

`parse(text) → JSON → parse(JSON) → fmt` equals `fmt ∘ parse` on the original text (after canonicalization), as asserted by the tests above.
