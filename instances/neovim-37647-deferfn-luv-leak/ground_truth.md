# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `runtime/lua/vim/_core/editor.lua :: defer_fn` (PR #37647, fix `1906da52dbc9`, base
`0501c5fd0959`).

`vim.defer_fn()` creates a libuv timer and, when it fires, calls `vim.schedule()`. If
`vim.schedule()` fails (e.g. during shutdown), `defer_fn` never closed the timer → leaked luv handle.

**Decoy:** `vim.schedule` (the C-backed scheduler in `executor.c` that can fail) and the uv timer
creation look responsible; the defect is `defer_fn` not closing its timer on the failure path.

**Fix:** make `vim.schedule()` return an error when scheduling fails, and make `vim.defer_fn()` close
the timer when `vim.schedule()` failed.

**Admission (C1–C6):** C1 — symptom (leaked uv handle at exit) ≠ cause (`defer_fn` failure path).
C2 ✓ symptom never names `defer_fn`. C3 ✓ PR #37647. C4 — niche shutdown path. C5 — resource-leak,
so *moderate* discrimination; kept for Lua-language coverage. C6 ✓ Lua stdlib ↔ C uv loop.
