# Naming: LYTR and LIR

| Name | Meaning |
|------|---------|
| **LYTR** | The **general-purpose (GP) language** and broader toolchain (in development). Built **on top of** LIR for fast data-processing fragments. This repository is the **LYTR** monorepo; the `lir` binary is the current command-line entry point for the **LIR** tier. |
| **LIR** | The **fast data-processing language** already specified and implemented here: typed streams, pipeline stages, `lir/1` programs, reference interpreter, LLVM IR, and WebAssembly backends. (Not the same abbreviation as LLVM’s internal “LIR.”) **Normative:** [LIR_V1_SPEC.md](LIR_V1_SPEC.md). |

**Relationship:** LIR remains a **first-class** language for pipeline-style workloads. LYTR will **embed** LIR (or lower from LIR-shaped syntax) so GP programs can call into the same fusion and codegen path without re-specifying those semantics.

**Casing:** Use **LYTR** / **LIR** in prose; `lytr` for the repo or package folder; `lir` for the current CLI crate/binary and the `lir/1` header line.
