# LLVM backend ABI (`lir compile`)

**LIR** tier of the **LYTR** project — see [NAMING.md](NAMING.md).

This describes the **LLVM IR** produced by this crate’s `emit_llvm_ir` (and the `lir compile` CLI). It implements the conceptual ABI sketched in [LIR_V1_SPEC.md §10](LIR_V1_SPEC.md).

## Entry symbol

- **`@lir_main`** — the only public function intended for the host or test harness.

## Signatures

| Source kind | LLVM signature |
|-------------|----------------|
| `input` (`input:i32`, `input:i64`, or `input:bool`) | `define <ret> @lir_main(<elem>* nocapture readonly %in, i32 %in_len)` |
| `range` / `lit` (materialized array) | `define <ret> @lir_main()` |

- **`<elem>`** is `i32`, `i64`, or `i8` (booleans use **`i8`** with `0` / `1`).
- **`<ret>`** is the scalar type of the final `reduce`:
  - `reduce count` → **`i32`** always.
  - Other reducers → **`i32`** or **`i64`** matching the stream element type (`i32` / `i64` streams only in v1; bool streams allow **`reduce count`** only).

## Memory and length

- **`%in`** points to a contiguous array of **`in_len`** elements of type **`<elem>`**.
- **`in_len`** is a **32-bit unsigned** element count (the reference interpreter uses the same logical length).
- There is **no** sentinel; out-of-bounds access must not occur (the lowered loop uses `0 .. count` after `drop` / `take`).

## Data sections

- Literal and `range` programs use a private global **`@lir_data`**: `[N x <elem>]`, read-only, alignment matches `<elem>` (1 / 4 / 8).

## Traps

Overflow, division by zero, and empty `min`/`max` lower to **`llvm.trap()`** (plus `unreachable`). The host should treat a trap as the structured runtime errors in §9 (`R_INTEGER_OVERFLOW`, `R_DIV_BY_ZERO`, `R_REDUCE_EMPTY_MINMAX`).

## Codegen subset

`emit_llvm_ir` (and therefore `emit_wasm`) accept only pipelines that match this **fusion shape**; otherwise they fail with **`T_CODEGEN_UNSUPPORTED`** (`T_CODEGEN_TOO_LARGE` if materialized `range` / `lit` exceeds the embed limit).

1. **Last stage** must be **`reduce`**.
2. **Prefix (optional):** only **`drop`**, **`take`**, and **`id`** — applied to the source **before** any **`filter`**, **`map`**, or **`scan`**.
3. **Middle:** zero or more **`filter`**, **`map`**, and **`id`**.
4. **At most one `scan`**, and only on **`i32` / `i64`** streams (the type checker already rejects `scan` on `bool`).
5. **After `scan` (if present):** only **`filter`**, **`map`**, and **`id`** — no second **`scan`**, and no **`take` / `drop`** in the middle or suffix.

Examples **outside** the subset (they still type-check and run in the interpreter): `take` / `drop` after **`map`**; **`scan`**, then **`take`**; two **`scan`** stages.

Machine-readable summary: [codegen_subset.json](codegen_subset.json). CLI probe: `lir codegen-check <file.lir>`.

## Environment

- **`LIR_LLVM_TRIPLE`**: if set to a **safe** value (ASCII, ≤128 chars, only letters, digits, `-`, `.`, `_`), overrides the emitted `target triple`. Otherwise the emitter falls back to `unknown-unknown-unknown` (avoids breaking IR text or downstream tools).

## Predicate lowering (§8)

- `filter` compiles **`or`** and **`&`** with **left-to-right short-circuit** control flow: the right-hand side is not evaluated in LLVM when the result is already fixed, matching the reference interpreter and the normative evaluation order in the spec.
