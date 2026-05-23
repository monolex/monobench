use crate::meter;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime};

fn niia() -> String {
    std::env::var("MONOBENCH_NIIA_BIN").unwrap_or_else(|_| "niia".into())
}

fn run_text(args: &[&str]) -> Result<String, String> {
    let out = Command::new(niia()).args(args).output()
        .map_err(|e| format!("niia {}: {e}", args.join(" ")))?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn run_status(args: &[&str]) -> Result<(), String> {
    let status = Command::new(niia()).args(args).status()
        .map_err(|e| format!("niia {}: {e}", args.join(" ")))?;
    if status.success() { Ok(()) } else { Err(format!("niia {} exited with {status}", args.join(" "))) }
}

fn live_session() -> Result<String, String> {
    if let Ok(s) = std::env::var("MONOBENCH_NIIA_SESSION") {
        if !s.trim().is_empty() { return Ok(s); }
    }
    let list = run_text(&["serve", "--list"])?;
    parse_live_session(&list).ok_or_else(|| "no live niia [ws] session - start one with: niia serve".into())
}

fn parse_live_session(list: &str) -> Option<String> {
    for line in list.lines().filter(|l| l.contains("[ws]")) {
        for word in line.split_whitespace() {
            let w = word.trim_matches(|c: char| c == ',' || c == ':' || c == ';');
            if w.starts_with("niia-") { return Some(w.to_string()); }
        }
    }
    None
}

fn write(session: &str, text: &str) -> Result<(), String> {
    run_status(&["write", "--session", session, text])
}

fn wait_idle(session: &str) -> Result<(), String> {
    run_status(&["wait-idle", "--session", session])
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn shell_join(words: &[String]) -> String {
    words.iter().map(|w| {
        if w.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/' | '.' | '=' | ':')) {
            w.clone()
        } else {
            shell_quote(w)
        }
    }).collect::<Vec<_>>().join(" ")
}

fn split_words(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}

fn spawn_command(effort: &str) -> String {
    let cli = std::env::var("MONOBENCH_CLI").unwrap_or_else(|_| "claude".into());
    let mut words = split_words(&cli);
    if words.is_empty() { words.push("claude".into()); }
    match words.first().map(String::as_str) {
        Some("claude") => {
            if !effort.is_empty() {
                words.push("--effort".into());
                words.push(effort.into());
            }
        }
        Some("codex") => {
            if !effort.is_empty() {
                words.push("-c".into());
                words.push(format!("model_reasoning_effort={effort}"));
            }
        }
        _ => {}
    }
    shell_join(&words)
}

fn flatten_prompt(prompt: &str) -> String {
    prompt.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn filtered_answer(raw: &str) -> String {
    raw.lines()
        .filter(|line| {
            let lc = line.to_lowercase();
            !line.trim_start().starts_with("[INFO]") && !lc.contains("next")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).ok().map(PathBuf::from)
}

fn collect_jsonl(dir: &Path, out: &mut Vec<(PathBuf, SystemTime)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl(&path, out);
        } else if path.extension().and_then(|x| x.to_str()) == Some("jsonl") {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    out.push((path, modified));
                }
            }
        }
    }
}

fn newest_claude_jsonl(since: SystemTime) -> Option<PathBuf> {
    let root = home_dir()?.join(".claude/projects");
    let mut files = Vec::new();
    collect_jsonl(&root, &mut files);
    files.sort_by_key(|(_, modified)| *modified);
    files.iter().rev().find(|(_, modified)| *modified >= since)
        .or_else(|| files.last())
        .map(|(path, _)| path.clone())
}

fn enrich_cost(meter_json: &mut Value) {
    let sid = meter_json.get("session_id").and_then(Value::as_str).unwrap_or("").to_string();
    if sid.is_empty() { return; }
    Command::new("monometer").args(["daemon", "recompute"]).output().ok();
    thread::sleep(Duration::from_secs(2));
    let out = Command::new("monometer").args(["sessions", "--recent", "40", "--json"]).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let Ok(v) = serde_json::from_str::<Value>(&out) else { return };
    let Some(arr) = v.as_array() else { return };
    let Some(cost) = arr.iter()
        .find(|x| x.get("session_id").and_then(Value::as_str) == Some(&sid))
        .and_then(|x| x.get("cost_usd").and_then(Value::as_f64)) else { return };
    meter_json["cost_usd"] = serde_json::json!(cost);
}

pub fn run(repo: &Path, prompt: &str, marker: &str, out_prefix: &Path, effort: &str) -> Result<(), String> {
    let session = live_session()?;
    let spawn = spawn_command(effort);
    println!("▶ niia runner · session={session} · spawn='{spawn}' · repo={}", repo.display());

    write(&session, &format!("cd {}\r", shell_quote(&repo.to_string_lossy())))?;
    wait_idle(&session)?;
    let since = SystemTime::now();

    write(&session, &format!("{spawn}\r"))?;
    thread::sleep(Duration::from_secs(3));
    wait_idle(&session)?;
    thread::sleep(Duration::from_secs(2));
    write(&session, "\r")?;
    wait_idle(&session)?;
    thread::sleep(Duration::from_secs(1));

    write(&session, &format!("{}\r", flatten_prompt(prompt)))?;
    thread::sleep(Duration::from_secs(2));
    write(&session, "\r")?;
    wait_idle(&session)?;
    thread::sleep(Duration::from_secs(3));

    let raw = run_text(&["get-answer", "--session", &session, marker])?;
    std::fs::write(out_prefix.with_extension("answer.txt"), filtered_answer(&raw))
        .map_err(|e| format!("write answer: {e}"))?;

    if let Some(jsonl) = newest_claude_jsonl(since) {
        let mut m = meter::meter_json(&jsonl.to_string_lossy());
        enrich_cost(&mut m);
        std::fs::write(out_prefix.with_extension("meter.json"), m.to_string())
            .map_err(|e| format!("write meter: {e}"))?;
    }

    write(&session, "\x03").ok();
    thread::sleep(Duration::from_secs(1));
    write(&session, "\x03").ok();
    println!("  answer -> {}.answer.txt", out_prefix.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ws_session() {
        let s = "abc [ws] niia-123, other";
        assert_eq!(parse_live_session(s).as_deref(), Some("niia-123"));
    }

    #[test]
    fn filters_answer_noise() {
        let raw = "[INFO] start\nROOTCAUSE: x\nNEXT do y\nFIX: z";
        assert_eq!(filtered_answer(raw), "ROOTCAUSE: x\nFIX: z");
    }

    #[test]
    fn builds_spawn_command() {
        std::env::set_var("MONOBENCH_CLI", "codex");
        assert_eq!(spawn_command("low"), "codex -c model_reasoning_effort=low");
        std::env::remove_var("MONOBENCH_CLI");
    }
}
