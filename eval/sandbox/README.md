# Sandbox image — full pilot A/B with baked-in `lir`

The image **builds `lir` once** (release) and installs **Python 3**. At runtime there is **no `cargo`**, only **`/usr/local/bin/lir`**.

## Why mount the repo?

`eval/*.py` and task files live in the working tree. Mount the repository at **`/work`** so NDJSON logs and code stay on the host.

## Whole pilot A/B (recommended)

**1. One-shot script** (builds the image if missing, then runs the pilot):

```bash
chmod +x eval/sandbox/run-pilot-ab.sh    # once
export OPENAI_API_KEY=...
eval/sandbox/run-pilot-ab.sh
# or dry-run (no API):
eval/sandbox/run-pilot-ab.sh --dry-run
```

**2. Docker Compose** (from **repo root**):

```bash
export OPENAI_API_KEY=...
docker compose -f eval/sandbox/docker-compose.yml run --rm pilot-ab
```

**3. Manual `docker run`**:

```bash
docker build -f eval/sandbox/Dockerfile -t lir-eval:sandbox .
docker run --rm -it -v "$PWD:/work:rw" -w /work \
  -e LIR=/usr/local/bin/lir \
  -e OPENAI_API_KEY -e LLM_MODEL \
  lir-eval:sandbox \
  python3 eval/run_pilot_ab.py
```

### Critical: `LIR` inside the container

The harness must use the **Linux** binary shipped in the image (**`/usr/local/bin/lir`**). The image sets **`ENV LIR=/usr/local/bin/lir`**.

Do **not** rely on the mounted repo’s **`target/release/lir`** from a **macOS** host — it will not run inside a Linux container. The script/compose above always sets **`LIR`**.

## Build image only

From **repository root**:

```bash
docker build -f eval/sandbox/Dockerfile -t lir-eval:sandbox .
```

The builder stage uses **Rust ≥ 1.85** so Cargo accepts **edition 2024** crates (e.g. `getrandom` 0.4.x). If `rust:1.85-bookworm` is unavailable on your registry, edit the Dockerfile to `FROM rust:bookworm` (latest stable).

## Network

- **Live OpenAI calls** need egress — do **not** use `--network none`.
- **Dry-run** only: you may use **`--network none`**:

```bash
docker run --rm -it --network none -v "$PWD:/work:rw" -w /work \
  -e LIR=/usr/local/bin/lir \
  lir-eval:sandbox \
  python3 eval/run_pilot_ab.py --dry-run
```

## Security model

- **Non-root** (`evuser` in the image).
- **Model-generated code** is still arbitrary Python / `lir` input — treat as **confinement**, not formal verification. For more isolation: VM, gVisor, CI with no secrets on the filesystem.

## “Better than Docker?”

**gVisor** (`runsc`), **rootless Podman**, **Firejail**, or **ephemeral CI VMs** — Docker here is a practical default.
