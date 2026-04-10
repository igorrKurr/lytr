# Baseline comparison (Tier A — optional)

To estimate **true incremental value** of the LIR toolchain for agents, mirror **`eval/tasks/*/prompt.md`** in another language (e.g. Python) with:

- A **small** standard library only (stdlib), no domain magic.
- Equivalent **numeric** behavior (overflow, empty reductions) if you compare fairly.

## Frozen Python scripts (in repo)

[`baseline/python/run_all.py`](baseline/python/run_all.py) implements the same **numeric outcomes** as the Tier A manifest for tasks **001–008** and **012–020** (stdout lines match `lir run` expectations). Tasks **009–011** are formatting / negative cases and are not mirrored there.

```bash
python3 eval/baseline/python/run_all.py
```

CI runs this after `eval/run_tier_a.py` so baselines stay aligned with the manifest.

## LYTR parity harness (`lytr_manifest.json`)

[`run_lytr_tier.py`](run_lytr_tier.py) runs the `lytr` CLI against [`lytr_manifest.json`](lytr_manifest.json): **closed-form** `.lytr` programs whose stdout matches the same numeric expectations as [`baseline/python/run_all.py`](baseline/python/run_all.py) for each included task id. This is the **regression track** for the LYTR bootstrap on shared **outcomes** with Tier A (not same source programs — LYTR has no pipelines yet). Task **006** uses `fn main() -> i64`; **009–011** are LIR-only. Override the binary with **`LYTR=/path/to/lytr`** (default: `target/release/lytr` if present, else `cargo run -q --bin lytr --`).

CI runs `run_lytr_tier.py` after unit tests; [`run_comparison.py`](run_comparison.py) includes it after the Python baseline step.

## One-shot comparison (LIR + Python + LYTR, optional LLM)

When you are ready to **run the comparison** (measure wall time; optionally LLM cost), use [`run_comparison.py`](run_comparison.py). A **`uv`** venv keeps a predictable interpreter (stdlib-only scripts; no extra packages required):

```bash
uv venv .venv
COMPARISON_JSON=1 .venv/bin/python eval/run_comparison.py
```

Or with the system `python3`:

```bash
# LIR Tier A + frozen Python baselines (no API calls)
python3 eval/run_comparison.py

# Same, plus write machine-readable summary (gitignored default path)
COMPARISON_JSON=1 python3 eval/run_comparison.py

# Explicit summary path
python3 eval/run_comparison.py --json-out eval/comparison_summary.json

# Include live LIR LLM eval (requires OPENAI_API_KEY; costs tokens; logs eval/results_llm.ndjson)
python3 eval/run_comparison.py --llm

# Live LYTR LLM eval only (logs eval/results_llm_lytr.ndjson)
python3 eval/run_comparison.py --llm-lytr

# Both LLM arms
python3 eval/run_comparison.py --llm --llm-lytr
```

Exit code is non-zero if any invoked step fails. **`comparison_summary.json`** records `tier_a`, `baseline_python`, optional `llm_eval` (exit + seconds), `git_rev` when available, and `schema_version`.

**What this does *not* automate:** agent **turn counts**, subjective effort, or head-to-head **tokens for a coding agent** on LIR vs Python — capture those in your harness or a spreadsheet keyed by `manifest.schema_version` and git revision.

Record per task (manual or external harness):

| Field | LIR (`run_tier_a.py`) | Baseline | LLM (`run_llm_eval.py`) |
|-------|------------------------|----------|-------------------------|
| Assertions / stdout | NDJSON + CLI | `run_all.py` | NDJSON + usage fields |
| Agent turns to pass | (your harness) | (your harness) | (your harness) |
| Tokens | N/A (CLI) | N/A (script) | `results_llm.ndjson` |
| Wall time | `run_comparison.py` | same | same |

Store ad-hoc results in a spreadsheet or `eval/baseline_results.json` (not committed if noisy). **Freeze** the baseline script version next to the eval **manifest** `schema_version`.
