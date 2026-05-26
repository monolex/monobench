# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `common/.../PlatformDependent.java :: directBuffer` (PR #12036, fix `71d829a34236`,
base `5b0e0a9820f1`).

`forEachReadable`/`forEachWritable` on an `UnsafeBuffer` can hand out a `java.nio.ByteBuffer` backed
by native memory. If that `ByteBuffer` is siphoned out of the iteration and outlives the
`UnsafeBuffer`, **nothing keeps the `UnsafeMemory` alive** → the GC/Cleaner frees the native memory →
UAF on later access.

**Decoy:** the crash is in plain `ByteBuffer` access; `UnsafeBuffer.readableBuffer` /
`ReadableComponent` hand out the buffer. The defect is `directBuffer` creating the buffer with no
attachment to root the native memory.

**Fix:** `PlatformDependent.directBuffer` takes an **attachment** (the owning `UnsafeMemory`) so GC
keeps the native memory alive while the `ByteBuffer` is reachable; `readableBuffer`/`writableBuffer`
pass it.

**Admission (C1–C6):** C1 ✓ crash (ByteBuffer access) ≠ cause (buffer creation w/o attachment).
C2 ✓ symptom names the iteration API but not `directBuffer`/`attachment`. C3 ✓ PR #12036. C4 — older
(2022) but niche new-Buffer-API internals. C5 — run baseline. C6 ✓ Java↔native(Unsafe) memory
lifetime. ⚠ 6-file fix → larger grade surface.
