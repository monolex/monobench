# Agy Parser Patterns - 2026-05-24

## Source Set

Analyzed existing agy transcripts under:

`results/bun-1.3.10-toThreadSafe/*.agy.jsonl`

No active benchmark sessions were modified.

## Observed Event Shapes

| Tool call name | Result event type | Parser mapping |
|----------------|-------------------|----------------|
| `run_command` | `RUN_COMMAND` | `Bash`, command from `args.CommandLine` |
| `grep_search` | `GREP_SEARCH` | `Grep`, query/path summary |
| `view_file` | `VIEW_FILE` | `Read`, path and line range |
| `list_dir` | `LIST_DIRECTORY` | `List`, directory path |
| `list_permissions` | `GENERIC` | permission listing, not a denied action |
| `manage_task` | `GENERIC` | task metadata |
| `schedule` | `GENERIC` | task scheduling metadata |

## Fixed Parser Bugs

- Previous parser used one FIFO queue for every tool call, so `list_permissions` could consume the
  next `RUN_COMMAND` result. The parser now keeps a pending queue per result event type.
- `grep_search` calls were counted as generic tool calls, so trace reported `grep/find 0` despite many
  agy-native searches. The parser now maps them to `Grep`.
- `view_file` and `list_dir` now carry path summaries, so trace/export can show what was inspected.
- Permission listings contain the word `denied` as data, not as execution failure. `GENERIC` result
  events are not marked denied unless the event status is `ERROR`.

## Fixture

Added `tests/fixtures/agy-mixed.jsonl` and a test covering:

- permission list before `run_command`
- `run_command` result pairing
- `grep_search` result pairing
- `view_file` line range summary
- `list_dir` error handling
- git-deny command result

## Smoke Result

`monobench trace bun-1.3.10-toThreadSafe monogram-agy-gemini-3.5-flash-medium-medium-r2-t1779612535950 10`
now reports `grep/find 23` instead of hiding agy-native grep activity.
