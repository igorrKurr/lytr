# A/B pilot — LLM → LIR vs LLM → Python

**Goal:** same four tasks, same model settings, compare pass rate and tokens between:

- **LIR arm:** [`run_llm_eval.py`](../run_llm_eval.py) with [`lir_manifest.json`](lir_manifest.json) (full LIR assertions + optional canonicalize / retry).
- **Python arm:** [`run_llm_python_eval.py`](../run_llm_python_eval.py) with [`python_manifest.json`](python_manifest.json) (stdout must match `expect_stdout`; stdlib only).

## Release `lir` (faster, no `cargo` per call)

```bash
cargo build --release --bin lir
```

Then Tier A / pilot use **`target/release/lir`** automatically (see [`tier_a_lib.lir_base_cmd`](../tier_a_lib.py)).

## Fair prompts (`--fair-prompts`)

`run_pilot_ab.py` passes **`--fair-prompts`** to both harnesses: shared stem [`system_shared.md`](system_shared.md) plus [`system_arm_lir.md`](system_arm_lir.md) or [`system_arm_python.md`](system_arm_python.md). See [`FAIRNESS.md`](FAIRNESS.md).

## Docker — whole pilot in sandbox

Use the image’s **Linux** `lir` (not the host `target/release/lir` on macOS). Easiest:

```bash
chmod +x eval/sandbox/run-pilot-ab.sh
export OPENAI_API_KEY=...
eval/sandbox/run-pilot-ab.sh
```

Details: [`../sandbox/README.md`](../sandbox/README.md).

## Run both (from repo root)

```bash
python3 eval/run_pilot_ab.py
```

Or separately:

```bash
OPENAI_API_KEY=... python3 eval/run_llm_eval.py \
  --manifest eval/pilot/lir_manifest.json \
  --results eval/results_pilot_lir_manual.ndjson

OPENAI_API_KEY=... python3 eval/run_llm_python_eval.py \
  --manifest eval/pilot/python_manifest.json \
  --results eval/results_pilot_python_manual.ndjson
```

(`run_pilot_ab.py` picks a new `run_id` per invocation and writes `eval/results_pilot_lir_<run_id>.ndjson`, etc., so totals are never mixed across runs.)

Dry-run (no API):

```bash
python3 eval/run_llm_eval.py --manifest eval/pilot/lir_manifest.json --dry-run
python3 eval/run_llm_python_eval.py --manifest eval/pilot/python_manifest.json --dry-run
```

## Python task contract

- One **complete** Python 3 script; **stdlib only** (do not invoke the `lir` binary).
- **Print exactly one line** to stdout: the decimal answer and a newline (same as `lir run` expectations).
- The harness appends the task’s **`starter.lir`** as a **reference pipeline** so the Python arm gets the same concrete bounds/ops as the LIR LLM eval (fair A/B).
- If **`stdin`** is empty in the manifest, the harness sends **no stdin** — the program must **not** read stdin (only **002** sends a JSON line).
- If **`stdin`** is set, the first line is a **JSON array of integers** (mirrors `lir run --input`).

Logs:

| File | Contents |
|------|----------|
| `eval/results_pilot_lir_<run_id>.ndjson` | Per-assertion + `llm_response` lines (LIR arm) |
| `eval/results_pilot_python_<run_id>.ndjson` | `llm_response` + grade lines (Python arm) |
| `eval/results_pilot_ab_<run_id>.ndjson` | **Combined** run: `pilot_ab_start` (env, git, `run_id`), `pilot_arm` (wall time, exit), `pilot_tokens` (totals + per-task), `pilot_ab_complete` |

Default `run_id` is a UTC timestamp (`YYYYMMDDTHHMMSSZ`). Override with **`python3 eval/run_pilot_ab.py --run-id myrun`** (and optionally **`--report eval/custom.ndjson`** for the combined file only). All are gitignored (`eval/results_pilot_*.ndjson`).

After a live run, **`python3 eval/run_pilot_ab.py`** prints paths, token + wall-time summary, **accuracy / performance** lines, and writes **`eval/results_pilot_comparison_<run_id>.json`** — structured metrics: pass counts and parity, wall times and throughput, token totals and per-task efficiency, output `lir_chars` / `py_chars`, LIR retry flags. Regenerate from existing logs:

```bash
python3 eval/pilot_comparison.py --run-id YYYYMMDDTHHMMSSZ
```

**Regression vs baseline** (Phase 3): [`../pilot_regression.py`](../pilot_regression.py) + [`../baselines/pilot_ab_reference.json`](../baselines/pilot_ab_reference.json) — see main [`eval/README.md`](../README.md#regression-snapshot-phase-3--pilot-ab).

The combined NDJSON report is still 6 lines (metadata, both arms, both token aggregates, completion row).

### Thesis metrics (`thesis_metrics` in the comparison JSON)

Aligned with three evidence checks (see **`pillars`** and **`pillar_summary`** in the JSON):

1. **Marginal after instruction** — user (tiktoken) + completion per task; **system** per task is shown so you see fair-prompt asymmetry. True “constant instruction” needs a separate harness.
2. **Success per spend + repair** — `successes_per_1000_*` on **total API** and on **marginal proxy**; LIR **retry** flags repair rounds (extra API cost in totals).
3. **Completion + repair** — completion tokens per task; repair via LIR retry.

**Ratios** `Python ÷ LIR` (e.g. `head_to_head`): **greater than 1** ⇒ Python spends more on that slice.

**Verdict** (`verdict.overall`): legacy composite (`supported_strict` / `supported_lenient` / …); use **`pillar_summary`** for the three bullet thesis.

See [`pilot_thesis_metrics.py`](../pilot_thesis_metrics.py).

## Fairness notes

LIR path may use **`LLM_CANONICALIZE`** and **`LLM_RETRY_ON_FAIL`** (see main [`eval/README.md`](../README.md)). Python path has **no auto-formatter** in the pilot; set the same **`LLM_MODEL`** and **`OPENAI_BASE_URL`** for both arms when comparing numbers.
