A PHP application that parses many URLs per request with the WHATWG URL API (`Uri\WhatWg\Url`) starts
segfaulting (ASAN: **heap-use-after-free**) when it reads properties of a `Url` object it parsed
earlier in the same request. The read is ordinary PHP; the memory behind the object was reclaimed
after a certain number of parses.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
