Grouping an iterable by a key — where the key objects define a custom `__eq__` — intermittently
crashes with a segfault / use-after-free. It happens when that `__eq__`, invoked while the grouping
machinery compares the current key against the target key, re-enters the same grouping iterator and
mutates its state (advances it / drops the current key) during the comparison.

The C grouping step is comparing two key objects by borrowed pointer; one of them gets freed inside
the user comparison, and the step then dereferences freed memory. Plain keys (default `__eq__`, no
re-entrancy) never crash.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
