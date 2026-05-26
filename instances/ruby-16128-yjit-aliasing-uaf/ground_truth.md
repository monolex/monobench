# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `yjit/src/core.rs :: add_block_version` (PR #16128, fix `e730ac41be4d`, base `906176adb49a`).

`add_block_version` obtained the iseq payload **twice** — once to reach `version_map` (through
`get_or_create_version_list` → `get_or_create_iseq_payload`) and once via
`get_iseq_payload(block.iseq.get()).unwrap()` to reach `.pages`. That produced two overlapping
`&'static mut IseqPayload` references. Because `&mut` is `noalias`, LLVM may cache the `version_map`
`Vec` header; after the other reference mutated/reallocated the map, the cached header pointed at
**freed backing storage** → use-after-free. The standalone `delayed_deallocation()` free-fn (called
from `invalidate_block_version`) re-fetched a second payload reference the same way.

**Mechanism:** mutable-aliasing UB → stale `Vec` header → UAF that manifests far from the cause, in
any op that later reads `version_map`.

**Decoys (crash sites, all correct):** `get_version_list` (block lookup), the GC mark path, and
`invalidate_block_version` (block removal). A shallow reader blames the crashing op.

**Fix:** take a single `iseq_payload` reference and access both `version_map` (new method
`IseqPayload::get_or_create_version_list`) and `pages` through it; convert `delayed_deallocation`
into an `IseqPayload` method so `invalidate_block_version` reuses its existing reference.

**Admission (C1–C6):** C1 ✓ crash op ≠ `add_block_version`. C2 ✓ no symptom token greps to the cause
("aliasing" / "add_block_version" never appear in the symptom). C3 ✓ merged PR #16128. C4 — recent
(2026), niche YJIT internals → low contamination; record. C5 — run baseline first. C6 ✓ the
cause→crash link is a Rust ownership/aliasing edge across the Ruby-C-VM iseq ↔ Rust-JIT payload
boundary (monogram's ownership/coupling surface).
