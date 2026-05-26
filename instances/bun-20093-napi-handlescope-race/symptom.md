A program that loads a native addon intermittently crashes under load. It is a SIGSEGV / heap
corruption that only reproduces when garbage collection runs concurrently with the addon doing
many short-lived native calls (high GC pressure + parallel work). The faulting access is a read
of a freed-or-garbage pointer, or of a growable internal buffer whose backing store appears to
have moved out from under the reader; the backtrace lands in garbage-collection sweep / value
finalization code, never in obviously related addon code.

The crash is timing-dependent: it vanishes under a debugger, with the GC effectively disabled,
or when the workload is single-threaded, and it does not reproduce deterministically. Correct,
long-standing GC code is on the stack — but it is faulting on state that something else mutated
without synchronization.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
