"""
Aggregate pilot NDJSON logs (tokens, per-task) and emit a combined report.
Used by run_pilot_ab.py.
"""
from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path


def git_rev(repo_root: Path) -> str | None:
    try:
        p = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=repo_root,
            capture_output=True,
            text=True,
            timeout=5,
        )
        if p.returncode == 0:
            return p.stdout.strip()
    except (OSError, subprocess.TimeoutExpired):
        pass
    return None


def aggregate_tokens_ndjson(path: Path) -> tuple[dict[str, dict], dict[str, int]]:
    """
    Sum tokens from lines with event=llm_response. Returns (per_task, totals).
    per_task[task_id] = {tokens_prompt, tokens_completion, ... arm-specific fields }

    tokens_prompt / tokens_completion come from the API. tokens_prompt_system /
    tokens_prompt_user are tiktoken counts of the system and first user message bodies
    (content only; chat framing is not included).
    """
    per_task: dict[str, dict] = {}
    totals = {
        "tokens_prompt": 0,
        "tokens_completion": 0,
        "tokens_prompt_system": 0,
        "tokens_prompt_user": 0,
    }
    if not path.is_file():
        return per_task, totals
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                o = json.loads(line)
            except json.JSONDecodeError:
                continue
            if o.get("event") != "llm_response":
                continue
            tid = o.get("task_id")
            if not tid:
                continue
            tp = o.get("tokens_prompt") or 0
            tc = o.get("tokens_completion") or 0
            ts = o.get("tokens_prompt_system")
            tu = o.get("tokens_prompt_user")
            totals["tokens_prompt"] += int(tp)
            totals["tokens_completion"] += int(tc)
            if ts is not None:
                totals["tokens_prompt_system"] += int(ts)
            if tu is not None:
                totals["tokens_prompt_user"] += int(tu)
            per_task[tid] = {
                "tokens_prompt": o.get("tokens_prompt"),
                "tokens_completion": o.get("tokens_completion"),
                "tokens_prompt_system": o.get("tokens_prompt_system"),
                "tokens_prompt_user": o.get("tokens_prompt_user"),
                "model": o.get("model"),
            }
            if "attempt" in o:
                per_task[tid]["attempt"] = o.get("attempt")
            if "retry" in o:
                per_task[tid]["retry"] = o.get("retry")
            if "canonicalized" in o:
                per_task[tid]["canonicalized"] = o.get("canonicalized")
            if "lir_chars" in o:
                per_task[tid]["lir_chars"] = o.get("lir_chars")
            if "py_chars" in o:
                per_task[tid]["py_chars"] = o.get("py_chars")
    return per_task, totals


def env_snapshot() -> dict:
    return {
        "LLM_MODEL": os.environ.get("LLM_MODEL", ""),
        "OPENAI_BASE_URL": os.environ.get("OPENAI_BASE_URL", ""),
        "LLM_CANONICALIZE": os.environ.get("LLM_CANONICALIZE", ""),
        "LLM_RETRY_ON_FAIL": os.environ.get("LLM_RETRY_ON_FAIL", ""),
    }
