An HTTP server crashes (segfault / use-after-free) when multiple pipelined requests arrive in a single
TCP segment and the connection is closed synchronously while that batch is still being parsed — for
example a request handler that destroys the socket on the first request. The incoming-data parser is
mid-way through the batch when the connection teardown frees its state; the parser then keeps invoking
its per-message callbacks and faults on freed memory.

It needs the combination: pipelined requests in one packet + a synchronous close during parsing. One
request at a time, or closing asynchronously, never crashes.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
