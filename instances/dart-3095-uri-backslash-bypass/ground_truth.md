# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `sdk/lib/core/uri.dart :: _Uri._normalizeOrSubstring` (and `_normalizePath`). Commit
`c4c802eeb6fe` (CVE-2022-3095), base `c42a304ce78f`.

Dart's `Uri` did **not** normalize backslash (`\`, 0x5C) to forward-slash (`/`) in the path and
authority. Browsers do. So for an input like `https://allowed.example\@evil.example/`, Dart's
`Uri.parse(...).host` returns `allowed.example` while a browser resolves to `evil.example` →
host-allowlist / open-redirect bypass.

**Decoy:** `Uri.parse` / the `_Uri` factory and the `host` getter are where the wrong value is
observed; the defect is the normalization helper not replacing backslash.

**Fix:** thread a `replaceBackslash` option through `_Uri._normalizeOrSubstring` (and path
normalization) so `\` is treated as `/`.

**Admission (C1–C6):** C1 — symptom (wrong `.host` trusted by an allowlist) ≠ cause (normalization
helper). C2 ✓ symptom never says "backslash". C3 ✓ commit `c4c802eeb6fe`. C4 — 2022 famous-ish CVE →
check contamination. C5 — run baseline. C6 — **pure Dart-language instance → exercises monogram's
Dart extractor (a NO-CALLS language).**
