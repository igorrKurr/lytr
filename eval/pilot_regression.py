#!/usr/bin/env python3
"""
Phase 3 regression: compare a ``results_pilot_comparison_<run_id>.json`` to a frozen baseline.

Exits 0 if all checks pass; 1 if any check fails (with ``--fail``).

Usage (repo root):
  python3 eval/pilot_regression.py --run-id 20260409T132924Z
  python3 eval/pilot_regression.py --comparison eval/results_pilot_comparison_X.json
  python3 eval/pilot_regression.py --run-id X --baseline eval/baselines/pilot_ab_reference.json --fail

  # Emit baseline template from an existing comparison (edit ceilings before committing)
  python3 eval/pilot_regression.py --emit-baseline-from eval/results_pilot_comparison_X.json
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
_EVAL = ROOT / "eval"


def _load_json(p: Path) -> dict:
    with open(p, encoding="utf-8") as f:
        return json.load(f)


def _pillar_summary(comp: dict) -> dict:
    tm = comp.get("thesis_metrics") or {}
    return tm.get("pillar_summary") or {}


def _tokens(comp: dict) -> dict:
    return comp.get("tokens") or {}


def run_checks(comparison: dict, baseline: dict) -> tuple[list[dict], bool]:
    """Returns (check rows, all_passed)."""
    checks: list[dict] = []
    ok = True

    exp = baseline.get("pillar_summary_expected") or {}
    cur = _pillar_summary(comparison)
    for k, want in exp.items():
        got = cur.get(k)
        passed = got == want
        checks.append(
            {
                "id": f"pillar_summary.{k}",
                "passed": passed,
                "expected": want,
                "actual": got,
            }
        )
        ok = ok and passed

    tok = _tokens(comparison)
    lir = (tok.get("lir") or {}).get("api_total")
    py = (tok.get("python") or {}).get("api_total")
    ceilings = baseline.get("api_token_ceiling_per_arm") or {}

    if isinstance(lir, int) and "lir" in ceilings:
        passed = lir <= int(ceilings["lir"])
        checks.append(
            {
                "id": "api_total_lir_ceiling",
                "passed": passed,
                "ceiling": ceilings["lir"],
                "actual": lir,
            }
        )
        ok = ok and passed
    if isinstance(py, int) and "python" in ceilings:
        passed = py <= int(ceilings["python"])
        checks.append(
            {
                "id": "api_total_python_ceiling",
                "passed": passed,
                "ceiling": ceilings["python"],
                "actual": py,
            }
        )
        ok = ok and passed

    both_max = baseline.get("api_token_ceiling_total_both_arms")
    if isinstance(both_max, int) and isinstance(lir, int) and isinstance(py, int):
        s = lir + py
        passed = s <= both_max
        checks.append(
            {
                "id": "api_total_combined_ceiling",
                "passed": passed,
                "ceiling": both_max,
                "actual": s,
            }
        )
        ok = ok and passed

    return checks, ok


def main() -> int:
    ap = argparse.ArgumentParser(description="Pilot A/B comparison vs frozen baseline (Phase 3 regression)")
    ap.add_argument("--run-id", default="", help="Load eval/results_pilot_comparison_<id>.json")
    ap.add_argument(
        "--comparison",
        default="",
        type=str,
        help="Explicit path to results_pilot_comparison_*.json",
    )
    ap.add_argument(
        "--baseline",
        default="eval/baselines/pilot_ab_reference.json",
        type=str,
        help="Frozen baseline JSON",
    )
    ap.add_argument(
        "--report",
        default="",
        type=str,
        help="Write regression report JSON (default: eval/regression_report_<run_id>.json)",
    )
    ap.add_argument(
        "--fail",
        action="store_true",
        help="Exit 1 if any check fails",
    )
    ap.add_argument(
        "--emit-baseline-from",
        default="",
        type=str,
        help="Print a baseline JSON template from a comparison file and exit",
    )
    args = ap.parse_args()

    if args.emit_baseline_from:
        comp_path = Path(args.emit_baseline_from)
        if not comp_path.is_absolute():
            comp_path = ROOT / comp_path
        comp = _load_json(comp_path)
        ps = _pillar_summary(comp)
        rid = comp.get("run_id") or "unknown"
        tok = _tokens(comp)
        lir_t = (tok.get("lir") or {}).get("api_total")
        py_t = (tok.get("python") or {}).get("api_total")
        template = {
            "schema_version": 1,
            "task_set": "pilot_ab_v1",
            "reference_run_id": rid,
            "comment": "Tighten api_token_ceiling_* after review; commit when stable.",
            "pillar_summary_expected": ps,
            "api_token_ceiling_per_arm": {
                "lir": int(lir_t * 1.25) if isinstance(lir_t, int) else 2500,
                "python": int(py_t * 1.25) if isinstance(py_t, int) else 2500,
            },
            "api_token_ceiling_total_both_arms": int((lir_t or 0) + (py_t or 0)) * 2
            if isinstance(lir_t, int) and isinstance(py_t, int)
            else 5000,
        }
        print(json.dumps(template, indent=2, ensure_ascii=False))
        return 0

    comp_path = Path(args.comparison) if (args.comparison or "").strip() else None
    if comp_path is None:
        rid = (args.run_id or "").strip()
        if not rid:
            print("need --run-id or --comparison", file=sys.stderr)
            return 2
        comp_path = _EVAL / f"results_pilot_comparison_{rid}.json"
    if not comp_path.is_absolute():
        comp_path = ROOT / comp_path

    if not comp_path.is_file():
        print(f"missing {comp_path}", file=sys.stderr)
        return 2

    baseline_path = Path(args.baseline)
    if not baseline_path.is_absolute():
        baseline_path = ROOT / baseline_path
    if not baseline_path.is_file():
        print(f"missing baseline {baseline_path}", file=sys.stderr)
        return 2

    comparison = _load_json(comp_path)
    baseline = _load_json(baseline_path)
    checks, all_ok = run_checks(comparison, baseline)

    run_id = comparison.get("run_id") or comp_path.stem.replace("results_pilot_comparison_", "")

    report = {
        "schema_version": 1,
        "event": "pilot_regression",
        "comparison_path": str(comp_path.relative_to(ROOT)),
        "baseline_path": str(baseline_path.relative_to(ROOT)),
        "run_id": run_id,
        "all_checks_passed": all_ok,
        "checks": checks,
    }

    report_path = Path(args.report) if (args.report or "").strip() else None
    if report_path is None:
        report_path = _EVAL / f"regression_report_{run_id}.json"
    if not report_path.is_absolute():
        report_path = ROOT / report_path
    report_path.parent.mkdir(parents=True, exist_ok=True)
    with open(report_path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)
        f.write("\n")

    print(f"pilot_regression: wrote {report_path.relative_to(ROOT)}")
    for c in checks:
        st = "PASS" if c["passed"] else "FAIL"
        print(f"  [{st}] {c['id']}")

    if args.fail and not all_ok:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
