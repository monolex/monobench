An HTTPS request to a server whose certificate does not match the expected hostname (TLS verification
on / `rejectUnauthorized: true`) intermittently crashes with a use-after-free — AddressSanitizer
reports use-after-poison on a WRITE. The faulting frame is the code that runs when the TLS handshake
completes: it writes to / tears down the connection object after the verification-failure path has
already closed and destroyed that same connection earlier in the same synchronous call.

Only the rejection path triggers it; valid certificates never crash. The free and the offending write
are reachable from the one handshake-completion handler, so the freed object is touched again before
the function returns.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
