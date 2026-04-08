#!/usr/bin/env bash
# Local smoke: lir check on task starters. Append NDJSON to eval/results.ndjson
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
LIR="${LIR:-cargo run -q --bin lir --}"
OUT="${ROOT}/eval/results.ndjson"
mkdir -p "$(dirname "$OUT")"
TS="$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u +%Y-%m-%dT%H:%M:%SZ)"

TASKS=(001_range_sum 002_input_i32 003_codegen_subset_ok)

for id in "${TASKS[@]}"; do
  starter="eval/tasks/${id}/starter.lir"
  if [[ ! -f "$starter" ]]; then
    echo "{\"task_id\":\"${id}\",\"ts\":\"${TS}\",\"tool\":\"skip\",\"exit_code\":null,\"note\":\"no starter.lir\"}" >>"$OUT"
    continue
  fi
  set +e
  $LIR check "$starter" >/dev/null 2>&1
  ec=$?
  set -e
  echo "{\"task_id\":\"${id}\",\"ts\":\"${TS}\",\"tool\":\"lir_check\",\"exit_code\":${ec}}" >>"$OUT"
done

echo "Appended results to ${OUT}"
