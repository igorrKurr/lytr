# Pilot A/B — what the numbers mean

## API tokens vs `lir` / Python subprocesses

- **OpenAI `prompt_tokens` / `completion_tokens`** bill **only the chat API** (system + user + model reply).
- Running **`lir check`**, **`lir run`**, **`python3 …`** locally does **not** add API tokens. Those costs are **CPU time** on your machine (or container).

So: **more LIR “steps” in the harness does not consume more LLM tokens** — it can add **wall time** only.

## When is “worse on tokens” meaningful?

Compare **completion tokens** (how much the model wrote) and **total API tokens** under **matched prompts**.

## Prompt parity (`--fair-prompts`)

`run_pilot_ab.py` passes **`--fair-prompts`** to both arms. That loads:

- `eval/pilot/system_shared.md` — **identical** stem  
- `eval/pilot/system_arm_lir.md` vs `system_arm_python.md` — **only dialect-specific** rules  

Full Tier A (`run_llm_eval.py` without `--fair-prompts`) still uses the longer baked-in **`LIR_SYSTEM`** for regression work.

**Residual asymmetry:** the arm-specific files are not byte-identical (LIR vs Python have different syntax rules). That’s unavoidable; keep arm sections short and parallel.

## Grading asymmetry (by design)

- **LIR:** `check` + `run` + `fmt` + `codegen-check` (strict).
- **Python:** stdout match only (narrower).

“Both passed” ≠ equal difficulty — label results accordingly.
