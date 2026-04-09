#!/usr/bin/env bash
# Run the full pilot A/B inside the sandbox image. The image ships a release
# /usr/local/bin/lir (built at docker build time). The repo is mounted at /work
# so eval/*.py and NDJSON logs stay on the host.
#
# Usage (from repo root):
#   chmod +x eval/sandbox/run-pilot-ab.sh   # once
#   export OPENAI_API_KEY=...
#   eval/sandbox/run-pilot-ab.sh
#   eval/sandbox/run-pilot-ab.sh --dry-run
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

IMAGE="${LIR_EVAL_IMAGE:-lir-eval:sandbox}"

if ! docker image inspect "$IMAGE" &>/dev/null; then
  echo "Building ${IMAGE} (release lir baked in)…" >&2
  docker build -f eval/sandbox/Dockerfile -t "$IMAGE" .
fi

# LIR must be the Linux binary inside the image. If we relied on the mounted
# repo’s target/release/lir, a macOS host build would break inside Linux.
exec docker run --rm -it \
  -v "${REPO_ROOT}:/work:rw" \
  -w /work \
  -e LIR=/usr/local/bin/lir \
  -e OPENAI_API_KEY="${OPENAI_API_KEY:-}" \
  -e OPENAI_BASE_URL="${OPENAI_BASE_URL:-https://api.openai.com/v1}" \
  -e LLM_MODEL="${LLM_MODEL:-gpt-4o-mini}" \
  -e LLM_CANONICALIZE="${LLM_CANONICALIZE:-}" \
  -e LLM_RETRY_ON_FAIL="${LLM_RETRY_ON_FAIL:-}" \
  "$IMAGE" \
  python3 eval/run_pilot_ab.py "$@"
