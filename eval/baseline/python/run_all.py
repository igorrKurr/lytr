#!/usr/bin/env python3
"""
Python numeric baselines for Tier A task intents (stdlib only).

Does not invoke LIR. Use to compare agent effort vs LIR for the same prompts.

Usage (from repo root):
  python3 eval/baseline/python/run_all.py
  python3 eval/baseline/python/run_all.py --task 002
"""
from __future__ import annotations

import argparse
import sys
from typing import Callable


def t001_range_sum() -> str:
    return str(sum(range(0, 5)))


def t002_input_i32() -> str:
    return str(sum([10, 20, 30]))


def t003_codegen_subset_ok() -> str:
    xs = list(range(0, 10))[1:5]
    ys = [x + 1 for x in xs if x % 2 == 0]
    return str(sum(ys))


def t004_empty_lit_count() -> str:
    return str(len(()))


def t005_filter_even_count() -> str:
    return str(sum(1 for x in range(0, 10) if x % 2 == 0))


def t006_i64_lit_sum() -> str:
    return str(3_000_000_000 + 3_000_000_000)


def t007_bool_filter_count() -> str:
    return str(sum(1 for b in (True, False, True) if b is True))


def t008_scan_add_sum() -> str:
    acc = 0
    partials = []
    for x in range(0, 3):
        acc = acc + x
        partials.append(acc)
    return str(sum(partials))


TASKS: dict[str, tuple[str, Callable[[], str]]] = {
    "001_range_sum": ("10\n", t001_range_sum),
    "002_input_i32": ("60\n", t002_input_i32),
    "003_codegen_subset_ok": ("8\n", t003_codegen_subset_ok),
    "004_empty_lit_count": ("0\n", t004_empty_lit_count),
    "005_filter_even_count": ("5\n", t005_filter_even_count),
    "006_i64_lit_sum": ("6000000000\n", t006_i64_lit_sum),
    "007_bool_filter_count": ("2\n", t007_bool_filter_count),
    "008_scan_add_sum": ("4\n", t008_scan_add_sum),
}


def main() -> int:
    ap = argparse.ArgumentParser(description="Python baselines for Tier A numeric tasks")
    ap.add_argument("--task", type=str, default="", help="Task id prefix, e.g. 001_range_sum")
    args = ap.parse_args()

    items = list(TASKS.items())
    if args.task:
        items = [(k, v) for k, v in items if args.task in k]
        if not items:
            print(f"unknown task {args.task!r}", file=sys.stderr)
            return 1

    failed = 0
    for tid, (want, fn) in items:
        got = fn()
        if not got.endswith("\n"):
            got = got + "\n"
        ok = got == want
        if not ok:
            failed += 1
            print(f"FAIL {tid}: want {want!r} got {got!r}", file=sys.stderr)
        else:
            print(f"ok {tid}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
