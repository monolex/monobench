// monobench — grading: score a run's ROOTCAUSE answer vs the instance's gold rules.
// Patterns are literal, case-insensitive substrings (the original JS escaped all regex metachars).
use crate::telemetry;
use crate::util::{cmd_has_word, load_jsonl};
use serde_json::Value;

pub struct Inst {
    pub full: Vec<String>,
    pub mech: Vec<String>,
    pub decoy: Vec<String>,
    pub invalid: Option<String>,
}

fn contains_todo(s: &str) -> bool {
    s.to_lowercase().contains("todo")
}

pub fn load_inst(path: &str) -> Inst {
    let v: Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null);
    let g = v.get("grading").cloned().unwrap_or(Value::Null);
    let arr = |k: &str| {
        g.get(k)
            .and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };
    let full: Vec<String> = arr("full_must_name");
    let mech: Vec<String> = arr("mechanism_keywords");
    let decoy: Vec<String> = arr("decoy_markers");
    let mut invalid = None;
    if v.is_null() {
        invalid = Some("instance.json missing or invalid JSON".to_string());
    } else if full.is_empty() || mech.is_empty() {
        invalid =
            Some("instance grading requires full_must_name and mechanism_keywords".to_string());
    } else if full
        .iter()
        .chain(mech.iter())
        .chain(decoy.iter())
        .any(|s| s.trim().is_empty() || contains_todo(s))
    {
        invalid = Some("instance grading still contains TODO/empty patterns".to_string());
    } else {
        for ptr in [
            "/ground_truth/root_cause_file",
            "/ground_truth/root_cause_fn",
            "/ground_truth/fix_summary",
        ] {
            if v.pointer(ptr)
                .and_then(Value::as_str)
                .is_some_and(contains_todo)
            {
                invalid = Some(format!("instance field {ptr} still contains TODO"));
                break;
            }
        }
    }
    Inst {
        full,
        mech,
        decoy,
        invalid,
    }
}

fn has(pats: &[String], text_lc: &str) -> bool {
    pats.iter().any(|p| text_lc.contains(&p.to_lowercase()))
}

pub fn grade_text_str(inst: &Inst, text: &str) -> &'static str {
    if inst.invalid.is_some() {
        return "INVALID";
    }
    if text.trim().is_empty() {
        return "NO_RESULT";
    }
    let t = text.to_lowercase();
    let root_line = rootcause_line(text).to_lowercase();
    let has_root_line = root_line != "(no rootcause line)";
    let root_named = has(&inst.full, &root_line);
    let root_decoy = has(&inst.decoy, &root_line);
    if root_named && has(&inst.mech, &t) {
        "FULL"
    } else if root_named {
        "NAME_ONLY"
    } else if root_decoy {
        "DECOY"
    } else if has_root_line {
        "MISS"
    } else if has(&inst.full, &t) && has(&inst.mech, &t) {
        "FULL"
    } else if has(&inst.full, &t) {
        "NAME_ONLY"
    } else if has(&inst.decoy, &t) {
        "DECOY"
    } else {
        "MISS"
    }
}

pub fn is_monogram_cmd(cmd: &str) -> bool {
    cmd_has_word(cmd, "monogram")
}

#[derive(Clone)]
pub struct RunStats {
    pub label: String,
    pub grade: String,
    pub auto_grade: String,
    pub final_grade: Option<String>,
    pub review_status: String,
    pub final_checked: bool,
    pub cost: f64,
    pub cost_available: bool,
    pub tok: i64,
    pub tokens_available: bool,
    pub calls: Option<i64>, // None only when the runner did not leave parseable telemetry.
    pub adopt: i64,
    pub time: i64,
    pub rootcause: String,
}

impl RunStats {
    pub fn new(
        label: String,
        grade: String,
        cost: f64,
        tok: i64,
        calls: Option<i64>,
        adopt: i64,
        time: i64,
        rootcause: String,
    ) -> Self {
        Self {
            label,
            auto_grade: grade.clone(),
            grade,
            final_grade: None,
            review_status: "unreviewed".into(),
            final_checked: false,
            cost,
            cost_available: true,
            tok,
            tokens_available: true,
            calls,
            adopt,
            time,
            rootcause,
        }
    }
}

// Solvers sometimes emit an absolute worktree path (`/tmp/monobench-work/wt/<run>/src/...`) that
// monobench itself created. Strip that prefix to the repo-relative tail so an absolute-path
// ROOTCAUSE grades identically to a relative one — otherwise the long prefix pushes the named
// symbol past the line-length cap below and a correct answer is graded MISS.
pub fn strip_worktree_prefix(line: &str) -> String {
    for marker in ["/private/tmp/monobench-work/wt/", "/tmp/monobench-work/wt/"] {
        if let Some(pos) = line.find(marker) {
            let after = &line[pos + marker.len()..];
            if let Some(slash) = after.find('/') {
                return format!("{}{}", &line[..pos], &after[slash + 1..]);
            }
        }
    }
    line.to_string()
}

const ROOTCAUSE_LINE_CAP: usize = 240;

fn rootcause_line(t: &str) -> String {
    // Demark markdown bold/code so a claude `**ROOTCAUSE**:` (colon outside the bold) or a
    // backticked `` `ROOTCAUSE:` `` is still recognized as the conclusion line. Otherwise it is
    // missed and grading falls through to imprecise whole-text matching that ignores the
    // root-line precedence (a decoy conclusion could then grade FULL off a body mention).
    let demark = |s: &str| s.replace("**", "").replace('`', "");
    t.lines()
        .find(|l| demark(&l.to_lowercase()).contains("rootcause:"))
        .map(|l| {
            strip_worktree_prefix(&demark(l))
                .chars()
                .take(ROOTCAUSE_LINE_CAP)
                .collect()
        })
        .unwrap_or_else(|| "(no ROOTCAUSE line)".into())
}

pub fn grade_jsonl(inst: &Inst, path: &str) -> RunStats {
    let evs = load_jsonl(path);
    let label = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".jsonl")
        .to_string();
    let (mut calls, mut adopt) = (0i64, 0i64);
    for e in &evs {
        if e.get("type").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        let Some(content) = e.pointer("/message/content").and_then(Value::as_array) else {
            continue;
        };
        for b in content {
            if b.get("type").and_then(Value::as_str) != Some("tool_use") {
                continue;
            }
            calls += 1;
            let name = b.get("name").and_then(Value::as_str).unwrap_or("");
            let cmd = b
                .pointer("/input/command")
                .and_then(Value::as_str)
                .unwrap_or("");
            let nlc = name.to_lowercase();
            if nlc.contains("codegraph")
                || nlc.starts_with("mcp__monogram")
                || (name == "Bash" && is_monogram_cmd(cmd))
            {
                adopt += 1;
            }
        }
    }
    match evs
        .iter()
        .rev()
        .find(|e| e.get("type").and_then(Value::as_str) == Some("result"))
    {
        None => RunStats::new(
            label,
            "NO_RESULT".into(),
            0.0,
            0,
            Some(calls),
            adopt,
            0,
            "(no ROOTCAUSE line)".into(),
        ),
        Some(r) => {
            let gi = |k: &str| {
                r.pointer(&format!("/usage/{k}"))
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
            };
            let tok = gi("input_tokens")
                + gi("cache_read_input_tokens")
                + gi("cache_creation_input_tokens")
                + gi("output_tokens");
            let text = r.get("result").and_then(Value::as_str).unwrap_or("");
            let cost = r
                .get("total_cost_usd")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let time = (r.get("duration_ms").and_then(Value::as_f64).unwrap_or(0.0) / 1000.0)
                .round() as i64;
            let api_error = r.get("is_error").and_then(Value::as_bool) == Some(true)
                || r.get("api_error_status").and_then(Value::as_i64).is_some();
            let grade = if api_error {
                "NO_RESULT"
            } else {
                grade_text_str(inst, text)
            };
            RunStats::new(
                label,
                grade.into(),
                cost,
                tok,
                Some(calls),
                adopt,
                time,
                rootcause_line(text),
            )
        }
    }
}

/// niia/codex runner: grade the answer.txt + read tokens/cost/duration from meter.json.
pub fn grade_text_file(inst: &Inst, answer: &str, meter: &str) -> RunStats {
    let text = std::fs::read_to_string(answer).unwrap_or_default();
    let label = answer
        .rsplit('/')
        .next()
        .unwrap_or(answer)
        .trim_end_matches(".answer.txt")
        .to_string();
    let m: Value = std::fs::read_to_string(meter)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null);
    let prefix = answer.trim_end_matches(".answer.txt");
    let mut events = telemetry::events_from_path(&format!("{prefix}.err"));
    if events.is_empty() {
        events = telemetry::events_from_path(&format!("{prefix}.agy.jsonl"));
    }
    let calls = if events.is_empty() {
        None
    } else {
        Some(events.len() as i64)
    };
    let adopt = telemetry::count_monogram(&events);
    let exit_failed = m
        .get("exit_success")
        .and_then(Value::as_bool)
        .map(|ok| !ok)
        .unwrap_or(false)
        || m.get("exit_status")
            .and_then(Value::as_i64)
            .map(|code| code != 0)
            .unwrap_or(false);
    let legacy_agy_zero_meter = label.contains("-agy-")
        && m.get("cost_available").is_none()
        && m.get("tokens_available").is_none()
        && m.get("cost_usd").and_then(Value::as_f64) == Some(0.0)
        && m.get("tokens").and_then(Value::as_i64) == Some(0);
    let mut stats = RunStats::new(
        label,
        if exit_failed {
            "NO_RESULT".into()
        } else {
            grade_text_str(inst, &text).into()
        },
        m.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0),
        m.get("tokens").and_then(Value::as_i64).unwrap_or(0),
        calls,
        adopt,
        m.get("duration_s").and_then(Value::as_i64).unwrap_or(0),
        if exit_failed {
            m.get("runner_error")
                .and_then(Value::as_str)
                .map(|s| format!("({s})"))
                .unwrap_or_else(|| "(runner exited non-zero)".into())
        } else {
            rootcause_line(&text)
        },
    );
    stats.cost_available = !legacy_agy_zero_meter
        && m.get("cost_available")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| m.get("cost_usd").map(|v| !v.is_null()).unwrap_or(false));
    stats.tokens_available = !legacy_agy_zero_meter
        && m.get("tokens_available")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| m.get("tokens").map(|v| !v.is_null()).unwrap_or(false));
    stats
}

pub fn print_grade(s: &RunStats) {
    let calls = s.calls.map(|c| c.to_string()).unwrap_or_else(|| "—".into());
    println!("\n── {} ──", s.label);
    println!(
        "grade={}  auto={}  review={}  checked={}  cost={}  tokens={}  time={}s  toolcalls={}  tool-adoption={}",
        s.grade,
        s.auto_grade,
        s.review_status,
        if s.final_checked { "yes" } else { "no" },
        if s.cost_available {
            format!("${:.2}", s.cost)
        } else {
            "—".into()
        },
        if s.tokens_available {
            s.tok.to_string()
        } else {
            "—".into()
        },
        s.time,
        calls,
        s.adopt
    );
    if let Some(final_grade) = &s.final_grade {
        println!("  final_grade={final_grade}");
    }
    println!("  {}", s.rootcause);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_error_jsonl_is_no_result() {
        let p =
            std::env::temp_dir().join(format!("monobench-api-error-{}.jsonl", std::process::id()));
        std::fs::write(
            &p,
            r#"{"type":"result","is_error":true,"api_error_status":403,"duration_ms":842,"result":"Your organization has disabled Claude subscription access for Claude Code","total_cost_usd":0,"usage":{"input_tokens":0,"cache_read_input_tokens":0,"cache_creation_input_tokens":0,"output_tokens":0}}"#,
        ).unwrap();
        let inst = Inst {
            full: vec!["disabled Claude subscription".into()],
            mech: vec!["Claude Code".into()],
            decoy: vec![],
            invalid: None,
        };
        let stats = grade_jsonl(&inst, &p.to_string_lossy());
        assert_eq!(stats.grade, "NO_RESULT");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn provisional_todo_instance_is_invalid_not_miss() {
        let inst = Inst {
            full: vec!["TODO".into()],
            mech: vec!["TODO".into()],
            decoy: vec!["TODO".into()],
            invalid: Some("instance grading still contains TODO/empty patterns".into()),
        };
        let answer =
            "ROOTCAUSE: Modules/itertoolsmodule.c::_grouper_next\nFIX: incref before compare";
        assert_eq!(grade_text_str(&inst, answer), "INVALID");
    }

    #[test]
    fn rootcause_line_dominates_body_mentions() {
        let inst = Inst {
            full: vec!["BunString__toThreadSafe".into()],
            mech: vec!["isolatedCopy".into(), "leakRef".into()],
            decoy: vec!["toCrossThreadShareable".into()],
            invalid: None,
        };
        let answer = "BunString__toThreadSafe calls isolatedCopy and leakRef.\n\
ROOTCAUSE: src/string.zig::String.toThreadSafe\n\
FIX: release the old ref";
        assert_eq!(grade_text_str(&inst, answer), "MISS");

        let decoy = "BunString__toThreadSafe calls isolatedCopy and leakRef.\n\
ROOTCAUSE: src/bun.js/bindings/BunString.cpp::toCrossThreadShareable\n\
FIX: release the old ref";
        assert_eq!(grade_text_str(&inst, decoy), "DECOY");
    }

    #[test]
    fn absolute_worktree_path_grades_like_relative() {
        let inst = Inst {
            full: vec!["runOutputSink".into(), "BufferOutputSink".into()],
            mech: vec!["use-after-free".into()],
            decoy: vec![],
            invalid: None,
        };
        let rel = "ROOTCAUSE: src/bun.js/api/html_rewriter.zig::BufferOutputSink.runOutputSink\nmechanism: use-after-free";
        // The absolute worktree prefix would push the symbol past the 92-char cap and MISS.
        let abs = "ROOTCAUSE: /tmp/monobench-work/wt/baseline-gpt-5.4-mini-low-r2-49663/src/bun.js/api/html_rewriter.zig::BufferOutputSink.runOutputSink\nmechanism: use-after-free";
        let abs_private = "ROOTCAUSE: /private/tmp/monobench-work/wt/baseline-haiku-r1-123/src/bun.js/api/html_rewriter.zig::BufferOutputSink.runOutputSink\nmechanism: use-after-free";
        assert_eq!(grade_text_str(&inst, rel), "FULL");
        assert_eq!(grade_text_str(&inst, abs), "FULL");
        assert_eq!(grade_text_str(&inst, abs_private), "FULL");
    }

    #[test]
    fn long_cpp_rootcause_line_keeps_function_name() {
        let inst = Inst {
            full: vec!["jsWorkerPrototypeFunction_getHeapSnapshotBody".into()],
            mech: vec!["Strong".into(), "worker thread".into(), "HandleSet".into()],
            decoy: vec!["visitStrongHandles".into()],
            invalid: None,
        };
        let answer = "The bug copies a Strong by value into a worker thread and mutates the parent HandleSet.\n\
ROOTCAUSE: src/bun.js/bindings/webcore/JSWorker.cpp::jsWorkerPrototypeFunction_getHeapSnapshotBody\n\
FIX: keep the Strong parent-thread-owned";
        assert_eq!(grade_text_str(&inst, answer), "FULL");
    }

    #[test]
    fn ghostty_split_tree_requires_rebuild_owner_not_file_only() {
        let inst = Inst {
            full: vec!["propTree".into(), "onRebuild".into()],
            mech: vec!["clear".into(), "rebuild".into(), "blank frame".into()],
            decoy: vec!["setTree".into(), "boxedCopy".into(), "deep clone".into()],
            invalid: None,
        };
        let full = "The split tree clear-then-rebuild path leaves tree_bin empty for one frame.\n\
ROOTCAUSE: src/apprt/gtk-ng/class/split_tree.zig::propTree\n\
FIX: debounce rebuild and swap atomically";
        assert_eq!(grade_text_str(&inst, full), "FULL");

        let file_only = "The split tree code clears before rebuild, causing a blank frame.\n\
ROOTCAUSE: src/apprt/gtk-ng/class/split_tree.zig\n\
FIX: debounce rebuild";
        assert_eq!(grade_text_str(&inst, file_only), "MISS");

        let decoy =
            "The flicker comes from boxed ownership and should be fixed with a deep clone.\n\
ROOTCAUSE: src/apprt/gtk-ng/class/split_tree.zig::setTree\n\
FIX: deep clone the tree";
        assert_eq!(grade_text_str(&inst, decoy), "DECOY");
    }

    #[test]
    fn markdown_wrapped_rootcause_keeps_line_precedence() {
        let inst = Inst {
            full: vec!["BunString__toThreadSafe".into()],
            mech: vec!["isolatedCopy".into()],
            decoy: vec!["toCrossThreadShareable".into()],
            invalid: None,
        };
        // `**ROOTCAUSE**:` (colon outside the bold) must still be read as the conclusion line.
        let full =
            "body notes isolatedCopy\n**ROOTCAUSE**: src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe";
        assert_eq!(grade_text_str(&inst, full), "FULL");
        // Decoy CONCLUSION while the real symbol only appears in the body must grade DECOY, not
        // FULL via the whole-text fallback — proves root-line precedence survives markdown.
        let decoy =
            "body mentions BunString__toThreadSafe and isolatedCopy\n**ROOTCAUSE**: src/bun.js/bindings/BunString.cpp::toCrossThreadShareable";
        assert_eq!(grade_text_str(&inst, decoy), "DECOY");
    }
}
