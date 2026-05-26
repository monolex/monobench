An OpenResty / lua-nginx-module server that uses `ngx.pipe` to spawn subprocesses crashes (segfault)
when a client connection is closed over HTTP/3 (QUIC). gdb shows the crash in a pipe stdout-read
cleanup handler that runs while the connection is being torn down — the handler dereferences memory
that was already freed when the connection's pool was destroyed.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
