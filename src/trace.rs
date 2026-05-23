// monobench — ordered tool-call timeline of ONE run. [M]=monogram [g]=grep/find/glob [git]=git(denied).
use crate::grade::is_monogram_cmd;
use crate::util::*;
use serde_json::Value;
use std::collections::HashMap;

const GREP_WORDS: [&str; 7] = ["grep", "egrep", "rg", "find", "fd", "ag", "ack"];

pub fn trace(path: &str, max: usize) {
    let evs = load_jsonl(path);
    if evs.is_empty() { println!("(no such run / empty)"); return; }
    let mut results: HashMap<String, &Value> = HashMap::new();
    for e in &evs {
        if e.get("type").and_then(Value::as_str) == Some("user") {
            if let Some(ct) = e.pointer("/message/content").and_then(Value::as_array) {
                for b in ct { if b.get("type").and_then(Value::as_str) == Some("tool_result") {
                    if let Some(id) = b.get("tool_use_id").and_then(Value::as_str) { results.insert(id.into(), b); }
                } }
            }
        }
    }
    let (mut i, mut mono, mut grep, mut git) = (0i64, 0i64, 0i64, 0i64);
    let mut lines: Vec<String> = vec![];
    for e in &evs {
        if e.get("type").and_then(Value::as_str) != Some("assistant") { continue; }
        let Some(ct) = e.pointer("/message/content").and_then(Value::as_array) else { continue };
        for b in ct {
            if b.get("type").and_then(Value::as_str) != Some("tool_use") { continue; }
            i += 1;
            let name = b.get("name").and_then(Value::as_str).unwrap_or("");
            let cmd = b.pointer("/input/command").and_then(Value::as_str).unwrap_or("");
            let tid = b.get("id").and_then(Value::as_str).unwrap_or("");
            let is_mono = (name == "Bash" && is_monogram_cmd(cmd)) || name.to_lowercase().contains("monogram");
            let is_git = name == "Bash" && cmd_has_word(cmd, "git");
            let is_grep = name == "Grep" || name == "Glob" || (name == "Bash" && GREP_WORDS.iter().any(|w| cmd_has_word(cmd, w)));
            let (tag, col) = if is_mono { mono += 1; ("[M]", COL_MONOGRAM) }
                else if is_git { git += 1; ("git", COL_OTHER) }
                else if is_grep { grep += 1; ("[g]", COL_OTHER) }
                else { ("   ", "0") };
            let d = if name == "Bash" { cmd.replace('\n', " ").chars().take(58).collect::<String>() } else { name.to_string() };
            if i as usize <= max {
                let mut line = c(col, &format!("  {}. {} {}", pad_start(&i.to_string(), 3), tag, d));
                if results.get(tid).map(|r| is_denied(r)).unwrap_or(false) { line.push_str(&c(DIM, " ⟶ denied")); }
                lines.push(line);
            }
        }
    }
    let label = path.rsplit('/').next().unwrap_or(path).trim_end_matches(".jsonl");
    let r = evs.iter().rev().find(|e| e.get("type").and_then(Value::as_str) == Some("result"));
    let text = r.and_then(|r| r.get("result").and_then(Value::as_str)).unwrap_or("");
    let rc = text.lines().find(|l| l.to_lowercase().contains("rootcause:"))
        .map(|l| l.chars().take(92).collect::<String>()).unwrap_or_else(|| "(no ROOTCAUSE line)".into());
    let running = if r.is_some() { "" } else { " · [running]" };
    println!("\n{}  {}", c("1", label), c(DIM, &format!("{i} calls · monogram {mono} · grep/find {grep} · git {git}{running}")));
    for l in lines { println!("{}", l); }
    println!("  {}", c("1", &rc));
}
