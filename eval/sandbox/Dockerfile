# Reproducible release `lir` + Python 3 for eval harnesses.
# Run from repo root: docker build -f eval/sandbox/Dockerfile -t lir-eval:sandbox .
#
# Example (no network inside container; mount repo read-write for logs):
#   docker run --rm -it --network none -v "$PWD:/work:rw" -w /work -e OPENAI_API_KEY lir-eval:sandbox \
#     python3 eval/run_pilot_ab.py

# Rust ≥ 1.85: Cargo supports edition 2024 (required by getrandom 0.4.x / modern lockfiles).
FROM rust:1.85-bookworm AS builder
WORKDIR /src
COPY . .
RUN cargo build --release --bin lir

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 1000 -s /bin/bash evuser

COPY --from=builder /src/target/release/lir /usr/local/bin/lir
RUN chmod 555 /usr/local/bin/lir

USER evuser
WORKDIR /work
ENV LIR=/usr/local/bin/lir

# Default: shell so you can run eval commands; override CMD as needed.
CMD ["/bin/bash"]
