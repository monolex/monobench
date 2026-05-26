Iterating query results with `for...of` over a prepared statement's row iterator occasionally crashes
(segfault / memory corruption) when garbage collection runs during the loop. The underlying native
statement gets finalized while the loop is still pulling rows, so the next step reads a freed handle.

Forcing GC pressure mid-iteration reproduces it; holding the statement in a variable and iterating
without GC does not. The crash is in the iterator's step accessing the native statement, which had
been collected out from under it.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
