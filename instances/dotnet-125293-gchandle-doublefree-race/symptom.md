A .NET service making HTTPS requests through the WinHTTP-based `HttpClient` handler crashes
intermittently under load with an access violation / "GCHandle was previously freed", originating in
native WinHTTP callback handling. It only reproduces when a request **completes and is cancelled at
nearly the same moment**.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
