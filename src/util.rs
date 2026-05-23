// monobench — shared helpers (ANSI palette, jsonl loading, arm parsing, formatting).
use serde_json::Value;
use std::io::IsTerminal;

pub const MODELS: [&str; 5] = ["opus", "sonnet", "haiku", "codex", "gemini"];
const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

// palette: yellow → orange → red ONLY (baseline=yellow stands apart from tool arms). Chrome stays in-palette.
pub const COL_BASELINE: &str = "38;5;226";
pub const COL_MONOGRAM: &str = "38;5;214";
pub const COL_CODEGRAPH: &str = "38;5;202";
pub const COL_OTHER: &str = "38;5;196";
pub const HEAD: &str = "1;38;5;208";
pub const DIM: &str = "2";

pub fn is_tty() -> bool { std::io::stdout().is_terminal() }

/// Wrap `s` in ANSI `code` when stdout is a TTY; otherwise return it plain (pipes stay clean).
pub fn c(code: &str, s: &str) -> String {
    if is_tty() { format!("\x1b[{code}m{s}\x1b[0m") } else { s.to_string() }
}

pub fn arm_code(tool: &str) -> &'static str {
    if tool.starts_with("baseline") { COL_BASELINE }
    else if tool.starts_with("monogram") { COL_MONOGRAM }
    else if tool.starts_with("codegraph") { COL_CODEGRAPH }
    else { COL_OTHER }
}

pub struct Arm { pub arm: String, pub tool: String, pub model: String, pub effort: String }

/// Parse a run label into (arm, tool, model, effort). The tool may itself contain '-' (e.g.
/// `monogram-mcp`), so the MODEL token is the discriminator — everything before it is the tool.
pub fn parse_arm(label: &str) -> Arm {
    let l = label.strip_suffix(".jsonl").unwrap_or(label);
    let no_run = strip_run_suffix(l);
    let seg: Vec<&str> = no_run.split('-').collect();

    if let Some(i) = seg.iter().position(|s| is_extended_model_start(s)) {
        let effort = seg.last().filter(|s| EFFORTS.contains(s)).copied().unwrap_or("");
        let model_end = if effort.is_empty() { seg.len() } else { seg.len() - 1 };
        return Arm {
            arm: no_run.into(),
            tool: seg[..i].join("-"),
            model: seg[i..model_end].join("-"),
            effort: effort.into(),
        };
    }

    match seg.iter().position(|s| MODELS.contains(s)) {
        None => Arm { arm: no_run.into(), tool: no_run.into(), model: "opus".into(), effort: String::new() },
        Some(i) => Arm {
            arm: no_run.into(),
            tool: seg[..i].join("-"),
            model: seg[i].into(),
            effort: seg[i + 1..].join("-"),
        },
    }
}

fn is_extended_model_start(s: &str) -> bool {
    s == "gpt" || s.starts_with("gpt") ||
    s == "claude" || s.starts_with("claude") ||
    s == "gemini" || s.starts_with("gemini") ||
    s == "codex" || s.starts_with("codex") ||
    s == "agy" || s.starts_with("agy") ||
    s.starts_with('o') && s.chars().skip(1).next().map(|c| c.is_ascii_digit()).unwrap_or(false)
}

/// Strip a trailing `-r<digits>` run suffix.
fn strip_run_suffix(s: &str) -> &str {
    if let Some(pos) = s.rfind("-r") {
        let tail = &s[pos + 2..];
        if !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()) { return &s[..pos]; }
    }
    s
}

pub fn disp_name(tool: &str, model: &str, effort: &str) -> String {
    let mut n = tool.to_string();
    if model != "opus" { n.push('-'); n.push_str(model); }
    if !effort.is_empty() { n.push('@'); n.push_str(effort); }
    n
}

pub fn hum_tok(n: i64) -> String {
    if n == 0 { "—".into() }
    else if n >= 1_000_000 { format!("{:.2}M", n as f64 / 1e6) }
    else if n >= 1000 { format!("{}k", (n as f64 / 1000.0).round() as i64) }
    else { n.to_string() }
}

pub fn median_f(v: &[f64]) -> f64 {
    if v.is_empty() { return 0.0; }
    let mut a = v.to_vec();
    a.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    a[a.len() / 2]
}

pub fn median_i(v: &[i64]) -> i64 {
    if v.is_empty() { return 0; }
    let mut a = v.to_vec();
    a.sort();
    a[a.len() / 2]
}

pub fn model_ord(m: &str) -> usize {
    MODELS.iter().position(|&x| x == m).unwrap_or_else(|| {
        if m.starts_with("gpt") || m.starts_with("codex") { 3 }
        else if m.starts_with("gemini") { 4 }
        else if m.starts_with("agy") { 5 }
        else { 99 }
    })
}

#[cfg(test)]
mod tests {
    use super::parse_arm;

    #[test]
    fn parses_legacy_model_labels() {
        let a = parse_arm("monogram-mcp-sonnet-r2");
        assert_eq!(a.tool, "monogram-mcp");
        assert_eq!(a.model, "sonnet");
        assert_eq!(a.effort, "");
    }

    #[test]
    fn parses_gpt_model_with_dot_version_and_effort() {
        let a = parse_arm("baseline-gpt-5.4-mini-low-r1");
        assert_eq!(a.tool, "baseline");
        assert_eq!(a.model, "gpt-5.4-mini");
        assert_eq!(a.effort, "low");
    }

    #[test]
    fn parses_gpt_model_with_hyphen_version_and_effort() {
        let a = parse_arm("monogram-preindexed-gpt-5-5-low-r3");
        assert_eq!(a.tool, "monogram-preindexed");
        assert_eq!(a.model, "gpt-5-5");
        assert_eq!(a.effort, "low");
    }

    #[test]
    fn parses_agy_model_labels() {
        let a = parse_arm("baseline-agy-low-r1");
        assert_eq!(a.tool, "baseline");
        assert_eq!(a.model, "agy");
        assert_eq!(a.effort, "low");
    }
}

/// Does a shell command invoke `word` as a token?  (^|[\s;|&(])word\b
pub fn cmd_has_word(cmd: &str, word: &str) -> bool {
    for (i, _) in cmd.match_indices(word) {
        let before = i == 0 || matches!(cmd.as_bytes()[i - 1], b' ' | b'\t' | b';' | b'|' | b'&' | b'(');
        let after = match cmd.as_bytes().get(i + word.len()) { None => true, Some(&b) => !(b.is_ascii_alphanumeric() || b == b'_') };
        if before && after { return true; }
    }
    false
}

/// Was a tool_result an error / permission denial?
pub fn is_denied(r: &serde_json::Value) -> bool {
    if r.get("is_error").and_then(|x| x.as_bool()) == Some(true) { return true; }
    let t = result_text(r).to_lowercase();
    t.contains("permission") || t.contains("not allowed") || t.contains("disallow") || t.contains("denied")
}

pub fn read_json(path: &std::path::Path) -> Value {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null)
}

/// Load a stream-json run file into one Value per line (bad lines skipped).
pub fn load_jsonl(path: &str) -> Vec<Value> {
    match std::fs::read_to_string(path) {
        Ok(s) => s.lines().filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<Value>(l).ok()).collect(),
        Err(_) => vec![],
    }
}

/// Flatten a tool_result `content` (string or array of {text}) into one string.
pub fn result_text(r: &Value) -> String {
    match r.get("content") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) => a.iter().filter_map(|x| x.get("text").and_then(|t| t.as_str())).collect::<Vec<_>>().join(" "),
        _ => String::new(),
    }
}

/// Pad-end to a visible width measured in chars (so multibyte box-drawing/× don't over-count).
pub fn pad_end(s: &str, w: usize) -> String {
    let len = s.chars().count();
    if len >= w { s.to_string() } else { format!("{}{}", s, " ".repeat(w - len)) }
}

pub fn pad_start(s: &str, w: usize) -> String {
    let len = s.chars().count();
    if len >= w { s.to_string() } else { format!("{}{}", " ".repeat(w - len), s) }
}

pub fn visible_len(s: &str) -> usize { s.chars().count() }

pub fn fit_middle(s: &str, w: usize) -> String {
    let len = visible_len(s);
    if len <= w { return s.to_string(); }
    if w == 0 { return String::new(); }
    if w == 1 { return "…".into(); }
    let keep = w - 1;
    let left = (keep + 1) / 2;
    let right = keep - left;
    let head: String = s.chars().take(left).collect();
    let tail: String = s.chars().rev().take(right).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{head}…{tail}")
}

pub fn pad_end_fit(s: &str, w: usize) -> String { pad_end(&fit_middle(s, w), w) }

pub fn pad_start_fit(s: &str, w: usize) -> String { pad_start(&fit_middle(s, w), w) }
