"""
Derive pilot A/B comparison metrics (accuracy, performance, tokens, output size) from arm NDJSON logs.

Can be run after ``run_pilot_ab.py`` or standalone:

  python3 eval/pilot_comparison.py --run-id 20260409T130238Z
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from pilot_report import aggregate_tokens_ndjson
from pilot_thesis_metrics import build_thesis_metrics


def _parse_python_passes(path: Path) -> dict[str, bool]:
    """task_id -> final grade pass (last wins)."""
    out: dict[str, bool] = {}
    if not path.is_file():
        return out
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                o = json.loads(line)
            except json.JSONDecodeError:
                continue
            if o.get("event") == "llm_response":
                continue
            tid = o.get("task_id")
            if tid is None or "pass" not in o:
                continue
            out[str(tid)] = bool(o["pass"])
    return out


def _parse_lir_passes(path: Path) -> dict[str, bool]:
    """task_id -> all assertions passed (and no top-level error for that task)."""
    llm_tasks: set[str] = set()
    failed: set[str] = set()
    if not path.is_file():
        return {}
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                o = json.loads(line)
            except json.JSONDecodeError:
                continue
            tid = o.get("task_id")
            if tid is None:
                continue
            tid = str(tid)
            if o.get("event") == "llm_response":
                llm_tasks.add(tid)
                continue
            if o.get("error"):
                failed.add(tid)
            if o.get("pass") is False:
                failed.add(tid)
    return {tid: tid not in failed for tid in llm_tasks}


def _parity(lir_ok: bool, py_ok: bool) -> str:
    if lir_ok and py_ok:
        return "both_pass"
    if lir_ok and not py_ok:
        return "lir_only"
    if not lir_ok and py_ok:
        return "python_only"
    return "both_fail"


def build_comparison(
    run_id: str,
    *,
    wall_seconds_lir: float,
    wall_seconds_python: float,
    wall_seconds_total: float,
    per_lir: dict[str, dict[str, Any]],
    per_py: dict[str, dict[str, Any]],
    tot_lir: dict[str, int],
    tot_py: dict[str, int],
    lir_log: Path,
    py_log: Path,
    ts: str,
) -> dict[str, Any]:
    """Single JSON-serializable comparison object (schema_version 1)."""
    py_pass = _parse_python_passes(py_log)
    lir_pass = _parse_lir_passes(lir_log)

    task_ids = sorted(
        set(per_lir.keys()) | set(per_py.keys()) | set(py_pass.keys()) | set(lir_pass.keys()),
    )
    n = len(task_ids)

    per_task: list[dict[str, Any]] = []
    both_pass = lir_only = py_only = both_fail = 0
    for tid in task_ids:
        lo = lir_pass.get(tid, False)
        po = py_pass.get(tid, False)
        p = _parity(lo, po)
        if p == "both_pass":
            both_pass += 1
        elif p == "lir_only":
            lir_only += 1
        elif p == "python_only":
            py_only += 1
        else:
            both_fail += 1

        pl = per_lir.get(tid) or {}
        pp = per_py.get(tid) or {}
        lc = pl.get("lir_chars")
        pc = pp.get("py_chars")
        row: dict[str, Any] = {
            "task_id": tid,
            "lir_pass": lo,
            "python_pass": po,
            "parity": p,
        }
        if lc is not None:
            row["lir_chars"] = lc
        if pc is not None:
            row["py_chars"] = pc
        if isinstance(lc, int) and isinstance(pc, int) and lc > 0:
            row["python_chars_per_lir_chars"] = round(pc / lc, 3)
        per_task.append(row)

    lir_pass_n = sum(1 for tid in task_ids if lir_pass.get(tid))
    py_pass_n = sum(1 for tid in task_ids if py_pass.get(tid))

    n_safe = max(n, 1)
    sp_lir = tot_lir.get("tokens_prompt", 0) + tot_lir.get("tokens_completion", 0)
    sp_py = tot_py.get("tokens_prompt", 0) + tot_py.get("tokens_completion", 0)

    def _eff(tot: dict[str, int], pass_n: int) -> dict[str, float]:
        pn = max(pass_n, 1)
        return {
            "api_total_tokens_per_task_avg": round(
                (tot.get("tokens_prompt", 0) + tot.get("tokens_completion", 0)) / n_safe,
                2,
            ),
            "api_prompt_tokens_per_task_avg": round(tot.get("tokens_prompt", 0) / n_safe, 2),
            "api_completion_tokens_per_task_avg": round(tot.get("tokens_completion", 0) / n_safe, 2),
            "api_total_tokens_per_passing_task_avg": round(
                (tot.get("tokens_prompt", 0) + tot.get("tokens_completion", 0)) / pn,
                2,
            ),
        }

    retry_lir = any((per_lir.get(t) or {}).get("retry") for t in task_ids)
    if task_ids:
        attempts_lir = max(
            ((per_lir.get(t) or {}).get("attempt") or 1) for t in task_ids
        )
    else:
        attempts_lir = 1

    speedup_py = None
    if wall_seconds_python > 0:
        speedup_py = round(wall_seconds_lir / wall_seconds_python, 3)

    accuracy_block = {
        "tasks": n,
        "lir_pass_count": lir_pass_n,
        "python_pass_count": py_pass_n,
        "both_pass_count": both_pass,
        "lir_only_pass_count": lir_only,
        "python_only_pass_count": py_only,
        "both_fail_count": both_fail,
        "agreement_rate": round((both_pass + both_fail) / n_safe, 4) if n else 0.0,
    }
    reliability_lir_inner = {
        "retry_used_any_task": bool(retry_lir),
        "max_attempt_any_task": int(attempts_lir),
    }
    thesis_metrics = build_thesis_metrics(
        accuracy_block,
        tot_lir,
        tot_py,
        reliability_lir=reliability_lir_inner,
    )

    return {
        "schema_version": 1,
        "event": "pilot_comparison",
        "run_id": run_id,
        "ts": ts,
        "accuracy": accuracy_block,
        "performance": {
            "wall_seconds_lir": round(wall_seconds_lir, 3),
            "wall_seconds_python": round(wall_seconds_python, 3),
            "wall_seconds_total_sequential": round(wall_seconds_total, 3),
            "throughput_tasks_per_minute_lir": round(n / max(wall_seconds_lir, 1e-9) * 60, 3),
            "throughput_tasks_per_minute_python": round(n / max(wall_seconds_python, 1e-9) * 60, 3),
            "lir_arm_slower_ratio_vs_python": speedup_py,
        },
        "tokens": {
            "lir": {
                "api_prompt": tot_lir.get("tokens_prompt", 0),
                "api_completion": tot_lir.get("tokens_completion", 0),
                "content_system": tot_lir.get("tokens_prompt_system", 0),
                "content_user": tot_lir.get("tokens_prompt_user", 0),
                "api_total": sp_lir,
            },
            "python": {
                "api_prompt": tot_py.get("tokens_prompt", 0),
                "api_completion": tot_py.get("tokens_completion", 0),
                "content_system": tot_py.get("tokens_prompt_system", 0),
                "content_user": tot_py.get("tokens_prompt_user", 0),
                "api_total": sp_py,
            },
            "delta_python_minus_lir_api_total": sp_py - sp_lir,
        },
        "efficiency": {
            "lir": _eff(tot_lir, lir_pass_n),
            "python": _eff(tot_py, py_pass_n),
        },
        "output_size": {"per_task": per_task},
        "reliability": {
            "lir": reliability_lir_inner,
            "python": {"retry_used_any_task": False},
        },
        "thesis_metrics": thesis_metrics,
    }


def write_comparison_json(repo_root: Path, payload: dict[str, Any]) -> Path:
    run_id = payload["run_id"]
    out = repo_root / "eval" / f"results_pilot_comparison_{run_id}.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    with open(out, "w", encoding="utf-8") as f:
        json.dump(payload, f, indent=2, ensure_ascii=False)
        f.write("\n")
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description="Build pilot comparison JSON from arm logs")
    ap.add_argument("--run-id", required=True, help="Same run_id as results_pilot_lir_<id>.ndjson")
    ap.add_argument(
        "--ab-report",
        default="",
        help="Path to results_pilot_ab_<id>.ndjson (default: eval/results_pilot_ab_<run-id>.ndjson)",
    )
    args = ap.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    run_id = args.run_id.strip()
    ab_path = Path(args.ab_report) if (args.ab_report or "").strip() else repo_root / "eval" / f"results_pilot_ab_{run_id}.ndjson"
    if not ab_path.is_absolute():
        ab_path = repo_root / ab_path

    lir_log = repo_root / "eval" / f"results_pilot_lir_{run_id}.ndjson"
    py_log = repo_root / "eval" / f"results_pilot_python_{run_id}.ndjson"

    if not ab_path.is_file():
        print(f"missing {ab_path}", file=sys.stderr)
        return 1

    rows = []
    with open(ab_path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                rows.append(json.loads(line))

    done = next((r for r in rows if r.get("event") == "pilot_ab_complete"), {})
    tok_lir = next((r for r in rows if r.get("event") == "pilot_tokens" and r.get("arm") == "lir"), {})
    tok_py = next((r for r in rows if r.get("event") == "pilot_tokens" and r.get("arm") == "python"), {})
    start = next((r for r in rows if r.get("event") == "pilot_ab_start"), {})

    per_lir, tot_lir = aggregate_tokens_ndjson(lir_log)
    per_py, tot_py = aggregate_tokens_ndjson(py_log)

    payload = build_comparison(
        run_id,
        wall_seconds_lir=float(done.get("wall_seconds_lir") or 0),
        wall_seconds_python=float(done.get("wall_seconds_python") or 0),
        wall_seconds_total=float(done.get("wall_seconds_total") or 0),
        per_lir=per_lir,
        per_py=per_py,
        tot_lir=tot_lir,
        tot_py=tot_py,
        lir_log=lir_log,
        py_log=py_log,
        ts=str(start.get("ts") or done.get("ts") or ""),
    )
    out = write_comparison_json(repo_root, payload)
    print(f"wrote {out.relative_to(repo_root)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
