# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `io_buffer.c :: io_buffer_and` (PR #16964, fix `2552db04ddc4`, base `356c0cd0e795`).

`io_buffer_and` read `buffer->base` and `mask_buffer->base` **directly**, without checking the buffers
were still live. A slice whose parent buffer was freed (`IO::Buffer#free`) keeps a stale `base`
pointer, so `&` dereferences freed memory → UAF.

**Decoy:** `IO::Buffer#free` / `io_buffer_free` is where the memory is actually released, and the
sibling operators `io_buffer_or` / `io_buffer_xor` look identical — but the graded defect is
`io_buffer_and` failing to validate liveness before access.

**Fix:** route both operands through `io_buffer_get_bytes_for_reading`, which raises
`IO::Buffer::InvalidatedError` before any memory access.

**Admission (C1–C6):** C1 ✓ crash op = `&`, cause = a missing liveness check + an invalidation that
happened earlier on the parent. C2 ✓ symptom never says "invalidated"/"liveness". C3 ✓ merged PR
#16964. C4 — 2026, niche → low contamination. C5 — single file; run baseline, down-weight if it
greps to FULL. C6 ✓ Ruby slice lifetime ↔ C `base` pointer.
