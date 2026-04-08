# LYTR memory model — options (draft)

Supporting Phase 6 / decision **D1** in [LYTR_GLOBAL_IMPLEMENTATION_PLAN.md](LYTR_GLOBAL_IMPLEMENTATION_PLAN.md).

## Option A — Tracing GC

**Pros:** Familiar to agents; fewer use-after-free bugs; easier heap graphs for dynamic structures.  
**Cons:** Pause times; complex LLVM integration (stack maps, safe points); tuning for latency-sensitive code.

## Option B — Explicit allocation + arenas / regions

**Pros:** Predictable performance; straightforward LLVM lowering; good fit for **data-processing** and **embedded** subsets.  
**Cons:** More annotations or discipline; agents may generate incorrect lifetimes without strong tooling.

## Option C — Hybrid

**Pros:** GC for managed user objects; explicit or arena-scoped buffers for hot paths and LIR interop.  
**Cons:** Two sublanguages to specify, teach, and test; FFI boundaries more complex.

## Recommendation process

1. Sketch **LYTR v0.1** type and allocation primitives on paper.  
2. Prototype **one** option with **microbench + small agent-written programs**.  
3. Choose default for **edition 0.1**; document non-goals for the other options.

No decision is recorded here yet — fill [LYTR_GLOBAL_IMPLEMENTATION_PLAN.md](LYTR_GLOBAL_IMPLEMENTATION_PLAN.md) §8 decision log when chosen.
