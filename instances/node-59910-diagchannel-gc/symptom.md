A subscriber registered to a named publish/subscribe diagnostics hook intermittently stops receiving
messages after garbage collection runs — the subscription silently vanishes even though nothing ever
unsubscribed it. No crash, no error: published events just stop being delivered.

It reproduces when channels are churned (references to like-named channel objects are created and
dropped so GC fires) and a channel with the same name is re-created. The internal weak-reference
bookkeeping drops a live entry: a delayed cleanup for an old, collected reference removes the freshly
re-created one.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
