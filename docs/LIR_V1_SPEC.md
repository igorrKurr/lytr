# LIR Language Specification v1.0 (frozen)

**LIR** (this project’s name for the stream DSL—not LLVM’s unrelated “LIR”) is the **fast data-processing language**: integer streams, linear pipelines, and strong LLVM/WASM oracles for the supported subset. **LYTR** is the **general-purpose language** built on top of LIR (see [NAMING.md](NAMING.md)); this specification defines **only** LIR `lir/1`.

This document is **normative** for `lir/1`. Integer-only surface; **LLVM-backed execution defines official results** for programs the toolchain lowers to LLVM IR (see §13); the **reference interpreter** is authoritative for every well-typed program. **WebAssembly** must match LLVM on programs both backends accept; differential tests cover that subset.

---

## 1. File format

- **Encoding:** UTF-8.
- **Header:** The first line MUST be exactly `lir/1` followed by a newline (`\n`).
- **No comments** in v1.

---

## 2. Types

- `i32`, `i64`, `bool`.
- **No floating-point types** in v1.
- **`bool` is distinct:** no `i32` truthiness coercion.

---

## 3. Program shape

```text
lir/1
<source> ("|" <stage>)+
```

- A program is one **source**, then **`|`** and a **stage**; further stages are separated by `|`.
- Stages are a **linear chain** (implicit pipeline). No user-defined names.

---

## 4. Sources

### 4.1 `input`

```text
input
input:i32
input:i64
input:bool
```

- If the type suffix is omitted, it means **`i32`**.
- **Canonical form (formatter):** always prints `input:i32`, `input:i64`, or `input:bool` explicitly.

### 4.2 `range`

```text
range ( <int> , <int> )
range ( <int> , <int> , <int> )
```

- Semantics: **half-open** `[start, stop)` in steps of `step`.
- Two-argument form: `step = 1` if `start <= stop`, else `step = -1` (implementation must match reference).
- **Empty range is valid** (zero elements).
- **Compile-time rejection:** if `start`, `stop`, `step` are **all literals** and the loop is provably infinite (e.g. step 0, or step sign never moves toward `stop`).

### 4.3 `lit`

```text
lit ( <literal> { "," <literal> }* )
```

- All literals in one `lit` must have the **same type** (`i32`, `i64`, or `bool`).
- **`lit()` with no arguments** is allowed and yields an **empty** stream with element type **`i32`** (no elements).

---

## 5. Stages

### 5.1 `filter`

```text
filter <predicate>
```

**Predicate grammar (EBNF):**

```ebnf
predicate   ::= or_expr ;
or_expr     ::= and_expr { "or" and_expr }* ;
and_expr    ::= not_expr { "&" not_expr }* ;
not_expr    ::= "not"? primary_pred ;
primary_pred ::= "even" | "odd"
               | ("eq"|"lt"|"le"|"gt"|"ge") <int>
               | "eq" ("true"|"false")
               | "(" predicate ")" ;
```

- **`&`:** left-to-right short-circuit (see §8).
- **`or`:** left-to-right short-circuit.
- **`even` / `odd`:** defined for integer streams only; **type error** on `bool`.
- **`bool` streams:** only **`eq true`** and **`eq false`** comparisons (no integer rhs); other comparison ops on bool are a **type error**.

### 5.2 `map`

```text
map <expr>
```

**Expression grammar:** arithmetic on **`i32`/`i64`** with **anonymous current element** written **`.`** (dot). This is a fixed pipeline slot, not a user-defined variable name.

```ebnf
expr       ::= add_expr ;
add_expr   ::= mul_expr { ("add"|"sub") mul_expr }* ;
mul_expr   ::= unary { ("mul"|"div"|"mod") unary }* ;
unary      ::= "neg"? primary ;
primary    ::= <int> | "." | "(" expr ")" ;
```

- **`square`** is a **unary keyword** accepted by the parser as sugar: `square` ≡ `mul . .` (expanded to `Expr::Mul` of two `Expr::Dot` nodes during parsing; there is no separate AST variant).
- On an **`i32`** stream, integer **literals** in `map` must fit **`i32`** unless the whole expression widens to **`i64`** (tracked by the type checker).

### 5.3 `scan`

```text
scan <int_literal> , <scan_op>
```

- `<scan_op>` is one of: `add`, `sub`, `mul` (accumulator `acc`, element `elem`; `acc = op(acc, elem)` in that order for `sub`: `acc - elem`).
- Initial accumulator is `<int_literal>` (must fit the stream element type after promotion rules below).

### 5.4 `reduce`

```text
reduce <reducer>
```

**Reducers:** `sum`, `prod`, `count`, `min`, `max`.

- **Empty stream:** `sum → 0`, `prod → 1`, `count → 0`; **`min` / `max` → runtime error** `R_REDUCE_EMPTY_MINMAX`.

### 5.5 `take` / `drop`

```text
take <int>
drop <int>
```

- Argument is a non-negative `i32` literal in v1 (fits in `i32`).

### 5.6 `id`

No-op stage.

---

## 6. Typing rules

- **`range`** produces `i32` elements (v1); `lit` produces the unified literal type.
- **`input:i64`:** all downstream numeric ops are **`i64`** unless explicitly narrowed (v1: no narrow); **`filter` comparisons** use the stream type.
- **Promotion:** mixed `i32` literal in `i64` stream context is promoted; otherwise **type error**.
- **`map` / `scan`:** result element type matches the inferred numeric type of the expression (Widen to `i64` if needed).
- **After `reduce`:** the pipeline becomes a **scalar**; **no further stages** in v1 (type error).

---

## 7. Integer semantics

- **Overflow:** `add`, `sub`, `mul`, `scan` steps, and **reductions** use **checked arithmetic**. On overflow → **trap** `R_INTEGER_OVERFLOW`.
- **Division:** `/` is not a token; `div` in `map` uses **truncating division toward zero**; **remainder** (`mod`) matches Rust: `div_trunc` + `mod` semantics.
- **Divide by zero** in `div` or `mod` → **trap** `R_DIV_BY_ZERO`.
- **`INT_MIN / -1` and `INT_MIN % -1`** → **trap** `R_INTEGER_OVERFLOW` (no representable quotient).
- **`mod`:** Rust-style `%` remainder with **truncating division toward zero** (same as Rust `a % b`).

---

## 8. Determinism

- No randomness, no wall-clock.
- **Short-circuit:** `filter` must evaluate `&` and `or` left-to-right and skip remaining evaluations once result is fixed **for non-pure extensions**; in v1 predicates are pure, but order is still **specified** for future-proofing.

---

## 9. Runtime errors (structured)

All traps include: `kind: "runtime"`, `code`, `message`, optional `stage_index`, `element_index`.

| Code | Condition |
|------|-----------|
| `R_INTEGER_OVERFLOW` | Checked arithmetic overflow |
| `R_DIV_BY_ZERO` | `div` or `mod` by zero |
| `R_REDUCE_EMPTY_MINMAX` | `min`/`max` on empty |
| `R_INDEX` | Internal invariant / bounds (should not occur if compiler correct) |
| `R_INPUT_TY` | Host input value does not match declared `input:*` type |
| `R_SCAN_INIT_RANGE` | Scan initializer does not fit stream element type (interpreter) |
| `R_RANGE_STEP` | Zero `range` step (should not occur after parse) |
| `R_RANGE_ELEM_RANGE` | `range` value outside `i32` (v1 interpreter materialization) |

---

## 10. ABI (reference interpreter / LLVM oracle)

**Function signature (conceptual):**

- `run(ptr: *mut T, len: u32) -> ResultScalar` for `input` programs, or host supplies buffer.
- **Length** is explicit; no sentinel termination.

*(Full WASM/LLVM struct packing is implementation-defined in the compiler crate; must be documented per backend.)*

For the LLVM IR emitter in this repository, see [LLVM_ABI.md](LLVM_ABI.md). For the WebAssembly path (`emit_wasm` / `lir wasm`), see [WASM_ABI.md](WASM_ABI.md).

**CLI (`lir run`):** the `--input '[…]'` array is parsed so each element matches the program’s `input:i32`, `input:i64`, or `input:bool` source (decimal integers as `i32` / `i64` respectively, or `true` / `false` for bool).

---

## 11. Canonical formatting

- Lowercase keywords.
- Spaces: single space around `|`; space after commas in lists.
- **Always print explicit `input:i32` / `input:i64` / `input:bool`.**
- Reference: `lir fmt <file.lir>` prints this form to stdout (parse errors are reported; type errors are not required for printing).

---

## 12. LLVM and WASM as semantic oracles

- Golden tests compare the **reference interpreter** vs **native code** produced from the same LLVM IR (`lir compile` + `clang`) on identical inputs.
- The **WASM** backend reuses that IR (with a wasm32 module header) and must match the interpreter on non-trapping programs; on traps, the wasm module **traps** (no structured error payload), while the interpreter reports the runtime codes in §9 where applicable.

**Implementation note (this repo):** `emit_llvm_ir`, **`emit_wasm`** (clang `--target=wasm32-unknown-unknown`), and **`lir wasm`** are implemented. Differential tests live in `tests/llvm_golden.rs` and `tests/wasm_golden.rs` (the latter skips automatically when no wasm-capable `clang` is available, e.g. typical Xcode installs). See [WASM_ABI.md](WASM_ABI.md) for exports and memory layout.

---

## 13. Codegen coverage (this repository)

The **parser, type checker, formatter, and reference interpreter** implement **all** of §1–§11 for well-typed programs.

The **LLVM IR** and **WebAssembly** emitters intentionally support a **structured subset** of pipelines (single fused loop, optional one `scan` stage) so that codegen stays small and testable. Valid programs **outside** that subset still type-check and run under `lir run` but `lir compile` / `lir wasm` return **`T_CODEGEN_UNSUPPORTED`** (or size limits **`T_CODEGEN_TOO_LARGE`**) with a fix hint. The exact shape is documented in [LLVM_ABI.md — Codegen subset](LLVM_ABI.md#codegen-subset).
