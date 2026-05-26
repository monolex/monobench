// monobench — shared helpers (ANSI palette, jsonl loading, arm parsing, formatting).
use serde_json::Value;
use std::io::IsTerminal;

pub const MODELS: [&str; 5] = ["opus", "sonnet", "haiku", "codex", "gemini"];
pub const CLIS: [&str; 5] = ["claude", "codex", "agy", "gemini", "grok"];
const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

// palette: yellow → orange → red ONLY (baseline=yellow stands apart from tool arms). Chrome stays in-palette.
pub const COL_BASELINE: &str = "38;5;226";
pub const COL_MONOGRAM: &str = "38;5;214";
pub const COL_CODEGRAPH: &str = "38;5;202";
pub const COL_OTHER: &str = "38;5;196";
pub const HEAD: &str = "1;38;5;208";
pub const DIM: &str = "2";

pub fn is_tty() -> bool {
    std::io::stdout().is_terminal()
}

/// Wrap `s` in ANSI `code` when stdout is a TTY; otherwise return it plain (pipes stay clean).
pub fn c(code: &str, s: &str) -> String {
    if is_tty() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn arm_code(tool: &str) -> &'static str {
    if tool.starts_with("baseline") {
        COL_BASELINE
    } else if tool.starts_with("monogram") {
        COL_MONOGRAM
    } else if tool.starts_with("codegraph") {
        COL_CODEGRAPH
    } else {
        COL_OTHER
    }
}

pub struct Arm {
    pub arm: String,
    pub tool: String,
    /// Tool version (semver) parsed from the label's `<tool>-<version>-…` segment, e.g. monogram
    /// `0.52.1`. Empty for baseline, for tools without a `version_bin`, and for legacy labels
    /// recorded before version capture existed.
    pub version: String,
    pub cli: String,
    pub model: String,
    pub effort: String,
}

/// Parse a run label into (arm, tool, cli, model, effort). New labels are
/// `<tool>-<cli>-<model>-<effort>-rN-t<start_ms>`; legacy `-rN` and
/// `<tool>-<model>-<effort>-rN` labels are still accepted.
pub fn parse_arm(label: &str) -> Arm {
    let l = label.strip_suffix(".jsonl").unwrap_or(label);
    let no_run = strip_run_suffix(l);
    let seg: Vec<&str> = no_run.split('-').collect();
    let effort = seg
        .last()
        .filter(|s| EFFORTS.contains(s))
        .copied()
        .unwrap_or("");
    let body_end = if effort.is_empty() {
        seg.len()
    } else {
        seg.len() - 1
    };

    if let Some(i) = seg[..body_end].iter().position(|s| is_cli_token(s)) {
        if i + 1 < body_end {
            let model = seg[i + 1..body_end].join("-");
            let (tool, version) = split_tool_version(&seg[..i]);
            return Arm {
                arm: no_run.into(),
                tool,
                version,
                cli: seg[i].into(),
                model,
                effort: effort.into(),
            };
        }
    }

    if let Some(i) = seg[..body_end]
        .iter()
        .position(|s| is_extended_model_start(s))
    {
        let model = seg[i..body_end].join("-");
        let (tool, version) = split_tool_version(&seg[..i]);
        return Arm {
            arm: no_run.into(),
            tool,
            version,
            cli: default_cli_for_model(&model),
            model,
            effort: effort.into(),
        };
    }

    match seg[..body_end].iter().position(|s| MODELS.contains(s)) {
        None => Arm {
            arm: no_run.into(),
            tool: no_run.into(),
            version: String::new(),
            cli: "claude".into(),
            model: "opus".into(),
            effort: String::new(),
        },
        Some(i) => {
            let (tool, version) = split_tool_version(&seg[..i]);
            Arm {
                arm: no_run.into(),
                tool,
                version,
                cli: default_cli_for_model(seg[i]),
                model: seg[i].into(),
                effort: effort.into(),
            }
        }
    }
}

/// Split a trailing semver-shaped segment off the tool region:
/// `["monogram","0.52.1"]` → (`monogram`, `0.52.1`); `["monogram","mcp","0.52.1"]` →
/// (`monogram-mcp`, `0.52.1`). Requires a tool name *before* the version, so a lone version is
/// never stolen. Legacy tool regions with no version segment → (joined tool, "").
fn split_tool_version(tool_seg: &[&str]) -> (String, String) {
    if tool_seg.len() >= 2 {
        if let Some(last) = tool_seg.last() {
            if is_version_token(last) {
                return (
                    tool_seg[..tool_seg.len() - 1].join("-"),
                    (*last).to_string(),
                );
            }
        }
    }
    (tool_seg.join("-"), String::new())
}

/// A semver-shaped token: a numeric major, a dot, then anything — `0.52.1`, `1.0`, `0.52.1+abc`.
/// Used ONLY on the tool region (before the CLI token), so a dotted *model* like `gpt-5.4` is never
/// mistaken for a tool version.
fn is_version_token(s: &str) -> bool {
    let mut parts = s.splitn(2, '.');
    match (parts.next(), parts.next()) {
        (Some(major), Some(_rest)) => {
            !major.is_empty() && major.bytes().all(|b| b.is_ascii_digit())
        }
        _ => false,
    }
}

pub fn is_cli_token(s: &str) -> bool {
    CLIS.contains(&s)
}

fn is_extended_model_start(s: &str) -> bool {
    s == "gpt"
        || s.starts_with("gpt")
        || s == "claude"
        || s.starts_with("claude")
        || s == "gemini"
        || s.starts_with("gemini")
        || s == "codex"
        || s.starts_with("codex")
        || s == "agy"
        || s.starts_with("agy")
        || s == "grok"
        || s.starts_with("grok")
        || s.starts_with('o')
            && s.chars()
                .skip(1)
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
}

/// Strip a trailing `-r<digits>` run suffix, with optional `-t<digits>` timestamp.
fn strip_run_suffix(s: &str) -> &str {
    let mut end = s.len();
    if let Some(pos) = s.rfind("-t") {
        let tail = &s[pos + 2..];
        if !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()) {
            end = pos;
        }
    }
    let base = &s[..end];
    if let Some(pos) = base.rfind("-r") {
        let tail = &base[pos + 2..];
        if !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()) {
            return &base[..pos];
        }
    }
    s
}

/// Epoch-ms start time embedded in a timestamped run label or filename (`…-rN-t<ms>` optionally
/// followed by any `.ext` such as `.jsonl` / `.answer.txt` / `.running`); None for legacy labels
/// with no `-t` (caller may fall back to file mtime). Reads the digit run after the last `-t`, so
/// non-numeric tails like `-test` are correctly rejected.
pub fn label_start_ms(label: &str) -> Option<u64> {
    let pos = label.rfind("-t")?;
    let digits: String = label[pos + 2..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    (!digits.is_empty())
        .then(|| digits.parse::<u64>().ok())
        .flatten()
}

/// Compact UTC wall-clock for an epoch-ms start time: "MM-DD HH:MMZ" (minute resolution; the trailing
/// Z marks UTC). Dependency-free — epoch is already UTC, so no timezone offset is needed.
pub fn fmt_utc_ms(ms: u64) -> String {
    let (_y, mo, d, hh, mm, _ss) = civil_from_epoch(ms / 1000);
    format!("{mo:02}-{d:02} {hh:02}:{mm:02}Z")
}

/// Civil UTC date/time from epoch seconds (Howard Hinnant's algorithm) → (year, month, day, h, m, s).
fn civil_from_epoch(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (secs / 86400) as i64;
    let rem = (secs % 86400) as u32;
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m as u32, d as u32, hh, mm, ss)
}

pub fn full_arm_name(tool: &str, version: &str, cli: &str, model: &str, effort: &str) -> String {
    let mut n = String::from(tool);
    if !version.is_empty() {
        n.push('-');
        n.push_str(version);
    }
    n.push('-');
    n.push_str(cli);
    n.push('-');
    n.push_str(model);
    if !effort.is_empty() {
        n.push('-');
        n.push_str(effort);
    }
    n
}

/// Resolve a tool binary on PATH, follow symlinks, and read its OpenCLIs-install semver from the
/// canonical path `…/versions/<name>/<semver>/<ts>/…`. Returns "" when the binary is missing or is
/// not OpenCLIs-installed (e.g. a `target/debug` or worktree build) — it NEVER fabricates a version.
/// This is the monogram on PATH at run start, the same one a CLI-delivered solver invokes.
pub fn capture_semver(bin: &str) -> String {
    resolve_on_path(bin)
        .as_deref()
        .and_then(semver_from_install_path)
        .unwrap_or_default()
}

fn resolve_on_path(bin: &str) -> Option<std::path::PathBuf> {
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {bin} 2>/dev/null"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if p.is_empty() {
        return None;
    }
    std::fs::canonicalize(p).ok()
}

fn semver_from_install_path(p: &std::path::Path) -> Option<String> {
    let comps: Vec<&str> = p.to_str()?.split('/').collect();
    let i = comps.iter().position(|&c| c == "versions")?;
    let ver = comps.get(i + 2)?; // versions/<name>/<semver>/…
    is_version_token(ver).then(|| (*ver).to_string())
}

pub fn env_name(cli: &str, model: &str, effort: &str) -> String {
    let mut n = format!("{cli}/{model}");
    if !effort.is_empty() {
        n.push('@');
        n.push_str(effort);
    }
    n
}

pub fn env_ord(cli: &str, model: &str, effort: &str) -> (usize, usize, String, String) {
    let cli_ord = CLIS.iter().position(|&x| x == cli).unwrap_or(99);
    (
        cli_ord,
        model_ord(model),
        model.to_string(),
        effort.to_string(),
    )
}

pub fn hum_tok(n: i64) -> String {
    if n == 0 {
        "—".into()
    } else if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1e6)
    } else if n >= 1000 {
        format!("{}k", (n as f64 / 1000.0).round() as i64)
    } else {
        n.to_string()
    }
}

pub fn median_f(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut a = v.to_vec();
    a.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    let mid = a.len() / 2;
    if a.len() % 2 == 0 {
        (a[mid - 1] + a[mid]) / 2.0
    } else {
        a[mid]
    }
}

pub fn median_i(v: &[i64]) -> i64 {
    if v.is_empty() {
        return 0;
    }
    let mut a = v.to_vec();
    a.sort();
    let mid = a.len() / 2;
    if a.len() % 2 == 0 {
        ((a[mid - 1] as f64 + a[mid] as f64) / 2.0).round() as i64
    } else {
        a[mid]
    }
}

pub fn model_ord(m: &str) -> usize {
    MODELS.iter().position(|&x| x == m).unwrap_or_else(|| {
        if m.starts_with("gpt") || m.starts_with("codex") {
            3
        } else if m.starts_with("gemini") {
            4
        } else if m.starts_with("agy") {
            5
        } else if m.starts_with("grok") {
            6
        } else {
            99
        }
    })
}

pub fn default_cli_for_model(model: &str) -> String {
    if model.starts_with("gpt")
        || model.starts_with("codex")
        || model.starts_with('o')
            && model
                .chars()
                .nth(1)
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
    {
        "codex".into()
    } else if model.starts_with("gemini") {
        "gemini".into()
    } else if model.starts_with("agy") {
        "agy".into()
    } else if model.starts_with("grok") {
        "grok".into()
    } else {
        "claude".into()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cmd_has_unquoted_pipe, cmd_has_word, cmd_word_pos, fmt_utc_ms, full_arm_name,
        label_start_ms, median_f, median_i, parse_arm, word_in_command_position,
    };

    #[test]
    fn label_start_ms_reads_timestamp_and_ignores_legacy() {
        assert_eq!(
            label_start_ms("monogram-claude-haiku-r1-t1779647883683"),
            Some(1779647883683)
        );
        assert_eq!(
            label_start_ms("monogram-claude-haiku-r1-t1779647883683.jsonl"),
            Some(1779647883683)
        );
        assert_eq!(
            label_start_ms("monogram-claude-haiku-r1-t1779647883683.answer.txt"),
            Some(1779647883683)
        );
        assert_eq!(label_start_ms("baseline-haiku-r3"), None); // legacy, no -t
        assert_eq!(label_start_ms("baseline-claude-test-r1"), None); // -test is not a timestamp
    }

    #[test]
    fn fmt_utc_ms_renders_known_epochs_in_utc() {
        // 1700000000s = 2023-11-14 22:13:20 UTC (well-known epoch).
        assert_eq!(fmt_utc_ms(1_700_000_000_000), "11-14 22:13Z");
        // epoch 0 = 1970-01-01 00:00:00 UTC.
        assert_eq!(fmt_utc_ms(0), "01-01 00:00Z");
        // A real run start time from the benchmark data (verified against `date -u`).
        assert_eq!(fmt_utc_ms(1_779_647_883_683), "05-24 18:38Z");
    }

    #[test]
    fn parses_legacy_model_labels() {
        let a = parse_arm("monogram-mcp-sonnet-r2");
        assert_eq!(a.tool, "monogram-mcp");
        assert_eq!(a.cli, "claude");
        assert_eq!(a.model, "sonnet");
        assert_eq!(a.effort, "");
    }

    #[test]
    fn parses_cli_first_labels_with_full_model_names() {
        let a = parse_arm("monogram-mcp-agy-claude-opus-4.1-low-r2");
        assert_eq!(a.tool, "monogram-mcp");
        assert_eq!(a.cli, "agy");
        assert_eq!(a.model, "claude-opus-4.1");
        assert_eq!(a.effort, "low");

        let b = parse_arm("baseline-codex-gpt-5.4-mini-low-r1");
        assert_eq!(b.tool, "baseline");
        assert_eq!(b.cli, "codex");
        assert_eq!(b.model, "gpt-5.4-mini");
        assert_eq!(b.effort, "low");
    }

    #[test]
    fn parses_grok_cli_labels() {
        // The cli token "grok" must win over the homonym in the model "grok-build".
        let a = parse_arm("baseline-grok-grok-build-low-r1-t1779773782544");
        assert_eq!(a.tool, "baseline");
        assert_eq!(a.cli, "grok");
        assert_eq!(a.model, "grok-build");
        assert_eq!(a.effort, "low");

        let b = parse_arm("monogram-grok-grok-build-medium-r2");
        assert_eq!(b.tool, "monogram");
        assert_eq!(b.cli, "grok");
        assert_eq!(b.model, "grok-build");
        assert_eq!(b.effort, "medium");
    }

    #[test]
    fn parses_timestamped_run_labels() {
        let a = parse_arm("monogram-codex-gpt-5.5-low-r7-t1779581234567");
        assert_eq!(a.arm, "monogram-codex-gpt-5.5-low");
        assert_eq!(a.tool, "monogram");
        assert_eq!(a.cli, "codex");
        assert_eq!(a.model, "gpt-5.5");
        assert_eq!(a.effort, "low");
    }

    #[test]
    fn parses_gpt_model_with_dot_version_and_effort() {
        let a = parse_arm("baseline-gpt-5.4-mini-low-r1");
        assert_eq!(a.tool, "baseline");
        assert_eq!(a.cli, "codex");
        assert_eq!(a.model, "gpt-5.4-mini");
        assert_eq!(a.effort, "low");
    }

    #[test]
    fn parses_gpt_model_with_hyphen_version_and_effort() {
        let a = parse_arm("monogram-preindexed-gpt-5-5-low-r3");
        assert_eq!(a.tool, "monogram-preindexed");
        assert_eq!(a.cli, "codex");
        assert_eq!(a.model, "gpt-5-5");
        assert_eq!(a.effort, "low");
    }

    #[test]
    fn parses_agy_model_labels() {
        let a = parse_arm("baseline-agy-low-r1");
        assert_eq!(a.tool, "baseline");
        assert_eq!(a.cli, "agy");
        assert_eq!(a.model, "agy");
        assert_eq!(a.effort, "low");
    }

    #[test]
    fn reads_semver_from_openclis_install_path() {
        use std::path::Path;
        // canonical OpenCLIs layout: …/versions/<name>/<semver>/<timestamp>/<name>/<bin>
        assert_eq!(
            super::semver_from_install_path(Path::new(
                "/Users/x/.openclis/versions/monogram/0.52.1/2026-05-24-041040/monogram/monogram"
            )),
            Some("0.52.1".to_string())
        );
        // a non-OpenCLIs build (target/debug, worktree) has no version to read → None, never faked.
        assert_eq!(
            super::semver_from_install_path(Path::new("/repo/target/debug/monogram")),
            None
        );
    }

    #[test]
    fn parses_and_separates_tool_version() {
        let a = parse_arm("monogram-0.52.1-claude-haiku-r1-t1779701244954");
        assert_eq!(a.tool, "monogram");
        assert_eq!(a.version, "0.52.1");
        assert_eq!(a.cli, "claude");
        assert_eq!(a.model, "haiku");
        assert_eq!(a.arm, "monogram-0.52.1-claude-haiku");
    }

    #[test]
    fn version_separates_multiword_tool() {
        let a = parse_arm("monogram-mcp-0.52.1-agy-claude-opus-4.1-low-r2");
        assert_eq!(a.tool, "monogram-mcp");
        assert_eq!(a.version, "0.52.1");
        assert_eq!(a.cli, "agy");
        assert_eq!(a.model, "claude-opus-4.1");
        assert_eq!(a.effort, "low");
    }

    #[test]
    fn legacy_label_has_empty_version() {
        // Runs recorded before version capture parse identically to before, with version "".
        let a = parse_arm("monogram-claude-haiku-r1");
        assert_eq!(a.tool, "monogram");
        assert_eq!(a.version, "");
        assert_eq!(a.cli, "claude");
        assert_eq!(a.model, "haiku");
    }

    #[test]
    fn model_dot_version_is_not_mistaken_for_tool_version() {
        // `5.4` lives in the MODEL region (after the cli token) and must NOT become the arm version.
        let a = parse_arm("baseline-codex-gpt-5.4-mini-low-r1");
        assert_eq!(a.tool, "baseline");
        assert_eq!(a.version, "");
        assert_eq!(a.model, "gpt-5.4-mini");
    }

    #[test]
    fn full_arm_name_round_trips_version() {
        let lbl = full_arm_name("monogram", "0.52.1", "claude", "haiku", "");
        assert_eq!(lbl, "monogram-0.52.1-claude-haiku");
        let a = parse_arm(&lbl);
        assert_eq!(a.tool, "monogram");
        assert_eq!(a.version, "0.52.1");
        // No version → no segment, identical to the legacy form.
        assert_eq!(
            full_arm_name("baseline", "", "claude", "haiku", "low"),
            "baseline-claude-haiku-low"
        );
    }

    #[test]
    fn median_averages_even_sized_sets() {
        assert!((median_f(&[0.30, 0.18]) - 0.24).abs() < 0.000001);
        assert_eq!(median_i(&[23, 33]), 28);
    }

    #[test]
    fn cmd_word_ignores_quoted_regex_alternation() {
        assert!(!cmd_has_word(
            "ps aux | rg 'cargo|rustc|monogram-a33|lib_monogram' | rg -v 'rg '",
            "monogram"
        ));
    }

    #[test]
    fn pipe_detector_ignores_regex_alternation_inside_quotes() {
        assert!(cmd_has_unquoted_pipe(
            "ps aux | rg 'cargo|rustc|monogram-a33|lib_monogram' | rg -v 'rg '"
        ));
        assert!(!cmd_has_unquoted_pipe(
            "monogram grep \"ZigString__fromUTF16\\|ZigString__fromUTF8\" --chain"
        ));
    }

    #[test]
    fn cmd_word_still_counts_pipeline_command() {
        assert!(cmd_has_word("rg foo | monogram search bar", "monogram"));
    }

    #[test]
    fn cmd_word_pos_skips_path_names() {
        let cmd = "cd /tmp/wt/monogram-gpt-5.3-r2 && monogram context Foo";
        let pos = cmd_word_pos(cmd, "monogram").unwrap();
        assert_eq!(&cmd[pos..pos + 16], "monogram context");
    }

    #[test]
    fn command_position_distinguishes_kill_command_from_kill_argument() {
        // Real kill commands: at the start, or after a shell separator.
        assert!(word_in_command_position("kill 82606 84467", "kill"));
        assert!(word_in_command_position(
            "ps -ef | grep mono; kill 9001",
            "kill"
        ));
        // `kill` as an argument to another command is NOT a process kill.
        assert!(!word_in_command_position("monogram search kill", "kill"));
        assert!(!word_in_command_position(
            "monogram context kill --code 80",
            "kill"
        ));
    }
}

fn quote_mask(cmd: &str) -> Vec<bool> {
    let mut mask = vec![false; cmd.len()];
    let mut single = false;
    let mut double = false;
    let mut escaped = false;
    for (i, b) in cmd.bytes().enumerate() {
        let in_quote = single || double;
        if b == b'\\' && !escaped {
            mask[i] = in_quote;
            escaped = true;
            continue;
        }
        if b == b'\'' && !double && !escaped {
            mask[i] = true;
            single = !single;
        } else if b == b'"' && !single && !escaped {
            mask[i] = true;
            double = !double;
        } else {
            mask[i] = in_quote;
        }
        escaped = false;
    }
    mask
}

/// Find `word` as an unquoted shell token.  (^|[\s;|&(])word\b
pub fn cmd_word_pos(cmd: &str, word: &str) -> Option<usize> {
    let quoted = quote_mask(cmd);
    for (i, _) in cmd.match_indices(word) {
        if quoted.get(i).copied().unwrap_or(false) {
            continue;
        }
        let before = i == 0
            || matches!(
                cmd.as_bytes()[i - 1],
                b' ' | b'\t' | b';' | b'|' | b'&' | b'('
            );
        let after = match cmd.as_bytes().get(i + word.len()) {
            None => true,
            Some(&b) => !(b.is_ascii_alphanumeric() || b == b'_'),
        };
        if before && after {
            return Some(i);
        }
    }
    None
}

/// Does a shell command invoke `word` as an unquoted token?
pub fn cmd_has_word(cmd: &str, word: &str) -> bool {
    cmd_word_pos(cmd, word).is_some()
}

/// True when `word` is the command being executed — at the start of the line or right after a
/// shell separator (`; | & (`) — not merely an argument to another command. Lets us tell a real
/// `kill <pid>` from a `monogram search kill` (searching for a symbol named "kill").
pub fn word_in_command_position(cmd: &str, word: &str) -> bool {
    let mut start = 0;
    while let Some(rel) = cmd_word_pos(&cmd[start..], word) {
        let idx = start + rel;
        let prev = cmd[..idx].bytes().rev().find(|b| !b.is_ascii_whitespace());
        if prev
            .map(|b| matches!(b, b';' | b'|' | b'&' | b'('))
            .unwrap_or(true)
        {
            return true;
        }
        start = idx + word.len();
    }
    false
}

/// Does a shell command contain an actual unquoted pipeline marker?
pub fn cmd_has_unquoted_pipe(cmd: &str) -> bool {
    let quoted = quote_mask(cmd);
    cmd.bytes()
        .enumerate()
        .any(|(i, b)| b == b'|' && !quoted.get(i).copied().unwrap_or(false))
}

/// Was a tool_result an error / permission denial?
pub fn is_denied(r: &serde_json::Value) -> bool {
    if r.get("is_error").and_then(|x| x.as_bool()) == Some(true) {
        return true;
    }
    let t = result_text(r).to_lowercase();
    t.contains("permission")
        || t.contains("not allowed")
        || t.contains("disallow")
        || t.contains("denied")
}

pub fn read_json(path: &std::path::Path) -> Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null)
}

/// Load a stream-json run file into one Value per line (bad lines skipped).
pub fn load_jsonl(path: &str) -> Vec<Value> {
    match std::fs::read_to_string(path) {
        Ok(s) => s
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .collect(),
        Err(_) => vec![],
    }
}

/// Flatten a tool_result `content` (string or array of {text}) into one string.
pub fn result_text(r: &Value) -> String {
    match r.get("content") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|x| x.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

/// Pad-end to a visible width measured in chars (so multibyte box-drawing/× don't over-count).
pub fn pad_end(s: &str, w: usize) -> String {
    let len = s.chars().count();
    if len >= w {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(w - len))
    }
}

pub fn pad_start(s: &str, w: usize) -> String {
    let len = s.chars().count();
    if len >= w {
        s.to_string()
    } else {
        format!("{}{}", " ".repeat(w - len), s)
    }
}

pub fn visible_len(s: &str) -> usize {
    s.chars().count()
}

pub fn fit_middle(s: &str, w: usize) -> String {
    let len = visible_len(s);
    if len <= w {
        return s.to_string();
    }
    if w == 0 {
        return String::new();
    }
    if w == 1 {
        return "…".into();
    }
    let keep = w - 1;
    let left = (keep + 1) / 2;
    let right = keep - left;
    let head: String = s.chars().take(left).collect();
    let tail: String = s
        .chars()
        .rev()
        .take(right)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}…{tail}")
}

pub fn pad_end_fit(s: &str, w: usize) -> String {
    pad_end(&fit_middle(s, w), w)
}

pub fn pad_start_fit(s: &str, w: usize) -> String {
    pad_start(&fit_middle(s, w), w)
}
