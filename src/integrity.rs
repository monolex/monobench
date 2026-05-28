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
    instance_id: &str,
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
            scan_events(&mut signals, path, own_runid_from_label(label), instance_id);
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

/// Like `scan_run`, but also runs the foreign-repo-marker / cross-instance-path
/// validator on the final answer text. Use this from the CLI when the caller has
/// already loaded the answer (e.g. `run_answer_text(...)`); the wrapper passes a
/// `&str` straight through and recomputes score+level so the new signals affect
/// the final risk verdict.
pub fn scan_run_with_answer(
    label: &str,
    grade: &str,
    event_path: Option<&Path>,
    index_log: &Path,
    answer_text: &str,
    running_path: &Path,
    instance_id: &str,
) -> Finding {
    let mut f = scan_run(
        label,
        grade,
        event_path,
        index_log,
        !answer_text.trim().is_empty(),
        running_path,
        instance_id,
    );
    scan_answer_artifact(
        &mut f.signals,
        answer_text,
        own_runid_from_label(label),
        instance_id,
    );
    // Recompute score + level since new signals may have been added.
    f.score = f
        .signals
        .iter()
        .map(|s| s.points as u16)
        .sum::<u16>()
        .min(100) as u8;
    f.level = level_for(f.score);
    f.signals.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then_with(|| a.kind.cmp(b.kind))
            .then_with(|| a.evidence.cmp(&b.evidence))
    });
    f
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

fn scan_events(
    signals: &mut Vec<Signal>,
    path: &Path,
    own_runid: &str,
    own_instance_id: &str,
) {
    for ev in telemetry::events_from_path(&path.to_string_lossy()) {
        let cmd = ev.cmd.trim();
        if cmd.is_empty() {
            continue;
        }
        scan_command(signals, cmd, ev.denied, own_runid, own_instance_id);
        scan_result(signals, &ev.result, cmd);
    }
}

fn scan_command(
    signals: &mut Vec<Signal>,
    cmd: &str,
    denied: bool,
    own_runid: &str,
    own_instance_id: &str,
) {
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
            Some("index") | Some("reindex") | Some("prune") if denied => {
                add(signals, "watch", 15, "solver_prepared_reindex_blocked", cmd)
            }
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
        if denied && monogram_boot_init(cmd) {
            add(signals, "watch", 15, "solver_prepared_reindex_blocked", cmd);
        } else if monogram_boot_init(cmd) {
            add(signals, "high", 30, "solver_mutated_monogram_index", cmd);
        }
        if denied && monogram_reindex_flag(cmd) {
            add(signals, "watch", 15, "solver_prepared_reindex_blocked", cmd);
        } else if monogram_reindex_flag(cmd) {
            add(signals, "high", 35, "solver_reindexed_monogram", cmd);
        }
    }
    if (cmd_has_word(cmd, "pgrep") || cmd_has_word(cmd, "ps")) && targets_tool_process(&low) {
        add(signals, "watch", 10, "solver_process_probe", cmd);
    }

    // -- foreign worktree access --
    // Detects the "agent broadly searched /private/tmp and stumbled into another instance's
    // worktree" contamination mechanism.
    //
    // Layouts handled:
    //   OLD: wt/<runid>-<pid>/...           → first segment after wt/ contains own_runid
    //   NEW: wt/<instance>/<runid>-<pid>/...→ first segment after wt/ equals own_instance_id
    //                                          (the agent may also probe just wt/<instance>/
    //                                          for ls/find — its own instance subdir, parent
    //                                          of its assigned worktree; that's denied by the
    //                                          kernel sandbox but the path itself is NOT a
    //                                          sibling-access signal)
    //
    // Any wt/-rooted path whose first segment is neither own_instance_id NOR contains
    // own_runid is by definition a sibling run we should never have read.
    if !own_runid.is_empty() || !own_instance_id.is_empty() {
        'outer: for marker in ["/private/tmp/monobench-work/wt/", "/tmp/monobench-work/wt/"] {
            let mut rest = cmd;
            while let Some(idx) = rest.find(marker) {
                let after = &rest[idx + marker.len()..];
                let path_end = after
                    .find(|c: char| {
                        c.is_whitespace()
                            || matches!(c, '"' | '\'' | ';' | '&' | '|' | '<' | '>' | ')')
                    })
                    .unwrap_or(after.len());
                let full_path = &after[..path_end];
                // First slash-delimited segment after wt/ — instance id in NEW layout, run-id
                // in OLD layout. Strip a trailing slash so `php-19591-…/` compares equal to
                // `php-19591-…`.
                let first_seg = full_path
                    .split('/')
                    .next()
                    .unwrap_or("")
                    .trim_end_matches('/');
                let is_own_runid = !own_runid.is_empty() && full_path.contains(own_runid);
                let is_own_instance =
                    !own_instance_id.is_empty() && first_seg == own_instance_id;
                if !first_seg.is_empty() && !is_own_runid && !is_own_instance {
                    add(signals, "critical", 65, "sibling_worktree_access", cmd);
                    break 'outer;
                }
                rest = &after[path_end..];
            }
        }
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

/// Scan the run's final answer text for foreign-repo signals.
///
/// Two checks:
///   1. ROOTCAUSE answer line references a `/tmp/monobench-work/wt/...` path that
///      does NOT contain our own run-id → `cross_instance_answer_path` (+70).
///   2. ROOTCAUSE answer file-path contains a curated foreign-repo marker
///      (from `FOREIGN_REPO_MARKERS`) → `foreign_repo_marker_in_answer` (+60).
///
/// Catches the wrong-base-clone contamination mechanism that cmd-scan misses
/// (the agent legitimately searches its own worktree but the worktree itself
/// is bound to the wrong repo).
fn scan_answer_artifact(
    signals: &mut Vec<Signal>,
    answer_text: &str,
    own_runid: &str,
    instance_id: &str,
) {
    if answer_text.trim().is_empty() {
        return;
    }
    let own_prefix = problem_prefix(instance_id);

    let answer_line = answer_text
        .lines()
        .find(|l| l.to_lowercase().contains("rootcause:"))
        .unwrap_or("");
    if answer_line.is_empty() {
        return;
    }
    let answer_clean = crate::grade::strip_worktree_prefix(answer_line);

    // Check 1: raw answer line references a wt/<seg> path not containing our run-id.
    if !own_runid.is_empty() {
        'outer: for marker in ["/private/tmp/monobench-work/wt/", "/tmp/monobench-work/wt/"] {
            if let Some(idx) = answer_line.find(marker) {
                let after = &answer_line[idx + marker.len()..];
                let path_end = after
                    .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\''))
                    .unwrap_or(after.len());
                let path_seg = &after[..path_end];
                if !path_seg.is_empty() && !path_seg.contains(own_runid) {
                    add(
                        signals,
                        "critical",
                        70,
                        "cross_instance_answer_path",
                        answer_line.trim(),
                    );
                    break 'outer;
                }
            }
        }
    }

    // Check 2: cleaned answer path contains a marker that belongs to a DIFFERENT problem.
    for (prefix, markers) in FOREIGN_REPO_MARKERS {
        if *prefix == own_prefix {
            continue;
        }
        for mk in *markers {
            if answer_clean.contains(mk) {
                add(
                    signals,
                    "critical",
                    60,
                    "foreign_repo_marker_in_answer",
                    format!(
                        "marker='{}' belongs to {}*: {}",
                        mk,
                        prefix,
                        answer_clean.trim()
                    ),
                );
                return;
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

/// Extract the run-id from the label that `scan_run` receives.
/// Currently `label` IS the run-id (e.g. "baseline-claude-haiku-r1-t1779934411735").
/// Kept as a fn so a future label-vs-runid split has one place to update.
fn own_runid_from_label(label: &str) -> &str {
    label
}

/// Extract the problem prefix from an instance id, e.g.
/// "netty-12036-unsafe-bytebuffer-uaf" -> "netty".
fn problem_prefix(instance_id: &str) -> &str {
    instance_id.split('-').next().unwrap_or(instance_id)
}

/// Foreign-repo markers: substrings that, if present in an *answer file path*,
/// unambiguously identify the answer as belonging to a different problem's repo.
///
/// Used by `scan_answer_artifact` to catch the "wrong base clone bound to this
/// run" contamination mechanism that pure cmd-scan cannot see (e.g. a `node`
/// instance whose own worktree was bound to deno-base and answered with
/// `ext/node/ops/sqlite/database.rs`).
///
/// Add a marker only when it would NEVER plausibly appear in another problem's
/// codebase — that avoids false positives on generic words like "src" or "lib".
const FOREIGN_REPO_MARKERS: &[(&str, &[&str])] = &[
    ("bun", &["src/bun.js/", "BunString", "src/napi/napi.zig"]),
    (
        "cpython",
        &["Modules/_json.c", "Modules/_grouper", "_PyRawMutex"],
    ),
    (
        "dart",
        &["dart-lang/sdk", "sdk/runtime/", "sdk/lib/core/uri.dart"],
    ),
    (
        "deno",
        &["ext/node/ops/sqlite", "ext/node/ops/zlib", "deno_node"],
    ),
    (
        "dotnet",
        &[
            "System.Management/",
            "InteropClasses/WMIInterop",
            "GCHandle.cs",
        ],
    ),
    ("envoy", &["envoy/source/extensions", "FileStreamer"]),
    ("flutter", &["flutter/engine", "flow/SurfaceTexture"]),
    (
        "ghostty",
        &["apprt/gtk", "apprt/gtk-ng/class/split_tree.zig"],
    ),
    ("grpc", &["src/core/ext/filters/rls", "lb_policy/rls"]),
    ("ksmbd", &["smb2pdu.c", "mgmt/user_session"]),
    ("ktor", &["ktor-io/", "readChannel"]),
    ("kubernetes", &["staging/src/k8s.io", "pkg/api/v1/pod"]),
    ("neovim", &["src/nvim/", "neovim/runtime"]),
    (
        "netty",
        &["io/netty/", "PlatformDependent.java", "UnsafeBuffer.java"],
    ),
    (
        "node",
        &["deps/llhttp/", "src/node_zlib", "src/node_sqlite"],
    ),
    ("numpy", &["numpy/_core", "nditer_constr"]),
    ("openresty", &["lualib/ngx/pipe", "lua-resty"]),
    (
        "php",
        &[
            "php-src/",
            "Zend/zend.h",
            "ext/lexbor/",
            "ext/com_dotnet/",
            "Zend/Optimizer",
        ],
    ),
    ("pytorch", &["aten/src/ATen", "torch/csrc/jit/tensorexpr"]),
    ("redis", &["src/streamCommand.c", "src/restoreCommand"]),
    ("ruby", &["io_buffer.c", "yjit/src/"]),
    ("spark", &["org/apache/spark/", "core/src/main/scala/"]),
    ("swift", &["lib/Demangling/Demangler.cpp", "apple/swift"]),
    ("vapor", &["Sources/Vapor/", "FileMiddleware.swift"]),
];

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

fn monogram_reindex_flag(cmd: &str) -> bool {
    let Some(idx) = crate::util::cmd_word_pos(cmd, "monogram") else {
        return false;
    };
    cmd[idx + 8..]
        .split_whitespace()
        .any(|tok| matches!(tok.trim_matches(['"', '\'']), "-r" | "--reindex"))
}

fn monogram_boot_init(cmd: &str) -> bool {
    let Some(idx) = crate::util::cmd_word_pos(cmd, "monogram") else {
        return false;
    };
    let mut words = cmd[idx + 8..]
        .split_whitespace()
        .map(|tok| tok.trim_matches(['"', '\'']));
    matches!(words.next(), Some("boot" | "b")) && matches!(words.next(), Some("init"))
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
    use super::{scan_answer_artifact, scan_command, scan_index_log, scan_run, Signal};
    use std::path::{Path, PathBuf};

    #[test]
    fn flags_solver_db_surgery() {
        let mut signals: Vec<Signal> = vec![];
        scan_command(
            &mut signals,
            "sqlite3 ~/.monolex/monogram/foo.db 'select count(*) from files'",
            false,
            "",
            "",
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
            "",
            "",
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
        scan_command(&mut signals, "kill 82606 84467", false, "", "");
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
        scan_command(&mut named, "pkill -f 'monogram index'", false, "", "");
        assert!(named.iter().any(|s| s.kind == "solver_killed_tool_process"));
        assert!(!named.iter().any(|s| s.kind == "solver_killed_process"));
    }

    #[test]
    fn blocked_prepared_reindex_is_not_scored_as_mutation() {
        let mut signals: Vec<Signal> = vec![];
        scan_command(&mut signals, "monogram index . -r", true, "", "");
        assert!(signals
            .iter()
            .any(|s| s.kind == "solver_prepared_reindex_blocked"));
        assert!(!signals
            .iter()
            .any(|s| s.kind == "solver_reindexed_monogram"));
    }

    #[test]
    fn blocked_prepared_mutation_commands_are_not_scored_as_mutation() {
        for cmd in ["monogram prune --force", "monogram boot init"] {
            let mut signals: Vec<Signal> = vec![];
            scan_command(&mut signals, cmd, true, "", "");
            assert!(signals
                .iter()
                .any(|s| s.kind == "solver_prepared_reindex_blocked"));
            assert!(!signals
                .iter()
                .any(|s| s.kind == "solver_mutated_monogram_index"));
        }
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
        let with_answer = scan_run("x-t1", "FULL", Some(nope), nope, true, nope, "");
        assert!(!with_answer
            .signals
            .iter()
            .any(|s| s.kind == "telemetry_without_answer"));
        // genuinely answerless run with telemetry → the signal still fires.
        let no_answer = scan_run("x-t1", "?", Some(nope), nope, false, nope, "");
        assert!(no_answer
            .signals
            .iter()
            .any(|s| s.kind == "telemetry_without_answer"));
    }

    // ----- Cross-instance contamination detectors (added 2026-05-28) -----
    //
    // Pinned against the three real contaminations the audit found:
    //   dotnet-124796/baseline-claude-haiku-r1-t1779934411735  (answered php content)
    //   node-56840/baseline-claude-haiku-r1-t1779688284216     (answered deno content)
    //   node-62325/baseline-claude-haiku-r1-t1779951615391     (answered netty content)

    #[test]
    fn sibling_worktree_cmd_is_critical() {
        let mut signals: Vec<Signal> = vec![];
        let cmd = "ls -la /private/tmp/monobench-work/wt/baseline-claude-haiku-r2-t1779934441176-96254/ext/com_dotnet/";
        scan_command(
            &mut signals,
            cmd,
            false,
            "baseline-claude-haiku-r1-t1779934411735",
            "",
        );
        let s = signals
            .iter()
            .find(|s| s.kind == "sibling_worktree_access")
            .expect("sibling worktree access must be flagged");
        assert!(
            s.points >= 60,
            "must reach CONTAMINATED threshold, got {}",
            s.points
        );
    }

    #[test]
    fn own_worktree_cmd_does_not_flag_sibling() {
        let mut signals: Vec<Signal> = vec![];
        // OLD layout: wt/<runid>-<pid>/...
        let cmd_old = "find /private/tmp/monobench-work/wt/baseline-claude-haiku-r1-t1779934411735-96254 -type f";
        scan_command(
            &mut signals,
            cmd_old,
            false,
            "baseline-claude-haiku-r1-t1779934411735",
            "",
        );
        assert!(
            !signals.iter().any(|s| s.kind == "sibling_worktree_access"),
            "own-runid worktree must NOT trigger sibling_worktree_access (OLD layout), got: {:?}",
            signals
        );

        let mut signals2: Vec<Signal> = vec![];
        // NEW layout: wt/<instance>/<runid>-<pid>/... — own_runid still appears in the path.
        let cmd_new = "ls /private/tmp/monobench-work/wt/netty-12036-unsafe-bytebuffer-uaf/baseline-claude-haiku-r1-t1779934411735-96254/io/netty/";
        scan_command(
            &mut signals2,
            cmd_new,
            false,
            "baseline-claude-haiku-r1-t1779934411735",
            "",
        );
        assert!(
            !signals2.iter().any(|s| s.kind == "sibling_worktree_access"),
            "own-runid worktree must NOT trigger sibling_worktree_access (NEW layout), got: {:?}",
            signals2
        );
    }

    #[test]
    fn own_instance_subdir_under_new_layout_is_NOT_flagged() {
        // NEW path layout puts each instance in its own subdir under wt/.
        // An agent probing JUST `wt/<instance>/` (the parent of its assigned worktree)
        // is hitting its own instance dir, not a sibling — must NOT trigger sibling_worktree_access.
        // The kernel sandbox itself denies the read (the more-specific allow only covers the
        // <runid>-<pid> child), but the integrity detector must not double-count the denied
        // attempt as evidence of cross-instance contamination.
        let mut signals: Vec<Signal> = vec![];
        let cmd = "ls -la /private/tmp/monobench-work/wt/php-19591-lexbor-mraw-uaf/ 2>&1 | head -20";
        scan_command(
            &mut signals,
            cmd,
            false,
            "baseline-claude-haiku-r1-t1779978991110", // own_runid
            "php-19591-lexbor-mraw-uaf",               // own_instance_id (new layout)
        );
        assert!(
            !signals.iter().any(|s| s.kind == "sibling_worktree_access"),
            "own-instance subdir probe under NEW layout must NOT flag sibling access, got: {:?}",
            signals
        );

        // Sanity check: a different-instance subdir under NEW layout SHOULD still be flagged.
        let mut signals2: Vec<Signal> = vec![];
        let cmd_sibling = "ls /private/tmp/monobench-work/wt/dotnet-124796-wmiinterop-keepalive/";
        scan_command(
            &mut signals2,
            cmd_sibling,
            false,
            "baseline-claude-haiku-r1-t1779978991110",
            "php-19591-lexbor-mraw-uaf",
        );
        assert!(
            signals2.iter().any(|s| s.kind == "sibling_worktree_access"),
            "cross-instance subdir probe MUST flag sibling access, got: {:?}",
            signals2
        );
    }

    #[test]
    fn empty_own_runid_disables_sibling_check() {
        // When tests or legacy callers pass own_runid="" the detector must NOT fire
        // (avoids false positives in callers that lack the run-id context).
        let mut signals: Vec<Signal> = vec![];
        let cmd =
            "ls /private/tmp/monobench-work/wt/baseline-claude-haiku-r2-t1779934441176-96254/";
        scan_command(&mut signals, cmd, false, "", "");
        assert!(!signals.iter().any(|s| s.kind == "sibling_worktree_access"));
    }

    #[test]
    fn answer_with_php_marker_in_dotnet_instance_is_contaminated() {
        let mut signals: Vec<Signal> = vec![];
        let answer = "Root cause analysis:\nROOTCAUSE: ext/com_dotnet/com_handlers.c::php_com_object_clone\nFIX: keep arg alive.";
        scan_answer_artifact(
            &mut signals,
            answer,
            "baseline-claude-haiku-r1-t1779934411735",
            "dotnet-124796-wmiinterop-keepalive",
        );
        let s = signals
            .iter()
            .find(|s| s.kind == "foreign_repo_marker_in_answer")
            .expect("dotnet instance answering with php marker must be flagged");
        assert!(s.points >= 60);
    }

    #[test]
    fn answer_with_deno_marker_in_node_instance_is_contaminated() {
        let mut signals: Vec<Signal> = vec![];
        let answer = "ROOTCAUSE: ext/node/ops/sqlite/database.rs::prepare\nFIX: tag arc.";
        scan_answer_artifact(
            &mut signals,
            answer,
            "baseline-claude-haiku-r1-t1779688284216",
            "node-56840-statementsync-gc",
        );
        assert!(
            signals
                .iter()
                .any(|s| s.kind == "foreign_repo_marker_in_answer"),
            "node instance answering with deno marker must be flagged, got: {:?}",
            signals
        );
    }

    #[test]
    fn answer_with_netty_marker_in_node_instance_is_contaminated() {
        let mut signals: Vec<Signal> = vec![];
        let answer = "ROOTCAUSE: codec/src/main/java/io/netty/handler/codec/compression/Compressor.java::reset\nFIX: …";
        scan_answer_artifact(
            &mut signals,
            answer,
            "baseline-claude-haiku-r1-t1779951615391",
            "node-62325-zlib-reset-write",
        );
        assert!(
            signals
                .iter()
                .any(|s| s.kind == "foreign_repo_marker_in_answer"),
            "node instance answering with netty marker must be flagged, got: {:?}",
            signals
        );
    }

    #[test]
    fn own_repo_answer_is_clean() {
        let mut signals: Vec<Signal> = vec![];
        // netty instance answering with a netty path — must NOT trigger any foreign signal.
        let answer = "ROOTCAUSE: common/src/main/java/io/netty/util/internal/PlatformDependent.java::directBuffer\nFIX: attach UnsafeMemory.";
        scan_answer_artifact(
            &mut signals,
            answer,
            "baseline-claude-haiku-r1-t1779934411735",
            "netty-12036-unsafe-bytebuffer-uaf",
        );
        assert!(
            !signals
                .iter()
                .any(|s| s.kind.starts_with("foreign_") || s.kind == "cross_instance_answer_path"),
            "own-repo answer must NOT be flagged, got: {:?}",
            signals
        );
    }

    #[test]
    fn cross_instance_wt_path_in_answer_is_contaminated() {
        let mut signals: Vec<Signal> = vec![];
        let answer = "ROOTCAUSE: /private/tmp/monobench-work/wt/baseline-claude-haiku-r2-t1779934441176-96254/Zend/zend.h::zend_compile\nFIX: …";
        scan_answer_artifact(
            &mut signals,
            answer,
            "baseline-claude-haiku-r1-t1779934411735",
            "dotnet-124796-wmiinterop-keepalive",
        );
        let s = signals
            .iter()
            .find(|s| s.kind == "cross_instance_answer_path")
            .expect("answer naming another run's wt path must be flagged");
        assert!(s.points >= 60);
    }
}
