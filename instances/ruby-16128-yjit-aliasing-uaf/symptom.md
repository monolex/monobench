Building Ruby with the JIT enabled (`./miniruby --yjit`, `RUBY_YJIT_ENABLE=1`) and running a
workload that defines, invalidates, and redefines many methods makes the interpreter crash
intermittently with a SIGSEGV. Under ASAN the report is a **heap-use-after-free reading the backing
buffer of a `Vec`**.

The faulting frame varies between runs — sometimes while looking up a compiled block version,
sometimes during GC marking of compiled blocks, sometimes while invalidating/removing a block. Each
of those code paths is individually correct: the `Vec` they read was valid when the function was
entered, and was freed/reallocated out from under them by something that ran earlier.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
