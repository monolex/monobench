# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `ext/uri/uri_parser_whatwg.c :: php_uri_parser_whatwg_parse_ex` (the periodic
`lexbor_mraw_clean()` in the WhatWG parser lifecycle). PR #19591, fix `423960aad30b`, base
`90822f7692e7`.

The parser periodically called `lexbor_mraw_clean(lexbor_parser.mraw)`, which destroys the data in
the shared lexbor memory arena — including the data still owned by **live** `Uri\WhatWg\Url` objects
→ UAF when PHP later reads such an object.

**Decoy:** `php_uri_parser_whatwg_free` is the per-object teardown the fix *adds* (correct), and the
`RSHUTDOWN` handler is a plausible-but-wrong suspect. The defect is the periodic arena clean.

**Fix:** remove the periodic `lexbor_mraw_clean()`; add `php_uri_parser_whatwg_free()` (free per
object); move lexbor teardown from `RSHUTDOWN` to `POST_ZEND_DEACTIVATE` so per-object frees run
before the arena is destroyed.

**Admission (C1–C6):** C1 ✓ crash (reading a Url object) ≠ cause (arena clean elsewhere in the parser
lifecycle). C2 ✓ symptom never says "lexbor"/"mraw". C3 ✓ PR #19591. C4 — recent, niche ext/uri. C5
— run baseline. C6 ✓ PHP object lifetime ↔ lexbor C arena.
