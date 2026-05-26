A Ruby program crashes with SIGSEGV (ASAN reports a **heap-use-after-free**) when applying the
bitwise `&` operator to an `IO::Buffer`. The buffers involved were produced with `IO::Buffer#slice`.
ASAN points at a read of the buffer's base pointer inside the C implementation of `&`.

That operator's code is correct in isolation — the memory it dereferences had already been released
before `&` was called.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
