╔══════════════════════════════════════════════════════════════════════════════╗
║                                                                              ║
║   █▄ ▄█ █▀▀█ █▀▀▄ █▀▀█ █▀▀▀ █▀▀█ █▀▀█ █▄ ▄█   Trigram Fuzzy Search           ║
║   █ ▀ █ █  █ █  █ █  █ █ ▀█ █▄▄▀ █▄▄█ █ ▀ █   Cross-Language Call Chain      ║
║   ▀   ▀ ▀▀▀▀ ▀  ▀ ▀▀▀▀ ▀▀▀▀ ▀ ▀▀ ▀  ▀ ▀   ▀   Grep → Function → Chain        ║
║                                                                              ║
║   AI Code Context Engine — built on Monotology for AI & Human                ║
║   24 Languages: TS JS Rust Py Go Zig PHP C C++ Java Swift Lua Bash           ║
║   Ruby Kotlin Dart Scala HCL TOML YAML JSON Dockerfile CSS HTML              ║
║   Chain: RS invoke → TS classList/var() → CSS token → CSS primitive          ║
║   by Monolex — https://monolex.ai                                            ║
║                                                                              ║
╚══════════════════════════════════════════════════════════════════════════════╝

monogram {VERSION}
USAGE: monogram <COMMAND> [OPTIONS]

FLOW  every command ends in a [NEXT] hint — follow it and the trail walks the whole toolset
      (no command memorized in advance).  context ⇄ chain is the hub.  Full map: flow-guide.md
  ENTRY     index·reindex → search·stats·errors    prune → stats    boot → chain·deps
            stats → search·coupling·metrics
  FIND      search → region·symbols·grep/refgrep   css → symbols·chain
  NAVIGATE  region → context·shallow-chain    symbols → context·chain
            grep/refgrep → chain·context    chain → tree·context
            tree → context            context → chain·coupling    deps ⇄ rdeps → context
  AUDIT     coupling → context·chain·tauri-bind·css·important    errors → context
            metrics → context·chain    uncalled → chain    important → chain (+ migrate)
  TERMINAL  serve/mcp (MCP server)     help

┌──────────────────────────────────────────────────────────────────────────────┐
│  COMMANDS                                                                    │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  grep, g <pattern>    Search raw code lines + structural refs                │
│                       CODE HITS scan indexed files directly (native, no rg)  │
│                       STRUCTURAL REFS search call/reference targets          │
│                       --raw          Code hits only                          │
│                       --refs         Structural refs only                    │
│                       --comments     Include comment-only raw hits           │
│                       -n <limit>     Max results (default: 20)               │
│                       --chain        Show callers of containing function     │
│                       --tree         Show call tree from containing function │
│                       --depth N      Chain/tree depth (default: 2)           │
│                       --json         JSON output                             │
│                                                                              │
│  refgrep, refs <pat>  Search call/reference targets → function → chain       │
│                       Structural-only entry point to call graph. Broad       │
│                       --chain output is capped with region-first NEXT.       │
│                       -n <limit>     Max results (default: 20)               │
│                       --chain        Show callers of containing function     │
│                       --tree         Show call tree from containing function │
│                       --depth N      Chain/tree depth (default: 2)           │
│                       --json         JSON output                             │
│                                                                              │
│  search, s <query>    Search for files matching query                        │
│                       -n <limit>     Max results (default: 10)               │
│                       --cwd          Filter to current directory only        │
│                       --explain      Show why; high limits compact + NEXT    │
│                       --json         JSON output                             │
│                                                                              │
│  region, locate <q>   Rank functional regions from fuzzy, raw, refs, graph   │
│  discover <q>         and coupling evidence. Best step after search when     │
│                       you need "where is this implemented?", not only files. │
│                       -n <limit>      Max regions (default: 5)               │
│                       --score-debug   Show score components for tuning       │
│                       --domain D      Bias coupling evidence by domain       │
│                       --json          JSON output                            │
│                                                                              │
│  index, i <path>      Index source files in directory                        │
│                       --ext ts,rs,js Custom extensions                       │
│                       --add <dir>    Include extra dir (skips exclusions)    │
│                                      Can be used multiple times              │
│                       --no-workspace Disable Cargo/npm workspace auto-detect │
│                       --skip-if-locked Daemon mode: yield if index is locked │
│                       Symbols extracted by default (Tree-sitter call graph)  │
│                                                                              │
│  deps <file>          Show what file imports (dependencies)                  │
│                       --json         JSON output                             │
│                                                                              │
│  rdeps <file>         Show who imports file (reverse dependencies)           │
│                       --json         JSON output                             │
│                                                                              │
│  css <entry.css>      Flatten CSS @import graph in source/cascade order      │
│  css-order <entry.css> Alias for css                                         │
│                       --json         JSON output                             │
│                                                                              │
│  coupling, bind, edges Cross-process binding audit. Joins client call sites  │
│                       to server route definitions across the network/DB edge.│
│                       Domains: http / sql / pubsub / tauri-ipc / ffi /       │
│                       event / css-token / export-import.                     │
│                         OK_Bound          → defined + called both present    │
│                         ORPHAN_Call       → caller, no handler (runtime risk)│
│                         ORPHAN_Define     → handler, no caller (dead code)   │
│                         AMBIGUOUS         → key defined in multiple places   │
│                       Languages: TS/JS, Python, PHP, Rust + cross-lang Tauri.│
│                       HTTP frameworks: Next.js pages/app, Express, Hono,     │
│                       Koa, Fastify, FastAPI, Flask, Laravel, Axum + fetch,   │
│                       axios, useSwr, useQuery, requests, httpx, reqwest.     │
│                       SQL: CREATE/ALTER/DROP TABLE ↔ SELECT/INSERT/UPDATE.   │
│                       Pubsub: emit/publish/fire/trigger ↔ on/subscribe/bind. │
│                       FFI: Rust #[no_mangle]/#[uniffi::export] + Zig export  │
│                       fn ↔ Swift @_silgen_name/C-call, Kotlin external fun.  │
│                       Each binding ships a [NEXT] hint with reason +         │
│                       recommended_edit + verify_command (same shape as       │
│                       `errors` so AI agents can chain).                      │
│                       --domain D   http|sql|pubsub|tauri|ffi|event|          │
│                                    css-token|export-import                   │
│                       --framework FW   Substring match on site.framework     │
│                                        (nextjs-pages, nextjs-app, hono,      │
│                                         chained-method, fetch, axios, swr,   │
│                                         react-query, tauri, ...)             │
│                       --pattern P      Substring match on key                │
│                       --category C     Substring match on category name      │
│                       --json           AI-friendly JSON output               │
│                       --all            Also list OK_Bound (hidden by default)│
│                       --min-confidence F  Confidence floor (default 0.50)    │
│                                                                              │
│  tauri-bind, tauri,    Alias for `coupling --domain tauri-ipc`. Preserves    │
│  ipc                   the original Tauri-IPC audit. Categories map to:      │
│                         ORPHAN_Invoke / Listener → ORPHAN_Call               │
│                         ORPHAN_Handler / Emit    → ORPHAN_Define             │
│                       --name N → --pattern N (substring filter on key).      │
│                                                                              │
│  errors, err          Categorise cargo / tsc build errors with NEXT hints.   │
│                       Auto-runs `cargo check` or reads from a log file.      │
│                         MERGE_CONFLICT          → leftover git markers       │
│                         MISSING_RESOURCE        → tauri externalBin missing  │
│                         FRONTEND_DIST_MISSING   → vite build not run yet     │
│                         PROC_MACRO_PANIC        → macro expansion failure    │
│                         UNRESOLVED_IMPORT       → use/import cannot resolve  │
│                         SYNTAX_ERROR            → expected X, found Y        │
│                         TYPE_MISMATCH           → E0308 / E0004 / E0061      │
│                         TRAIT_BOUND             → E0277 / E0271 / E0282      │
│                         BORROW_CHECK            → E05xx / E07xx              │
│                         TS_TYPE_MISMATCH        → tsc TS2322 / TS2345        │
│                       --from <log>     Parse pre-saved log file              │
│                       --stdin, -       Read from stdin (pipe mode)           │
│                       --json           AI-friendly JSON output               │
│                       --category C     Filter (substring match)              │
│                                                                              │
│  important report <path>    Pure analysis aggregator (no prescriptions):     │
│                       hot spots / property heatmap / selector clusters /     │
│                       cross-file conflict pairs / JS-CSS bridge inventory.   │
│                       --json           AI-friendly JSON output               │
│                                                                              │
│  important migrate <path>   Aggregate audit findings into structural         │
│                       refactor plans (cascade layers / JS-to-CSS-var /       │
│                       layer hierarchy / utility layer / specificity match /  │
│                       runtime-to-static var). Each plan ranks rules          │
│                       eliminated against estimated risk.                     │
│                         CASCADE_LAYERS_VENDOR     → vendor stylesheets       │
│                         JS_TO_CSS_VARIABLE_BRIDGE → JS .style.X refactor     │
│                         LAYER_HIERARCHY_INTERNAL  → resolve CONFLICTING      │
│                         UTILITY_LAYER             → .hidden/.sr-only         │
│                         SPECIFICITY_MATCH         → raise selector spec      │
│                         RUNTIME_TO_STATIC_VAR     → kill runtime <style>     │
│                         PSEUDO_STATE_HIERARCHY    → :hover via :where()      │
│                         CASCADE_ORDER_FIX         → @import or @layer order  │
│                       --json           AI-friendly JSON output               │
│                       --pattern P      Filter (substring match)              │
│                       --limit N        Top N plans by priority               │
│                                                                              │
│  important [path]     Audit every `!important` declaration. Each one is      │
│                       classified into 12 categories with an AI-actionable    │
│                       NEXT hint:                                             │
│                         REDUNDANT_NoCompetitor   → safe to remove            │
│                         REDUNDANT_Outspeced      → safe to remove            │
│                         NEEDED_Specificity       → refactor selector         │
│                         NEEDED_CascadeOrder      → reorder @import           │
│                         NEEDED_MediaQuery        → keep (print/a11y)         │
│                         NEEDED_Animation         → keep (motion stop)        │
│                         NEEDED_UtilityClass      → keep (.hidden, .sr-only…) │
│                         NEEDED_PseudoState       → keep (:hover/:focus)      │
│                         OVERRIDES_Vendor         → keep (xterm/monaco/…)     │
│                         OVERRIDES_InlineStyle    → keep (JS .style.X = …)    │
│                         OVERRIDES_RuntimeStyleTag → keep (JS createElement)  │
│                         CONFLICTING_BothImportant → human review             │
│                       --json         JSON output                             │
│                       --category C   Filter (substring match)                │
│                       --file PAT     Filter by file path substring           │
│                       --selector S   Filter by selector substring            │
│                       --no-inline-style    Skip B1 (inline style) detector   │
│                       --no-runtime-style   Skip B3 (runtime tag) detector    │
│                       --inline-style-confidence high  Hide inferred B1 hits  │
│                       --runtime-style-confidence high Hide inferred B3 hits  │
│                                                                              │
│  chain, c <symbol>    Trace symbol call chain (BFS, flat list)               │
│                       --callers      Show who calls symbol (default)         │
│                       --callees      Show what symbol calls                  │
│                       --depth N      Max depth (default: 3)                  │
│                       --strict       No fuzzy matching (exact + normalized)  │
│                       --fuzzy=N      Set fuzzy threshold (0.0-1.0, def: 0.5) │
│                       --lang <ext>   Filter by language                      │
│                                      ts,rs,py,go,zig,php,c,cpp,java,         │
│                                      swift,lua,sh,rb,kt,dart,scala,          │
│                                      tf,toml,yml,json,css,html               │
│                       --file <pat>   Filter by file path substring           │
│                       --through <sym> Only paths through this symbol         │
│                       --in-class C   PHP: restrict to method defs in class C │
│                       --json         JSON output                             │
│                                                                              │
│  tree, t <symbol>     DFS call tree (hierarchical structure)                 │
│                       --callers      Show who calls symbol (default)         │
│                       --callees      Show what symbol calls                  │
│                       --depth N      Max depth (default: 3)                  │
│                       --strict       No fuzzy matching (exact + normalized)  │
│                       --fuzzy=N      Set fuzzy threshold (0.0-1.0, def: 0.5) │
│                       --lang <ext>   Filter by language                      │
│                                      ts,rs,py,go,zig,php,c,cpp,java,         │
│                                      swift,lua,sh,rb,kt,dart,scala,          │
│                                      tf,toml,yml,json,css,html               │
│                       --file <pat>   Filter by file path substring           │
│                       --through <sym> Only paths through this symbol         │
│                       --in-class C   PHP: restrict to method defs in class C │
│                       --json         JSON output                             │
│                                                                              │
│  symbols <query>      Search symbols (functions, classes, etc.)              │
│                       --comments     Also concept-match query vs docstrings  │
│                       --json         JSON output                             │
│                       CSS: 2+ custom-property hits → token-family audit:     │
│                       value map · duplicate collisions · raw-literal drift   │
│                                                                              │
│  context, ctx <q>     One-shot context bundle for a symbol (fuzzy ok):       │
│                       entry + signature + calls/called-by + source. Lets an  │
│                       agent get structure AND code in one call (fewer Reads).│
│                       Source blocks are cat -n line-numbered for citation.   │
│                       --code N       Source lines per entry (default: 40)    │
│                       --no-code      Structure only (skip source)            │
│                       -n N           Max entry points (default: 8)           │
│                       --json         JSON output                             │
│                                                                              │
│  mcp, serve           Run as an MCP server (stdio JSON-RPC). Exposes search, │
│                       symbols, grep, chain, tree, deps, rdeps, coupling,     │
│                       context, metrics, uncalled — 11 tools for agents.      │
│  mcp-schema           Print the MCP tool schema JSON used by wrappers.       │
│                                                                              │
│  boot, b              Trace app boot sequence (ESM → DOM → init)             │
│                       init           Auto-detect → .monogram/boot.toml       │
│                       <entry>        Override entry point file               │
│                       Config: .monogram/boot.toml (priority arrays,          │
│                       dead guards, entry point)                              │
│                                                                              │
│  stats                Show database statistics                               │
│  metrics, m           Per-symbol metrics: length, fan-in/out, params, depth  │
│                       --summary rollup · --over · --json · -n N · --max-len N│
│                       --max-params N · --lang <ext> · --file <pat>           │
│  uncalled             Functions with no inbound call-edge (fact, not verdict)│
│                       not exported · unique-named                            │
│                       --lang <ext> · --file <pat> · -n N · --json            │
│  reindex <path>       Clear and reindex (supports --add, --ext)              │
│  prune                Remove deleted files from index                        │
│                       --dry-run      Preview only                            │
│  help                 Show this help                                         │
│                                                                              │
│  GLOBAL FLAGS (any command)                                                  │
│  -r, --reindex        Reindex current dir before running (latest results)    │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────────────┐
│  AGENT WORKFLOWS                                                             │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Search is not the whole tool. It only ranks candidates. After `search`,     │
│  move to structure:                                                          │
│                                                                              │
│    monogram symbols "<name>"             exact definitions + line numbers    │
│    monogram grep "<call>" --chain        call expression → function → callers│
│    monogram context <symbol> --code 80   source + callers/callees bundle     │
│    monogram chain <symbol> --callers     prove inbound paths                 │
│    monogram tree <symbol> --callees      prove downstream effects            │
│    monogram coupling --summary           HTTP/SQL/pubsub/IPC/FFI/event/CSS   │
│    monogram metrics --summary            structural risk / fan-in / fan-out  │
│    monogram uncalled --lang <ext>        no inbound edge candidates          │
│    monogram stats                        index health + maintenance entry    │
│    monogram prune --dry-run              check deleted-file index rows       │
│    monogram boot init                    discover app boot entry config      │
│    monogram mcp-schema                   wrapper-visible tool surface        │
│                                                                              │
│  Ownership / FFI / lifetime bugs                                             │
│  ────────────────────────────────                                            │
│  For UAF, leak, refcount, retain/release, cross-thread, C ABI, Zig/C++, or   │
│  "corrupt value" symptoms, do not repeat broad `search`. Pivot by ownership  │
│  verbs and boundary contracts:                                               │
│                                                                              │
│    monogram region "ownership boundary ref deref leakRef isolatedCopy" -n 5  │
│    monogram refgrep "isolatedCopy" --chain --depth 2                         │
│    monogram refgrep "leakRef" --chain --depth 2                              │
│    monogram refgrep "deref" --chain --depth 2                                │
│    monogram refgrep "ref" --chain --depth 2                                  │
│    monogram coupling --domain ffi --pattern "<candidate>" --all              │
│    monogram context <candidate> --code 80                                    │
│                                                                              │
│  OK_Bound means the boundary is wired, not that ownership is correct.        │
│  A same-file sibling helper is a decoy until you prove the balance. Broad    │
│  ecosystem symbols like String/toSlice/fromJS/ref/deref need region/context  │
│  before deep caller expansion.                                               │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────────────┐
│  FORMULA                                                                     │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│      Score = Tri% × (1 + Σmonogram³ / ³√size / 5)                            │
│              ────────     ─────────────────                                  │
│                 │              │                                             │
│                 │              └── Compound Bonus Term                       │
│                 │                                                            │
│                 └── Base trigram match percentage                            │
│                                                                              │
│  WHERE:                                                                      │
│    Tri%       = matched / total query trigrams (0.0 to 1.0)                  │
│    Σmonogram³ = sum: n=0→0, n=1→1, n≥2→(n+1)³ per identifier                 │
│    ³√size    = cbrt(identifier_count) - size normalization                   │
│    5         = tuning constant                                               │
│                                                                              │
│  EXAMPLE:                                                                    │
│    Query: "terminal font size"                                               │
│    Trigrams: [ter][erm][rmi][min][ina][nal][fon][ont][siz][ize]              │
│                                                                              │
│    terminalFontSize → 3 words → (3+1)³ = 64 bonus                            │
│    fontSize         → 2 words → (2+1)³ = 27 bonus                            │
│    terminal         → 1 word  → 1 bonus                                      │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────────────┐
│  CODE GRAPH (deps / rdeps)                                                   │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Import relationships are extracted during indexing and stored in the        │
│  'relations' table. This enables dependency graph navigation.                │
│                                                                              │
│  deps <file>    "What does this file depend on?"                             │
│  ────────────────────────────────────────────────                            │
│  Shows all import/require statements found in the file.                      │
│  Results are split into Internal (./relative) and External (@pkg).           │
│  Plain output preserves source order; --json includes line/order/raw text.   │
│                                                                              │
│      main.ts                                                                 │
│         ├── import ./module-loader        (Internal)                         │
│         ├── import ./modules/terminal     (Internal)                         │
│         └── import @tauri-apps/api/core   (External)                         │
│                                                                              │
│  rdeps <file>   "Who imports this file?" (reverse dependencies)              │
│  ────────────────────────────────────────────────                            │
│  Shows all files that import the given file.                                 │
│  Matches by resolved target file, exact path, filename, or import spelling.  │
│                                                                              │
│      terminal-instance.ts                                                    │
│         ↑── main.ts                                                          │
│         ↑── module-loader.ts                                                 │
│         ↑── terminal-creation.ts                                             │
│                                                                              │
│  css <entry.css>   "What is the CSS cascade import order?"                   │
│  ────────────────────────────────────────────────                            │
│  Follows CSS @import edges recursively, preserving source_order.             │
│  Marks duplicate imports and exposes line/order/raw text in --json.          │
│                                                                              │
│  SUPPORTED LANGUAGES:                                                        │
│    TypeScript/JavaScript: import/export from, require()                      │
│    Rust: use crate::, mod name;                                              │
│    Python: import, from ... import                                           │
│    Go: import "path"                                                         │
│    Zig: @import("path")                                                      │
│    PHP: use Namespace\\Class, require/include                                │
│    C: #include "file.h"                                                      │
│    Java: import com.example.Class                                            │
│    Swift: import Module                                                      │
│    Lua: require("module")                                                    │
│    Bash: source file, . file                                                 │
│    Ruby: require, require_relative                                           │
│    C++: #include "file.h"                                                    │
│    Kotlin: import com.example.Class                                          │
│    Dart: import 'package:...'                                                │
│    Scala: import scala.collection                                            │
│    HCL/Terraform: module source                                              │
│    Dockerfile, TOML, YAML, JSON: indexed for search                          │
│    CSS: @import url()                                                        │
│    HTML: <link href>, <script src>                                           │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

EXAMPLES:
  # Grep: search code patterns → find containing function → trace chain
  $ monogram grep "conn.execute"
  $ monogram grep "conn.execute" --chain            # + show callers
  $ monogram grep "conn.execute" --tree --depth 3   # + show call tree
  $ monogram grep "localStorage.setItem" --chain
  $ monogram grep "emit(" -n 5 --json

  $ monogram search "terminal font size"
  $ monogram search "session manager" -n 5 --json
  $ monogram index ~/Projects/myapp --ext ts,rs,css,html
  $ monogram index . --ext ts --add node_modules/@tauri-apps/api
  $ monogram deps main.ts                       # What does main.ts import?
  $ monogram deps src/styles/main.css --json    # Ordered imports + line metadata
  $ monogram css src/styles/main.css             # Flatten recursive CSS import order
  $ monogram css src/styles/main.css --json      # Machine-readable cascade graph
  $ monogram rdeps terminal-instance.ts         # Who imports this file?

  # Chain graph (who calls / what calls)
  $ monogram chain create_session --callers --depth 2
  $ monogram tree file-tree-header --callees --strict --depth 3

  # Context bundle — structure + source in one call (cuts file Reads)
  $ monogram context setup_session_actor         # entry + calls/called-by + line# source
  $ monogram context TerminalSession --code 60    # more source per entry
  #   NEXT: the bundle lists `calls:` / `called by:` — pick one and
  #         `monogram context <that>` to expand, or `chain <sym> --callees` for the tree.

  # Concept search — find code by what it DOES (matches docstrings, not names)
  $ monogram symbols --comments "retry with backoff"
  $ monogram symbols --comments "clipboard escape sequence"
  #   NEXT: take the matched symbol → `monogram context <symbol>` for source + chain.

  # Correctness — is it wired / dead / broken?
  $ monogram coupling --summary                   # health across all domains
  $ monogram coupling --domain export-import      # dead exports + broken imports
  $ monogram coupling --domain export-import --all # also show OK_Bound (healthy)
  #   NEXT: ORPHAN_Define = dead export (drop it, unless public API / window.X /
  #         re-export); ORPHAN_Call = broken import (typo / rename / missing export).
  #         Every finding ends in a [NEXT] hint with a verify_command.

  # MCP server — expose the tools to an agent
  $ monogram mcp                                  # stdio JSON-RPC (search/chain/coupling/context/…)

  # CSS Design Token chain (variables)
  $ monogram chain "--color-text-tertiary" --depth 2    # Who uses this token?
  $ monogram tree "--color-text-tertiary" --callees --strict --depth 3  # Nested vars
  $ monogram tree ":root" --callees --strict --depth 1  # All tokens defined in :root
  $ monogram tree "body" --callees --strict --depth 1   # Theme-overridable tokens

  # CSS token-family audit — auto-appended when symbols matches 2+ custom properties.
  # Treats a token scale as a SET: per-token value + use-count, duplicate-value
  # collisions, and raw literals that bypass the tokens (mapped to the right token).
  # Literal-drift fires when the stem is a real CSS property (font-size, border-radius);
  # families like spacing/color still get the value map + collision view.
  $ monogram symbols "font-size"      # value map, dup collisions, font-size literal drift
  $ monogram symbols "border-radius"  # raw radius literals → var(--border-radius-*)

  # CSS Selector chain (classes / ids / elements / pseudos)
  # Symbol names preserve the user's syntax (`.foo`, `#bar`, `:root`, `body`)
  # so queries match exactly.
  $ monogram chain ".scroll-view" --depth 2 --strict   # Who styles .scroll-view?
  $ monogram chain "#md-fileList" --depth 2 --strict   # All rule sites for #md-fileList
  $ monogram chain "body.theme-dracula" --strict       # Theme-scoped overrides
  $ monogram chain "body" --strict                     # Tag selector callers
  $ monogram chain "*" --strict --lang css             # Universal-selector rules

  # Compound selectors register both pieces — `body.theme-x` indexes under
  # `body.theme-x` AND `body` AND `.theme-x`, so any of those queries works.
  # `:is(#a, .b, c)` recurses into its arguments so inner selectors are
  # traceable individually (great for design-system audits).

  # Multi-definition surface — when the same symbol exists in many files
  # (e.g. a CSS custom property re-declared per theme), chain/tree expose every
  # definition site under "+ also @ ..." or "+ N more definitions".

  # Cascade-order diagnosis — flag CSS @import duplicates that shift cascade
  # in build vs dev (Vite/Rollup dedupe symptom).
  $ monogram css src/styles/main.css                    # Recursive cascade order
  $ monogram css src/styles/main.css --json |\
      jq '[.[] | select(.duplicate_of != null)]'        # Find duplicates only

  # !important audit — classify every !important and emit NEXT hints
  # so an AI agent can mechanically fix REDUNDANT_* and surface
  # CONFLICTING_BothImportant for human review.
  $ monogram important src/styles                       # text grouped output
  $ monogram important src/styles --json | jq '[.[] | select(.category=="REDUNDANT_NoCompetitor")]'
  $ monogram important --category REDUNDANT --file prompt.css   # narrow
  $ monogram important --selector '#md-fileList'        # one selector

  # Scope-mismatch hint — if a token is defined under `body` but referenced
  # by a rule that matches `<html>` (e.g. `* { color: var(--x) }`), the
  # variable will be unresolved on the html element. `tree "body" --callees`
  # vs `tree ":root" --callees` makes this diff visible at a glance.

  # Filter: narrow by language or file
  $ monogram chain "--color-text-tertiary" --depth 1 --lang ts   # TS only
  $ monogram chain "--color-text-tertiary" --depth 1 --lang css  # CSS only
  $ monogram tree get_theme_ui_colors --callees --lang rs        # Rust only

DATABASE: ~/.monolex/monogram/<project>-<hash>.db (per-project)

To know what you don't know — that is true knowledge.
不知爲不知 是知也
Close the path first, then search within it.


──────────────────────────────────────────────────────────────────────
[DB] Indexed: 1122 files (642 ts, 190 css, 182 rs, 66 html, 26 js, 8 py, 4 sh, 4 json) | 59370 identifiers
[DB] Symbols: 46208 (20832 css, 16000 ts, 7558 rs, 1492 html, 230 js, 90 py, 6 sh)
[DB] Refs: 167631 (73305 call, 44 import, 93540 read, 742 write) — chain/tree ready

[TIP] 8-language cross-chain available (ts → css → rs → html → js → py → sh → json)
  Try: monogram chain "<symbol>" --depth 2
  Try: monogram tree "<css-var>" --callees --strict --depth 3
──────────────────────────────────────────────────────────────────────
