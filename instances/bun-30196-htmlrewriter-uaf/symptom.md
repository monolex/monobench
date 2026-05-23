You are working in the Bun codebase — a large JavaScript runtime (Zig core, C++/JSC bindings, plus
generated glue under codegen/). The bug is in Bun's `HTMLRewriter` API.

Symptom: a flaky, fuzzer-found SIGSEGV. On ASAN builds it surfaces as a use-after-poison
(use-after-free) whose faulting access is inside the WebCore Response finalizer, hit during a LATER
garbage-collection sweep:

    AddressSanitizer: use-after-poison
      JSRef.deinit             src/bun.js/bindings/JSRef.zig
      JSRef.finalize           src/bun.js/bindings/JSRef.zig
      Response.finalize        src/bun.js/webcore/Response.zig
      ResponseClass__finalize  codegen/ZigGeneratedClasses.zig
      JSResponse::~JSResponse   codegen/ZigGeneratedClasses.cpp   (crash)

So: the GC sweeps a Response's JS wrapper, runs its destructor, and the Response object it points at
has already been freed.

Reproduction needs ALL of: build an `HTMLRewriter`; register a document/element handler whose `text()`
callback returns a REJECTED promise for the final chunk (`chunk.lastInTextNode`); call
`rewriter.transform()` on a BUFFERED input (an `ArrayBuffer` or string); then force GC (`Bun.gc(true)`).
If nothing rejects, or the streaming path is used instead of a buffered transform, there is no crash.

The Response finalizer / generated JS destructor where the crash is OBSERVED is NOT the bug — it is
correctly destroying what it believes it solely owns. Find the ROOT CAUSE: the exact function whose
handling of the buffered transform, on the handler-rejection error path, ALSO releases that same
Response — even though its JS wrapper is still responsible for it — producing the second free that the
GC later trips over. Explain the mechanism and the fix.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
