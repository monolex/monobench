A Dart web service validates redirect / callback URLs by parsing them with `Uri.parse()` and checking
the resulting `.host` against an allowlist. An attacker supplies a URL that **Dart's `Uri` parser and
a web browser disagree about**: Dart reports one host (an allowed one) while a browser navigating the
same string goes to a different, attacker-controlled host. The allowlist check passes and the request
is redirected/forwarded to the attacker's host. No exception is thrown — only the parsed host is
"wrong".

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
