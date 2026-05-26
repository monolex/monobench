Loading a Redis stream from an RDB file (or via `RESTORE`) where a consumer's pending-entries list
contains the **same message ID twice** makes the server abort with a double free / heap corruption.
The crash surfaces while the stream object is being torn down — not at the point the malformed entry
is read.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
