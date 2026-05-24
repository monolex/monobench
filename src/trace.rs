// monobench — ordered tool-call timeline of ONE run. [M]=monogram [g]=grep/find/glob [git]=git(denied).
use crate::grade::is_monogram_cmd;
use crate::telemetry;
use crate::util::*;
use serde_json::Value;
use std::collections::HashMap;

const GREP_WORDS: [&str; 7] = ["grep", "egrep", "rg", "find", "fd", "ag", "ack"];
const TRACE_CMD_WIDTH: usize = 104;
const TRACE_ROOTCAUSE_WIDTH: usize = 180;

fn trace_cmd(name: &str, cmd: &str) -> String {
    let cmd = cmd.trim();
    if name == "Bash" {
        fit_middle(&cmd.replace('\n', " "), TRACE_CMD_WIDTH)
    } else if cmd.is_empty() {
        name.to_string()
    } else {
        fit_middle(
            &format!("{name} {}", cmd.replace('\n', " ")),
            TRACE_CMD_WIDTH,
        )
    }
}

fn trace_rootcause(text: &str) -> String {
    text.lines()
        .find(|l| l.to_lowercase().contains("rootcause:"))
        .map(|l| fit_middle(l, TRACE_ROOTCAUSE_WIDTH))
        .unwrap_or_else(|| "(no ROOTCAUSE line)".into())
}

pub fn trace_with_answer(path: &str, max: usize, answer_override: Option<&str>) {
    if path.ends_with(".err") || path.ends_with(".agy.jsonl") {
        trace_events(path, max, answer_override);
        return;
    }
    let evs = load_jsonl(path);
    if evs.is_empty() {
        println!("(no such run / empty)");
        return;
    }
    let agy = evs
        .iter()
        .any(|e| e.get("type").and_then(Value::as_str) == Some("PLANNER_RESPONSE"));
    if agy {
        trace_events(path, max, answer_override);
        return;
    }
    let mut results: HashMap<String, &Value> = HashMap::new();
    for e in &evs {
        if e.get("type").and_then(Value::as_str) == Some("user") {
            if let Some(ct) = e.pointer("/message/content").and_then(Value::as_array) {
                for b in ct {
                    if b.get("type").and_then(Value::as_str) == Some("tool_result") {
                        if let Some(id) = b.get("tool_use_id").and_then(Value::as_str) {
                            results.insert(id.into(), b);
                        }
                    }
                }
            }
        }
    }
    let (mut i, mut mono, mut mono_grep, mut grep, mut git) = (0i64, 0i64, 0i64, 0i64, 0i64);
    let mut lines: Vec<String> = vec![];
    for e in &evs {
        if e.get("type").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        let Some(ct) = e.pointer("/message/content").and_then(Value::as_array) else {
            continue;
        };
        for b in ct {
            if b.get("type").and_then(Value::as_str) != Some("tool_use") {
                continue;
            }
            i += 1;
            let name = b.get("name").and_then(Value::as_str).unwrap_or("");
            let cmd = b
                .pointer("/input/command")
                .and_then(Value::as_str)
                .unwrap_or("");
            let tid = b.get("id").and_then(Value::as_str).unwrap_or("");
            let is_mono = (name == "Bash" && is_monogram_cmd(cmd))
                || name.to_lowercase().contains("monogram");
            let is_git = name == "Bash" && cmd_has_word(cmd, "git");
            let is_grep = name == "Grep"
                || name == "Glob"
                || (name == "Bash" && GREP_WORDS.iter().any(|w| cmd_has_word(cmd, w)));
            let (tag, col) = if is_mono {
                mono += 1;
                if cmd_has_word(cmd, "grep") {
                    mono_grep += 1;
                }
                ("[M]", COL_MONOGRAM)
            } else if is_git {
                git += 1;
                ("git", COL_OTHER)
            } else if is_grep {
                grep += 1;
                ("[g]", COL_OTHER)
            } else {
                ("   ", "0")
            };
            let d = trace_cmd(name, cmd);
            if i as usize <= max {
                let mut line = c(
                    col,
                    &format!("  {}. {} {}", pad_start(&i.to_string(), 3), tag, d),
                );
                if results.get(tid).map(|r| is_denied(r)).unwrap_or(false) {
                    line.push_str(&c(DIM, " ⟶ denied"));
                }
                lines.push(line);
            }
        }
    }
    let label = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".jsonl");
    let r = evs
        .iter()
        .rev()
        .find(|e| e.get("type").and_then(Value::as_str) == Some("result"));
    let text = r
        .and_then(|r| r.get("result").and_then(Value::as_str))
        .unwrap_or("");
    let rc = trace_rootcause(text);
    let running = if r.is_some() { "" } else { " · [running]" };
    println!(
        "\n{}  {}",
        c("1", label),
        c(
            DIM,
            &format!("{i} calls · monogram {mono} · grep/find {grep} · git {git}{running}")
        )
    );
    if mono_grep > 0 {
        println!(
            "  {}",
            c(DIM, &format!("monogram grep subcommands: {mono_grep}"))
        );
    }
    for l in lines {
        println!("{}", l);
    }
    println!("  {}", c("1", &rc));
}

fn trace_events(path: &str, max: usize, answer_override: Option<&str>) {
    let calls = telemetry::events_from_path(path);
    if calls.is_empty() {
        println!("(no such run / empty)");
        return;
    }
    let (mut mono, mut mono_grep, mut grep, mut git) = (0i64, 0i64, 0i64, 0i64);
    let mut lines: Vec<String> = vec![];
    for (idx, b) in calls.iter().enumerate() {
        let i = idx as i64 + 1;
        let is_mono = (b.name == "Bash" && is_monogram_cmd(&b.cmd))
            || b.name.to_lowercase().contains("monogram");
        let is_git = b.name == "Bash" && cmd_has_word(&b.cmd, "git");
        let is_grep = b.name == "Grep"
            || b.name == "Glob"
            || (b.name == "Bash" && GREP_WORDS.iter().any(|w| cmd_has_word(&b.cmd, w)));
        let (tag, col) = if is_mono {
            mono += 1;
            if cmd_has_word(&b.cmd, "grep") {
                mono_grep += 1;
            }
            ("[M]", COL_MONOGRAM)
        } else if is_git {
            git += 1;
            ("git", COL_OTHER)
        } else if is_grep {
            grep += 1;
            ("[g]", COL_OTHER)
        } else {
            ("   ", "0")
        };
        let d = trace_cmd(&b.name, &b.cmd);
        if idx < max {
            let mut line = c(
                col,
                &format!("  {}. {} {}", pad_start(&i.to_string(), 3), tag, d),
            );
            if b.denied {
                line.push_str(&c(DIM, " ⟶ denied"));
            }
            lines.push(line);
        }
    }
    let label = telemetry::label_from_path(path);
    let ans = if let Some(answer) = answer_override {
        answer.to_string()
    } else if path.ends_with(".err") {
        std::fs::read_to_string(format!("{}.answer.txt", path.trim_end_matches(".err")))
            .unwrap_or_default()
    } else {
        std::fs::read_to_string(format!(
            "{}.answer.txt",
            path.trim_end_matches(".agy.jsonl")
        ))
        .unwrap_or_default()
    };
    let rc = trace_rootcause(&ans);
    println!(
        "\n{}  {}",
        c("1", &label),
        c(
            DIM,
            &format!(
                "{} calls · monogram {mono} · grep/find {grep} · git {git}",
                calls.len()
            )
        )
    );
    if mono_grep > 0 {
        println!(
            "  {}",
            c(DIM, &format!("monogram grep subcommands: {mono_grep}"))
        );
    }
    for l in lines {
        println!("{}", l);
    }
    println!("  {}", c("1", &rc));
}
