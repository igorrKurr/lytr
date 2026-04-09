#!/usr/bin/env python3
"""
Pilot: LLM writes Python 3 (stdlib only); grade by matching stdout to expected (see eval/pilot/python_manifest.json).

Usage (repo root):
  python3 eval/run_llm_python_eval.py --manifest eval/pilot/python_manifest.json --dry-run
  OPENAI_API_KEY=... python3 eval/run_llm_python_eval.py --manifest eval/pilot/python_manifest.json
"""
from __future__ import annotations

import argparse
import json
import os
import ssl
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

_eval_dir = Path(__file__).resolve().parent
if str(_eval_dir) not in sys.path:
    sys.path.insert(0, str(_eval_dir))

from pilot_prompts import fair_python_system
from prompt_token_split import split_system_user_tokens
from tier_a_lib import norm_out, repo_root


PYTHON_SYSTEM = """You write one complete Python 3 program. Standard library only (no pip packages).

Stdout: print exactly one line — the decimal integer answer and a newline. No other printing.

The user message may include a **Reference LIR pipeline** (same file the LIR eval arm sees as starter.lir). Implement the **same semantics** in Python; do not shell out to `lir`.

Stdin (read the user message carefully):
- If the user message says **stdin is empty**, do **not** read stdin at all (no json.loads, no readline). Implement the answer from the task using literals and logic only.
- Only if the user message gives a **JSON array line for stdin**, read that one line and json.loads it.

Do not use markdown fences in your reply; output Python source only."""


def extract_python(text: str) -> str:
    """Take Python source from model output (```python fences or plain code)."""
    t = text.replace("\r\n", "\n").strip()
    if "```" in t:
        parts = t.split("```")
        best = ""
        for block in parts:
            block = block.strip()
            if not block:
                continue
            lines = block.split("\n")
            if lines:
                fl = lines[0].strip()
                if fl.startswith("python"):
                    block = "\n".join(lines[1:]).strip()
            if "print(" in block or block.startswith(("import ", "from ", "def ", "#")):
                if len(block) > len(best):
                    best = block
        if best:
            return best + ("\n" if not best.endswith("\n") else "")
    return t + ("\n" if not t.endswith("\n") else "")


def chat_complete(base_url: str, api_key: str, model: str, messages: list[dict]) -> tuple[str, dict]:
    url = base_url.rstrip("/") + "/chat/completions"
    body = json.dumps(
        {"model": model, "messages": messages, "temperature": 0}
    ).encode("utf-8")
    req = urllib.request.Request(url, data=body, method="POST")
    req.add_header("Content-Type", "application/json")
    req.add_header("Authorization", f"Bearer {api_key}")
    ctx = ssl.create_default_context()
    with urllib.request.urlopen(req, context=ctx, timeout=180) as resp:
        data = json.loads(resp.read().decode("utf-8"))
    content = data["choices"][0]["message"]["content"] or ""
    usage = data.get("usage") or {}
    return content, usage


def main() -> int:
    ap = argparse.ArgumentParser(description="Pilot LLM → Python eval")
    ap.add_argument(
        "--manifest",
        type=str,
        default="eval/pilot/python_manifest.json",
        help="Path relative to repo root (or absolute)",
    )
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--results", type=str, default="", help="NDJSON log path")
    ap.add_argument(
        "--fair-prompts",
        action="store_true",
        help="Pilot A/B: use eval/pilot/system_shared.md + system_arm_python.md (parity with LIR arm)",
    )
    args = ap.parse_args()

    root = repo_root()
    system_content = fair_python_system(root) if args.fair_prompts else PYTHON_SYSTEM
    mp = Path(args.manifest)
    manifest_path = mp if mp.is_absolute() else root / mp
    out = Path(args.results) if args.results else root / "eval" / "results_pilot_python.ndjson"
    if not out.is_absolute():
        out = root / out
    out.parent.mkdir(parents=True, exist_ok=True)

    with open(manifest_path, encoding="utf-8") as f:
        cfg = json.load(f)

    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    pilot_id = cfg.get("pilot_id", "unknown")
    tasks = cfg["tasks"]

    api_key = os.environ.get("OPENAI_API_KEY", "").strip()
    _def_base = "https://api.openai.com/v1"
    _def_model = "gpt-4o-mini"
    base_url = ((os.environ.get("OPENAI_BASE_URL") or _def_base).strip() or _def_base)
    model = ((os.environ.get("LLM_MODEL") or _def_model).strip() or _def_model)

    if args.dry_run:
        print(f"dry-run: pilot={pilot_id!r}  {len(tasks)} Python task(s)  model={model!r}")
        for t in tasks:
            print(f"  - {t['id']}")
        return 0

    if not api_key:
        print("OPENAI_API_KEY not set; use --dry-run", file=sys.stderr)
        return 2

    failed = 0
    for task in tasks:
        tid = task["id"]
        prompt_rel = task["prompt_rel"]
        prompt_path = root / "eval" / prompt_rel
        if not prompt_path.is_file():
            rec = {
                "pilot_id": pilot_id,
                "task_id": tid,
                "ts": ts,
                "runner": "llm_python",
                "pass": False,
                "error": f"missing prompt: {prompt_rel}",
            }
            with open(out, "a", encoding="utf-8") as lf:
                lf.write(json.dumps(rec) + "\n")
            print(json.dumps(rec), file=sys.stderr)
            failed += 1
            continue

        prompt_text = prompt_path.read_text(encoding="utf-8")
        starter_path = prompt_path.parent / "starter.lir"
        ref_block = ""
        if starter_path.is_file():
            ref_block = (
                "\n\nReference LIR pipeline (parity with the LIR LLM eval — implement equivalent Python):\n"
                + starter_path.read_text(encoding="utf-8").strip()
                + "\n"
            )
        if task.get("stdin"):
            stdin_hint = (
                "\n\nExecution (grading harness): stdin will be exactly one line:\n"
                + repr(task["stdin"])
                + "\nParse it as JSON to get the list of integers."
            )
        else:
            stdin_hint = (
                "\n\nExecution (grading harness): **stdin is empty** — your program must not read stdin. "
                "Match the reference pipeline semantics above."
            )
        user = f"{prompt_text.strip()}{ref_block}{stdin_hint}\n"
        messages = [
            {"role": "system", "content": system_content},
            {"role": "user", "content": user},
        ]
        tok_sys, tok_user = split_system_user_tokens(model, system_content, user)
        try:
            raw, usage = chat_complete(base_url, api_key, model, messages)
        except urllib.error.HTTPError as e:
            err_body = e.read().decode("utf-8", errors="replace")[:500]
            rec = {
                "pilot_id": pilot_id,
                "task_id": tid,
                "ts": ts,
                "runner": "llm_python",
                "pass": False,
                "error": f"http {e.code}: {err_body}",
            }
            with open(out, "a", encoding="utf-8") as lf:
                lf.write(json.dumps(rec) + "\n")
            print(json.dumps(rec), file=sys.stderr)
            failed += 1
            continue
        except OSError as e:
            rec = {
                "pilot_id": pilot_id,
                "task_id": tid,
                "ts": ts,
                "runner": "llm_python",
                "pass": False,
                "error": str(e),
            }
            with open(out, "a", encoding="utf-8") as lf:
                lf.write(json.dumps(rec) + "\n")
            print(json.dumps(rec), file=sys.stderr)
            failed += 1
            continue

        src = extract_python(raw)
        meta = {
            "runner": "llm_python",
            "model": model,
            "tokens_prompt": usage.get("prompt_tokens"),
            "tokens_completion": usage.get("completion_tokens"),
            "tokens_prompt_system": tok_sys,
            "tokens_prompt_user": tok_user,
        }
        with open(out, "a", encoding="utf-8") as lf:
            lf.write(
                json.dumps(
                    {
                        "pilot_id": pilot_id,
                        "task_id": tid,
                        "ts": ts,
                        "event": "llm_response",
                        **meta,
                        "py_chars": len(src),
                        "py_preview": src[:800],
                    }
                )
                + "\n"
            )

        want = norm_out(task["expect_stdout"])
        stdin_data = task.get("stdin") or ""

        with tempfile.NamedTemporaryFile(
            mode="w",
            suffix=".py",
            delete=False,
            encoding="utf-8",
        ) as tmp:
            tmp.write(src)
            tmp_path = Path(tmp.name)

        try:
            proc = subprocess.run(
                [sys.executable, str(tmp_path)],
                cwd=root,
                input=stdin_data,
                text=True,
                capture_output=True,
                timeout=30,
            )
        except subprocess.TimeoutExpired:
            ok = False
            got = ""
            err_tail = "timeout after 30s"
            code = -1
        else:
            code = proc.returncode
            got = norm_out(proc.stdout)
            err_tail = norm_out(proc.stderr)[:1200]
            ok = code == 0 and got == want

        tmp_path.unlink(missing_ok=True)

        rec = {
            "pilot_id": pilot_id,
            "task_id": tid,
            "ts": ts,
            "pass": ok,
            **meta,
            "exit_code": code,
            "stdout_got": got[:200],
            "stdout_want": want[:200],
        }
        if not ok and err_tail:
            rec["stderr_got"] = err_tail
        with open(out, "a", encoding="utf-8") as lf:
            lf.write(json.dumps(rec) + "\n")
        if not ok:
            print(json.dumps(rec), file=sys.stderr)
            failed += 1

    if failed:
        print(f"llm_python_eval: {failed} task(s) failed (see {out})", file=sys.stderr)
        return 1
    print(f"llm_python_eval: all tasks passed ({len(tasks)} pilot task(s))")
    return 0


if __name__ == "__main__":
    sys.exit(main())
