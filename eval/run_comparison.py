#!/usr/bin/env python3
"""
Orchestrate a **local comparison dry run**: Tier A manifest (LIR CLI) + Python baselines.
Optional **`--llm`** runs live [`run_llm_eval.py`](run_llm_eval.py) (needs `OPENAI_API_KEY`, costs tokens).

Does not replace CI (CI still runs `run_tier_a`, baseline, and `run_llm_eval --dry-run` separately).
Use this when you are ready to **record** a comparison session (wall time; LLM tokens in `results_llm.ndjson`).

Usage (repo root):
  python3 eval/run_comparison.py
  python3 eval/run_comparison.py --llm
  python3 eval/run_comparison.py --json-out eval/comparison_summary.json
  COMPARISON_JSON=1 python3 eval/run_comparison.py
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def _run_step(argv: list[str]) -> tuple[int, float]:
    t0 = time.perf_counter()
    p = subprocess.run(argv, cwd=ROOT)
    elapsed = time.perf_counter() - t0
    return p.returncode, elapsed


def _git_rev() -> str | None:
    try:
        p = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=5,
        )
        if p.returncode == 0:
            return p.stdout.strip()
    except (OSError, subprocess.TimeoutExpired):
        pass
    return None


def main() -> int:
    ap = argparse.ArgumentParser(description="Tier A + Python baseline comparison orchestrator")
    ap.add_argument(
        "--llm",
        action="store_true",
        help="Also run live LLM eval (requires OPENAI_API_KEY; costs tokens)",
    )
    ap.add_argument(
        "--json-out",
        type=Path,
        default=None,
        help="Write machine-readable summary to this path (also set COMPARISON_JSON=1 for default path)",
    )
    args = ap.parse_args()

    py = sys.executable
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    steps: dict[str, dict] = {}

    code, sec = _run_step([py, str(ROOT / "eval" / "run_tier_a.py")])
    steps["tier_a"] = {"exit_code": code, "seconds": round(sec, 3)}
    if code != 0:
        _print_summary(ts, steps, skipped_llm=None)
        _maybe_write_json(args, ts, steps)
        return code

    code, sec = _run_step([py, str(ROOT / "eval" / "baseline" / "python" / "run_all.py")])
    steps["baseline_python"] = {"exit_code": code, "seconds": round(sec, 3)}
    if code != 0:
        _print_summary(ts, steps, skipped_llm=None)
        _maybe_write_json(args, ts, steps)
        return code

    if args.llm:
        if not os.environ.get("OPENAI_API_KEY", "").strip():
            print("error: --llm requires OPENAI_API_KEY", file=sys.stderr)
            _print_summary(ts, steps, skipped_llm="missing OPENAI_API_KEY (cannot run llm_eval)")
            _maybe_write_json(args, ts, steps)
            return 2
        code, sec = _run_step([py, str(ROOT / "eval" / "run_llm_eval.py")])
        steps["llm_eval"] = {"exit_code": code, "seconds": round(sec, 3)}
        _print_summary(ts, steps, skipped_llm=None)
        _maybe_write_json(args, ts, steps)
        return code

    _print_summary(
        ts,
        steps,
        skipped_llm="pass --llm and OPENAI_API_KEY for live LLM grading (costs tokens)",
    )
    _maybe_write_json(args, ts, steps)
    return 0


def _print_summary(ts: str, steps: dict, skipped_llm: str | None) -> None:
    print(f"comparison — {ts}")
    for name, data in steps.items():
        status = "PASS" if data["exit_code"] == 0 else "FAIL"
        print(f"  {name:20} {status}  ({data['seconds']}s)  exit={data['exit_code']}")
    if skipped_llm:
        print(f"  {'llm_eval':20} SKIP  — {skipped_llm}")


def _maybe_write_json(args: argparse.Namespace, ts: str, steps: dict) -> None:
    out = args.json_out
    if out is None and os.environ.get("COMPARISON_JSON"):
        out = ROOT / "eval" / "comparison_summary.json"
    if out:
        _write_json(out, ts, steps)


def _write_json(path: Path, ts: str, steps: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    doc = {
        "schema_version": 1,
        "ts": ts,
        "git_rev": _git_rev(),
        "tier_a": steps.get("tier_a"),
        "baseline_python": steps.get("baseline_python"),
        "llm_eval": steps.get("llm_eval"),
        "notes": (
            "Agent turns and subjective effort are recorded outside this repo; "
            "LLM prompt/completion tokens appear in eval/results_llm.ndjson per assertion when llm_eval runs."
        ),
    }
    path.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {path}")


if __name__ == "__main__":
    sys.exit(main())
