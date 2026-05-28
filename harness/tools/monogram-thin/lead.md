This repository is ALREADY INDEXED by monogram. monogram is your investigation tool — use it
instead of grep, find, or scanning files by hand.

    monogram

Prepared-index rule: this benchmark run already installed the monogram DB before the solver
started. Do NOT run `monogram index`, `monogram i`, `monogram reindex`, `monogram prune`,
`monogram boot init`, or any command with `-r` / `--reindex`. If `monogram stats` shows 0 files, a
tiny DB, an unrelated DB path, or the wrong repo, report `HARNESS_DB_MISMATCH` instead of repairing
it by mutating the index.

Run it with no arguments first to see its commands. Then, on every step, run the monogram command
that fits your question, and READ the `[NEXT]` hint(s) monogram prints after each result — follow
them. monogram's output is structural (call graph, cross-language bindings, definitions); let it
lead you from the symptom to the root cause. Do not stop at `search` — climb to the structural
commands it suggests.

Run `monogram` now, then use it to solve the task below.
================================================================================
