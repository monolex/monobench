You are working in the ksmbd codebase — the Linux in-kernel SMB3 file server (C, built as a kernel
module). `smb2pdu.c` holds the SMB2 command handlers; sessions and connections are separate objects
(one session can be shared by several connections — SMB 3.x session binding / multichannel).

Symptom: under load with a client that opens MULTIPLE connections bound to the SAME session, ksmbd
intermittently corrupts memory or crashes (KASAN reports a use-after-free; sometimes a NULL
dereference). The faulting access is a READ of the session's user object `sess->user` — its `uid`,
or a `user_guest(sess->user)` check — performed by a worker thread while it services an ordinary
request on one connection. By the time that read happens, `sess->user` has already been freed.

The race is timing-dependent: it opens when one of the bound connections ENDS / TEARS DOWN the shared
session at about the same moment another connection is still mid-request against that same session.
The worker that crashes is simply reading a field it has every reason to expect is valid — so the
function where the crash is OBSERVED is NOT the function that contains the bug.

Find the ROOT CAUSE — the exact function (and file) whose handling of the shared session FREES
`sess->user` without first guaranteeing that no other connection bound to that session can still be
running a request that uses it. Explain the mechanism and the fix. (Note: more than one place in the
session lifecycle frees this object — identify the one that matches the symptom above.)

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
