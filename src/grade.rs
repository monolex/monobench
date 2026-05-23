// monobench — grading: score a run's ROOTCAUSE answer vs the instance's gold rules.
// Patterns are literal, case-insensitive substrings (the original JS escaped all regex metachars).
use crate::util::load_jsonl;
use serde_json::Value;

pub struct Inst { pub full: Vec<String>, pub mech: Vec<String>, pub decoy: Vec<String> }

pub fn load_inst(path: &str) -> Inst {
    let v: Value = std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    let g = v.get("grading").cloned().unwrap_or(Value::Null);
    let arr = |k: &str| g.get(k).and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    Inst { full: arr("full_must_name"), mech: arr("mechanism_keywords"), decoy: arr("decoy_markers") }
}

fn has(pats: &[String], text_lc: &str) -> bool {
    pats.iter().any(|p| text_lc.contains(&p.to_lowercase()))
}

pub fn grade_text_str(inst: &Inst, text: &str) -> &'static str {
    if text.trim().is_empty() { return "NO_RESULT"; }
    let t = text.to_lowercase();
    let named = has(&inst.full, &t);
    if named && has(&inst.mech, &t) { "FULL" }
    else if named { "NAME_ONLY" }
    else if has(&inst.decoy, &t) { "DECOY" }
    else { "MISS" }
}

/// Does a Bash command invoke `monogram` as a word? (^|[\s;|&(])monogram\b
pub fn is_monogram_cmd(cmd: &str) -> bool {
    for (i, _) in cmd.match_indices("monogram") {
        let before_ok = i == 0 || matches!(cmd.as_bytes()[i - 1], b' ' | b'\t' | b';' | b'|' | b'&' | b'(');
        let after_ok = match cmd.as_bytes().get(i + 8) { None => true, Some(&b) => !(b.is_ascii_alphanumeric() || b == b'_') };
        if before_ok && after_ok { return true; }
    }
    false
}

#[derive(Clone)]
pub struct RunStats {
    pub label: String,
    pub grade: String,
    pub cost: f64,
    pub tok: i64,
    pub calls: Option<i64>, // None for niia/codex (no tool-call telemetry)
    pub adopt: i64,
    pub time: i64,
    pub rootcause: String,
}

fn rootcause_line(t: &str) -> String {
    t.lines().find(|l| l.to_lowercase().contains("rootcause:"))
        .map(|l| l.chars().take(92).collect())
        .unwrap_or_else(|| "(no ROOTCAUSE line)".into())
}

pub fn grade_jsonl(inst: &Inst, path: &str) -> RunStats {
    let evs = load_jsonl(path);
    let label = path.rsplit('/').next().unwrap_or(path).trim_end_matches(".jsonl").to_string();
    let (mut calls, mut adopt) = (0i64, 0i64);
    for e in &evs {
        if e.get("type").and_then(Value::as_str) != Some("assistant") { continue; }
        let Some(content) = e.pointer("/message/content").and_then(Value::as_array) else { continue };
        for b in content {
            if b.get("type").and_then(Value::as_str) != Some("tool_use") { continue; }
            calls += 1;
            let name = b.get("name").and_then(Value::as_str).unwrap_or("");
            let cmd = b.pointer("/input/command").and_then(Value::as_str).unwrap_or("");
            let nlc = name.to_lowercase();
            if nlc.contains("codegraph") || nlc.starts_with("mcp__monogram") || (name == "Bash" && is_monogram_cmd(cmd)) {
                adopt += 1;
            }
        }
    }
    match evs.iter().rev().find(|e| e.get("type").and_then(Value::as_str) == Some("result")) {
        None => RunStats { label, grade: "NO_RESULT".into(), cost: 0.0, tok: 0, calls: Some(calls), adopt, time: 0, rootcause: "(no ROOTCAUSE line)".into() },
        Some(r) => {
            let gi = |k: &str| r.pointer(&format!("/usage/{k}")).and_then(Value::as_i64).unwrap_or(0);
            let tok = gi("input_tokens") + gi("cache_read_input_tokens") + gi("cache_creation_input_tokens") + gi("output_tokens");
            let text = r.get("result").and_then(Value::as_str).unwrap_or("");
            let cost = r.get("total_cost_usd").and_then(Value::as_f64).unwrap_or(0.0);
            let time = (r.get("duration_ms").and_then(Value::as_f64).unwrap_or(0.0) / 1000.0).round() as i64;
            RunStats { label, grade: grade_text_str(inst, text).into(), cost, tok, calls: Some(calls), adopt, time, rootcause: rootcause_line(text) }
        }
    }
}

/// niia/codex runner: grade the answer.txt + read tokens/cost/duration from meter.json.
pub fn grade_text_file(inst: &Inst, answer: &str, meter: &str) -> RunStats {
    let text = std::fs::read_to_string(answer).unwrap_or_default();
    let label = answer.rsplit('/').next().unwrap_or(answer).trim_end_matches(".answer.txt").to_string();
    let m: Value = std::fs::read_to_string(meter).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    RunStats {
        label,
        grade: grade_text_str(inst, &text).into(),
        cost: m.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0),
        tok: m.get("tokens").and_then(Value::as_i64).unwrap_or(0),
        calls: None,
        adopt: 0,
        time: m.get("duration_s").and_then(Value::as_i64).unwrap_or(0),
        rootcause: rootcause_line(&text),
    }
}

pub fn print_grade(s: &RunStats) {
    let calls = s.calls.map(|c| c.to_string()).unwrap_or_else(|| "—".into());
    println!("\n── {} ──", s.label);
    println!("grade={}  cost=${:.2}  tokens={}  time={}s  toolcalls={}  tool-adoption={}", s.grade, s.cost, s.tok, s.time, calls, s.adopt);
    println!("  {}", s.rootcause);
}
