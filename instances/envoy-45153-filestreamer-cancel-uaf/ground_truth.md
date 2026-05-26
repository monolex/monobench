# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `source/extensions/filters/http/file_server/file_streamer.cc :: FileStreamer`
(the methods issuing async ops — `begin`/`startDir`/`readBodyChunk`). PR #45153, fix `96294652f7f2`,
base `671bb592d454`.

`FileStreamer` captured a bare `[this]` into the callbacks returned by `AsyncFileManager::stat()` /
`read()`. The async-file manager **guarantees the callback fires even if `cancel()` was called**, so
when the `FileStreamer` is destroyed (request cancelled) before the callback runs, `[this]` dangles
and the callback dereferences freed memory.

**Decoy:** the callback bodies and `readBodyChunk` (where it crashes) are correct; the defect is the
bare `[this]` capture in the issuing methods.

**Fix:** guard the callbacks against running after destruction (cancel / weak-handle the captured
`this`).

**Admission (C1–C6):** C1 ✓ crash (callback) ≠ cause (the capture in the issuing method). C2 ✓
symptom names the filter, not the capture. C3 ✓ PR #45153. C4 — recent, niche filter. C5 — baseline.
C6 ✓ C++ lifetime across an async-callback boundary.
