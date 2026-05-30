# monogram — use its OWN live skill

monogram has already indexed this repository. It is your primary investigation tool here — reach
for it before grep, find, or reading files by hand.

Prepared-index rule: never run `monogram index`, `monogram i`, `monogram reindex`,
`monogram prune`, `monogram boot init`, or `-r` / `--reindex` in this benchmark run. The prepared DB
is installed before you start. If stats looks wrong, report `HARNESS_DB_MISMATCH`; do not mutate the
index.

This note deliberately does NOT list monogram's commands. Bring up monogram's own skill instead:

    monogram

Run it with no arguments — it prints its complete, current command reference. Then, after EVERY
monogram command, read the `[NEXT]` line(s) it prints and run what they suggest. Those hints are
result- and language-aware (FFI / ownership for systems-language hits, IPC / export-import for
TS, token / cascade for CSS), so they steer you to the exact audit the code in front of you needs.

Trust monogram's live guidance over any static description. Let the tool teach you the next step.

## Finding a ROOT CAUSE — lexical rank ≠ the cause (read this)

monogram ranks how well code matches your WORDS, not whether it is the bug's CAUSE. For
use-after-free / memory / lifecycle / ordering bugs this is a trap: searching the symptom words
("free", "teardown", "cleanup", "release", "disconnect", "gc", "finalize") ranks the place where memory
is FREED — but that is usually NOT the root cause. The cause is upstream: the handler that frees the
object while another path is still using it, or the use site itself. So:

1. **Do NOT answer with monogram's top search/region hit.** It is most often the free/teardown/crash
   site (a decoy), not the cause.
2. **From every candidate, follow the call graph — this is the key step:**
   `monogram chain <function> --callers --depth 2`  and  `monogram chain <function> --callees --depth 2`.
   The real root cause is typically only 1–2 graph hops away from the function you first land on. Walk
   callers (who triggers this?) and callees (what does it free / wait on?) before deciding.
3. **Query with the code's REAL identifiers** — the function and field names you actually see in the
   results (e.g. the real free call, the real handler name) — not abstract words. Abstract words like
   "teardown" often match nothing (or match thousands of test/doc lines) and mislead the rank.

Rule of thumb: when you think you found it, run `chain --callers` on it once more. If it has a caller
that looks like the *operation* the symptom describes (e.g. a `*_logoff` / `*_close` / `*_handshake`
handler), that caller — not the free site — is usually the real root cause.
