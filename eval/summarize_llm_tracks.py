#!/usr/bin/env python3
"""
Aggregate **LLM eval** NDJSON logs for **LIR** (`run_llm_eval.py`) and **LYTR** (`run_llm_lytr_eval.py`):
per-task pass/fail, tokens from `event=llm_response`, and a side-by-side comparison on **shared task ids**.

Usage (repo root):

  python3 eval/summarize_llm_tracks.py
  python3 eval/summarize_llm_tracks.py --lir eval/results_llm.ndjson --lytr eval/results_llm_lytr.ndjson
  python3 eval/summarize_llm_tracks.py --json-out eval/llm_tracks_summary.json

Missing files are skipped (only arms with existing logs are reported).
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

_eval_dir = Path(__file__).resolve().parent
if str(_eval_dir) not in sys.path:
    sys.path.insert(0, str(_eval_dir))

from pilot_report import git_rev

ROOT = Path(__file__).resolve().parent.parent


def _iter_ndjson(path: Path) -> list[dict]:
    if not path.is_file():
        return []
    out: list[dict] = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                out.append(json.loads(line))
            except json.JSONDecodeError:
                continue
    return out


def _parse_task_passes_last_block(path: Path) -> dict[str, bool]:
    """
    task_id -> True iff the **last** run for that task (each run starts at ``llm_response``)
    had no top-level error and all following assertion lines passed until the next ``llm_response``.
    Logs may append multiple experiments; last run wins.
    """
    rows = _iter_ndjson(path)
    last_pass: dict[str, bool] = {}
    i = 0
    n = len(rows)
    while i < n:
        o = rows[i]
        if o.get("event") != "llm_response":
            i += 1
            continue
        tid = str(o["task_id"])
        i += 1
        ok = True
        while i < n:
            o2 = rows[i]
            if o2.get("event") == "llm_response":
                break
            if str(o2.get("task_id", "")) != tid:
                i += 1
                continue
            if o2.get("error"):
                ok = False
            if o2.get("pass") is False:
                ok = False
            i += 1
        last_pass[tid] = ok
    return last_pass


def _last_llm_response_tokens(path: Path) -> tuple[dict[str, dict], dict[str, int]]:
    """
    Last ``llm_response`` line per task_id wins (append-friendly logs).
    Returns (per_task detail dict, per_task prompt+completion sum).
    """
    rows = _iter_ndjson(path)
    last: dict[str, dict] = {}
    for o in rows:
        if o.get("event") != "llm_response":
            continue
        tid = o.get("task_id")
        if not tid:
            continue
        last[str(tid)] = dict(o)
    per_tokens: dict[str, int] = {}
    for tid, o in last.items():
        tp = int(o.get("tokens_prompt") or 0)
        tc = int(o.get("tokens_completion") or 0)
        per_tokens[tid] = tp + tc
    return last, per_tokens


def _tokens_totals_from_last_response(path: Path) -> dict[str, int]:
    """Sum API tokens_prompt / tokens_completion using last llm_response per task only."""
    _, per_task = _last_llm_response_tokens(path)
    tot = {"tokens_prompt": 0, "tokens_completion": 0, "prompt_plus_completion": 0}
    rows = _iter_ndjson(path)
    last_resp: dict[str, dict] = {}
    for o in rows:
        if o.get("event") != "llm_response":
            continue
        tid = o.get("task_id")
        if tid:
            last_resp[str(tid)] = o
    for o in last_resp.values():
        tot["tokens_prompt"] += int(o.get("tokens_prompt") or 0)
        tot["tokens_completion"] += int(o.get("tokens_completion") or 0)
    tot["prompt_plus_completion"] = tot["tokens_prompt"] + tot["tokens_completion"]
    return tot


def main() -> int:
    ap = argparse.ArgumentParser(description="Summarize LIR + LYTR LLM eval NDJSON logs")
    ap.add_argument(
        "--lir",
        type=Path,
        default=ROOT / "eval" / "results_llm.ndjson",
        help="Path to LIR LLM results NDJSON",
    )
    ap.add_argument(
        "--lytr",
        type=Path,
        default=ROOT / "eval" / "results_llm_lytr.ndjson",
        help="Path to LYTR LLM results NDJSON",
    )
    ap.add_argument(
        "--json-out",
        type=Path,
        default=None,
        help="Write full summary JSON to this path",
    )
    args = ap.parse_args()

    lir_path = args.lir if args.lir.is_absolute() else ROOT / args.lir
    lytr_path = args.lytr if args.lytr.is_absolute() else ROOT / args.lytr

    arms: dict[str, dict] = {}

    if lir_path.is_file():
        lir_pass = _parse_task_passes_last_block(lir_path)
        per_lir, tok_lir = _last_llm_response_tokens(lir_path)
        tot_lir = _tokens_totals_from_last_response(lir_path)
        arms["lir"] = {
            "path": str(lir_path.resolve()),
            "tasks_n": len(lir_pass),
            "pass_n": sum(1 for v in lir_pass.values() if v),
            "pass_by_task": lir_pass,
            "tokens_total": {
                "prompt": tot_lir["tokens_prompt"],
                "completion": tot_lir["tokens_completion"],
                "prompt_plus_completion": tot_lir["prompt_plus_completion"],
            },
            "per_task_tokens": tok_lir,
            "per_task": per_lir,
        }
    else:
        arms["lir"] = {"path": str(lir_path), "missing": True}

    if lytr_path.is_file():
        lytr_pass = _parse_task_passes_last_block(lytr_path)
        per_ly, tok_ly = _last_llm_response_tokens(lytr_path)
        tot_ly = _tokens_totals_from_last_response(lytr_path)
        arms["lytr"] = {
            "path": str(lytr_path.resolve()),
            "tasks_n": len(lytr_pass),
            "pass_n": sum(1 for v in lytr_pass.values() if v),
            "pass_by_task": lytr_pass,
            "tokens_total": {
                "prompt": tot_ly["tokens_prompt"],
                "completion": tot_ly["tokens_completion"],
                "prompt_plus_completion": tot_ly["prompt_plus_completion"],
            },
            "per_task_tokens": tok_ly,
            "per_task": per_ly,
        }
    else:
        arms["lytr"] = {"path": str(lytr_path), "missing": True}

    # Shared task ids (intersection) for pipeline-cost comparison
    lir_tasks = set()
    lytr_tasks = set()
    if not arms["lir"].get("missing"):
        lir_tasks = set(arms["lir"]["pass_by_task"].keys())
    if not arms["lytr"].get("missing"):
        lytr_tasks = set(arms["lytr"]["pass_by_task"].keys())
    shared = sorted(lir_tasks & lytr_tasks)

    comparison: list[dict] = []
    sum_lir_tc = sum_lytr_tc = 0
    for tid in shared:
        lp = arms["lir"]["pass_by_task"].get(tid)
        yp = arms["lytr"]["pass_by_task"].get(tid)
        lt = arms["lir"]["per_task_tokens"].get(tid, 0)
        yt = arms["lytr"]["per_task_tokens"].get(tid, 0)
        sum_lir_tc += lt
        sum_lytr_tc += yt
        ratio = None
        if lt > 0 and yt > 0:
            ratio = round(yt / lt, 4)
        row = {
            "task_id": tid,
            "lir_pass": lp,
            "lytr_pass": yp,
            "tokens_lir": lt,
            "tokens_lytr": yt,
            "lytr_over_lir_token_ratio": ratio,
        }
        comparison.append(row)

    doc = {
        "schema_version": 1,
        "git_rev": git_rev(ROOT),
        "shared_task_ids": shared,
        "shared_token_totals": {
            "sum_tokens_lir": sum_lir_tc,
            "sum_tokens_lytr": sum_lytr_tc,
            "lytr_over_lir_ratio": round(sum_lytr_tc / sum_lir_tc, 4)
            if sum_lir_tc > 0 and sum_lytr_tc > 0
            else None,
        },
        "comparison": comparison,
        "arms": arms,
    }

    if args.json_out:
        out_p = args.json_out if args.json_out.is_absolute() else ROOT / args.json_out
        out_p.parent.mkdir(parents=True, exist_ok=True)
        out_p.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {out_p}")

    # Human-readable summary
    print("llm_tracks_summary")
    if arms["lir"].get("missing"):
        print(f"  LIR:  (missing) {arms['lir']['path']}")
    else:
        a = arms["lir"]
        t = a["tokens_total"]
        print(
            f"  LIR:  {a['pass_n']}/{a['tasks_n']} tasks pass | "
            f"tokens {t['prompt']}+{t['completion']}={t['prompt_plus_completion']} (prompt+completion)"
        )
    if arms["lytr"].get("missing"):
        print(f"  LYTR: (missing) {arms['lytr']['path']}")
    else:
        a = arms["lytr"]
        t = a["tokens_total"]
        print(
            f"  LYTR: {a['pass_n']}/{a['tasks_n']} tasks pass | "
            f"tokens {t['prompt']}+{t['completion']}={t['prompt_plus_completion']} (prompt+completion)"
        )

    if shared:
        st = doc["shared_token_totals"]
        print(f"  Shared tasks ({len(shared)}): token sums  LIR={st['sum_tokens_lir']}  LYTR={st['sum_tokens_lytr']}  ratio={st['lytr_over_lir_ratio']}")
        for row in comparison:
            r = row["lytr_over_lir_token_ratio"]
            rs = f"{r}" if r is not None else "—"
            print(
                f"    {row['task_id']}:  LIR pass={row['lir_pass']} tok={row['tokens_lir']} | "
                f"LYTR pass={row['lytr_pass']} tok={row['tokens_lytr']} | lytr/lir tok {rs}"
            )
    else:
        print("  (no shared task ids in both logs — run both LLM evals on overlapping tasks to compare)")

    return 0


if __name__ == "__main__":
    sys.exit(main())
