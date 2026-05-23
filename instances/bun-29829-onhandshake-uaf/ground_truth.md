# Ground truth — SPOILER (the real fix PR — author the instance FROM this, never feed to the solver)

**oven-sh/bun PR #29829** — http: fix use-after-free in onHandshake when checkServerIdentity rejects (u9sf0l)

**fix commit:** fad191de5856 · **base (merge^):** 245c1e4e84ae · merged 2026-04-28

## Changed source files (test/fixture files filtered out)
- src/boringssl.zig
- src/http/HTTPContext.zig
- src/http/ProxyTunnel.zig
- src/js/node/_http_client.ts
- src/js/node/_http_server.ts

## PR body

## What

Fixes an ASAN use-after-poison in `HTTPContext.onHandshake` and ships the related `node:https` compat pieces that the grouped tests exercised once the crash was out of the way.

## Repro

Any HTTPS request via `node:https` (or `fetch()`) with `rejectUnauthorized: true`, a trusted CA, and a hostname that does **not** match the certificate's identity:

```js
const https = require("https");
// server uses agent1-cert.pem (CN=agent1, no SAN)
https.request({ port, ca, servername: "not-agent1" }, ...).end();
```

On ASAN builds this crashes with `use-after-poison` at `src/http/HTTPContext.zig:420`.

## Cause

`checkServerIdentity()` returning `false` means it already called `closeAndFail()` → `terminateSocket()` + `fail()` → `unregisterAbortTracker()` + result callback → `onAsyncHTTPCallback()` → **`threadlocal_http.deinit()` destroys the `AsyncHTTP` that embeds the `HTTPClient`**.

`onHandshake` then wrote `client.flags.did_have_handshaking_error = true` and called `client.unregisterAbortTracker()` on the freed `client`. Both operations were redundant (already done inside `closeAndFail` / `fail`).

ASAN poison-history trace:
```
Memory was manually poisoned by thread T12:
    ... mi_free ...
    http.AsyncHTTP.onAsyncHTTPCallback  src/http/AsyncHTTP.zig:402
    http.HTTPClientResult.Callback.run  src/http.zig:2296
    http.fail                           src/http.zig:2059
    http.closeAndFail                   src/http.zig:1712
    http.checkServerIdentity            src/http.zig:132
    http.HTTPContext...onHandshake      src/http/HTTPContext.zig:419
```

## Fix

### Use-after-free

In `HTTPContext.zig` and `ProxyTunnel.zig`, when `checkServerIdentity` returns `false`, return immediately without touching `client`/`this` — the socket is already terminated, the abort tracker already unregistered, and the result callback may have freed the client.

### CN fallback in `checkX509ServerIdentity`

Once the crash was gone, all seven tests still hung: they use Node's fixture certs (`agent1`, `agent3`, `rsa_cert`) which carry **only** a Subject CN with no SAN extension. Bun's native `checkX509ServerIdentity` checked SAN only and rejected these. Node's `tls.checkServerIdentity` (and undici's `fetch`) fall back to the Subject CN when the certificate has no DNS/IP/URI SAN entries. Added that fallback for non-IP hostnames.

### `checkServerIdentity` forwarding in `https.request`

`_http_client.ts` copied `ca`/`cert`/`key`/etc. from the merged agent/request options into the native TLS config but dropped `checkServerIdentity`. A user-supplied callback was never invoked; the native check ran instead. Now forwarded and validated.

### `requestCert`/`rejectUnauthorized` forwarding in `https.Server`

`_http_server.ts` passed `ca` to `Bun.serve({ tls })` but not `requestCert`/`rejectUnauthorized`. The uSockets SSL context treats `ca` with the (defaulted) `rejectUnauthorized: true` as `SSL_VERIFY_PEER | SSL_VERIFY_FAIL_IF_NO_PEER_CERT`, so `https.createServer({ key, cert, ca })` rejected every client that didn't present a client cert. Node only requests a client cert when `requestCert: true`. Mirror the existing `tls.Server` workaround: default `requestCert` to `false` and, when not requesting, force `rejectUnauthorized` to `false` so the CA is loaded without requiring a client cert.

## Verification

New `test/js/node/http/node-https-checkServerIdentity.test.ts` exercises all four fixes. Without this change it crashes with ASAN use-after-poison / times out; with it, all four cases pass.

The seven originally-grouped Node tests no longer crash but still need separate feature work to pass end-to-end (`req.socket.authorized`, `agent.sockets`/`freeSockets` tracking, `res.socket.getSession()`, `server.setSecureContext()`), so they are not checked in here.
