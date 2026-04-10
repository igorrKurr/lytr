# Experiment: LLM pilot A/B (LIR vs Python on four tasks)

**Status:** design frozen; **live API** runs optional (cost). **Phase 4** requires a short conclusion per experiment — this document is the record for the **pilot harness** itself.

## Hypothesis

For the same four numeric tasks, compare **LLM→LIR** (full toolchain checks) vs **LLM→Python** (stdout match) under **fair prompts** (`--fair-prompts`), measuring API tokens, wall time, pass rates, and **thesis-aligned** pillar booleans ([`eval/pilot_thesis_metrics.py`](../eval/pilot_thesis_metrics.py)).

## Setup (reproducible)

- Manifests: [`eval/pilot/lir_manifest.json`](../eval/pilot/lir_manifest.json), [`eval/pilot/python_manifest.json`](../eval/pilot/python_manifest.json).
- Fairness notes: [`eval/pilot/FAIRNESS.md`](../eval/pilot/FAIRNESS.md).
- Run: `python3 eval/run_pilot_ab.py` (add `--dry-run` for CI; no API).
- Outputs: `eval/results_pilot_ab_<run_id>.ndjson`, per-arm logs, `eval/results_pilot_comparison_<run_id>.json`.

## Metrics before / after

| Stage | What changed |
|--------|----------------|
| **Baseline (frozen)** | Pillar booleans + API token **ceilings** in [`eval/baselines/pilot_ab_reference.json`](../eval/baselines/pilot_ab_reference.json); regression via [`eval/pilot_regression.py`](../eval/pilot_regression.py) + CI fixture. |
| **After intentional prompt/model changes** | Re-run live pilot, run `--emit-baseline-from` on the new comparison JSON, **tighten ceilings** after review, commit updated reference. |

There is no single “before/after verbosity” A/B in-repo yet; the harness is the **instrumentation** for such comparisons.

## Decision (current)

1. Keep the pilot as the **canonical** Phase 4 surface for LLM pipeline cost on a **small** frozen task set; Tier A remains the broader LIR regression set ([`eval/README.md`](../eval/README.md)).
2. **CI** stays **dry-run** only; live runs are manual or scheduled with a **repo secret** (optional follow-up).
3. **Grading asymmetry** (LIR stricter than Python) is **documented** in FAIRNESS — interpret “both pass” cautiously.

## Next experiment (optional)

Structured authoring (Phase 2 JSON AST) on a **subset** of Tier A tasks: compare token cost vs plain `prompt.md` — **not started**; would get its own one-pager when run.
