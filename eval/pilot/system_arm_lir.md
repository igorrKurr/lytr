# Output: LIR (`lir/1`)

- Line 1 must be exactly: `lir/1`
- One pipeline per line or stages joined with `|`. Use **canonical §11 spacing** (same as `lir fmt`).
- `range ( lo , hi )` uses an **exclusive** upper bound for integer streams.
- i64: use decimal literals in `lit ( … )` tuples; do not invent invalid `map` fragments.
- Do not use backtick characters in source — invalid in LIR v1.

Example (sum 0..4 → stdout line 10 then newline):

lir/1
range ( 0 , 5 ) | reduce sum
