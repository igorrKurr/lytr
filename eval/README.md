# LYTR / LIR eval harness (skeleton)

Phase 3 of `docs/LYTR_GLOBAL_IMPLEMENTATION_PLAN.md`: measure **user → LLM → program → hardware**.

## Layout

- **`tasks/`** — one directory per task (`task_id/`) with:
  - `prompt.md` — what the model (or human) should do
  - `starter.lir` — optional starting source (may be broken for “fix” tasks)
  - `hidden/` — **not** shown to the model in real eval; contains checker scripts or expected outputs (gitignored if secrets)

## Local runner (no API)

`run_local.sh` runs **`lir check`** on each `starter.lir` that exists and appends a JSON line to `eval/results.ndjson`. Use this for CI smoke and agent smoke tests before wiring an LLM API.

## Log format (one JSON object per line)

```json
{
  "task_id": "001_parse_ok",
  "ts": "2026-04-07T12:00:00Z",
  "tool": "lir_check",
  "exit_code": 0,
  "tokens_prompt": null,
  "tokens_completion": null,
  "retries": 0
}
```

When an LLM runner exists, fill `tokens_*` and `retries` from the API trace.

## Adding tasks

1. Create `eval/tasks/<id>/prompt.md` and optional `starter.lir`.
2. Register the task id in `run_local.sh` `TASKS` array (until a manifest exists).
