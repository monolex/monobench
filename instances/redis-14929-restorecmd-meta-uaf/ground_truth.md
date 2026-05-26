# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `src/cluster.c :: restoreCommand` (PR #14929, fix `40c140bf16ca`, base `bbc0dcbb9af7`).

`restoreCommand` kept a local `kv` pointer across `notifyKeyspaceEvent()`. A module's notification
callback can call `RedisModule_SetKeyMeta` for the first time → `kvobjSet` **reallocates** the kvobj
(to add a metadata slot) and frees the old one → the local `kv` dangles → the next `kv->type` read is
a UAF.

**Decoy:** `kvobjSet` is where the realloc/free visibly happens (looks like the culprit) and
`RedisModule_SetKeyMeta` is the module entry — both correct. The fix lands in `restoreCommand`.

**Fix:** save `kv->type` into a local **before** the notification calls.

**Admission (C1–C6):** C1 — crash fn == fix fn (`restoreCommand`), so C1 is *moderate*; the real
difficulty (and the tool value) is tracing the **re-entrancy**: cause runs deep inside a module
notification callback (`notifyKeyspaceEvent → module → RedisModule_SetKeyMeta → kvobjSet`), far from
the read. C2 ✓ symptom states the repro condition, not the function. C3 ✓ PR #14929. C4 — needs a
module that sets key meta on notification → low contamination. C5 — run baseline. C6 ✓ re-entrant
call edge through the module C-API.
