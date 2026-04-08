# LIR Language Specification v1.0 (frozen)

This document is **normative** for `lir/1`. Integer-only surface; **LLVM-backed execution defines official results**; WASM must match via differential tests.

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
```

- If the type suffix is omitted, it means **`i32`**.
- **Canonical form (formatter):** always prints `input:i32` or `input:i64` explicitly.

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
               | "(" predicate ")" ;
```

- **`&`:** left-to-right short-circuit (see §8).
- **`or`:** left-to-right short-circuit.
- **`even` / `odd`:** defined for integer streams only; **type error** on `bool`.

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

- **`square`** is a **unary keyword** accepted by the parser as sugar: `square` ≡ `mul . .` (desugared in AST).
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

---

## 10. ABI (reference interpreter / LLVM oracle)

**Function signature (conceptual):**

- `run(ptr: *mut T, len: u32) -> ResultScalar` for `input` programs, or host supplies buffer.
- **Length** is explicit; no sentinel termination.

*(Full WASM/LLVM struct packing is implementation-defined in the compiler crate; must be documented per backend.)*

---

## 11. Canonical formatting

- Lowercase keywords.
- Spaces: single space around `|`; space after commas in lists.
- **Always print explicit `input:i32` / `input:i64`.**

---

## 12. LLVM as semantic oracle

- Golden tests compare **reference interpreter** vs **LLVM JIT/AOT** (when implemented) on identical inputs.
- WASM backend must match LLVM on all non-trap programs; on traps, error codes must match where applicable.
