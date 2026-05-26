# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `torch/csrc/jit/tensorexpr/kernel.cpp :: TensorExprKernel::run` (PR #73029, fix
`7fa092949eec`, base `74cd18623ec3`).

A `TensorExprKernel` is created once and `run()` many times, possibly in parallel. With **dynamic
shapes**, `run()` modified the kernel's `sizes`/`strides` vectors — shared state — so concurrent calls
race.

**Decoy:** `prepareRunArgs` / `runWithAllocatedOutputs` touch the vectors and look responsible; the
defect is that `run()` mutates shared state at all.

**Fix:** make `run()` non-mutating (compute per-call args without writing back into the kernel).

**Admission (C1–C6):** C1 — MODERATE (the race is in `run()`'s own mutation → crash≈cause); the
difficulty is recognizing `run()` must be non-mutating and that the write is on the dynamic-shapes
path only. C2 ✓ symptom never names `run`/`sizes`. C3 ✓ PR #73029. C4 — **2022, check
contamination**. C5 — baseline. C6 ✓ C++ concurrency/shared-state.
