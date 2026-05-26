# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `Sources/Vapor/Middleware/FileMiddleware.swift :: respond` (PR #2500, CVE-2020-15230;
fix `cf1651f7ff76`, base `236c616ca1d7`).

`respond` performed the `..` containment check on the **raw, still-percent-encoded** path, and only
percent-decoded afterward. So `/%2e%2e/` passed the check, then decoded to `../` → path traversal.

**Decoy:** the percent-decode call (`removingPercentEncoding`) and the file-read site look
responsible; the defect is the **order** — check before decode.

**Fix:** decode percent-encoding before the containment check.

**Admission (C1–C6):** C1 — symptom (out-of-root file served) ≠ the order-of-operations cause.
C2 — literal `..` is rejected, so a naive grep for `..` looks fine. C3 ✓ PR #2500. C4 — 2020, **famous-ish CVE → check contamination**. C5 — single-file; run baseline. C6 — pure Swift-language
instance (exercises monogram's Swift extractor).
