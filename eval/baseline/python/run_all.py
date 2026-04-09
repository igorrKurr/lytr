#!/usr/bin/env python3
"""
Python numeric baselines for Tier A task intents (stdlib only).

Covers manifest tasks 001–008 and 012–020 (stdout-aligned with `lir run`).
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


def t012_prefix_take_sum() -> str:
    return str(sum(range(0, 10)[:3]))


def t013_drop_count() -> str:
    return str(len(list(range(0, 10))[5:]))


def t014_filter_odd_sum() -> str:
    return str(sum(x for x in range(0, 7) if x % 2 == 1))


def t015_lit_map_mul_sum() -> str:
    return str(sum(x * 2 for x in (1, 2, 3, 4)))


def t016_filter_even_map_mul_sum() -> str:
    return str(sum(x * 2 for x in range(0, 6) if x % 2 == 0))


def t017_lit_map_add_sum() -> str:
    return str(sum(x + 5 for x in (10, 20, 30)))


def t018_take_filter_even_count() -> str:
    head = list(range(0, 100))[:10]
    return str(sum(1 for x in head if x % 2 == 0))


def t019_lit_reduce_min() -> str:
    return str(min((5, 2, 9)))


def t020_drop_take_sum() -> str:
    xs = list(range(0, 8))[2:][:3]
    return str(sum(xs))


TASKS: dict[str, tuple[str, Callable[[], str]]] = {
    "001_range_sum": ("10\n", t001_range_sum),
    "002_input_i32": ("60\n", t002_input_i32),
    "003_codegen_subset_ok": ("8\n", t003_codegen_subset_ok),
    "004_empty_lit_count": ("0\n", t004_empty_lit_count),
    "005_filter_even_count": ("5\n", t005_filter_even_count),
    "006_i64_lit_sum": ("6000000000\n", t006_i64_lit_sum),
    "007_bool_filter_count": ("2\n", t007_bool_filter_count),
    "008_scan_add_sum": ("4\n", t008_scan_add_sum),
    "012_prefix_take_sum": ("3\n", t012_prefix_take_sum),
    "013_drop_count": ("5\n", t013_drop_count),
    "014_filter_odd_sum": ("9\n", t014_filter_odd_sum),
    "015_lit_map_mul_sum": ("20\n", t015_lit_map_mul_sum),
    "016_filter_even_map_mul_sum": ("12\n", t016_filter_even_map_mul_sum),
    "017_lit_map_add_sum": ("75\n", t017_lit_map_add_sum),
    "018_take_filter_even_count": ("5\n", t018_take_filter_even_count),
    "019_lit_reduce_min": ("2\n", t019_lit_reduce_min),
    "020_drop_take_sum": ("9\n", t020_drop_take_sum),
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
