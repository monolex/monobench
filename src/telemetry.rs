// monobench — normalize tool-call telemetry across runner log formats.
// Claude emits stream-json. Codex logs exec blocks to stderr. Agy writes transcript JSONL.
use crate::util::{cmd_has_word, load_jsonl, result_text};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug, Default)]
pub struct ToolEvent {
    pub name: String,
    pub cmd: String,
    pub result: String,
    pub denied: bool,
}

fn denied_text(t: &str) -> bool {
    let lc = t.to_lowercase();
    lc.contains("permission")
        || lc.contains("not allowed")
        || lc.contains("disallow")
        || lc.contains("denied")
        || lc.contains("git is disabled during solver runs")
        || lc.contains("operation not permitted") // Seatbelt sandbox-exec deny (git/.git/answer-key)
}

/// A git invocation in `cmd` — bare `git`, absolute `/usr/bin/git`, or after `cd x &&`.
/// Used to apply the run-level sandbox git-deny override.
fn is_git_command(cmd: &str) -> bool {
    cmd.split(|c: char| c.is_whitespace() || matches!(c, ';' | '&' | '|' | '('))
        .any(|w| w == "git" || w.ends_with("/git"))
}

pub fn label_from_path(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    for suffix in [".agy.jsonl", ".jsonl", ".err"] {
        if let Some(s) = name.strip_suffix(suffix) {
            return s.to_string();
        }
    }
    name.to_string()
}

pub fn events_from_path(path: &str) -> Vec<ToolEvent> {
    if path.ends_with(".err") {
        codex_err_events(path)
    } else {
        let evs = load_jsonl(path);
        let agy = evs
            .iter()
            .any(|e| e.get("type").and_then(Value::as_str) == Some("PLANNER_RESPONSE"));
        if agy {
            agy_jsonl_events(&evs)
        } else {
            claude_jsonl_events(&evs)
        }
    }
}

pub fn claude_jsonl_events(evs: &[Value]) -> Vec<ToolEvent> {
    let mut calls: Vec<ToolEvent> = vec![];
    let mut ids: Vec<String> = vec![];
    for e in evs {
        match e.get("type").and_then(Value::as_str) {
            Some("assistant") => {
                if let Some(ct) = e.pointer("/message/content").and_then(Value::as_array) {
                    for b in ct {
                        if b.get("type").and_then(Value::as_str) == Some("tool_use") {
                            ids.push(
                                b.get("id")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string(),
                            );
                            calls.push(ToolEvent {
                                name: b
                                    .get("name")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string(),
                                cmd: b
                                    .pointer("/input/command")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string(),
                                result: String::new(),
                                denied: false,
                            });
                        }
                    }
                }
            }
            Some("user") => {
                if let Some(ct) = e.pointer("/message/content").and_then(Value::as_array) {
                    for b in ct {
                        if b.get("type").and_then(Value::as_str) == Some("tool_result") {
                            let tid = b.get("tool_use_id").and_then(Value::as_str).unwrap_or("");
                            if let Some(pos) = ids.iter().position(|x| x == tid) {
                                let text = result_text(b);
                                calls[pos].denied = b.get("is_error").and_then(Value::as_bool)
                                    == Some(true)
                                    || denied_text(&text);
                                calls[pos].result = text;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    calls
}

pub fn agy_jsonl_events(evs: &[Value]) -> Vec<ToolEvent> {
    let mut calls: Vec<ToolEvent> = vec![];
    let mut pending: HashMap<&'static str, VecDeque<usize>> = HashMap::new();
    for e in evs {
        match e.get("type").and_then(Value::as_str) {
            Some("PLANNER_RESPONSE") => {
                if let Some(ts) = e.get("tool_calls").and_then(Value::as_array) {
                    for t in ts {
                        let name = t
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let args = t.get("args").unwrap_or(&Value::Null);
                        let cmd = agy_tool_cmd(&name, args);
                        let event_name = agy_event_name(&name);
                        if let Some(kind) = agy_result_type(&name) {
                            pending.entry(kind).or_default().push_back(calls.len());
                        }
                        calls.push(ToolEvent {
                            name: event_name,
                            cmd,
                            result: String::new(),
                            denied: false,
                        });
                    }
                }
            }
            Some(
                kind @ ("RUN_COMMAND" | "VIEW_FILE" | "GREP_SEARCH" | "LIST_DIRECTORY" | "GENERIC"),
            ) => {
                if let Some(pos) = pending.get_mut(kind).and_then(VecDeque::pop_front) {
                    let text = agy_result_text(e);
                    calls[pos].denied = agy_denied(kind, e, &text);
                    calls[pos].result = text;
                }
            }
            _ => {}
        }
    }
    calls
}

fn agy_result_type(name: &str) -> Option<&'static str> {
    match name {
        "run_command" => Some("RUN_COMMAND"),
        "view_file" => Some("VIEW_FILE"),
        "grep_search" => Some("GREP_SEARCH"),
        "list_dir" => Some("LIST_DIRECTORY"),
        "list_permissions" | "manage_task" | "schedule" => Some("GENERIC"),
        _ => None,
    }
}

fn agy_event_name(name: &str) -> String {
    match name {
        "run_command" => "Bash".into(),
        "grep_search" => "Grep".into(),
        "view_file" => "Read".into(),
        "list_dir" => "List".into(),
        _ => name.to_string(),
    }
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

fn agy_tool_cmd(name: &str, args: &Value) -> String {
    let direct = str_arg(args, "CommandLine")
        .or_else(|| str_arg(args, "command"))
        .or_else(|| str_arg(args, "cmd"));
    if let Some(cmd) = direct {
        return cmd.to_string();
    }
    match name {
        "grep_search" => {
            let query = str_arg(args, "Query").unwrap_or("");
            let path = str_arg(args, "SearchPath").unwrap_or(".");
            if query.is_empty() {
                format!("grep_search {path}")
            } else {
                format!("grep_search {query:?} {path}")
            }
        }
        "view_file" => {
            let path = str_arg(args, "AbsolutePath").unwrap_or("");
            let start = args.get("StartLine").and_then(Value::as_i64);
            let end = args.get("EndLine").and_then(Value::as_i64);
            match (path.is_empty(), start, end) {
                (false, Some(s), Some(e)) => format!("{path}:{s}-{e}"),
                (false, Some(s), None) => format!("{path}:{s}"),
                (false, _, _) => path.to_string(),
                _ => str_arg(args, "toolSummary")
                    .unwrap_or("view_file")
                    .to_string(),
            }
        }
        "list_dir" => str_arg(args, "DirectoryPath")
            .or_else(|| str_arg(args, "Path"))
            .unwrap_or("list_dir")
            .to_string(),
        "manage_task" | "schedule" | "list_permissions" => str_arg(args, "toolAction")
            .or_else(|| str_arg(args, "toolSummary"))
            .unwrap_or(name)
            .to_string(),
        _ => str_arg(args, "toolAction")
            .or_else(|| str_arg(args, "toolSummary"))
            .unwrap_or("")
            .to_string(),
    }
}

fn agy_result_text(e: &Value) -> String {
    e.get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn agy_denied(kind: &str, e: &Value, text: &str) -> bool {
    if e.get("status").and_then(Value::as_str) == Some("ERROR") {
        return true;
    }
    if kind == "GENERIC" {
        return false;
    }
    denied_text(text)
}

fn extract_codex_cmd(line: &str) -> String {
    let Some(start) = line.find(" -lc ") else {
        return line.to_string();
    };
    let rest = &line[start + 5..];
    if let Some(q) = rest.strip_prefix('\'') {
        if let Some(end) = q.rfind("' in ") {
            return q[..end].to_string();
        }
    }
    if let Some(q) = rest.strip_prefix('"') {
        if let Some(end) = q.rfind("\" in ") {
            return q[..end].to_string();
        }
    }
    rest.split(" in ").next().unwrap_or(rest).to_string()
}

fn codex_boundary(line: &str) -> bool {
    line == "codex"
        || line == "exec"
        || line.starts_with("hook: ")
        || line.starts_with("tokens used")
}

pub fn codex_err_events(path: &str) -> Vec<ToolEvent> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return vec![];
    };
    let lines: Vec<&str> = text.lines().collect();
    let mut out = vec![];
    let mut i = 0usize;
    while i < lines.len() {
        if lines[i].trim() == "exec" && i + 1 < lines.len() {
            let cmd_line = lines[i + 1].trim();
            let cmd = extract_codex_cmd(cmd_line);
            let mut j = i + 2;
            let mut status = "";
            if j < lines.len()
                && (lines[j].contains(" succeeded ")
                    || lines[j].contains(" exited ")
                    || lines[j].contains(" failed "))
            {
                status = lines[j];
                j += 1;
            }
            let start = j;
            while j < lines.len() && !codex_boundary(lines[j].trim()) {
                j += 1;
            }
            let body = lines[start..j].join("\n");
            let result = if status.is_empty() {
                body
            } else if body.is_empty() {
                status.to_string()
            } else {
                format!("{status}\n{body}")
            };
            out.push(ToolEvent {
                name: "Bash".into(),
                cmd,
                denied: denied_text(&result),
                result,
            });
            i = j;
        } else {
            i += 1;
        }
    }
    // sandbox-exec (Seatbelt) denies git AT EXEC; "operation not permitted: git" lands in stderr
    // but not always inside the matching exec-block body, so per-call denial under-counts. The
    // sandbox is categorical: if it denied git anywhere in this run, EVERY git command was denied.
    let lc_all = text.to_lowercase();
    if lc_all.contains("operation not permitted: git")
        || lc_all.contains("operation not permitted: /usr/bin/git")
    {
        for ev in out.iter_mut() {
            if is_git_command(&ev.cmd) {
                ev.denied = true;
            }
        }
    }
    out
}

pub fn count_monogram(calls: &[ToolEvent]) -> i64 {
    calls
        .iter()
        .filter(|e| {
            let nlc = e.name.to_lowercase();
            nlc.contains("codegraph")
                || nlc.starts_with("mcp__monogram")
                || nlc.contains("monogram")
                || (e.name == "Bash" && cmd_has_word(&e.cmd, "monogram"))
        })
        .count() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_codex_exec_blocks() {
        let p =
            std::env::temp_dir().join(format!("monobench-codex-telemetry-{}", std::process::id()));
        std::fs::write(&p, "codex\nhook: PreToolUse\nexec\n/bin/zsh -lc 'monogram search x' in /tmp/repo\n succeeded in 1ms:\nfound\ncodex\nexec\n/bin/zsh -lc 'git log -1' in /tmp/repo\n succeeded in 1ms:\nok\n").unwrap();
        let evs = codex_err_events(&p.to_string_lossy());
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].cmd, "monogram search x");
        assert_eq!(count_monogram(&evs), 1);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn marks_codex_git_wrapper_as_denied() {
        let p =
            std::env::temp_dir().join(format!("monobench-codex-git-denied-{}", std::process::id()));
        std::fs::write(&p, "codex\nexec\n/bin/zsh -lc 'git diff -- src/main.rs' in /tmp/repo\n exited 126 in 1ms:\nmonobench: git is disabled during solver runs (anti-contamination)\n").unwrap();
        let evs = codex_err_events(&p.to_string_lossy());
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].cmd, "git diff -- src/main.rs");
        assert!(evs[0].denied);
        assert!(evs[0].result.contains("exited 126"));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn parses_agy_tool_calls() {
        let evs = vec![
            serde_json::json!({"type":"PLANNER_RESPONSE","tool_calls":[{"name":"run_command","args":{"CommandLine":"monogram symbols Foo"}}]}),
            serde_json::json!({"type":"RUN_COMMAND","content":"Output:\nFoo"}),
        ];
        let calls = agy_jsonl_events(&evs);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "Bash");
        assert_eq!(calls[0].cmd, "monogram symbols Foo");
        assert_eq!(count_monogram(&calls), 1);
    }

    #[test]
    fn parses_agy_mixed_transcript_by_result_type() {
        let evs: Vec<Value> = include_str!("../tests/fixtures/agy-mixed.jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let calls = agy_jsonl_events(&evs);
        assert_eq!(calls.len(), 6);

        assert_eq!(calls[0].name, "list_permissions");
        assert!(calls[0].result.contains("command(*)"));
        assert!(!calls[0].denied);

        assert_eq!(calls[1].name, "Bash");
        assert_eq!(calls[1].cmd, "monogram stats");
        assert!(calls[1].result.contains("Files: 10"));

        assert_eq!(calls[2].name, "Grep");
        assert!(calls[2].cmd.contains("needle"));
        assert!(calls[2].result.contains("src/lib.rs"));

        assert_eq!(calls[3].name, "Read");
        assert!(calls[3].cmd.contains("/repo/src/lib.rs:10-20"));
        assert!(calls[3].result.contains("Showing lines 10 to 20"));

        assert_eq!(calls[4].name, "List");
        assert!(calls[4].denied);

        assert_eq!(calls[5].name, "Bash");
        assert_eq!(calls[5].cmd, "git grep needle");
        assert!(calls[5].denied);
        assert_eq!(count_monogram(&calls), 1);
    }
}
