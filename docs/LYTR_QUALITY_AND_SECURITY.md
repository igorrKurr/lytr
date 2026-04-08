# LYTR quality, fuzzing, performance CI, and security (draft)

**GA gates G7, G8, G9** — correctness at scale and supply chain.

---

## 1. Fuzzing (G7)

| Target | Goal |
|--------|------|
| **Parser / lexer** | No panics; bounded memory; valid error paths for garbage |
| **Typechecker** | No ICE on random AST-shaped inputs (where generator exists) |
| **Codegen** (LIR/LYTR) | No invalid LLVM IR emitted for in-domain inputs |

**CI:** `cargo fuzz` or dedicated job; **nightly** full run; **PR** smoke (bounded time).

**Policy:** **Zero** known critical fuzzer findings at GA; others triaged with SLA.

---

## 2. Performance regression CI (G8)

- **Benchmark suite** versioned in repo (micro + representative workloads).
- **Thresholds:** e.g. >5% regression fails **nightly**; **release** branch blocks on agreed subset.
- Separate tracks: **compile time**, **runtime**, **binary size** (WASM + native).

---

## 3. Threat model sketch (G9)

| Asset | Threat | Mitigation |
|-------|--------|------------|
| **Compiler** | Supply-chain compromise | Lockfiles, pinned deps, signed releases, SBOM |
| **User code** | Malicious package | Sandboxed test/eval; optional WASM tier |
| **Stdlib** | CVE in bundled C lib | Update policy; advisory channel |
| **Secrets in eval** | Leak via logs | Redact tokens in eval runner; document retention |

---

## 4. Supply chain minimum (G9)

- **SBOM** per release (SPDX or CycloneDX).
- **Signed** release artifacts + checksum file.
- **Dependency audit** job (`cargo deny` or equivalent) on default features.

---

## 5. Eval harness security

- **Hidden tests** never committed in cleartext in task prompts; runner loads from protected path.
- **Network** disabled by default in sandboxed eval runs.

---

## 6. Localization

- v1 may be **English-only**; document that as **implementation-defined** for diagnostics; plan **message IDs** for future i18n.
