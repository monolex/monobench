# Ground truth — SPOILER (the real fix PR — author the instance FROM this, never feed to the solver)

**denoland/deno PR #31770** — fix(ext/node): fix use-after-free in StatementSync JS iterator

**fix commit:** 8d47d7e24402 · **base (merge^):** 7558cafe636c · merged 2026-01-08

## Changed source files (test/fixture files filtered out)
- ext/node/ops/sqlite/statement.rs

## PR body

Keep a reference of StatementSync alive to prevent GC from destroying statement handle while iterator is being used.

Fixes https://github.com/denoland/deno/issues/31744
