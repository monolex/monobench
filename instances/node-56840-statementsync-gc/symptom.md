A program opens a synchronous embedded-database connection, prepares a statement from it, then lets
the connection value go out of scope while keeping the prepared statement. After garbage collection
runs, the next call on the statement (e.g. executing it / reading rows) segfaults — a use-after-free.

Keeping a reference to the connection, or avoiding GC, makes it disappear. The crash is in a statement
method dereferencing its parent connection, which was collected even though the statement was still
alive and holding it by a bare pointer.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
