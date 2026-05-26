A Vapor server that serves static files with `FileMiddleware` discloses files **outside** the
configured public directory. A request whose path contains percent-encoded traversal sequences (for
example `/%2e%2e/%2e%2e/etc/passwd`) is served the out-of-root file. Requests with literal `..` are
correctly rejected.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
