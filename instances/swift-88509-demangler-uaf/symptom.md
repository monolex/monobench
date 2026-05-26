The Swift demangler crashes (ASAN **heap-use-after-free**) when demangling certain mangled symbols
that cause a nested demangle operation. A saved "words" (substitution) entry ends up pointing at
storage that the inner demangle reused. The nested-demangle code itself is correct.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
