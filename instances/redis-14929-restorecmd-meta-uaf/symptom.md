With a Redis module loaded that subscribes to keyspace notifications and attaches metadata to keys,
issuing a `RESTORE` command intermittently crashes the server with an ASAN **heap-use-after-free**.
The faulting read is the restored object's `type` field, right after the key event fires. The code
performing the read is correct in isolation — the object it points at is no longer valid.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
