#!/usr/bin/env python3
"""
Run the A/B pilot: LLM → LIR (pilot manifest) then LLM → Python (pilot manifest),
then write a combined NDJSON report with wall times, env, git rev, and token totals.

Usage (repo root, requires OPENAI_API_KEY for live runs):
  python3 eval/run_pilot_ab.py
  python3 eval/run_pilot_ab.py --dry-run

Each run uses a fresh run_id (UTC timestamp) so arm logs and the combined report do not
append to previous runs. Override with --run-id / --report (see --help).
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
_eval = ROOT / "eval"
sys.path.insert(0, str(_eval))

from pilot_comparison import build_comparison, write_comparison_json
from pilot_report import aggregate_tokens_ndjson, env_snapshot, git_rev
from tier_a_lib import lir_base_cmd


def main() -> int:
    ap = argparse.ArgumentParser(description="Pilot A/B: LIR LLM eval + Python LLM eval + report")
    ap.add_argument("--dry-run", action="store_true", help="Forward --dry-run to both harnesses")
    ap.add_argument(
        "--run-id",
        type=str,
        default="",
        help=(
            "Suffix for arm log filenames (default: UTC wall time YYYYMMDDTHHMMSSZ). "
            "Arms write eval/results_pilot_lir_<id>.ndjson and eval/results_pilot_python_<id>.ndjson."
        ),
    )
    ap.add_argument(
        "--report",
        type=str,
        default="",
        help="Combined NDJSON report (default: eval/results_pilot_ab_<run-id>.ndjson under repo root)",
    )
    args = ap.parse_args()

    py = sys.executable
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    run_id = (args.run_id or "").strip() or datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")

    lir_log = ROOT / "eval" / f"results_pilot_lir_{run_id}.ndjson"
    py_log = ROOT / "eval" / f"results_pilot_python_{run_id}.ndjson"

    if (args.report or "").strip():
        report_path = Path(args.report.strip())
    else:
        report_path = ROOT / "eval" / f"results_pilot_ab_{run_id}.ndjson"
    if not report_path.is_absolute():
        report_path = ROOT / report_path

    meta = {
        "event": "pilot_ab_start",
        "schema_version": 1,
        "ts": ts,
        "run_id": run_id,
        "pilot_id": "ab-v1",
        "dry_run": args.dry_run,
        "git_rev": git_rev(ROOT),
        "env": env_snapshot(),
        "lir_invoke": " ".join(lir_base_cmd()),
        "fair_prompts": True,
    }

    common = [
        py,
        str(_eval / "run_llm_eval.py"),
        "--manifest",
        "eval/pilot/lir_manifest.json",
        "--results",
        str(lir_log.relative_to(ROOT)),
        "--fair-prompts",
    ]
    if args.dry_run:
        common.append("--dry-run")

    t0 = time.perf_counter()
    r1 = subprocess.run(common, cwd=ROOT)
    t_lir = time.perf_counter() - t0

    common_py = [
        py,
        str(_eval / "run_llm_python_eval.py"),
        "--manifest",
        "eval/pilot/python_manifest.json",
        "--results",
        str(py_log.relative_to(ROOT)),
        "--fair-prompts",
    ]
    if args.dry_run:
        common_py.append("--dry-run")

    t1 = time.perf_counter()
    r2 = subprocess.run(common_py, cwd=ROOT)
    t_py = time.perf_counter() - t1
    t_total = time.perf_counter() - t0

    arm_lir = {
        "event": "pilot_arm",
        "arm": "lir",
        "ts": ts,
        "exit_code": r1.returncode,
        "wall_seconds": round(t_lir, 3),
        "results_file": str(lir_log.relative_to(ROOT)),
    }
    arm_py = {
        "event": "pilot_arm",
        "arm": "python",
        "ts": ts,
        "exit_code": r2.returncode,
        "wall_seconds": round(t_py, 3),
        "results_file": str(py_log.relative_to(ROOT)),
    }

    if args.dry_run:
        z = {"tokens_prompt": 0, "tokens_completion": 0, "tokens_prompt_system": 0, "tokens_prompt_user": 0}
        per_lir, tot_lir = {}, z.copy()
        per_py, tot_py = {}, z.copy()
    else:
        per_lir, tot_lir = aggregate_tokens_ndjson(lir_log)
        per_py, tot_py = aggregate_tokens_ndjson(py_log)

    tok_lir = {
        "event": "pilot_tokens",
        "arm": "lir",
        "ts": ts,
        "dry_run_skipped": args.dry_run,
        "total_tokens_prompt": tot_lir["tokens_prompt"],
        "total_tokens_completion": tot_lir["tokens_completion"],
        "total_tokens_prompt_system": tot_lir["tokens_prompt_system"],
        "total_tokens_prompt_user": tot_lir["tokens_prompt_user"],
        "per_task": per_lir,
    }
    tok_py = {
        "event": "pilot_tokens",
        "arm": "python",
        "ts": ts,
        "dry_run_skipped": args.dry_run,
        "total_tokens_prompt": tot_py["tokens_prompt"],
        "total_tokens_completion": tot_py["tokens_completion"],
        "total_tokens_prompt_system": tot_py["tokens_prompt_system"],
        "total_tokens_prompt_user": tot_py["tokens_prompt_user"],
        "per_task": per_py,
    }

    done = {
        "event": "pilot_ab_complete",
        "ts": ts,
        "lir_exit_code": r1.returncode,
        "python_exit_code": r2.returncode,
        "wall_seconds_lir": round(t_lir, 3),
        "wall_seconds_python": round(t_py, 3),
        "wall_seconds_total": round(t_total, 3),
        "lir_tokens_prompt": tot_lir["tokens_prompt"],
        "lir_tokens_completion": tot_lir["tokens_completion"],
        "lir_tokens_prompt_system": tot_lir["tokens_prompt_system"],
        "lir_tokens_prompt_user": tot_lir["tokens_prompt_user"],
        "python_tokens_prompt": tot_py["tokens_prompt"],
        "python_tokens_completion": tot_py["tokens_completion"],
        "python_tokens_prompt_system": tot_py["tokens_prompt_system"],
        "python_tokens_prompt_user": tot_py["tokens_prompt_user"],
        "dry_run": args.dry_run,
    }

    report_path.parent.mkdir(parents=True, exist_ok=True)
    with open(report_path, "w", encoding="utf-8") as out:
        for row in (meta, arm_lir, arm_py, tok_lir, tok_py, done):
            out.write(json.dumps(row, ensure_ascii=False) + "\n")

    comparison = None
    comparison_path = None
    token_sum = (
        tot_lir["tokens_prompt"]
        + tot_lir["tokens_completion"]
        + tot_py["tokens_prompt"]
        + tot_py["tokens_completion"]
    )
    if not args.dry_run and token_sum > 0:
        comparison = build_comparison(
            run_id,
            wall_seconds_lir=t_lir,
            wall_seconds_python=t_py,
            wall_seconds_total=t_total,
            per_lir=per_lir,
            per_py=per_py,
            tot_lir=tot_lir,
            tot_py=tot_py,
            lir_log=lir_log,
            py_log=py_log,
            ts=ts,
        )
        comparison_path = write_comparison_json(ROOT, comparison)

    print(f"pilot_ab: run_id={run_id}  wrote {report_path.relative_to(ROOT)}")
    print(f"  LIR log: {lir_log.relative_to(ROOT)}")
    print(f"  Python log: {py_log.relative_to(ROOT)}")
    if not args.dry_run and token_sum > 0:
        print(
            f"  LIR:    {tot_lir['tokens_prompt']} prompt (API) = "
            f"{tot_lir['tokens_prompt_system']} system + {tot_lir['tokens_prompt_user']} user (content) + "
            f"{tot_lir['tokens_completion']} completion  ({t_lir:.1f}s wall)"
        )
        print(
            f"  Python: {tot_py['tokens_prompt']} prompt (API) = "
            f"{tot_py['tokens_prompt_system']} system + {tot_py['tokens_prompt_user']} user (content) + "
            f"{tot_py['tokens_completion']} completion  ({t_py:.1f}s wall)"
        )
        print(f"  Total wall (both arms): {t_total:.1f}s")
        if comparison is not None:
            print(f"  Comparison: {comparison_path.relative_to(ROOT)}")
            acc = comparison["accuracy"]
            perf = comparison["performance"]
            print(
                f"  Accuracy: both_pass {acc['both_pass_count']}/{acc['tasks']} | "
                f"lir {acc['lir_pass_count']} | python {acc['python_pass_count']}"
            )
            if perf.get("lir_arm_slower_ratio_vs_python") is not None:
                print(
                    f"  Performance: Python wall {perf['wall_seconds_python']}s vs LIR "
                    f"{perf['wall_seconds_lir']}s (~{perf['lir_arm_slower_ratio_vs_python']}× LIR time)"
                )
            tm = comparison.get("thesis_metrics") or {}
            ver = (tm.get("verdict") or {}).get("overall")
            hh = tm.get("head_to_head") or {}
            ps = tm.get("pillar_summary") or {}
            if ver:
                print(f"  Thesis verdict: {ver}")
            if ps:
                print(
                    "  Thesis pillars: "
                    f"1_marginal→LIR {ps.get('evidence_1_marginal_after_instruction_lir_wins')} | "
                    f"2_success/API→LIR {ps.get('evidence_2_success_per_token_lir_higher_api')} "
                    f"marginal→LIR {ps.get('evidence_2_success_per_token_lir_higher_marginal')} | "
                    f"2_no_retry {ps.get('evidence_2_no_lir_retry_rounds')} | "
                    f"3_completion→LIR {ps.get('evidence_3_completion_lir_wins')}"
                )
            if hh.get("ratio_python_div_lir_total_api_per_task") is not None:
                print(
                    f"  Thesis ratios (Python÷LIR, >1 means LIR cheaper on that slice): "
                    f"total_api {hh['ratio_python_div_lir_total_api_per_task']} | "
                    f"marginal {hh.get('ratio_python_div_lir_marginal_proxy_per_task')} | "
                    f"completion {hh.get('ratio_python_div_lir_completion_per_task')}"
                )
    elif args.dry_run:
        print("  (dry-run: no API tokens in logs)")

    if r1.returncode != 0:
        return r1.returncode
    if r2.returncode != 0:
        return r2.returncode
    return 0


if __name__ == "__main__":
    sys.exit(main())
