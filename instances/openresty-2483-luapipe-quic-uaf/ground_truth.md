# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `src/ngx_http_lua_pipe.c :: ngx_http_lua_ffi_pipe_proc_destroy` (PR #2483, fix
`cac9cbc3b700`, base `a44c67f41876`).

On the QUIC connection-close path the connection pool is destroyed, which runs the pipe's pending
cleanup handlers against **already-freed** data → UAF. The crash surfaces in
`ngx_http_lua_pipe_proc_read_stdout_cleanup` (gdb frame #0), reached via
`ngx_http_lua_cleanup_pending_operation`.

**Decoy:** the four `*_cleanup` handlers are the crash sites but are correct; the defect is the
close/destroy **ordering** owned by `ngx_http_lua_ffi_pipe_proc_destroy`.

**Fix:** ensure the pipe's connections/cleanups are torn down before pool destruction.

**Admission (C1–C6):** C1 ✓ crash (cleanup handler) ≠ cause (destroy ordering). C2 ✓ symptom names
the crash handler, not the destroy fn. C3 ✓ PR #2483. C4 — QUIC + ngx.pipe path, niche. C5 — baseline.
C6 ✓ Lua `ngx.pipe` lifetime ↔ C nginx pool/cleanup chain.
