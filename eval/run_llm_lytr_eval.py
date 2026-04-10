#!/usr/bin/env python3
"""
LYTR LLM eval: send each task's prompt + starter `.lytr` to an OpenAI-compatible chat API,
write the model's program to a temp file, run the same merged manifest + hidden assertions as run_lytr_tier.py.

Usage (from repo root):
  python3 eval/run_llm_lytr_eval.py --dry-run
  OPENAI_API_KEY=... python3 eval/run_llm_lytr_eval.py

Env:
  OPENAI_API_KEY   — required for live runs (unless --dry-run)
  OPENAI_BASE_URL  — default https://api.openai.com/v1
  LLM_MODEL        — default gpt-4o-mini
  LLM_RETRY_ON_FAIL — if 1/true (default), one extra API round-trip when `lytr check` fails (stderr in prompt)

Tasks with manifest \"llm_eval\": false are skipped (same convention as run_llm_eval.py).
Logs to eval/results_llm_lytr.ndjson (gitignored).
"""
from __future__ import annotations

import argparse
import json
import os
import ssl
import sys
import tempfile
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

_eval_dir = Path(__file__).resolve().parent
if str(_eval_dir) not in sys.path:
    sys.path.insert(0, str(_eval_dir))

from prompt_token_split import split_system_user_tokens
from tier_a_lib import (
    HiddenAssertionsError,
    load_hidden_assertions,
    merge_manifest_and_hidden,
    repo_root,
    run_lytr_assertions_on_file,
)

LYTR_SYSTEM = """You output ONLY a valid LYTR 0.1 program (bootstrap subset).

Rules:
- Line 1 must be exactly: lytr/0.1
- Then: fn main() -> i32 { ... } or fn main() -> i64 { ... } with a single return expression (you may use let bindings and if/match).
- Types: i32, i64, bool, Result<i32, i32> or Result<i64, i64> (must match main's integer width). No user generics. Integer arithmetic: + - * / %; comparisons: == != < > <= >=.
- The program must print one integer line to stdout (the i32 result of main) — same contract as the starter.

Do not output LIR, Python, or prose before the program. Prefer raw lines starting with lytr/0.1 (no markdown fences).

Never use backtick characters (`) in the program — strip them if you must output code-like text inside strings is not supported in v0.1; avoid backticks entirely."""


def _merge_usage(a: dict, b: dict) -> dict:
    return {
        "prompt_tokens": (a.get("prompt_tokens") or 0) + (b.get("prompt_tokens") or 0),
        "completion_tokens": (a.get("completion_tokens") or 0) + (b.get("completion_tokens") or 0),
    }


def _append_records_ndjson(path: Path, records: list[dict]) -> None:
    with open(path, "a", encoding="utf-8") as lf:
        for r in records:
            lf.write(json.dumps(r) + "\n")


def _sanitize_lytr_source(s: str) -> str:
    s = s.replace("\r\n", "\n").replace("`", "")
    s = s.strip()
    return s + "\n" if s else ""


def extract_lytr(text: str) -> str:
    """Take the LYTR program from model output (handles ``` fences, strips stray backticks)."""
    t = text.replace("\r\n", "\n").strip()
    if "```" in t:
        parts = t.split("```")
        for block in parts:
            block = block.strip()
            if not block:
                continue
            lines = block.split("\n")
            if lines:
                fl = lines[0].strip()
                if fl in ("lytr", "rust", "text", "plaintext"):
                    block = "\n".join(lines[1:]).strip()
            if block.lstrip().startswith("lytr/"):
                return _sanitize_lytr_source(block)
    for i, line in enumerate(t.split("\n")):
        if line.strip().startswith("lytr/"):
            return _sanitize_lytr_source("\n".join(t.split("\n")[i:]).strip())
    return _sanitize_lytr_source(t)


def chat_complete(base_url: str, api_key: str, model: str, messages: list[dict]) -> tuple[str, dict]:
    url = base_url.rstrip("/") + "/chat/completions"
    body = json.dumps(
        {
            "model": model,
            "messages": messages,
            "temperature": 0,
        }
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
    ap = argparse.ArgumentParser(description="LYTR LLM eval (OpenAI-compatible API)")
    ap.add_argument("--dry-run", action="store_true", help="List tasks only; no network")
    ap.add_argument("--limit", type=int, default=0, help="Max LLM tasks (0 = no limit)")
    ap.add_argument("--task", type=str, default="", help="Run single task id substring (e.g. 001_range_sum)")
    ap.add_argument(
        "--manifest",
        type=str,
        default="eval/lytr_manifest.json",
        help="Manifest JSON path relative to repo root (or absolute)",
    )
    ap.add_argument(
        "--results",
        type=str,
        default="",
        help="NDJSON log path (default eval/results_llm_lytr.ndjson)",
    )
    args = ap.parse_args()

    root = repo_root()
    mp = Path(args.manifest)
    manifest_path = mp if mp.is_absolute() else root / mp
    out_log = Path(args.results) if args.results else root / "eval" / "results_llm_lytr.ndjson"
    if not out_log.is_absolute():
        out_log = root / out_log
    out_log.parent.mkdir(parents=True, exist_ok=True)

    with open(manifest_path, encoding="utf-8") as f:
        manifest = json.load(f)

    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    tier = manifest.get("tier", "LYTR")
    api_key = os.environ.get("OPENAI_API_KEY", "").strip()
    _def_base = "https://api.openai.com/v1"
    _def_model = "gpt-4o-mini"
    base_url = ((os.environ.get("OPENAI_BASE_URL") or _def_base).strip() or _def_base)
    model = ((os.environ.get("LLM_MODEL") or _def_model).strip() or _def_model)

    tasks = manifest["tasks"]
    llm_tasks = [t for t in tasks if t.get("llm_eval", True) is not False]
    if args.task:
        llm_tasks = [t for t in llm_tasks if args.task in t["id"]]
        if not llm_tasks:
            print(f"no task matches --task {args.task!r}", file=sys.stderr)
            return 1
    if args.limit > 0:
        llm_tasks = llm_tasks[: args.limit]

    if args.dry_run:
        print(f"dry-run: would run {len(llm_tasks)} task(s) with model={model!r}")
        for t in llm_tasks:
            starter = root / "eval" / t["starter"]
            n_m = len(t.get("assertions", []))
            try:
                n_h = len(load_hidden_assertions(starter))
            except HiddenAssertionsError as e:
                print(f"hidden assertions: {e}", file=sys.stderr)
                return 1
            print(f"  - {t['id']}: {n_m} manifest + {n_h} hidden assertion(s)")
        return 0

    if not api_key:
        print("OPENAI_API_KEY not set; use --dry-run to list tasks", file=sys.stderr)
        return 2

    retry_default = os.environ.get("LLM_RETRY_ON_FAIL", "1").strip().lower() in (
        "1",
        "true",
        "yes",
        "on",
    )

    failed = 0
    for task in llm_tasks:
        tid = task["id"]
        starter_rel = task["starter"]
        starter_path = root / "eval" / starter_rel
        prompt_path = starter_path.parent / "prompt.md"
        if not starter_path.is_file() or not prompt_path.is_file():
            rec = {
                "tier": tier,
                "task_id": tid,
                "ts": ts,
                "runner": "llm_lytr",
                "pass": False,
                "error": "missing program.lytr or prompt.md",
            }
            with open(out_log, "a", encoding="utf-8") as lf:
                lf.write(json.dumps(rec) + "\n")
            print(json.dumps(rec), file=sys.stderr)
            failed += 1
            continue

        try:
            combined_assertions = merge_manifest_and_hidden(
                task.get("assertions", []), starter_path
            )
        except HiddenAssertionsError as e:
            rec = {
                "tier": tier,
                "task_id": tid,
                "ts": ts,
                "runner": "llm_lytr",
                "pass": False,
                "error": str(e),
            }
            with open(out_log, "a", encoding="utf-8") as lf:
                lf.write(json.dumps(rec) + "\n")
            print(json.dumps(rec), file=sys.stderr)
            failed += 1
            continue

        prompt_text = prompt_path.read_text(encoding="utf-8")
        starter_text = starter_path.read_text(encoding="utf-8")
        user = (
            f"{prompt_text.strip()}\n\n"
            f"Current starter program (fix or replace as needed):\n\n{starter_text}\n"
        )
        messages = [
            {"role": "system", "content": LYTR_SYSTEM},
            {"role": "user", "content": user},
        ]
        tok_sys, tok_user = split_system_user_tokens(model, LYTR_SYSTEM, user)
        try:
            raw, usage = chat_complete(base_url, api_key, model, messages)
        except urllib.error.HTTPError as e:
            err_body = e.read().decode("utf-8", errors="replace")[:500]
            rec = {
                "tier": tier,
                "task_id": tid,
                "ts": ts,
                "runner": "llm_lytr",
                "pass": False,
                "error": f"http {e.code}: {err_body}",
            }
            with open(out_log, "a", encoding="utf-8") as lf:
                lf.write(json.dumps(rec) + "\n")
            print(json.dumps(rec), file=sys.stderr)
            failed += 1
            continue
        except Exception as e:  # noqa: BLE001
            rec = {
                "tier": tier,
                "task_id": tid,
                "ts": ts,
                "runner": "llm_lytr",
                "pass": False,
                "error": str(e),
            }
            with open(out_log, "a", encoding="utf-8") as lf:
                lf.write(json.dumps(rec) + "\n")
            print(json.dumps(rec), file=sys.stderr)
            failed += 1
            continue

        lytr_src = extract_lytr(raw)
        usage_total = usage
        meta = {
            "runner": "llm_lytr",
            "model": model,
            "tokens_prompt": usage.get("prompt_tokens"),
            "tokens_completion": usage.get("completion_tokens"),
            "tokens_prompt_system": tok_sys,
            "tokens_prompt_user": tok_user,
            "attempt": 1,
        }

        with tempfile.NamedTemporaryFile(
            mode="w",
            suffix=".lytr",
            delete=False,
            encoding="utf-8",
        ) as tmp:
            tmp.write(lytr_src)
            tmp_path = Path(tmp.name)

        try:
            fc, records = run_lytr_assertions_on_file(
                root,
                tmp_path,
                tid,
                tier,
                combined_assertions,
                ts,
                None,
                extra_fields=meta,
            )

            chk_idx = next(
                (
                    i
                    for i, a in enumerate(combined_assertions)
                    if a.get("kind") == "lytr_check"
                ),
                None,
            )
            if (
                retry_default
                and fc > 0
                and chk_idx is not None
                and chk_idx < len(records)
                and not records[chk_idx]["pass"]
            ):
                err_msg = records[chk_idx].get("stderr_got") or ""
                retry_messages = [
                    *messages,
                    {"role": "assistant", "content": raw},
                    {
                        "role": "user",
                        "content": (
                            "The program failed `lytr check`. Fix it.\n\n"
                            f"stderr:\n{err_msg}\n\n"
                            "Reply with ONLY the corrected LYTR program (line 1: lytr/0.1). "
                            "No markdown fences and no backtick (`) characters anywhere in the source."
                        ),
                    },
                ]
                try:
                    raw2, usage2 = chat_complete(
                        base_url, api_key, model, retry_messages
                    )
                except (urllib.error.HTTPError, OSError) as e:
                    print(json.dumps({"task_id": tid, "retry_error": str(e)}), file=sys.stderr)
                else:
                    usage_total = _merge_usage(usage, usage2)
                    lytr_src = extract_lytr(raw2)
                    raw = raw2
                    meta = {
                        "runner": "llm_lytr",
                        "model": model,
                        "tokens_prompt": usage_total.get("prompt_tokens"),
                        "tokens_completion": usage_total.get("completion_tokens"),
                        "tokens_prompt_system": tok_sys,
                        "tokens_prompt_user": tok_user,
                        "attempt": 2,
                        "retry": True,
                    }
                    tmp_path.write_text(lytr_src, encoding="utf-8")
                    fc, records = run_lytr_assertions_on_file(
                        root,
                        tmp_path,
                        tid,
                        tier,
                        combined_assertions,
                        ts,
                        None,
                        extra_fields=meta,
                    )
        finally:
            tmp_path.unlink(missing_ok=True)

        log_line = {
            "tier": tier,
            "task_id": tid,
            "ts": ts,
            "event": "llm_response",
            "runner": "llm_lytr",
            "model": model,
            "tokens_prompt": meta.get("tokens_prompt"),
            "tokens_completion": meta.get("tokens_completion"),
            "tokens_prompt_system": meta.get("tokens_prompt_system"),
            "tokens_prompt_user": meta.get("tokens_prompt_user"),
            "attempt": meta.get("attempt", 1),
            "retry": meta.get("retry", False),
            "lytr_chars": len(lytr_src),
            "lytr_preview": lytr_src[:800],
        }
        with open(out_log, "a", encoding="utf-8") as lf:
            lf.write(json.dumps(log_line) + "\n")
        _append_records_ndjson(out_log, records)

        failed += fc
        for r in records:
            if not r["pass"]:
                print(json.dumps(r), file=sys.stderr)

    if failed:
        print(f"llm_lytr_eval: {failed} failure(s) (see {out_log})", file=sys.stderr)
        return 1
    print(f"llm_lytr_eval: all assertions passed ({len(llm_tasks)} task(s))")
    return 0


if __name__ == "__main__":
    sys.exit(main())
