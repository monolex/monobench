# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `ktor-utils/jvm/src/io/ktor/util/cio/FileChannels.kt :: readChannel` (PR #5626, fix
`94c346349a7c`, base `3ccad96fbb89`).

`File.readChannel`'s failure/cleanup path could close a file that had **not been opened** (or close it
twice) when an early return / exception occurred during setup → spurious failures (observed as curl
WebSocket handle leaks/timeouts on Linux).

**Decoy:** the `close()` / `use{}` site and the `RandomAccessFile`/`FileChannel` open look
responsible; the defect is `readChannel` closing a handle it never opened.

**Fix:** only close the file if it was actually opened.

**Admission (C1–C6):** C1 — failure surfaces at the close/use site, cause is the cleanup path.
C2 ✓ symptom never names `readChannel`. C3 ✓ PR #5626. C4 — recent, niche. C5 — single-file Kotlin;
run baseline. C6 — pure Kotlin-language instance (exercises monogram's Kotlin extractor — a NO-CALLS
language).
