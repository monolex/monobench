Neovim leaks libuv (luv) timer handles when a timer-based deferred Lua callback is registered shortly
before the editor exits — Nvim reports lingering handles and does not shut down cleanly. The leak
only happens when the deferred callback cannot be scheduled (which is what occurs during shutdown).
The leak report points at the libuv event loop, not at the code that registered the callback.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
