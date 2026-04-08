# LYTR semantics, undefined behavior, and FFI (draft)

**Status:** draft toward **GA gate G1** ([LYTR_PRODUCTION_READINESS.md](LYTR_PRODUCTION_READINESS.md)).

---

## 1. Why this exists

The **reference interpreter** is an implementation, not a contract. Production needs:

- **Observable semantics** for valid programs (evaluation order, type rules, runtime errors).
- A **catalog of undefined behavior (UB)** and **implementation-defined** behavior.
- A **strict FFI boundary** with C (and host embedders).

---

## 2. Deliverables (path to normative)

| Artifact | Content |
|----------|---------|
| **LYTR_SPEC.md** (future) | Formal-ish operational semantics; error taxonomy aligned with JSON diagnostics |
| **UB.md** | List of UB cases (e.g. data races, `unsafe` misuse, invalid FFI) |
| **FFI.md** | Calling conventions, layout guarantees, panic/trap across boundary, thread affinity |
| **Concurrency.md** | Memory model (happens-before), `Send`/`Sync` analogs if applicable |

---

## 3. Template: UB table (fill during implementation)

| ID | Condition | Outcome |
|----|-----------|---------|
| UB-001 | Data race on shared mutable without synchronization | Undefined |
| UB-002 | Use-after-free / double-free in `unsafe` | Undefined |
| … | … | … |

---

## 4. LIR alignment

- LIR v1 traps (`R_*`) remain **defined** for the interpreter and must **match** compiled lowering where LLVM uses `llvm.trap`.
- LYTR **numeric** behavior in embedded LIR fragments must **match** standalone LIR oracle tests ([global plan](LYTR_GLOBAL_IMPLEMENTATION_PLAN.md) B4).

---

## 5. Work order

1. Freeze **LYTR v0.1** expression and statement forms (see [LYTR_CHARTER_DRAFT.md](LYTR_CHARTER_DRAFT.md) appendix).
2. Write **operational semantics** for those forms only.
3. Expand with generics/async/threads as features ship.
