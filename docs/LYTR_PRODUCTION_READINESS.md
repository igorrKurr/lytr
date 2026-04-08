# LYTR production readiness

This document defines what **production grade** means for the LYTR platform, **binary gates** before General Availability (GA), and where each item is specified or scheduled.

**Related:** [LYTR_GLOBAL_IMPLEMENTATION_PLAN.md](LYTR_GLOBAL_IMPLEMENTATION_PLAN.md) (engineering phases), [LYTR_GOALS_AND_TIERS.md](LYTR_GOALS_AND_TIERS.md) (dual product framing).

---

## 1. Definitions

| Milestone | Meaning |
|-----------|---------|
| **Experimental** | Breaking changes allowed; no stability promises. |
| **Beta** | Feature set mostly frozen for an edition; migration notes required for breaks. |
| **GA (LYTR 1.0)** | Meets **all mandatory gates** in §3; editions and deprecation policy in force. |

**Completing Phases 0–10 of the global plan alone does not imply GA.** It is necessary infrastructure; GA additionally requires the **production tracks** in §11 of the global plan (semantics, stdlib scope, platform, tooling, quality, security).

---

## 2. Mandatory GA gates (all required)

| # | Gate | Evidence | Spec / plan |
|---|------|----------|-------------|
| G1 | **Written language semantics** for LYTR (evaluation, types, errors) with **UB/FFI** boundaries documented | Published spec + review | [LYTR_SEMANTICS_AND_UB_DRAFT.md](LYTR_SEMANTICS_AND_UB_DRAFT.md) → normative `LYTR_SPEC` |
| G2 | **Concurrency + memory model** documented (happens-before, data races, `unsafe` rules) | Spec section + tests | Global plan Phases 6–7 + semantics doc |
| G3 | **Stdlib charter** implemented for declared **v1 scope** (no silent scope creep) | Checklist + API docs | [LYTR_STDLIB_CHARTER_DRAFT.md](LYTR_STDLIB_CHARTER_DRAFT.md) |
| G4 | **Edition + versioning + deprecation** policy operational | Doc + compiler flags | [LYTR_PLATFORM_AND_EDITIONS_DRAFT.md](LYTR_PLATFORM_AND_EDITIONS_DRAFT.md) |
| G5 | **LSP** (diagnostics, go-to-def, hover) for LYTR + LIR embed | Release artifact + matrix | [LYTR_TOOLING_TRACK.md](LYTR_TOOLING_TRACK.md) |
| G6 | **Packages / build** (resolve, lockfile, reproducible `lytr build`) | Doc + reference project | Global plan Phase 9 extension (P9) |
| G7 | **Fuzzing** on parser + (where applicable) codegen; **no open criticals** | CI job + policy | [LYTR_QUALITY_AND_SECURITY.md](LYTR_QUALITY_AND_SECURITY.md) |
| G8 | **Perf regression CI** on agreed benchmark suite with **budgets** | CI + documented thresholds | Same |
| G9 | **Security: threat model + dependency/supply-chain minimum** (SBOM, signed releases) | Doc + CI hooks | [LYTR_QUALITY_AND_SECURITY.md](LYTR_QUALITY_AND_SECURITY.md) |
| G10 | **LIR strategy** resolved for users (subset vs full; see [LIR_PRODUCT_STRATEGY.md](LIR_PRODUCT_STRATEGY.md)) | Product doc + UX in diagnostics | LIR doc + `codegen-check` messaging |
| G11 | **Eval / agent harness** on **frozen task set** with regression policy | Baseline + CI | Global plan Phase 3–4 |
| G12 | **Debugger or agreed substitute** (e.g. source maps + lldb where AOT) | Doc | [LYTR_TOOLING_TRACK.md](LYTR_TOOLING_TRACK.md) |

---

## 3. Strongly recommended (not all mandatory for minimal GA)

- Structured **CVE / advisory** process for stdlib and toolchain.
- **Cross-compilation** matrix documented (tier 1 / 2 / 3 targets).
- **Localization** strategy for diagnostics (even if English-only v1, state it).
- **Formal methods** or **verified subset** (optional product differentiator).

---

## 4. Release checklist template (per version)

Use before tagging `lytr-X.Y.Z`:

- [ ] Changelog + migration notes (if any breaking change under edition rules)
- [ ] All CI gates green (test, fuzz smoke, perf smoke, eval smoke)
- [ ] SBOM artifact attached to release
- [ ] Binaries signed (where applicable)
- [ ] Spec and `lytr --version` / edition banner aligned

---

## 5. Mapping: global plan phases → GA gates

| Global phase block | Contributes to |
|--------------------|----------------|
| 0–1 | G10 messaging, G11 groundwork |
| 2 | G5 (LSP feeds on AST schema) |
| 3–4 | G11 |
| 5–8 | G1, G2 (implementation), G3 (core + FFI), G10 |
| 9 | G4, G6 |
| 10 | G8 (optional advanced perf) |
| **P11–P16** (global plan §11) | G1–G4, G7–G10, G12 |

---

*Update this file when GA criteria change; bump `schema_version` in a front matter block if you add machine-readable export later.*
