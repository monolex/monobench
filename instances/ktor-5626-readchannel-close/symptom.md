A Ktor service intermittently fails serving files / streaming over channels — on Linux it shows up as
curl WebSocket handle leaks and timeouts. The failure traces to a file-channel helper closing a file
handle that was never actually opened (or closing it on a setup path that errored out before the open
succeeded).

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
