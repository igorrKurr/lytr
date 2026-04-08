# LYTR standard library charter (draft)

**GA gate G3** — scope must be **explicit** to avoid unbounded security and maintenance liability.

---

## 1. Principles

- **Minimal surface in v0.1:** enough for charter workloads ([LYTR_CHARTER_DRAFT.md](LYTR_CHARTER_DRAFT.md)).
- **Security-sensitive APIs** (crypto, TLS, subprocess) are **opt-in modules** with **documented threat model**, not accidental `import`.
- **No silent network** — ambient authority rejected unless edition explicitly allows (see platform doc).

---

## 2. Proposed tiers

### Tier 0 (preview / agent tier)

- Core types: integers, booleans, tuples/records as per language.
- `io` (stdin/stdout/stderr or capability handles — TBD with memory model).
- `panic` / `Result` / error types aligned with B2.
- **No** crypto, **no** HTTP client/server in Tier 0 unless charter expands.

### Tier 1 (GA-minimal)

- File I/O (paths, errors) with **documented** symlink / TOCTOU stance.
- Time (monotonic vs wall — pick one semantics).
- Process spawn **behind** explicit API with argv/env contract.
- Collections: growable array, map (hash — collision / DoS policy documented).

### Tier 2 (post-GA)

- Networking (TCP/UDP or HTTP) with **timeouts and cancellation**.
- Crypto: delegate to **vetted** C libs or platform APIs; **no** roll-your-own in stdlib.

---

## 3. Explicitly out of scope (initial editions)

- Full Unicode collation / locale database (state minimal Unicode policy if strings exist).
- GUI frameworks.
- ML training frameworks.

---

## 4. Crypto policy

- Stdlib **does not** implement novel cryptography.
- If crypto appears: **only** wrappers around well-audited libraries; document **algorithm agility** and **deprecation** path.

---

## 5. LIR interop

- Stdlib may expose **constructors** that build LIR pipelines; behavior must match [LIR_V1_SPEC.md](LIR_V1_SPEC.md) for embedded fragments.
