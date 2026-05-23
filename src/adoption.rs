// monobench — per-run tool-call + monogram-subcommand breakdown (+ git-integrity), grouped by model.
// "Did the agent actually USE monogram — how much, how early, how well; and did it try to cheat via git?"
use crate::grade::is_monogram_cmd;
use crate::util::*;
use serde_json::Value;
use std::collections::HashMap;

const W: usize = 80;

struct Row { label: String, tool: String, model: String, total: i64, mono: i64, first: i64, failed: i64, git: i64, git_denied: i64, share: i64, subs: String }

fn meaningless(t: &str) -> bool {
    let body: String = t.lines()
        .filter(|l| { let s = l.trim_start(); !l.trim().is_empty() && !s.starts_with("[INFO]") && !s.starts_with("[NEXT]") })
        .collect::<Vec<_>>().join(" ");
    let b = body.trim();
    let bl = b.to_lowercase();
    bl.contains("no match") || bl.contains("not found") || bl.starts_with("0 results") || bl.contains("no results") || bl.contains("no symbol") || b.chars().count() < 3
}

/// Extract the monogram subcommand verb from a Bash `monogram <sub>` or an MCP `monogram_<sub>` tool.
fn monogram_sub(name: &str, cmd: &str) -> Option<String> {
    if name == "Bash" && is_monogram_cmd(cmd) {
        let idx = cmd.find("monogram").unwrap_or(0);
        let tok = cmd[idx + 8..].split_whitespace().next().unwrap_or("");
        return Some(if tok.is_empty() { "?".into() } else { tok.into() });
    }
    let nlc = name.to_lowercase();
    if let Some(i) = nlc.rfind("monogram") {
        let s = name[i + 8..].trim_start_matches(|c| c == '_' || c == '-');
        return Some(if s.is_empty() { "?".into() } else { s.into() });
    }
    None
}

pub fn adoption(id: &str, files: &[String]) {
    let mut rows: Vec<Row> = vec![];
    for f in files {
        let evs = load_jsonl(f);
        let mut calls: Vec<&Value> = vec![];
        let mut results: HashMap<String, &Value> = HashMap::new();
        for e in &evs {
            match e.get("type").and_then(Value::as_str) {
                Some("assistant") => if let Some(ct) = e.pointer("/message/content").and_then(Value::as_array) {
                    for b in ct { if b.get("type").and_then(Value::as_str) == Some("tool_use") { calls.push(b); } }
                },
                Some("user") => if let Some(ct) = e.pointer("/message/content").and_then(Value::as_array) {
                    for b in ct { if b.get("type").and_then(Value::as_str) == Some("tool_result") {
                        if let Some(tid) = b.get("tool_use_id").and_then(Value::as_str) { results.insert(tid.into(), b); }
                    } }
                },
                _ => {}
            }
        }
        let label = f.rsplit('/').next().unwrap_or(f).trim_end_matches(".jsonl").to_string();
        let (mut mono, mut first, mut failed, mut git, mut git_denied) = (0i64, -1i64, 0i64, 0i64, 0i64);
        let mut subs_ord: Vec<(String, i64)> = vec![]; // insertion order (matches the JS object), tie-stable
        for (i, b) in calls.iter().enumerate() {
            let name = b.get("name").and_then(Value::as_str).unwrap_or("");
            let cmd = b.pointer("/input/command").and_then(Value::as_str).unwrap_or("");
            let tid = b.get("id").and_then(Value::as_str).unwrap_or("");
            if name == "Bash" && cmd_has_word(cmd, "git") {
                git += 1;
                if results.get(tid).map(|r| is_denied(r)).unwrap_or(false) { git_denied += 1; }
            }
            if let Some(sub) = monogram_sub(name, cmd) {
                mono += 1;
                if first < 0 { first = i as i64 + 1; }
                match subs_ord.iter_mut().find(|(k, _)| *k == sub) { Some(e) => e.1 += 1, None => subs_ord.push((sub, 1)) }
                let bad = results.get(tid).map(|r| is_denied(r) || meaningless(&result_text(r))).unwrap_or(true);
                if bad { failed += 1; }
            }
        }
        let mut sarr = subs_ord;
        sarr.sort_by(|a, b| b.1.cmp(&a.1)); // stable ⇒ equal counts keep insertion order
        let mut subs = sarr.iter().take(6).map(|(k, v)| format!("{k}×{v}")).collect::<Vec<_>>().join("  ");
        if sarr.len() > 6 { subs.push_str("  …"); }
        let total = calls.len() as i64;
        let share = if total > 0 { (100.0 * mono as f64 / total as f64).round() as i64 } else { 0 };
        let a = parse_arm(&label);
        rows.push(Row { label, tool: a.tool, model: a.model, total, mono, first, failed, git, git_denied, share, subs });
    }
    if rows.is_empty() { println!("(no claude-p runs to analyze — adoption needs tool-call logs)"); return; }

    println!("{}{}{}", c(DIM, "[INFO] adoption · "), c("1", id), c(DIM, &format!(" · {} runs", rows.len())));
    println!("\n{}", c(HEAD, &"═".repeat(W)));
    println!("{}", c(HEAD, "TOOL ADOPTION  (did the agent USE monogram — how much, how early, how well)"));
    println!("{}", c(HEAD, &"═".repeat(W)));

    let mut models: Vec<String> = vec![];
    for r in &rows { if !models.contains(&r.model) { models.push(r.model.clone()); } }
    models.sort_by_key(|m| model_ord(m));
    let prow = |l: &str, t: &str, m: &str, sh: &str, fi: &str, fa: &str|
        format!("  {} {} {} {} {} {}", pad_end(l, 24), pad_start(t, 5), pad_start(m, 5), pad_start(sh, 6), pad_start(fi, 6), pad_start(fa, 6));
    for model in &models {
        println!("\n  {}\n  {}", c("1", &model.to_uppercase()), c(DIM, &"─".repeat(W - 2)));
        println!("{}", c(DIM, &prow("run", "calls", "mono", "share", "first", "fails")));
        let mut mrows: Vec<&Row> = rows.iter().filter(|r| &r.model == model).collect();
        mrows.sort_by(|a, b| a.label.cmp(&b.label));
        for x in mrows {
            let first = if x.mono > 0 { format!("#{}", x.first) } else { "—".into() };
            let fail = if x.mono > 0 { if x.failed > 0 { format!("⚠{}", x.failed) } else { "0".into() } } else { "—".into() };
            println!("{}", c(arm_code(&x.tool), &prow(&x.label, &x.total.to_string(), &x.mono.to_string(), &format!("{}%", x.share), &first, &fail)));
            let gitnote = if x.git > 0 {
                let st = if x.git_denied == x.git { " (all denied ✓)".to_string() }
                    else if x.git_denied > 0 { format!(" ({} denied)", x.git_denied) } else { " ⚠ NOT denied".into() };
                format!("   · git {} attempt{}{}", x.git, if x.git > 1 { "s" } else { "" }, st)
            } else { String::new() };
            let body = if x.mono > 0 { format!("        ↳ {}{}", x.subs, gitnote) }
                else if x.tool == "baseline" { format!("        ↳ (control — no tool){}", gitnote) }
                else { format!("        ↳ (tool never called){}", gitnote) };
            println!("{}", c(DIM, &body));
        }
    }
    println!("{}", c(DIM, "\nfirst = call # of first monogram use (late ⇒ grepped first)"));
    println!("{}", c(DIM, "fails = monogram calls that returned nothing · git = history-access attempts (must be denied)"));
    println!("{}", c(DIM, "low share or late first-use ⇒ the tool was not really tested (SPEC §7)"));
}
