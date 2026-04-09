# Tier A eval — LIR toolchain ([LYTR_GOALS_AND_TIERS.md](../docs/LYTR_GOALS_AND_TIERS.md))

This harness measures **what exists today**: the **LIR** CLI and semantics. It is the primary regression suite for **Tier A** (agent-optimal) before investing in **LYTR** general-purpose syntax.

## Run

From repo root:

```bash
python3 eval/run_tier_a.py
```

Same as `bash eval/run_local.sh`.

**Invoking `lir`:** If **`LIR`** is unset, the harness uses **`target/release/lir`** when that file exists (run `cargo build --release --bin lir` once — **no `cargo` on the hot path**). Otherwise it falls back to **`cargo run -q --bin lir --`**. Override with **`LIR=/path/to/lir`** anytime.

When the fallback **`cargo run`** is used, the harness sets **`RUSTFLAGS`** to include **`-A warnings`** so **stderr** is mostly LIR diagnostics (not rustc warnings).

**API billing:** OpenAI **token counts** are **only** for chat requests. Local **`lir`** / **`python3`** subprocesses add **wall time**, not LLM API tokens.

**Exit 0** = all assertions passed; **exit 1** = at least one failure. Lines are appended to `eval/results.ndjson` (gitignored).

CI also runs **`eval/baseline/python/run_all.py`** and **`eval/run_llm_eval.py --dry-run`** after the manifest eval (no API key required).

## Comparison run (local)

To execute **LIR + Python baseline** in one go (and optionally **live LLM eval**), see [`run_comparison.py`](run_comparison.py) and [`BASELINE.md`](BASELINE.md).

## A/B pilot — LLM (LIR vs Python)

[`pilot/README.md`](pilot/README.md) describes the **four-task** pilot; [`pilot/FAIRNESS.md`](pilot/FAIRNESS.md) explains tokens vs subprocesses and **`--fair-prompts`**. [`run_pilot_ab.py`](run_pilot_ab.py) uses **shared + arm-specific** system text and writes **`eval/results_pilot_ab_<run_id>.ndjson`** (plus per-arm logs with the same id). It also writes **`eval/results_pilot_comparison_<run_id>.json`** (accuracy, performance, tokens, efficiency, **`thesis_metrics`** — LIR vs Python efficiency ratios and verdict); see [`pilot_comparison.py`](pilot_comparison.py) and [`pilot_thesis_metrics.py`](pilot_thesis_metrics.py). Containerized **`lir`**: [`sandbox/README.md`](sandbox/README.md).

## Manifest

**[`manifest.json`](manifest.json)** lists **tasks** and **assertions** per task:

| `kind` | Meaning |
|--------|---------|
| `lir_check` | `lir check` exit code (0 = ok, 1 = expected failure for negative tasks) |
| `lir_run` | `lir run` stdout must match `expect_stdout`; optional `input` for `--input` |
| `fmt_check` | `lir fmt --check` (§11 canonical) |
| `codegen_check` | `lir codegen-check` (LLVM/WASM subset) |

Add a task: new directory under `tasks/<id>/` with `starter.lir` + `prompt.md`, then append an entry to `manifest.json`.

Optional task fields:

| Field | Meaning |
|-------|---------|
| `llm_eval` | If `false`, [`run_llm_eval.py`](run_llm_eval.py) skips the task (e.g. negative **010** / **011** cases). Default is to include the task. |

**Starters for positive tasks must be §11-canonical** so `fmt_check` passes (run `lir fmt` once and paste). Task **010** is intentionally non-canonical; **011** is a syntax error.

## Hidden assertions (optional)

Beside **`starter.lir`**, you may add **`hidden/assertions.json`** under the same task directory. It must be a JSON **array** of assertion objects, or an object `{"assertions": [...]}`, using the **same `kind` fields** as the manifest. These run **after** the manifest assertions against the **same** `.lir` file (starter in `run_tier_a.py`, model output in `run_llm_eval.py`). NDJSON lines include **`"hidden": true`**. Invalid JSON or a bad shape yields a clear error (including in **`run_llm_eval.py --dry-run`**, which lists manifest vs hidden counts per task).

Use this for extra **`lir_run`** cases (e.g. alternate `--input`) that should not appear in the public **`prompt.md`**. Example: [`tasks/002_input_i32/hidden/assertions.json`](tasks/002_input_i32/hidden/assertions.json).

## LLM eval

[`run_llm_eval.py`](run_llm_eval.py) sends each included task’s **`prompt.md`** plus **`starter.lir`** to an OpenAI-compatible chat API, extracts a `.lir` program from the reply, then runs the **merged manifest + hidden** assertions like `run_tier_a.py` (via shared [`tier_a_lib.py`](tier_a_lib.py)).

From repo root:

```bash
# List tasks that would run (no API calls)
python3 eval/run_llm_eval.py --dry-run

# Live run (costs tokens; optional filters)
OPENAI_API_KEY=... python3 eval/run_llm_eval.py
OPENAI_API_KEY=... python3 eval/run_llm_eval.py --limit 3
OPENAI_API_KEY=... python3 eval/run_llm_eval.py --task 001_range_sum
```

Environment:

| Variable | Role |
|----------|------|
| `OPENAI_API_KEY` | Required for a live run (omit only with `--dry-run`). |
| `OPENAI_BASE_URL` | Chat Completions base URL; default `https://api.openai.com/v1`. |
| `LLM_MODEL` | Model name; default `gpt-4o-mini`. Stronger models (e.g. `gpt-4o`) often pass more tasks. |
| `LLM_RETRY_ON_FAIL` | If `1` / `true` (default), when `lir check` fails the harness sends **one** follow-up with compiler stderr. Set `0` to disable. |
| `LLM_CANONICALIZE` | If `1` / `true` (default), run `lir fmt` on the model’s extracted file **before** assertions (simulates format-on-save). Set `0` to grade raw spacing and keep **`fmt_check`** strict. |

Each `llm_response` line includes **`tokens_prompt`** / **`tokens_completion`** from the API (billing) and **`tokens_prompt_system`** / **`tokens_prompt_user`**: tiktoken counts of the system and first user message bodies only (no chat framing). Install **`tiktoken`** for accuracy (`pip install -r eval/requirements-eval.txt`); without it, a rough character fallback is used.

**Exit codes:** `0` = all assertions passed; `1` = at least one failure; `2` = missing `OPENAI_API_KEY` (and not `--dry-run`). NDJSON lines are appended to **`eval/results_llm.ndjson`** (gitignored): per-assertion records (including **`stderr_got`** when a check/run fails), `llm_response` lines with **`lir_preview`**, and merged `tokens_*` when a retry runs.

## Baseline comparison (Python)

Frozen stdlib baselines for the numeric tasks **001–008** live in [`baseline/python/run_all.py`](baseline/python/run_all.py):

```bash
python3 eval/baseline/python/run_all.py
python3 eval/baseline/python/run_all.py --task 002
```

For methodology and recording agent/token comparisons, see [`BASELINE.md`](BASELINE.md).

## Regression snapshot (Phase 3 — pilot A/B)

After **`run_pilot_ab.py`**, compare the generated **`eval/results_pilot_comparison_<run_id>.json`** to the frozen baseline **[`baselines/pilot_ab_reference.json`](baselines/pilot_ab_reference.json)** (pillar booleans + API token ceilings):

```bash
python3 eval/pilot_regression.py --run-id <run_id> --fail
```

Writes **`eval/regression_report_<run_id>.json`** (gitignored). Refresh the baseline when prompts or fair-prompts change on purpose:

```bash
python3 eval/pilot_regression.py --emit-baseline-from eval/results_pilot_comparison_<run_id>.json
```

Edit ceilings before committing [`baselines/pilot_ab_reference.json`](baselines/pilot_ab_reference.json).
