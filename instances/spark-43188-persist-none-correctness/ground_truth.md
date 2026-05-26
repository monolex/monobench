# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `sql/core/.../execution/CacheManager.scala :: cacheQuery` (PR #43188, SPARK-45386; fix
`a0c9ab63f3bc`, base `8f1b02880cb9`).

`cacheQuery` did not special-case `StorageLevel.NONE`. It registered an `InMemoryRelation` that caches
**no data**, so later reads of the "cached" plan returned nothing / wrong counts.

**Decoy:** `InMemoryRelation` and `lookupCachedData` (and the executed plan returning wrong rows) look
responsible; the defect is `cacheQuery` caching at all for `NONE`.

**Fix:** do nothing in `cacheQuery` when `storageLevel == StorageLevel.NONE`.

**Admission (C1–C6):** C1 — symptom (wrong count downstream) ≠ cause (`cacheQuery`). C2 — *moderate*:
`StorageLevel.NONE` is greppable, so this leans navigation. C3 ✓ PR #43188. C4 — 2023. C5 — run
baseline (may solve). C6 — **Scala is OK-tier in monogram → this is a regression guard**, not a
fix-test.
