# LYTR goals and product tiers

The project pursues **two related goals** that are **not identical**. Naming them avoids optimizing one while claiming victory on the other.

---

## Tier A — Agent-optimal LYTR

**Goal:** Maximize **user → LLM → working program** (precision, tokens, time-to-green).

**Emphasis:**

- Strict, regular syntax; small surface; excellent diagnostics and `fmt`.
- Eval harness and **frozen task sets** as the primary regression authority.
- Stdlib can be **minimal** if agents + FFI cover gaps.

**Success metric:** Eval regressions + human/agent study (optional).

**Implementation:** [`eval/README.md`](../eval/README.md) — **Tier A** manifest (`eval/manifest.json`) + `python3 eval/run_tier_a.py` (CI). This suite is **LIR-only** today; it is the yardstick before **LYTR** GP work. Compare against an incumbent using [`eval/BASELINE.md`](../eval/BASELINE.md).

---

## Tier B — Production-ecosystem LYTR

**Goal:** **General availability** language: packages, LSP, stability, security posture, long-lived codebases.

**Emphasis:**

- [LYTR_PRODUCTION_READINESS.md](LYTR_PRODUCTION_READINESS.md) **GA gates**.
- Stdlib breadth per [LYTR_STDLIB_CHARTER_DRAFT.md](LYTR_STDLIB_CHARTER_DRAFT.md).
- Platform policy per [LYTR_PLATFORM_AND_EDITIONS_DRAFT.md](LYTR_PLATFORM_AND_EDITIONS_DRAFT.md).

**Success metric:** GA checklist + external adopters (optional).

---

## Relationship

- **Tier A** can ship **earlier** (e.g. “LYTR for agents” preview).
- **Tier B** **subsumes** Tier A’s technical requirements plus ecosystem and governance work.
- **LIR** remains the fast data-processing tier inside both; see [LIR_PRODUCT_STRATEGY.md](LIR_PRODUCT_STRATEGY.md).

---

## Planning consequence

The [global implementation plan](LYTR_GLOBAL_IMPLEMENTATION_PLAN.md) **Phases 0–10** lean Tier A + core Tier B (compiler, memory, concurrency). **§11 production tracks** close the remaining Tier B gaps.
