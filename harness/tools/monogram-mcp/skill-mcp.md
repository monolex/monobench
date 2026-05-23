# monogram — your indexed code-intelligence (as `monogram_*` MCP tools)

This repo's call graph, symbols, and cross-language references are already indexed and exposed as the
`monogram_*` tools (their schemas are in your tool list — you don't need to memorize them). Reach for
them BEFORE grep/read; they answer structural questions text search can't. Start with `monogram_search`
on the symptom, then **follow the next-step hints each tool returns**, climbing: search → pin a
definition → read its context + call graph → trace callers/callees → audit the boundary. `monogram_grep`
is the last resort (an empty grep is not proof of absence). The bug is a call/ownership edge — don't
stop at search.
