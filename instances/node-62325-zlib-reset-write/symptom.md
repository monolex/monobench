A compression stream (deflate/gzip-style) crashes — segfault / use-after-free — when its `reset()` is
called while an asynchronous write or flush is still in flight (the work was dispatched to a
background threadpool thread and its callback hasn't fired yet). The background worker faults on the
compression state that `reset()` tore down underneath it.

Calling `reset()` immediately after a write, before the write's callback, triggers it; resetting only
when idle is fine. Note that close() and write() guard against this same situation, but reset() does
not.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
