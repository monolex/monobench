In the dev server with hot reloading, after you edit a component file to remove its `"use client"`
boundary, the next file-change event intermittently crashes with a use-after-free — AddressSanitizer
reports use-after-poison while the watch event is processing a file path. A path string read during
the change event points to memory that was already freed when the graph dropped the demoted file.

The watch-event reader itself is correct (it just splits/hashes the path); it has been handed a
dangling pointer. It only happens after a boundary demotion, and it is flaky / timing-dependent.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
