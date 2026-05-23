# Ground truth — SPOILER (the real fix PR — author the instance FROM this, never feed to the solver)

**nodejs/node PR #62095** — http: fix use-after-free when freeParser is called during llhttp_execute

**fix commit:** a06e789625ec · **base (merge^):** 8edeff9aa715 · merged 2026-03-06

## Changed source files (test/fixture files filtered out)
- lib/_http_common.js
- src/node_http_parser.cc

## PR body

When pipelined requests arrive in one TCP segment, llhttp_execute() parses them all in a single call. If a synchronous 'close' event handler invokes freeParser() mid-execution, cleanParser() nulls out parser state while llhttp_execute() is still on the stack, crashing on the next callback.

Add an is_being_freed_ flag that freeParser() sets via parser.markFreed() before cleaning state. Proxy::Raw checks the flag before every callback and returns HPE_USER to abort execution early if set.

Fix/test was created using AI support.

Refs: https://github.com/nodejs/node/pull/61995#issuecomment-3975819280
