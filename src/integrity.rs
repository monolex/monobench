// monobench — score whether a recorded run may be contaminated as benchmark evidence.
// This is intentionally heuristic: it reports risk signals, not a final verdict.
use crate::telemetry;
use crate::util::{cmd_has_word, fit_middle, word_in_command_position};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Signal {
    pub severity: &'static str,
    pub points: u8,
    pub kind: &'static str,
    pub evidence: String,
}

#[derive(Clone, Debug)]
pub struct Finding {
    pub label: String,
    pub grade: String,
    pub score: u8,
    pub level: &'static str,
    pub signals: Vec<Signal>,
    pub source: String,
}

pub fn scan_run(
    label: &str,
    grade: &str,
    event_path: Option<&Path>,
    index_log: &Path,
    has_answer: bool,
    running_path: &Path,
) -> Finding {
    let mut signals = vec![];

    if !label.contains("-t") {
        add(
            &mut signals,
            "info",
            5,
            "legacy_run_label",
            "run label has no timestamp; identity can collide with repeated rN experiments",
        );
    }
    if running_path.is_file() && has_answer {
        add(
            &mut signals,
            "watch",
            15,
            "stale_running_marker",
            ".running marker still exists after an answer artifact was written",
        );
    }
    if !has_answer && event_path.is_some() {
        add(
            &mut signals,
            "watch",
            15,
            "telemetry_without_answer",
            "telemetry exists but no final answer artifact was found",
        );
    }

    let source = match event_path {
        Some(path) => {
            scan_events(&mut signals, path);
            path.display().to_string()
        }
        None => {
            add(
                &mut signals,
                "watch",
                20,
                "missing_telemetry",
                "no structured transcript or stderr trace source was found",
            );
            "-".into()
        }
    };

    scan_index_log(&mut signals, label, index_log);
    signals.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then_with(|| a.kind.cmp(b.kind))
            .then_with(|| a.evidence.cmp(&b.evidence))
    });
    let score = signals
        .iter()
        .map(|s| s.points as u16)
        .sum::<u16>()
        .min(100) as u8;
    Finding {
        label: label.into(),
        grade: grade.into(),
        score,
        level: level_for(score),
        signals,
        source,
    }
}

pub fn print_report(id: &str, findings: &[Finding], detail: bool) {
    let contaminated = findings
        .iter()
        .filter(|f| f.level == "CONTAMINATED")
        .count();
    let suspect = findings.iter().filter(|f| f.level == "SUSPECT").count();
    let watch = findings.iter().filter(|f| f.level == "WATCH").count();
    println!("INTEGRITY  {id}");
    println!(
        "runs={} contaminated={} suspect={} watch={} clean={}",
        findings.len(),
        contaminated,
        suspect,
        watch,
        findings
            .len()
            .saturating_sub(contaminated + suspect + watch)
    );
    println!("\nRUNS");
    let mut rows = findings.to_vec();
    rows.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.label.cmp(&b.label)));
    for f in &rows {
        let top = f
            .signals
            .iter()
            .take(3)
            .map(|s| format!("{}(+{})", s.kind, s.points))
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "  {:<72} grade={:<9} risk={:<12} score={:<3} signals={}",
            fit_middle(&f.label, 72),
            f.grade,
            f.level,
            f.score,
            if top.is_empty() { "-" } else { &top }
        );
    }
    if detail {
        for f in &rows {
            println!("\n{}", f.label);
            println!(
                "  grade={} risk={} score={} source={}",
                f.grade, f.level, f.score, f.source
            );
            if f.signals.is_empty() {
                println!("  signals: none");
            } else {
                println!("  signals:");
                for s in f.signals.iter().take(24) {
                    println!(
                        "    [{:<8} +{:>2}] {:<32} {}",
                        s.severity,
                        s.points,
                        s.kind,
                        fit_middle(&s.evidence, 130)
                    );
                }
            }
        }
    }
    println!("\n[NEXT]");
    println!(
        "  CONTAMINATED: keep for failure analysis only; rerun before counting in benchmark stats."
    );
    println!(
        "  SUSPECT: inspect/export the run and compare against a fresh rerun before final review."
    );
    println!("  CLEAN/WATCH: no obvious contamination signal was found; this is not a proof of validity.");
}

fn scan_events(signals: &mut Vec<Signal>, path: &Path) {
    for ev in telemetry::events_from_path(&path.to_string_lossy()) {
        let cmd = ev.cmd.trim();
        if cmd.is_empty() {
            continue;
        }
        scan_command(signals, cmd, ev.denied);
        scan_result(signals, &ev.result, cmd);
    }
}

fn scan_command(signals: &mut Vec<Signal>, cmd: &str, denied: bool) {
    let low = cmd.to_lowercase();
    let state_target = touches_monogram_state(&low);
    if cmd_has_word(cmd, "git") && !denied && !low.contains("monobench: git is disabled") {
        add(signals, "critical", 50, "git_history_access", cmd);
    }
    if cmd_has_word(cmd, "sqlite3") && (state_target || low.contains("monogram")) {
        add(signals, "critical", 45, "solver_sqlite_monogram_db", cmd);
    }
    if (cmd_has_word(cmd, "rm") || cmd_has_word(cmd, "unlink") || cmd_has_word(cmd, "rmdir"))
        && state_target
    {
        add(
            signals,
            "critical",
            45,
            "solver_deleted_monogram_state",
            cmd,
        );
    }
    // Require kill in command position so `monogram search kill` (a symbol lookup) is not mistaken
    // for the solver killing a process.
    if word_in_command_position(cmd, "kill")
        || word_in_command_position(cmd, "pkill")
        || word_in_command_position(cmd, "killall")
    {
        if targets_tool_process(&low) {
            add(signals, "high", 35, "solver_killed_tool_process", cmd);
        } else {
            // Bare-PID kills (e.g. `ps | grep monogram` then `kill <pid>`) name no tool, so they
            // evade the check above, yet they still interfere with the benchmark environment —
            // a lower-severity watch keeps the common probe→kill evasion pattern visible.
            add(signals, "watch", 10, "solver_killed_process", cmd);
        }
    }
    if cmd_has_word(cmd, "monogram") {
        match monogram_sub(cmd).as_deref() {
            Some("index") | Some("reindex") => {
                add(signals, "high", 35, "solver_reindexed_monogram", cmd)
            }
            Some("prune") | Some("clean") | Some("init") => {
                add(signals, "high", 30, "solver_mutated_monogram_index", cmd)
            }
            Some("serve") | Some("mcp") => {
                add(signals, "watch", 15, "solver_started_monogram_service", cmd)
            }
            _ => {}
        }
    }
    if (cmd_has_word(cmd, "pgrep") || cmd_has_word(cmd, "ps")) && targets_tool_process(&low) {
        add(signals, "watch", 10, "solver_process_probe", cmd);
    }
}

fn scan_result(signals: &mut Vec<Signal>, result: &str, cmd: &str) {
    let low = result.to_lowercase();
    if low.contains("database is locked") && low.contains("sqlite") {
        add(
            signals,
            "high",
            30,
            "monogram_sqlite_locked",
            &format!("cmd={}; result={}", cmd, first_line(result)),
        );
    }
    for phrase in [
        "registry race",
        "wrong db",
        "wrong database",
        "stale registry",
        "no such table",
    ] {
        if low.contains(phrase) {
            add(
                signals,
                "high",
                30,
                "monogram_state_mismatch",
                &format!("cmd={}; result={}", cmd, first_line(result)),
            );
            break;
        }
    }
}

fn scan_index_log(signals: &mut Vec<Signal>, label: &str, path: &Path) {
    let Ok(text) = std::fs::read_to_string(path) else {
        if label.starts_with("monogram") {
            add(
                signals,
                "watch",
                12,
                "missing_index_log",
                "monogram-like run has no index log artifact",
            );
        }
        return;
    };
    let low = text.to_lowercase();
    if low.contains("sqlite3 not found; skipped absolute path rewrite") {
        add(
            signals,
            "high",
            35,
            "prepared_path_rewrite_skipped",
            "prepared DB paths were not rewritten to the run worktree",
        );
    }
    if low.contains("path rewrite failed") || low.contains("mtime refresh failed") {
        add(
            signals,
            "critical",
            45,
            "prepared_db_rewrite_failed",
            "prepared DB rewrite/refresh failed during index setup",
        );
    }
    if low.contains("prepared index install failed") || low.contains("index failed") {
        add(
            signals,
            "critical",
            60,
            "index_setup_failed",
            "index setup failed before solver execution",
        );
    }
    for line in text.lines() {
        if line.contains("[prepared] installing monogram snapshot") {
            if let Some(dst) = line.split(" -> ").nth(1) {
                if !dst.contains(label) {
                    add(
                        signals,
                        "high",
                        35,
                        "prepared_db_not_run_scoped",
                        line.trim(),
                    );
                }
            }
        }
        if line.contains("[prepared] refreshed monogram mtimes") {
            let updated = number_after(line, "updated=");
            let missing = number_after(line, "missing=");
            if let (Some(updated), Some(missing)) = (updated, missing) {
                if missing > 0 {
                    let points = if updated > 0 && missing * 2 >= updated {
                        35
                    } else {
                        12
                    };
                    add(
                        signals,
                        if points >= 35 { "high" } else { "watch" },
                        points,
                        "prepared_mtime_missing",
                        line.trim(),
                    );
                }
            }
        }
    }
}

fn add(
    signals: &mut Vec<Signal>,
    severity: &'static str,
    points: u8,
    kind: &'static str,
    evidence: impl Into<String>,
) {
    let evidence = evidence.into();
    if signals
        .iter()
        .any(|s| s.kind == kind && s.evidence == evidence)
    {
        return;
    }
    signals.push(Signal {
        severity,
        points,
        kind,
        evidence,
    });
}

fn level_for(score: u8) -> &'static str {
    match score {
        0..=9 => "CLEAN",
        10..=29 => "WATCH",
        30..=59 => "SUSPECT",
        _ => "CONTAMINATED",
    }
}

fn touches_monogram_state(low: &str) -> bool {
    low.contains(".monolex/monogram")
        || low.contains("/monogram/")
        || low.contains("monogram.db")
        || low.contains(".registry")
        || low.contains("indexlock")
        || low.contains("index.lock")
}

fn targets_tool_process(low: &str) -> bool {
    ["monogram", "monobrain", "rustc", "cargo", "sqlite", "index"]
        .iter()
        .any(|needle| low.contains(needle))
}

fn monogram_sub(cmd: &str) -> Option<String> {
    let idx = crate::util::cmd_word_pos(cmd, "monogram")?;
    let tok = cmd[idx + 8..].split_whitespace().next().unwrap_or("");
    Some(if tok.is_empty() {
        "help".into()
    } else {
        tok.trim_matches(['"', '\'']).to_string()
    })
}

fn first_line(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .chars()
        .take(160)
        .collect()
}

fn number_after(line: &str, needle: &str) -> Option<u32> {
    let rest = line.split(needle).nth(1)?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{scan_command, scan_index_log, scan_run, Signal};
    use std::path::{Path, PathBuf};

    #[test]
    fn flags_solver_db_surgery() {
        let mut signals: Vec<Signal> = vec![];
        scan_command(
            &mut signals,
            "sqlite3 ~/.monolex/monogram/foo.db 'select count(*) from files'",
            false,
        );
        assert!(signals
            .iter()
            .any(|s| s.kind == "solver_sqlite_monogram_db"));
    }

    #[test]
    fn ignores_quoted_kill_in_monogram_query() {
        let mut signals: Vec<Signal> = vec![];
        scan_command(
            &mut signals,
            "monogram search \"kill runtime style\"",
            false,
        );
        // A quoted "kill" inside a search query is not a process kill of any severity.
        assert!(!signals
            .iter()
            .any(|s| s.kind == "solver_killed_tool_process" || s.kind == "solver_killed_process"));
    }

    #[test]
    fn flags_bare_pid_kill_as_watch() {
        // ps|grep monogram → kill <pid> names no tool, so it evades the tool-named check but must
        // still surface as a lower-severity watch signal.
        let mut signals: Vec<Signal> = vec![];
        scan_command(&mut signals, "kill 82606 84467", false);
        let s = signals
            .iter()
            .find(|s| s.kind == "solver_killed_process")
            .expect("bare-pid kill should flag solver_killed_process");
        assert_eq!(s.severity, "watch");
        assert!(!signals
            .iter()
            .any(|s| s.kind == "solver_killed_tool_process"));

        // A kill that names a tool process stays the high-severity signal.
        let mut named: Vec<Signal> = vec![];
        scan_command(&mut named, "pkill -f 'monogram index'", false);
        assert!(named.iter().any(|s| s.kind == "solver_killed_tool_process"));
        assert!(!named.iter().any(|s| s.kind == "solver_killed_process"));
    }

    #[test]
    fn flags_prepared_missing_mtimes() {
        let p = std::env::temp_dir().join(format!("monobench-integrity-{}", std::process::id()));
        std::fs::write(
            &p,
            "[prepared] refreshed monogram mtimes updated=9319 missing=9319\n",
        )
        .unwrap();
        let mut signals = vec![];
        scan_index_log(
            &mut signals,
            "monogram-codex-gpt-5.3-codex-spark-high-r2-t1",
            &PathBuf::from(&p),
        );
        assert!(signals
            .iter()
            .any(|s| s.kind == "prepared_mtime_missing" && s.points >= 35));
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn telemetry_without_answer_keys_off_has_answer_not_dot_answer_txt() {
        let nope = Path::new("/nonexistent/monobench-test/x");
        // claude-style: telemetry present, answer lives in the .jsonl (has_answer=true computed by
        // the caller via run_answer_text) → must NOT false-flag telemetry_without_answer.
        let with_answer = scan_run("x-t1", "FULL", Some(nope), nope, true, nope);
        assert!(!with_answer
            .signals
            .iter()
            .any(|s| s.kind == "telemetry_without_answer"));
        // genuinely answerless run with telemetry → the signal still fires.
        let no_answer = scan_run("x-t1", "?", Some(nope), nope, false, nope);
        assert!(no_answer
            .signals
            .iter()
            .any(|s| s.kind == "telemetry_without_answer"));
    }
}
