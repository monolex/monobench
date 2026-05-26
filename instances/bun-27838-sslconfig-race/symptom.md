Intermittent SIGSEGV — a read at address 0x0 — when issuing HTTPS requests through an HTTP proxy
under concurrency. The faulting frame is a `strlen()` on a NULL/garbage pointer inside the C code
that installs the certificate / private-key while building a connection's TLS context. It only
reproduces when many TLS requests run at once and one connection is being torn down at the same
moment another is starting: the teardown frees the cert/key buffers, and the starting connection
then reads them ~microseconds later.

Serial requests, a single request, or no proxy never crash. It is timing-dependent — a debug build
with extra logging around connection setup widens the window and crashes reliably (3/3), while a
clean build crashes only occasionally. The C TLS-context code on the stack is correct; it is being
handed an already-freed configuration from somewhere upstream.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
