Envoy crashes with an ASAN **heap-use-after-free** in the `file_server` HTTP filter when a client
**cancels** a request that is in the middle of serving a file. An asynchronous filesystem callback
fires after the request — and the object streaming the file — has already been destroyed, and the
callback dereferences freed memory. The callback body itself is correct.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
