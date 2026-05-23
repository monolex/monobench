# Ground truth — ⚠️ SPOILER (never fed to the agent)

**CVE:** CVE-2025-37899 (CVSS 7.8). **Root cause:** `smb2pdu.c :: smb2_session_logoff`.
**Mirror fix commit:** `5f81c9ec0a28` ("ksmbd: Fix use-after-free in session logoff", namjaejeon/ksmbd, 2025-04-22).
**Mainline fix:** `2fc9feff45d9` (Linux 6.15-rc5; stable 6.14.6 / 6.12.28).
**base_commit:** `28d72614627b` (parent of both the logoff fix and the kerberos fix → both UAFs live).

## Mechanism
SMB 3.x lets several **connections** bind to one **session**. `smb2_session_logoff()` tears the session
down: it frees `sess->user` via `ksmbd_free_user()` and sets it NULL. But it only waited for the
**logoff connection's own** outstanding requests to drain — it did **not** account for requests
in-flight on *other* connections bound to the same session. So:

- Worker-B (logoff path) runs `smb2_session_logoff()` → `ksmbd_free_user(sess->user)`.
- Worker-A (a different connection, e.g. inside `smb2_sess_setup` binding to that session, or any
  handler that reads the session user) still dereferences `sess->user` → `sess->user->uid`,
  `user_guest(sess->user)` → **use-after-free** (or NULL-deref after the field is cleared).

The fix blocks new requests on the session and waits for **all** bound connections' in-flight
requests to finish before freeing, so no concurrent user dereference can survive the free.

## Decoy (forced)
At `base_commit` the **kerberos-auth UAF is also present** — `smb2_sess_setup` frees `sess->user`
during Kerberos authentication assuming `ksmbd_krb5_authenticate()` will reinitialize it or it won't
be touched (both false) = **CVE-2025-37778**, fixed separately in `3e2842a1b335`. This frees the SAME
object and greps the same way ("use-after-free of sess->user"), so a shallow agent will name
`smb2_sess_setup` / the krb5 path. That is **DECOY**, not the answer: the symptom describes the
cross-connection *session-teardown* race (one connection ends the shared session while another is
mid-request), which is the **logoff** path, not the auth path. `destroy_previous_session` / preauth
are further plausible-but-wrong session-lifecycle functions.

## Why it meets C1–C6 (SPEC.md)
- **C1** symptom (freed READ in `smb2_sess_setup` / request handlers) ≠ cause (free in
  `smb2_session_logoff`) — different functions, different connections.
- **C2** no text bridge: the symptom never says "logoff"; `sess->user` greps to *every* free/read
  site (incl. the decoy), so text alone cannot pick the cause — the discriminator is the
  cross-connection ownership/lifetime edge (a `chain --callers` / who-frees-vs-who-reads question).
- **C3** objective truth: merged fix commits above.
- **C4** contamination: moderate. The CVE is famous (o3 writeup, May 2025) and likely in training
  data as *prose*; but the task is to localize in *this checkout* and beat the decoy, and the symptom
  is deliberately de-keyworded. Record the risk; do not treat a confident "CVE-2025-37899" recital
  without the correct in-tree function+mechanism as FULL.
- **C5** discriminating (expected): o3 found it at ~1/100 even with all SMB2 handlers (~12k LoC) in
  context (Heelan); the live kerberos decoy actively pulls shallow attempts the wrong way.
- **C6** tool capability on the path: the answer requires tracing who FREES vs who READS `sess->user`
  across handlers/connections = monogram call-graph / symbol-reference territory. codegraph indexes C
  (no forfeit) → a clean monogram-vs-baseline comparison on a single-language ownership bug.

## Provenance
Sean Heelan, "How I used o3 to find CVE-2025-37899…", 2025-05-22
(https://sean.heelan.io/2025/05/22/how-i-used-o3-to-find-cve-2025-37899-a-remote-zeroday-vulnerability-in-the-linux-kernels-smb-implementation/).
First public LLM-discovered novel kernel vulnerability. Control bug in that work = CVE-2025-37778 (the decoy here).
