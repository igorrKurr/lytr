#!/usr/bin/env python3
"""
Tier A eval: run manifest assertions against the `lir` CLI (see eval/manifest.json).

Usage (from repo root):
  python3 eval/run_tier_a.py
  LIR='cargo run -q --bin lir --' python3 eval/run_tier_a.py

Exit 0 if all assertions pass; 1 otherwise.
Appends one JSON line per assertion to eval/results.ndjson (see .gitignore).
Optional per-task hidden/assertions.json (see eval/README.md) is merged after manifest assertions.
"""
from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

_eval_dir = Path(__file__).resolve().parent
if str(_eval_dir) not in sys.path:
    sys.path.insert(0, str(_eval_dir))

from tier_a_lib import (
    HiddenAssertionsError,
    merge_manifest_and_hidden,
    repo_root,
    run_assertions_on_file,
)


def main() -> int:
    root = repo_root()
    manifest_path = root / "eval" / "manifest.json"
    out_log = root / "eval" / "results.ndjson"
    out_log.parent.mkdir(parents=True, exist_ok=True)

    with open(manifest_path, encoding="utf-8") as f:
        manifest = json.load(f)

    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    tier = manifest.get("tier", "A")
    failed = 0

    for task in manifest["tasks"]:
        tid = task["id"]
        starter_rel = task["starter"]
        starter = root / "eval" / starter_rel
        if not starter.is_file():
            line = {
                "tier": tier,
                "task_id": tid,
                "ts": ts,
                "kind": "error",
                "message": f"missing starter: {starter_rel}",
            }
            print(json.dumps(line), file=sys.stderr)
            failed += 1
            continue

        try:
            assertions = merge_manifest_and_hidden(task.get("assertions", []), starter)
        except HiddenAssertionsError as e:
            line = {
                "tier": tier,
                "task_id": tid,
                "ts": ts,
                "kind": "error",
                "message": str(e),
            }
            print(json.dumps(line), file=sys.stderr)
            failed += 1
            continue

        fc, records = run_assertions_on_file(
            root,
            starter,
            tid,
            tier,
            assertions,
            ts,
            out_log,
            extra_fields={"runner": "tier_a"},
        )
        failed += fc
        for r in records:
            if not r["pass"]:
                print(json.dumps(r), file=sys.stderr)

    if failed:
        print(f"tier_a: {failed} assertion(s) failed (see stderr and {out_log})", file=sys.stderr)
        return 1
    print(f"tier_a: all assertions passed ({len(manifest['tasks'])} tasks)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
