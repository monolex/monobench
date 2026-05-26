Under heavy lock contention combined with interrupts/signals, the interpreter occasionally aborts
with a fatal error / use-after-free deep in the low-level synchronization code. A thread that is
releasing a lock wakes a blocked waiter, but the small per-wait synchronization object it signals has
already been torn down — the waiter had been interrupted and returned early, destroying its
stack-allocated wait object before the waker finished signaling it.

It is rare and timing-dependent, only under contention with interruption, and the backtrace lands in
the semaphore/wakeup primitive touching freed stack memory — code that is itself correct, given a
valid object.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
