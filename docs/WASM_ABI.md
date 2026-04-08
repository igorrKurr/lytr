# WebAssembly backend ABI (`lir wasm` / `emit_wasm`)

**LIR** tier of the **LYTR** project — see [NAMING.md](NAMING.md).

This describes the **`.wasm`** produced from the same logical program as [LLVM_ABI.md](LLVM_ABI.md): the crate emits LLVM IR via `emit_llvm_ir`, rewrites the module header for **`wasm32-unknown-unknown`**, then invokes **`clang`** to link a wasm module.

## Toolchain

- **Command shape:** `clang --target=wasm32-unknown-unknown -nostdlib -fno-builtin -O2 -Wl,--no-entry -Wl,--export=lir_main -o out.wasm module.ll`
- **Discovery:** `LIR_CLANG`, then `WASI_SDK_PATH/bin/clang`, then `clang` on `PATH`.
- **`wasm_clang_target_ok()`** runs a one-time **link probe** (not only `-print-target-triple`), because some clang builds advertise wasm without being able to link.

## Exports

- **`lir_main`** — same signatures as `@lir_main` in [LLVM_ABI.md](LLVM_ABI.md) (Wasm function types use `i32` / `i64` matching LLVM’s `i32` / `i64`).
- **`memory`** — linear memory is exported for **`input`** programs so the host can pass the element array.

## Host contract for `input` programs

- Pass **`in_len`** as the second parameter (32-bit element count).
- Pass **`in` pointer** as the first parameter: a **byte offset** into the exported **`memory`** (Wasm has no raw pointers in the signature).
- Elements are tightly packed: **`i32`** (4 bytes LE), **`i64`** (8 bytes LE), **`i8`** for bool (`0` / `1`), same as LLVM layout.
- The reference tests in `tests/wasm_golden.rs` write inputs at **offset 1024** and grow memory as needed; that offset is **not** fixed by the ABI—only layout and export names are.

## Traps

Overflow, division by zero, and empty `min`/`max` lower to **`llvm.trap()`** in IR and become a **Wasm trap** at runtime. There is no mapping to JSON error objects inside the module.
