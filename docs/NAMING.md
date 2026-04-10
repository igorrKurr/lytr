# Naming: LYTR and LIR

| Name | Meaning |
|------|---------|
| **LYTR** | The **general-purpose (GP) language** and broader toolchain (in development). Built **on top of** LIR for fast data-processing fragments. This repository is the **LYTR** monorepo; the `lir` binary is the current command-line entry point for the **LIR** tier. |
| **LIR** | The **fast data-processing language** already specified and implemented here: typed streams, pipeline stages, `lir/1` programs, reference interpreter, LLVM IR, and WebAssembly backends. (Not the same abbreviation as LLVM’s internal “LIR.”) **Normative:** [LIR_V1_SPEC.md](LIR_V1_SPEC.md). |

**Relationship:** LIR remains a **first-class** language for pipeline-style workloads. LYTR will **embed** LIR (or lower from LIR-shaped syntax) so GP programs can call into the same fusion and codegen path without re-specifying those semantics.

**Casing:** Use **LYTR** / **LIR** in prose; `lytr` for the repo or package folder; `lir` for the current CLI crate/binary and the `lir/1` header line.

## Doc index (engineering & production)

| Doc | Purpose |
|-----|---------|
| [LYTR_GLOBAL_IMPLEMENTATION_PLAN.md](LYTR_GLOBAL_IMPLEMENTATION_PLAN.md) | Phased implementation + **§13 ordered backlog** (quarters) |
| [LYTR_PRODUCTION_READINESS.md](LYTR_PRODUCTION_READINESS.md) | GA gates & checklist |
| [LYTR_GOALS_AND_TIERS.md](LYTR_GOALS_AND_TIERS.md) | Agent-optimal vs production ecosystem |
| [LIR_PRODUCT_STRATEGY.md](LIR_PRODUCT_STRATEGY.md) | LIR interp vs codegen subset |
| [LYTR_SEMANTICS_AND_UB_DRAFT.md](LYTR_SEMANTICS_AND_UB_DRAFT.md) | Semantics / UB / FFI path |
| [LYTR_STDLIB_CHARTER_DRAFT.md](LYTR_STDLIB_CHARTER_DRAFT.md) | Stdlib scope & security |
| [LYTR_PLATFORM_AND_EDITIONS_DRAFT.md](LYTR_PLATFORM_AND_EDITIONS_DRAFT.md) | Editions, packages, releases |
| [LYTR_TOOLING_TRACK.md](LYTR_TOOLING_TRACK.md) | LSP, debugger |
| [LYTR_QUALITY_AND_SECURITY.md](LYTR_QUALITY_AND_SECURITY.md) | Fuzz, perf CI, supply chain |
| [eval/README.md](../eval/README.md) | **Tier A** LIR eval harness |
| [eval/BASELINE.md](../eval/BASELINE.md) | Optional incumbent comparison |
| [LYTR_CHARTER_DRAFT.md](LYTR_CHARTER_DRAFT.md) | GP charter (**LYTR 0.1**) |
| [LYTR_CORE_CALCULUS_DRAFT.md](LYTR_CORE_CALCULUS_DRAFT.md) | B1 types / surface calculus (draft) |
| [LYTR_EFFECTS_AND_FFI_DRAFT.md](LYTR_EFFECTS_AND_FFI_DRAFT.md) | B2 effects + FFI (draft) |
| [LYTR_LOWERING_SKETCH.md](LYTR_LOWERING_SKETCH.md) | LYIR lowering (draft) |
| [PHASE5_BOOTSTRAP.md](PHASE5_BOOTSTRAP.md) | Phase 5 parse/run milestone (after papers) |
| [EXPERIMENT_PILOT_LLM_AB.md](EXPERIMENT_PILOT_LLM_AB.md) | Phase 4 pilot A/B conclusion |
| [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) | Plan phases vs repo reality |
| [LYTR_MEMORY_OPTIONS_DRAFT.md](LYTR_MEMORY_OPTIONS_DRAFT.md) | Memory model options |
