# LYTR platform, editions, and releases (draft)

**GA gates G4, G6** — stability and reproducible builds.

---

## 1. Versioning model

| Axis | Mechanism |
|------|-----------|
| **Language** | Edition banner (e.g. `lytr/2026` or `lytr/1`) in source; compiler accepts **current** and **previous** edition with deprecation warnings |
| **Toolchain** | Semantic versioning `lytr-X.Y.Z` (compiler, LSP, formatter share major) |
| **ABI** | **Stable / unstable** labels per symbol for dynamic linking (if any); default **crate-private** until marked stable |

---

## 2. Deprecation policy

- **Minor** release: new features; deprecations with migration hint in diagnostic + doc.
- **Major** or **edition** boundary: allowed breaking changes per migration guide.
- **LIR v1** remains independently frozen unless **`lir/2`** is introduced ([LIR_V1_SPEC.md](LIR_V1_SPEC.md)).

---

## 3. Package and build (G6)

**Target capabilities:**

- Manifest (`Lytr.toml` or aligned with ecosystem choice): name, version, edition, dependencies.
- **Lockfile** for reproducible resolves.
- `lytr build` / `lytr test` with deterministic output paths.
- **Workspace** mode for monorepos (post-GA acceptable if documented).

**Dependency policy:**

- Allow **path**, **git** (pinned SHA), **registry** (when registry exists); document **integrity** (hash) requirements.

---

## 4. Cross-compilation

- Define **tier-1** targets (e.g. host native, `wasm32-unknown-unknown`).
- **Tier-2/3** best-effort; documented in release notes.

---

## 5. Release artifacts

- Per platform: compiler binary, **LSP** server, stdlib sources or snapshot.
- **SBOM** (SPDX or CycloneDX) attached ([LYTR_QUALITY_AND_SECURITY.md](LYTR_QUALITY_AND_SECURITY.md)).
- **Signed** checksums for binaries (G9).
