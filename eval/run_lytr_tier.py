#!/usr/bin/env python3
"""
LYTR eval: run lytr_manifest.json against the `lytr` CLI (numeric parity with Tier A intents).

Usage (from repo root):
  python3 eval/run_lytr_tier.py
  LYTR='cargo run -q --bin lytr --' python3 eval/run_lytr_tier.py

Exit 0 if all assertions pass; 1 otherwise.
Appends JSON lines to eval/results_lytr.ndjson (gitignored).
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
    run_lytr_assertions_on_file,
)


def main() -> int:
    root = repo_root()
    manifest_path = root / "eval" / "lytr_manifest.json"
    out_log = root / "eval" / "results_lytr.ndjson"
    out_log.parent.mkdir(parents=True, exist_ok=True)

    with open(manifest_path, encoding="utf-8") as f:
        manifest = json.load(f)

    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    tier = manifest.get("tier", "LYTR")
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
                "message": f"missing program: {starter_rel}",
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

        fc, records = run_lytr_assertions_on_file(
            root,
            starter,
            tid,
            tier,
            assertions,
            ts,
            out_log,
            extra_fields={"runner": "lytr_tier"},
        )
        failed += fc
        for r in records:
            if not r["pass"]:
                print(json.dumps(r), file=sys.stderr)

    if failed:
        print(
            f"lytr_tier: {failed} assertion(s) failed (see stderr and {out_log})",
            file=sys.stderr,
        )
        return 1
    print(f"lytr_tier: all assertions passed ({len(manifest['tasks'])} tasks)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
