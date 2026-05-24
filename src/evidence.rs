// monobench — focused evidence search inside one recorded run.
// Replaces ad hoc `rg results/<id>/<run>.err ...` with run-label-aware lookup.
use crate::telemetry;
use crate::util::{cmd_has_word, fit_middle};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const DEFAULT_PATTERN: &str =
    "ROOTCAUSE|SQLite failure|database is locked|wrong DB|indexlock|monogram index|kill ";

#[derive(Clone, Debug)]
struct PatternSet {
    raw: String,
    case_sensitive: bool,
    terms: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Summary {
    pub run: String,
    pub grade: String,
    pub tool_matches: usize,
    pub ans_matches: usize,
    pub raw_matches: usize,
    pub notable: usize,
    pub first: String,
}

pub fn summarize(
    run: &str,
    grade: &str,
    source: Option<&Path>,
    answer_text: &str,
    index_log: &Path,
    pattern: Option<&str>,
    case_sensitive: bool,
    include_prompt: bool,
) -> Summary {
    let patterns = PatternSet::new(pattern, case_sensitive);
    let events = source
        .map(|p| telemetry::events_from_path(&p.to_string_lossy()))
        .unwrap_or_default();
    let mut tool_matches = 0usize;
    let mut notable = 0usize;
    let mut tool_first = String::new();
    for (idx, ev) in events.iter().enumerate() {
        let monogram_state = invokes_monogram(&ev.cmd)
            && matches!(
                monogram_sub(&ev.cmd).as_deref(),
                Some("index" | "reindex" | "prune" | "clean" | "init")
            );
        if monogram_state || notable_kind(&ev.cmd).is_some() {
            notable += 1;
        }
        if patterns.matches(&ev.cmd) || patterns.matches(&ev.result) {
            tool_matches += 1;
            if tool_first.is_empty() {
                tool_first = format!("#{} {}", idx + 1, ev.cmd.replace('\n', " "));
            }
        }
    }
    // Answer/conclusion evidence is kept separate from process-log noise: when scanning
    // many runs the question is "which run concluded, and what did it say?". `answer_text` is
    // pre-extracted by the caller so it works for claude runs (answer lives in the .jsonl).
    let answer_hits: Vec<&str> = answer_text
        .lines()
        .filter(|l| patterns.matches(l))
        .collect();
    let ans_matches = answer_hits.len();
    // Prefer the definitive ROOTCAUSE verdict among matching answer lines over an
    // earlier mid-reasoning sentence that happens to mention the same term.
    // Normalize monobench's own worktree path prefix (same as grading) so the conclusion column
    // groups by eye: `src/…::sym` and `/tmp/monobench-work/wt/…/src/…::sym` are the same answer.
    let answer_first = answer_hits
        .iter()
        .find(|l| l.contains("ROOTCAUSE"))
        .or_else(|| answer_hits.first())
        .map(|l| clean_md_emphasis(&crate::grade::strip_worktree_prefix(l.trim())));
    // raw = source log + index log only (the answer is counted as `ans`, not raw).
    let mut files: Vec<(&str, PathBuf)> = vec![];
    if let Some(p) = source {
        files.push(("source", p.to_path_buf()));
    }
    if index_log.is_file() {
        files.push(("index", index_log.to_path_buf()));
    }
    let raw_matches = count_raw_matches(&files, &patterns, include_prompt);
    let first = answer_first
        .filter(|s| !s.is_empty())
        .or_else(|| (!tool_first.is_empty()).then(|| tool_first.clone()))
        .unwrap_or_else(|| {
            if raw_matches > 0 {
                "raw-line match".into()
            } else {
                String::new()
            }
        });
    Summary {
        run: run.into(),
        grade: grade.into(),
        tool_matches,
        ans_matches,
        raw_matches,
        notable,
        first,
    }
}

pub fn print_index(id: &str, pattern: Option<&str>, max: usize, summaries: &[Summary]) {
    let pat = PatternSet::new(pattern, false).raw;
    println!("EVIDENCE INDEX  {id}");
    println!("  pattern: {pat}");
    println!("  columns: ans=answer/conclusion hits  tool=matching tool calls  raw=source+index log hits  notable=state/process calls");
    let mut rows = summaries.to_vec();
    // Conclusion-bearing runs rank first, then total textual evidence, then state/process
    // calls as a tiebreaker. Contamination is surfaced (notable) but never inflates rank.
    rows.sort_by(|a, b| {
        b.ans_matches
            .cmp(&a.ans_matches)
            .then_with(|| (b.tool_matches + b.raw_matches).cmp(&(a.tool_matches + a.raw_matches)))
            .then_with(|| b.notable.cmp(&a.notable))
            .then_with(|| a.run.cmp(&b.run))
    });
    println!("\nRUNS");
    for s in rows.iter().filter(|s| row_qualifies(s)).take(max) {
        println!(
            "  {:<60} grade={:<9} ans={:<3} tool={:<3} raw={:<3} notable={:<2} {}",
            fit_middle(&s.run, 60),
            s.grade,
            s.ans_matches,
            s.tool_matches,
            s.raw_matches,
            s.notable,
            fit_middle(&s.first, 90)
        );
    }
    let hits = rows.iter().filter(|s| row_qualifies(s)).count();
    if hits == 0 {
        // Distinguish "empty instance" from "runs exist but none matched" (e.g. crashed runs
        // with telemetry but no answer): show the scanned count and where to look next.
        println!(
            "  (no run matched the pattern — scanned {} run(s); try a broader --pattern, or `monobench status {}` / `monobench integrity {}`)",
            summaries.len(),
            id,
            id
        );
    } else if hits > max {
        println!(
            "  ... {} more matching runs (raise --max or narrow with --pattern)",
            hits - max
        );
    }
    println!("\n[NEXT]");
    println!("  monobench evidence {id} <run> --pattern '<same-pattern>'   # drill into one run");
    println!(
        "  monobench integrity {id}                                  # contamination-risk view"
    );
}

// A run qualifies for the index only on actual pattern hits. `notable` (state/process calls) is
// pattern-independent, so treating it as a match would leak `integrity`'s contamination view into
// a pattern search and surface runs for a pattern they never matched.
fn row_qualifies(s: &Summary) -> bool {
    s.ans_matches > 0 || s.tool_matches > 0 || s.raw_matches > 0
}

// Strip markdown bold (`**`) and code (`` ` ``) decoration for the one-line conclusion column so
// claude's `**ROOTCAUSE:**` reads as plain `ROOTCAUSE:`. Single `*` is preserved (it appears in
// legitimate code like `*str` and glob patterns).
fn clean_md_emphasis(s: &str) -> String {
    s.replace("**", "").replace('`', "").trim().to_string()
}

impl PatternSet {
    fn new(pattern: Option<&str>, case_sensitive: bool) -> Self {
        let raw = pattern
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(DEFAULT_PATTERN)
            .to_string();
        let terms = raw
            .split('|')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        Self {
            raw,
            case_sensitive,
            terms,
        }
    }

    fn matches(&self, text: &str) -> bool {
        self.terms
            .iter()
            .any(|term| term_matches(term, text, self.case_sensitive))
    }
}

pub fn print_evidence(
    id: &str,
    run: &str,
    grade: &str,
    source: &Path,
    answer_text: &str,
    index_log: &Path,
    pattern: Option<&str>,
    context: usize,
    max: usize,
    case_sensitive: bool,
    include_prompt: bool,
) {
    let patterns = PatternSet::new(pattern, case_sensitive);
    println!("EVIDENCE  {id}");
    println!("  run: {run}");
    println!("  grade: {grade}");
    println!("  source: {}", source.display());
    println!("  pattern: {}", patterns.raw);

    let events = telemetry::events_from_path(&source.to_string_lossy());
    print_tool_matches(&events, &patterns, max);

    // The answer is passed as already-extracted text (claude runs keep it inside the .jsonl
    // transcript, not a .answer.txt file), so raw matching works on in-memory sections.
    let source_is_jsonl = source.extension().and_then(|s| s.to_str()) == Some("jsonl");
    let mut sections: Vec<(&str, String)> = vec![(
        "source",
        std::fs::read_to_string(source).unwrap_or_default(),
    )];
    if !answer_text.trim().is_empty() {
        sections.push(("answer", answer_text.to_string()));
    }
    if index_log.is_file() {
        sections.push((
            "index",
            std::fs::read_to_string(index_log).unwrap_or_default(),
        ));
    }
    print_raw_matches(
        &sections,
        source_is_jsonl,
        &patterns,
        context,
        max,
        include_prompt,
    );

    println!("\n[NEXT]");
    println!("  monobench trace {id} {run} 80");
    println!("  monobench export {id} {run}");
    println!("  monobench integrity {id} {run}");
}

fn print_tool_matches(events: &[telemetry::ToolEvent], patterns: &PatternSet, max: usize) {
    let mut shown = 0usize;
    let mut subcommands: BTreeMap<String, usize> = BTreeMap::new();
    let mut notable: Vec<(usize, &'static str, String)> = vec![];
    println!("\nTOOL-CALL MATCHES");
    for (idx, ev) in events.iter().enumerate() {
        if invokes_monogram(&ev.cmd) {
            let sub = monogram_sub(&ev.cmd).unwrap_or_else(|| "help".into());
            if matches!(
                sub.as_str(),
                "index" | "reindex" | "prune" | "clean" | "init"
            ) {
                notable.push((idx + 1, "monogram_state", ev.cmd.clone()));
            }
            *subcommands.entry(sub).or_default() += 1;
        }
        if let Some(kind) = notable_kind(&ev.cmd) {
            notable.push((idx + 1, kind, ev.cmd.clone()));
        }
        let cmd_hit = patterns.matches(&ev.cmd);
        let result_hit = patterns.matches(&ev.result);
        if !cmd_hit && !result_hit {
            continue;
        }
        shown += 1;
        if shown > max {
            println!("  ... truncated at {max} matches");
            break;
        }
        let tag = if invokes_monogram(&ev.cmd) {
            "[M]"
        } else if ev.name == "Grep"
            || ev.name == "Glob"
            || ["grep", "egrep", "rg", "find", "fd", "ag", "ack"]
                .iter()
                .any(|w| cmd_has_word(&ev.cmd, w))
        {
            "[g]"
        } else if cmd_has_word(&ev.cmd, "git") {
            "git"
        } else {
            "   "
        };
        println!(
            "  {:>3}. {} {}",
            idx + 1,
            tag,
            fit_middle(&ev.cmd.replace('\n', " "), 140)
        );
        if ev.denied {
            println!("       denied");
        }
        if result_hit {
            if let Some(line) = first_matching_line(&ev.result, patterns) {
                println!("       result: {}", fit_middle(line.trim(), 150));
            }
        }
    }
    if shown == 0 {
        println!("  (no matching tool calls)");
    }
    if !notable.is_empty() {
        println!("\nNOTABLE STATE/PROCESS CALLS");
        notable.sort_by_key(|(i, _, _)| *i);
        notable.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1 && a.2 == b.2);
        for (i, kind, cmd) in notable.iter().take(16) {
            println!("  {:>3}. {:<16} {}", i, kind, fit_middle(cmd, 140));
        }
    }
    if !subcommands.is_empty() {
        let pairs = subcommands
            .iter()
            .map(|(k, v)| format!("{k}:{v}"))
            .collect::<Vec<_>>()
            .join(" ");
        println!("  monogram-subcommands: {pairs}");
    }
}

fn notable_kind(cmd: &str) -> Option<&'static str> {
    if cmd_has_word(cmd, "kill") || cmd_has_word(cmd, "pkill") || cmd_has_word(cmd, "killall") {
        return Some("process_kill");
    }
    if cmd_has_word(cmd, "sqlite3")
        || cmd.contains("indexlock")
        || cmd.contains(".registry")
        || cmd.contains("monogram.db")
    {
        return Some("db_or_lock_probe");
    }
    None
}

fn print_raw_matches(
    sections: &[(&str, String)],
    source_is_jsonl: bool,
    patterns: &PatternSet,
    context: usize,
    max: usize,
    include_prompt: bool,
) {
    println!("\nRAW LINE MATCHES");
    let mut shown = 0usize;
    for (label, text) in sections {
        if *label == "source" && !include_prompt && source_is_jsonl {
            println!(
                "  source: structured JSONL raw lines suppressed (tool-call matches above; use --include-prompt)"
            );
            continue;
        }
        let lines: Vec<&str> = text.lines().collect();
        let start_at = if *label == "source" && !include_prompt {
            source_log_start(&lines)
        } else {
            0
        };
        if *label == "source" && start_at > 0 {
            println!(
                "  source: skipped prompt/header lines 1..{} (use --include-prompt)",
                start_at
            );
        }
        for (idx, line) in lines.iter().enumerate().skip(start_at) {
            if !patterns.matches(line) {
                continue;
            }
            shown += 1;
            if shown > max {
                println!("  ... truncated at {max} matches");
                return;
            }
            if context > 0 {
                let start = idx.saturating_sub(context);
                let end = (idx + context + 1).min(lines.len());
                for (i, ctx_line) in lines[start..end].iter().enumerate() {
                    let line_no = start + i + 1;
                    let mark = if line_no == idx + 1 { ">" } else { "-" };
                    println!(
                        "  {label}:{}:{mark} {}",
                        line_no,
                        fit_middle(ctx_line.trim(), 170)
                    );
                }
            } else {
                println!("  {label}:{}: {}", idx + 1, fit_middle(line.trim(), 170));
            }
        }
    }
    if shown == 0 {
        println!("  (no raw line matches)");
    }
}

fn count_raw_matches(
    files: &[(&str, PathBuf)],
    patterns: &PatternSet,
    include_prompt: bool,
) -> usize {
    let mut count = 0usize;
    for (label, path) in files {
        if *label == "source"
            && !include_prompt
            && path.extension().and_then(|s| s.to_str()) == Some("jsonl")
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let lines: Vec<&str> = text.lines().collect();
        let start_at = if *label == "source" && !include_prompt {
            source_log_start(&lines)
        } else {
            0
        };
        count += lines
            .iter()
            .skip(start_at)
            .filter(|line| patterns.matches(line))
            .count();
    }
    count
}

fn source_log_start(lines: &[&str]) -> usize {
    lines
        .iter()
        .position(|line| {
            let t = line.trim_start();
            t.starts_with("/bin/zsh -lc")
                || t.contains("\"type\":\"PLANNER_RESPONSE\"")
                || t.contains("\"type\": \"PLANNER_RESPONSE\"")
        })
        .unwrap_or(0)
}

fn term_matches(term: &str, text: &str, case_sensitive: bool) -> bool {
    let starts = term.strip_prefix('^');
    if case_sensitive {
        if let Some(prefix) = starts {
            text.trim_start().starts_with(prefix)
        } else {
            text.contains(term)
        }
    } else {
        let hay = text.to_lowercase();
        if let Some(prefix) = starts {
            hay.trim_start().starts_with(&prefix.to_lowercase())
        } else {
            hay.contains(&term.to_lowercase())
        }
    }
}

fn first_matching_line<'a>(text: &'a str, patterns: &PatternSet) -> Option<&'a str> {
    text.lines().find(|line| patterns.matches(line))
}

fn monogram_sub(cmd: &str) -> Option<String> {
    let idx = monogram_exec_pos(cmd)?;
    let tok = cmd[idx + 8..].split_whitespace().next().unwrap_or("");
    Some(if tok.is_empty() {
        "help".into()
    } else {
        tok.trim_matches(['"', '\'']).to_string()
    })
}

fn invokes_monogram(cmd: &str) -> bool {
    monogram_exec_pos(cmd).is_some()
}

fn monogram_exec_pos(cmd: &str) -> Option<usize> {
    let mut start = 0usize;
    while let Some(rel) = crate::util::cmd_word_pos(&cmd[start..], "monogram") {
        let idx = start + rel;
        let prev_non_space = cmd[..idx].bytes().rev().find(|b| !b.is_ascii_whitespace());
        if prev_non_space
            .map(|b| matches!(b, b';' | b'|' | b'&' | b'('))
            .unwrap_or(true)
        {
            return Some(idx);
        }
        start = idx + "monogram".len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{term_matches, PatternSet};

    #[test]
    fn pattern_pipe_is_or() {
        let p = PatternSet::new(Some("ROOTCAUSE|inline void ref"), false);
        assert!(p.matches("xx rootcause: yes"));
        assert!(p.matches("static inline void ref()"));
        assert!(!p.matches("nothing"));
    }

    #[test]
    fn caret_means_line_start() {
        assert!(term_matches(
            "^/bin/zsh -lc",
            "/bin/zsh -lc monogram",
            false
        ));
        assert!(term_matches(
            "^/bin/zsh -lc",
            "   /bin/zsh -lc monogram",
            false
        ));
        assert!(!term_matches(
            "^/bin/zsh -lc",
            "prefix /bin/zsh -lc monogram",
            false
        ));
    }

    #[test]
    fn monogram_must_be_command_position() {
        assert!(super::invokes_monogram("monogram search x"));
        assert!(super::invokes_monogram("cd repo && monogram search x"));
        assert!(!super::invokes_monogram("ps -ef | grep monogram"));
    }

    #[test]
    fn summarize_splits_answer_from_raw() {
        let dir = std::env::temp_dir().join(format!("monobench-ev-split-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let index = dir.join("a.index.log");
        std::fs::write(&index, "monogram index .\nunrelated line\n").unwrap();
        let s = super::summarize(
            "a",
            "FULL",
            None,
            "intro\nROOTCAUSE: src/foo.rs::bar\ntail\n",
            &index,
            Some("ROOTCAUSE|monogram index"),
            false,
            false,
        );
        // answer hits land in `ans`, index-log hits land in `raw`, no source = no tool calls.
        assert_eq!(s.ans_matches, 1);
        assert_eq!(s.raw_matches, 1);
        assert_eq!(s.tool_matches, 0);
        // `first` prefers the answer conclusion over a process-log line.
        assert_eq!(s.first, "ROOTCAUSE: src/foo.rs::bar");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn summarize_counts_monogram_state_calls_as_notable() {
        let dir = std::env::temp_dir().join(format!("monobench-ev-state-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("a.agy.jsonl");
        std::fs::write(
            &source,
            r#"{"type":"PLANNER_RESPONSE","tool_calls":[{"name":"run_command","args":{"CommandLine":"monogram reindex ."}}]}"#
                .to_string()
                + "\n"
                + r#"{"type":"RUN_COMMAND","content":"ok"}"#
                + "\n",
        )
        .unwrap();
        let index = dir.join("a.index.log");
        let s = super::summarize(
            "a",
            "MISS",
            Some(&source),
            "",
            &index,
            Some("monogram reindex"),
            false,
            false,
        );
        assert_eq!(s.tool_matches, 1);
        assert_eq!(s.notable, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn notable_only_run_does_not_qualify() {
        // A run with state/process calls but zero pattern hits must not appear in the index;
        // contamination is integrity's job, not an evidence-pattern match.
        let notable_only = super::Summary {
            run: "r".into(),
            grade: "MISS".into(),
            tool_matches: 0,
            ans_matches: 0,
            raw_matches: 0,
            notable: 13,
            first: String::new(),
        };
        assert!(!super::row_qualifies(&notable_only));
        let matched = super::Summary {
            ans_matches: 1,
            ..notable_only
        };
        assert!(super::row_qualifies(&matched));
    }

    #[test]
    fn clean_md_emphasis_strips_bold_and_code_keeps_single_star() {
        assert_eq!(
            super::clean_md_emphasis("**ROOTCAUSE:** `src/foo.cpp`::bar"),
            "ROOTCAUSE: src/foo.cpp::bar"
        );
        // single `*` (pointer/glob) is preserved
        assert_eq!(super::clean_md_emphasis("free(*str)"), "free(*str)");
    }
}
