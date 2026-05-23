You are working in the Bun runtime codebase (a large Zig + C++ project, ~1900 source files) at
version 1.3.10. Production users report a SERIOUS intermittent crash during long-running sessions
that use Worker threads, async I/O, and N-API native addons (it is crashing Claude Code in the field):

    panic(main thread): switch on corrupt value
        in src/string.zig

The crash is non-deterministic, shows up only after extended use, comes with steadily growing
memory usage (multi-GB), and occasional data corruption. It looks like a memory-safety defect
(use-after-free / double-free) involving Bun's string type and reference counting.

Find the ROOT CAUSE — the exact function and file responsible for the refcount/ownership defect,
NOT merely the crash site where the panic fires. Explain the mechanism and the fix. End your reply
with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
