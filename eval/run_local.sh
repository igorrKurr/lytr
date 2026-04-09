#!/usr/bin/env bash
# Tier A eval (manifest-driven). For legacy one-liner checks, use this script.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
exec python3 eval/run_tier_a.py "$@"
