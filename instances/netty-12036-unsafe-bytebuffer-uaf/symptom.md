A Netty application using the new `Buffer` API crashes the JVM with a SIGSEGV (or reads garbage) when
it accesses a `java.nio.ByteBuffer` that it obtained from inside `forEachReadable`/`forEachWritable`
and kept past the iteration. The `ByteBuffer` access itself is ordinary; the off-heap memory it
points to had already been released. Only happens with the `Unsafe`-backed buffer implementation.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
