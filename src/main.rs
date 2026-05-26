// monobench — code-intelligence-tool benchmark (mono-series CLI, Rust core).
// Management, analysis, run, and matrix orchestration are native Rust.
mod adoption;
mod evidence;
mod export;
mod grade;
mod integrity;
mod meter;
mod monogram_audit;
mod niia_runner;
mod report;
mod review;
mod run;
mod run_meta;
mod telemetry;
mod trace;
mod util;
use grade::{grade_jsonl, grade_text_file, load_inst, print_grade, RunStats};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}

fn arg_any_value(args: &[String], names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| arg_value(args, name))
}

fn flag_takes_value(flag: &str) -> bool {
    matches!(
        flag,
        "--cli"
            | "--model"
            | "--models"
            | "--via"
            | "--effort"
            | "--tools"
            | "--runs"
            | "--jobs"
            | "--isolate"
            | "--tag"
            | "--batch"
            | "--note"
            | "--memo"
            | "--pattern"
            | "--context"
            | "--max"
            | "--tail"
            | "--final"
            | "--reason"
            | "--judge-model"
            | "--status"
    )
}

fn run_number_and_positional_note(args: &[String], start: usize) -> (usize, Option<String>) {
    let mut n = 1usize;
    let mut words = vec![];
    let mut i = start;
    while i < args.len() {
        let s = &args[i];
        if s.starts_with("--") {
            i += if flag_takes_value(s) { 2 } else { 1 };
            continue;
        }
        if let Ok(x) = s.parse::<usize>() {
            n = x;
        } else {
            words.push(s.clone());
        }
        i += 1;
    }
    let note = (!words.is_empty()).then(|| words.join(" "));
    (n, note)
}

fn positional_note(args: &[String], start: usize) -> Option<String> {
    let mut words = vec![];
    let mut i = start;
    while i < args.len() {
        let s = &args[i];
        if s.starts_with("--") {
            i += if flag_takes_value(s) { 2 } else { 1 };
            continue;
        }
        words.push(s.clone());
        i += 1;
    }
    (!words.is_empty()).then(|| words.join(" "))
}

fn first_word(s: &str) -> Option<String> {
    s.split_whitespace().next().map(str::to_string)
}

fn axes_for(model: &str, cli_arg: Option<String>, via_arg: Option<String>) -> (String, String) {
    let legacy_runner = std::env::var("MONOBENCH_RUNNER").ok();
    let cli = cli_arg
        .or_else(|| std::env::var("MONOBENCH_CLI_NAME").ok())
        .or_else(|| match legacy_runner.as_deref() {
            Some("claude-p") => Some("claude".into()),
            Some("codex") => Some("codex".into()),
            Some("agy") => Some("agy".into()),
            Some("niia") => std::env::var("MONOBENCH_CLI")
                .ok()
                .and_then(|s| first_word(&s))
                .or_else(|| Some(util::default_cli_for_model(model))),
            _ => None,
        })
        .unwrap_or_else(|| util::default_cli_for_model(model))
        .to_lowercase();
    let via = via_arg
        .or_else(|| std::env::var("MONOBENCH_VIA").ok())
        .or_else(|| match legacy_runner.as_deref() {
            Some("niia") => Some("niia".into()),
            _ => None,
        })
        .unwrap_or_else(|| "direct".into())
        .to_lowercase();
    (cli, via)
}

fn find_root() -> PathBuf {
    if let Ok(r) = std::env::var("MONOBENCH_ROOT") {
        return PathBuf::from(r);
    }
    let mut cands: Vec<PathBuf> = vec![];
    if let Ok(exe) = std::env::current_exe() {
        let mut p = exe.parent().map(Path::to_path_buf);
        for _ in 0..4 {
            if let Some(pp) = p {
                cands.push(pp.clone());
                p = pp.parent().map(Path::to_path_buf);
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        cands.push(cwd);
    }
    for c in &cands {
        if is_monobench_root(c) {
            remember_root(c);
            return c.clone();
        }
    }
    if let Some(root) = infer_root_from_active_processes() {
        remember_root(&root);
        return root;
    }
    if let Some(root) = remembered_root() {
        return root;
    }
    // No on-disk problem set (installed / standalone binary): use the build-time-embedded set,
    // extracted once to a writable cache. Dev runs from the repo never reach here — the on-disk
    // search above wins, so the live files are always used in-tree.
    if let Some(root) = embedded::extract_root() {
        return root;
    }
    std::env::current_dir().unwrap_or_default()
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

fn is_monobench_root(path: &Path) -> bool {
    path.join("harness").is_dir() && path.join("instances").is_dir()
}

fn root_memory_file() -> Option<PathBuf> {
    Some(home_dir()?.join(".monobench").join("root"))
}

fn remember_root(root: &Path) {
    let Some(path) = root_memory_file() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, root.to_string_lossy().as_bytes());
}

fn remembered_root() -> Option<PathBuf> {
    let path = root_memory_file()?;
    let root = PathBuf::from(std::fs::read_to_string(path).ok()?.trim());
    is_monobench_root(&root).then_some(root)
}

fn root_from_results_path(cmd: &str) -> Option<PathBuf> {
    let pos = cmd.find("/results/")?;
    let prefix = &cmd[..pos];
    let start = prefix
        .rfind(" /")
        .map(|i| i + 1)
        .or_else(|| prefix.rfind("=/").map(|i| i + 1))
        .unwrap_or(0);
    let root = prefix[start..].trim_matches(['"', '\'']);
    root.starts_with('/').then(|| PathBuf::from(root))
}

fn infer_root_from_active_processes() -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = ps_rows()
        .iter()
        .filter_map(|p| root_from_results_path(&p.cmd))
        .filter(|p| is_monobench_root(p))
        .collect();
    roots.sort();
    roots.dedup();
    roots.into_iter().next()
}

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
    use std::path::PathBuf;
    /// Extract the embedded problem set to ~/.monobench/<ver>-<build_id> once; return it as the root.
    pub fn extract_root() -> Option<PathBuf> {
        if EMBEDDED.is_empty() {
            return None;
        }
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()?;
        let root = PathBuf::from(home).join(".monobench").join(format!(
            "{}-{}",
            env!("CARGO_PKG_VERSION"),
            BUILD_ID
        ));
        let marker = root.join(".extracted");
        if !marker.exists() {
            for (rel, bytes) in EMBEDDED {
                let dst = root.join(rel);
                if let Some(p) = dst.parent() {
                    let _ = std::fs::create_dir_all(p);
                }
                let _ = std::fs::write(&dst, bytes);
            }
            let _ = std::fs::create_dir_all(&root);
            let _ = std::fs::write(&marker, b"1");
        }
        if root.join("harness").is_dir() && root.join("instances").is_dir() {
            Some(root)
        } else {
            None
        }
    }
}

fn read_json(path: &Path) -> serde_json::Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::Value::Null)
}

fn list_dir_names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn live_loop(args: &[String]) -> ! {
    let every = arg_value(args, "--every")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(2)
        .max(1);
    let count = arg_value(args, "--count")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    let mut child_args = vec![];
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--live" => i += 1,
            "--every" | "--count" => i += 2,
            _ => {
                child_args.push(args[i].clone());
                i += 1;
            }
        }
    }
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("monobench"));
    let mut n = 0usize;
    loop {
        if n >= count {
            std::process::exit(0);
        }
        print!("\x1b[2J\x1b[H");
        println!(
            "monobench live · {} · refresh #{} · every {}s · Ctrl-C to stop\n",
            child_args.join(" "),
            n + 1,
            every
        );
        let out = Command::new(&exe).args(&child_args).output();
        match out {
            Ok(o) => {
                print!("{}", String::from_utf8_lossy(&o.stdout));
                eprint!("{}", String::from_utf8_lossy(&o.stderr));
            }
            Err(e) => eprintln!("monobench live failed: {e}"),
        }
        let _ = std::io::stdout().flush();
        n += 1;
        if n < count {
            std::thread::sleep(Duration::from_secs(every));
        }
    }
}

fn hum_bytes(n: u64) -> String {
    if n >= 1_048_576 {
        format!("{:.1}MB", n as f64 / 1_048_576.0)
    } else if n >= 1024 {
        format!("{:.1}KB", n as f64 / 1024.0)
    } else {
        format!("{n}B")
    }
}

const IDLE_WARN_SECS: u64 = 300; // no new output for 5 min ⇒ likely hung (generous vs reasoning pauses)

// Seconds since this run last wrote a STREAMING log (.err/.jsonl/.agy.jsonl/.agy.log). This is the
// real "alive but hung" signal — unlike CPU%, which sits near 0 while an API model reasons server-side.
fn live_output_age(dir: &Path, stem: &str) -> Option<u64> {
    [".err", ".jsonl", ".agy.jsonl", ".agy.log"]
        .iter()
        .filter_map(|sfx| {
            std::fs::metadata(dir.join(format!("{stem}{sfx}")))
                .ok()?
                .modified()
                .ok()?
                .elapsed()
                .ok()
                .map(|d| d.as_secs())
        })
        .min()
}

// Pure formatter (testable): "· idle Ns", or "· ⚠ idle Ns" past the warn threshold, "" if no output yet.
fn idle_label(age: Option<u64>) -> String {
    match age {
        Some(a) if a >= IDLE_WARN_SECS => format!(" · ⚠ idle {a}s"),
        Some(a) => format!(" · idle {a}s"),
        None => String::new(),
    }
}

// Concise stall indicator for an in-flight run.
fn idle_tag(dir: &Path, stem: &str) -> String {
    idle_label(live_output_age(dir, stem))
}

// A `.running` marker records `pid=<N>`; its RAII guard removes it on normal exit AND on panic, so a
// LINGERING marker whose pid is gone means the run was hard-killed (SIGKILL/OOM) — not still running.
// Returns that dead pid for the status message (instant crash detection vs the 5-min idle threshold).
fn dead_run_pid(dir: &Path, stem: &str) -> Option<u32> {
    let marker = std::fs::read_to_string(dir.join(format!("{stem}.running"))).ok()?;
    let pid = marker
        .split_whitespace()
        .next()?
        .strip_prefix("pid=")?
        .parse::<u32>()
        .ok()?;
    (!pid_alive(pid)).then_some(pid)
}

fn file_brief(path: &Path, label: &str) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    let age = meta
        .modified()
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Some(format!("{label}={} age={}s", hum_bytes(meta.len()), age))
}

fn io_brief(dir: &Path, stem: &str) -> String {
    let files = [
        (".index.log", "index"),
        (".jsonl", "jsonl"),
        (".err", "err"),
        (".codexlog", "codexlog"),
        (".agy.log", "agylog"),
        (".agy.jsonl", "agyjsonl"),
        (".answer.txt", "answer"),
        (".meter.json", "meter"),
        (".meta.json", "meta"),
        (".forfeit", "forfeit"),
    ];
    let parts: Vec<String> = files
        .iter()
        .filter_map(|(suffix, label)| file_brief(&dir.join(format!("{stem}{suffix}")), label))
        .collect();
    if parts.is_empty() {
        String::new()
    } else {
        format!(" · {}", parts.join(" "))
    }
}

fn agy_conversation_id_from_text(text: &str) -> Option<String> {
    for line in text.lines() {
        let Some(rest) = line.split("Print mode: conversation=").nth(1) else {
            continue;
        };
        let id = rest
            .split(|c: char| c == ',' || c.is_whitespace())
            .next()
            .unwrap_or("");
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    None
}

fn agy_conversation_id_from_log(log_path: &Path) -> Option<String> {
    agy_conversation_id_from_text(&std::fs::read_to_string(log_path).ok()?)
}

fn agy_observed_model_from_log(log_path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(log_path).ok()?;
    for line in text.lines().rev() {
        let Some(rest) = line
            .split("Propagating selected model override to backend:")
            .nth(1)
        else {
            continue;
        };
        if let Some(label) = rest.split("label=\"").nth(1) {
            if let Some(end) = label.find('"') {
                return Some(label[..end].to_string());
            }
        }
    }
    None
}

fn agy_transcript_path(cid: &str) -> Option<PathBuf> {
    Some(
        home_dir()?
            .join(".gemini/antigravity-cli/brain")
            .join(cid)
            .join(".system_generated/logs/transcript_full.jsonl"),
    )
}

fn agy_transcript_path_from_log(log_path: &Path) -> Option<PathBuf> {
    let cid = agy_conversation_id_from_log(log_path)?;
    let p = agy_transcript_path(&cid)?;
    p.is_file().then_some(p)
}

fn preferred_event_path(root: &Path, id: &str, run: &str) -> Option<PathBuf> {
    let jsonl = root.join(format!("results/{id}/{run}.jsonl"));
    if jsonl.is_file() {
        return Some(jsonl);
    }
    let agy_jsonl = root.join(format!("results/{id}/{run}.agy.jsonl"));
    if agy_jsonl.is_file() {
        return Some(agy_jsonl);
    }
    if let Some(transcript) =
        agy_transcript_path_from_log(&root.join(format!("results/{id}/{run}.agy.log")))
    {
        return Some(transcript);
    }
    let err = root.join(format!("results/{id}/{run}.err"));
    if err.is_file() {
        return Some(err);
    }
    None
}

fn run_answer_path(root: &Path, id: &str, run: &str) -> PathBuf {
    root.join(format!("results/{id}/{run}.answer.txt"))
}

fn file_detail_line(path: &Path, label: &str) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    let age = meta
        .modified()
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Some(format!(
        "  {:<11} {:>8} age={:<6}s {}",
        label,
        hum_bytes(meta.len()),
        age,
        path.display()
    ))
}

fn answer_source_label(dir: &Path, names: &[String], stem: &str) -> String {
    let meter = read_json(&dir.join(format!("{stem}.meter.json")));
    if let Some(cli) = meter.get("cli").and_then(|x| x.as_str()) {
        if !cli.is_empty() {
            return cli.to_string();
        }
    }
    if let Some(runner) = meter.get("runner").and_then(|x| x.as_str()) {
        if !runner.is_empty() {
            return runner.to_string();
        }
    }
    if names.contains(&format!("{stem}.agy.jsonl")) || names.contains(&format!("{stem}.agy.log")) {
        return "agy".into();
    }
    if names.contains(&format!("{stem}.codexlog")) {
        return "codex".into();
    }
    let arm = util::parse_arm(stem);
    if arm.cli.is_empty() {
        "answer".into()
    } else {
        arm.cli
    }
}

/// Gather every recorded run of an instance as typed RunStats (jsonl + answer + forfeit).
// Parse a --since duration like "9h" / "30m" / "2d" / "45s" / bare seconds → seconds.
fn parse_duration_secs(s: &str) -> Option<u64> {
    let s = s.trim();
    let (num, mult) = match s.chars().last()? {
        'h' | 'H' => (&s[..s.len() - 1], 3600u64),
        'm' | 'M' => (&s[..s.len() - 1], 60),
        'd' | 'D' => (&s[..s.len() - 1], 86400),
        's' | 'S' => (&s[..s.len() - 1], 1),
        _ => (s, 1),
    };
    num.trim().parse::<u64>().ok().map(|n| n * mult)
}

// Epoch-ms start time embedded in a timestamped run label (`…-t<ms>`); None for legacy `-rN` labels.
fn run_start_ms(label: &str) -> Option<u64> {
    let pos = label.rfind("-t")?;
    let tail = &label[pos + 2..];
    (!tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()))
        .then(|| tail.parse::<u64>().ok())
        .flatten()
}

// Was this run started at/after `cutoff_secs`? Prefers the label's own start time (`-t<ms>`, robust
// to file touches); falls back to freshest artifact mtime for legacy labels with no `-t`.
fn run_since(root: &Path, id: &str, label: &str, cutoff_secs: u64) -> bool {
    if let Some(ms) = run_start_ms(label) {
        return ms / 1000 >= cutoff_secs;
    }
    let d = root.join("results").join(id);
    [".answer.txt", ".jsonl", ".agy.jsonl", ".err"]
        .iter()
        .filter_map(|s| {
            std::fs::metadata(d.join(format!("{label}{s}")))
                .ok()?
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|x| x.as_secs())
        })
        .max()
        .map(|m| m >= cutoff_secs)
        .unwrap_or(true)
}

// If `--since <dur>` is present and valid, returns the cutoff epoch-seconds (keep runs at/after it).
fn since_cutoff(args: &[String]) -> Option<u64> {
    let dur = parse_duration_secs(&arg_value(args, "--since")?)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Some(now.saturating_sub(dur))
}

fn gather_runs(root: &Path, id: &str) -> Vec<RunStats> {
    let inst = load_inst(
        &root
            .join(format!("instances/{id}/instance.json"))
            .to_string_lossy(),
    );
    let d = root.join("results").join(id);
    let mut runs = vec![];
    let names = list_dir_names(&d);
    for n in &names {
        if n.ends_with(".agy.jsonl") {
            continue;
        }
        if let Some(stem) = n.strip_suffix(".jsonl") {
            let _ = stem;
            let stats = grade_jsonl(&inst, &d.join(n).to_string_lossy());
            runs.push(review::apply_review(root, id, stats));
        }
    }
    for n in &names {
        if let Some(stem) = n.strip_suffix(".answer.txt") {
            if names.contains(&format!("{stem}.jsonl")) {
                continue;
            }
            let stats = grade_text_file(
                &inst,
                &d.join(n).to_string_lossy(),
                &d.join(format!("{stem}.meter.json")).to_string_lossy(),
            );
            runs.push(review::apply_review(root, id, stats));
        }
    }
    for n in &names {
        if let Some(stem) = n.strip_suffix(".forfeit") {
            runs.push(review::apply_review(root, id, forfeit_stats(stem)));
        }
    }
    runs
}

fn telemetry_paths(d: &Path) -> Vec<String> {
    let names = list_dir_names(d);
    let mut v: Vec<String> = names
        .iter()
        .filter(|n| n.ends_with(".jsonl") && !n.ends_with(".agy.jsonl"))
        .map(|n| d.join(n).to_string_lossy().into_owned())
        .collect();
    for n in &names {
        let Some(stem) = n.strip_suffix(".answer.txt") else {
            continue;
        };
        if names.contains(&format!("{stem}.jsonl")) {
            continue;
        }
        if names.contains(&format!("{stem}.agy.jsonl")) {
            v.push(
                d.join(format!("{stem}.agy.jsonl"))
                    .to_string_lossy()
                    .into_owned(),
            );
        } else if let Some(transcript) =
            agy_transcript_path_from_log(&d.join(format!("{stem}.agy.log")))
        {
            v.push(transcript.to_string_lossy().into_owned());
        } else if names.contains(&format!("{stem}.err")) {
            v.push(d.join(format!("{stem}.err")).to_string_lossy().into_owned());
        }
    }
    v.sort();
    v
}

fn result_stems(d: &Path) -> Vec<String> {
    let suffixes = [
        ".answer.txt",
        ".agy.jsonl",
        ".agy.log",
        ".meter.json",
        ".running",
        ".forfeit",
        ".jsonl",
        ".err",
    ];
    let mut stems: Vec<String> = vec![];
    for n in list_dir_names(d) {
        for suffix in suffixes {
            if let Some(stem) = n.strip_suffix(suffix) {
                if !stem.starts_with("mcp-")
                    && !stem.starts_with("mcp-empty-")
                    && !stems.iter().any(|s| s == stem)
                {
                    stems.push(stem.to_string());
                }
                break;
            }
        }
    }
    stems.sort();
    stems
}

fn resolve_run_stem(root: &Path, id: &str, run: &str) -> String {
    let d = root.join("results").join(id);
    let stems = result_stems(&d);
    if stems.iter().any(|s| s == run) {
        return run.to_string();
    }
    let prefix = format!("{run}-t");
    let matches: Vec<String> = stems
        .into_iter()
        .filter(|stem| stem.starts_with(&prefix))
        .collect();
    match matches.len() {
        0 => run.to_string(),
        1 => matches[0].clone(),
        _ => die(&format!(
            "run label '{run}' is ambiguous; include timestamp: {}",
            matches.join(" ")
        )),
    }
}

fn forfeit_stats(label: &str) -> RunStats {
    let mut stats = RunStats::new(
        label.into(),
        "FORFEIT".into(),
        0.0,
        0,
        None,
        0,
        0,
        String::new(),
    );
    stats.review_status = "system".into();
    stats.final_checked = true;
    stats.cost_available = false;
    stats.tokens_available = false;
    stats
}

fn grade_one_run(root: &Path, id: &str, run: &str) -> RunStats {
    let inst = load_inst(
        &root
            .join(format!("instances/{id}/instance.json"))
            .to_string_lossy(),
    );
    let d = root.join("results").join(id);
    let jsonl = d.join(format!("{run}.jsonl"));
    let answer = d.join(format!("{run}.answer.txt"));
    let forfeit = d.join(format!("{run}.forfeit"));
    let stats = if jsonl.is_file() {
        grade_jsonl(&inst, &jsonl.to_string_lossy())
    } else if answer.is_file() {
        grade_text_file(
            &inst,
            &answer.to_string_lossy(),
            &d.join(format!("{run}.meter.json")).to_string_lossy(),
        )
    } else if forfeit.is_file() {
        forfeit_stats(run)
    } else {
        die("no such run (see: monobench status <id>)");
    };
    review::apply_review(root, id, stats)
}

fn print_grade_with_next(id: &str, stats: &RunStats) {
    print_grade(stats);
    if !stats.final_checked {
        println!("[NEXT] {}", review::unreviewed_next(id, &stats.label));
    }
}

fn run_answer_text(root: &Path, id: &str, run: &str) -> String {
    let d = root.join("results").join(id);
    let answer = d.join(format!("{run}.answer.txt"));
    if answer.is_file() {
        return std::fs::read_to_string(answer).unwrap_or_default();
    }
    let jsonl = d.join(format!("{run}.jsonl"));
    if jsonl.is_file() {
        return util::load_jsonl(&jsonl.to_string_lossy())
            .into_iter()
            .rev()
            .find(|e| e.get("type").and_then(serde_json::Value::as_str) == Some("result"))
            .and_then(|e| {
                e.get("result")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_default();
    }
    String::new()
}

fn judge_prompt(root: &Path, id: &str, run: &str, model: &str, stats: &RunStats) -> String {
    let inst_dir = root.join("instances").join(id);
    let d = root.join("results").join(id);
    let symptom = std::fs::read_to_string(inst_dir.join("symptom.md")).unwrap_or_default();
    let ground_truth =
        std::fs::read_to_string(inst_dir.join("ground_truth.md")).unwrap_or_default();
    let instance = std::fs::read_to_string(inst_dir.join("instance.json")).unwrap_or_default();
    let answer = run_answer_text(root, id, run);
    let transcript = d.join(format!("{run}.md"));
    let transcript_note = if transcript.is_file() {
        format!("Transcript markdown exists: {}", transcript.display())
    } else {
        format!("No transcript markdown yet. Optional: monobench export {id} {run}")
    };
    format!(
        "# monobench final judge prompt\n\n\
Judge model: {model}\n\
Instance: {id}\n\
Run: {run}\n\
Auto grade: {}\n\
Effective grade before review: {}\n\
Review status: {}\n\
{}\n\n\
## Task\n\
You are the final grader, not the solver. You may see the answer key. Decide whether the solver's final answer correctly identifies the true root cause and mechanism.\n\n\
Allowed final grades: FULL, NAME_ONLY, DECOY, MISS, NO_RESULT, INVALID, FORFEIT.\n\n\
Return exactly:\n\n\
FINAL_GRADE: <one allowed grade>\n\
REASON: <short evidence-based reason>\n\n\
## Symptom Given To Solver\n\n{}\n\n\
## Solver Final Answer\n\n{}\n\n\
## Instance JSON / Grading Rules\n\n```json\n{}\n```\n\n\
## Ground Truth\n\n{}\n\n\
## NEXT\n\n\
After judging, record it with:\n\n\
monobench review {id} {run} --final <GRADE> --reason \"<reason>\" --judge-model {model}\n",
        stats.auto_grade,
        stats.grade,
        stats.review_status,
        transcript_note,
        symptom.trim(),
        answer.trim(),
        instance.trim(),
        ground_truth.trim()
    )
}

#[derive(Clone)]
struct ProcInfo {
    pid: String,
    ppid: String,
    etime: String,
    cmd: String,
}

fn ps_rows() -> Vec<ProcInfo> {
    let out = Command::new("ps")
        .args(["-axo", "pid=,ppid=,etime=,command="])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    out.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let pid = parts.next()?.to_string();
            let ppid = parts.next()?.to_string();
            let etime = parts.next()?.to_string();
            let cmd = parts.collect::<Vec<_>>().join(" ");
            if cmd.is_empty() {
                None
            } else {
                Some(ProcInfo {
                    pid,
                    ppid,
                    etime,
                    cmd,
                })
            }
        })
        .collect()
}

fn active_run_procs(rows: &[ProcInfo], id: Option<&str>) -> Vec<ProcInfo> {
    let mut v: Vec<ProcInfo> = rows
        .iter()
        .filter(|p| {
            let is_controller = !looks_like_wrapper(&p.cmd)
                && (p.cmd.contains("monobench run ") || p.cmd.contains("monobench matrix "));
            is_controller
                && id
                    .map(|id| {
                        p.cmd.contains(&format!("monobench run {id} "))
                            || p.cmd.contains(&format!("monobench matrix {id} "))
                    })
                    .unwrap_or(true)
        })
        .cloned()
        .collect();
    v.sort_by(|a, b| a.pid.cmp(&b.pid));
    v
}

fn child_phase(rows: &[ProcInfo], pid: &str) -> String {
    let kids: Vec<&ProcInfo> = rows.iter().filter(|p| p.ppid == pid).collect();
    if let Some(p) = kids.iter().find(|p| p.cmd.contains("monogram index")) {
        return format!("index(pid={}, {})", p.pid, p.etime);
    }
    if let Some(p) = kids.iter().find(|p| p.cmd.contains("codex exec")) {
        return format!("codex(pid={}, {})", p.pid, p.etime);
    }
    if let Some(p) = kids.iter().find(|p| p.cmd.contains("claude -p")) {
        return format!("claude(pid={}, {})", p.pid, p.etime);
    }
    if let Some(p) = kids
        .iter()
        .find(|p| p.cmd.contains("agy ") && p.cmd.contains("--print"))
    {
        return format!("agy(pid={}, {})", p.pid, p.etime);
    }
    "controller".into()
}

// A process that merely *mentions* a solver string — a shell running a grep/script, a search tool,
// an editor — is NOT a live solver. Real solver/index processes start with the binary
// (node/codex/claude/monogram/agy), never a shell. Without this guard the active-worker counts
// over-report for any command line that happens to contain "codex exec"/"claude -p"/etc.
fn looks_like_wrapper(cmd: &str) -> bool {
    let c = cmd.trim_start();
    c.starts_with('-') // login shells: -zsh, -bash
        || c.starts_with("/bin/zsh")
        || c.starts_with("/bin/bash")
        || c.starts_with("/bin/sh")
        || c.starts_with("zsh ")
        || c.starts_with("bash ")
        || c.starts_with("sh ")
        || c.starts_with("grep")
        || c.starts_with("egrep")
        || c.starts_with("rg ")
        || c.contains("shell-snapshots") // Claude Code's bash wrapper
}

fn active_counts(rows: &[ProcInfo]) -> (usize, usize, usize, usize, usize) {
    let live = |needle: &str| {
        rows.iter()
            .filter(|p| !looks_like_wrapper(&p.cmd) && p.cmd.contains(needle))
            .count()
    };
    let runs = active_run_procs(rows, None).len();
    let index = live("monogram index");
    let claude = live("claude -p");
    let codex = live("codex exec");
    let agy = rows
        .iter()
        .filter(|p| {
            !looks_like_wrapper(&p.cmd) && p.cmd.contains("agy ") && p.cmd.contains("--print")
        })
        .count();
    (runs, index, claude, codex, agy)
}

fn print_active_runs(rows: &[ProcInfo], id: &str) {
    let runs = active_run_procs(rows, Some(id));
    if runs.is_empty() {
        return;
    }
    println!("  active runs:");
    for p in runs {
        println!(
            "    pid={} elapsed={} phase={} cmd={}",
            p.pid,
            p.etime,
            child_phase(rows, &p.pid),
            p.cmd
        );
    }
}

fn inspect_run(root: &Path, id: &str, run_arg: &str, tail: usize) {
    let run = resolve_run_stem(root, id, run_arg);
    let d = root.join("results").join(id);
    let names = list_dir_names(&d);
    let source = answer_source_label(&d, &names, &run);
    println!("inspect: {id}");
    println!("  run: {run}");
    println!("  source: {source}");
    run_meta::print(root, id, &run);

    let answer = run_answer_path(root, id, &run);
    let jsonl = d.join(format!("{run}.jsonl"));
    let forfeit = d.join(format!("{run}.forfeit"));
    if jsonl.is_file() || answer.is_file() || forfeit.is_file() {
        let stats = grade_one_run(root, id, &run);
        println!(
            "  grade: {} auto={} review={} checked={} time={}s",
            stats.grade, stats.auto_grade, stats.review_status, stats.final_checked, stats.time
        );
    } else {
        println!("  grade: pending");
    }

    println!("\nartifacts");
    for (suffix, label) in [
        (".running", "running"),
        (".index.log", "index"),
        (".jsonl", "jsonl"),
        (".err", "err"),
        (".codexlog", "codexlog"),
        (".agy.log", "agylog"),
        (".agy.jsonl", "agyjsonl"),
        (".answer.txt", "answer"),
        (".meter.json", "meter"),
        (".meta.json", "meta"),
        (".forfeit", "forfeit"),
        (".md", "export"),
    ] {
        if let Some(line) = file_detail_line(&d.join(format!("{run}{suffix}")), label) {
            println!("{line}");
        }
    }

    let agy_log = d.join(format!("{run}.agy.log"));
    if agy_log.is_file() {
        println!("\nagy");
        if let Some(cid) = agy_conversation_id_from_log(&agy_log) {
            println!("  conversation: {cid}");
            if let Some(tp) = agy_transcript_path(&cid) {
                if let Some(line) = file_detail_line(&tp, "transcript") {
                    println!("{line}");
                } else {
                    println!("  transcript: missing {}", tp.display());
                }
            }
        }
        if let Some(model) = agy_observed_model_from_log(&agy_log) {
            println!("  observed_model: {model}");
        }
        let text = std::fs::read_to_string(&agy_log).unwrap_or_default();
        let interesting: Vec<&str> = text
            .lines()
            .filter(|l| {
                l.starts_with('E')
                    || l.starts_with('W')
                    || l.contains("Model output error")
                    || l.contains("invalid tool call")
                    || l.contains("not logged")
                    || l.contains("Print mode: conversation=")
            })
            .collect();
        if !interesting.is_empty() {
            println!("  notable log lines:");
            for line in interesting
                .iter()
                .rev()
                .take(tail)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                println!("    {}", util::fit_middle(line.trim(), 150));
            }
        }
    }

    if let Some(path) = preferred_event_path(root, id, &run) {
        let events = telemetry::events_from_path(&path.to_string_lossy());
        let mono = events
            .iter()
            .filter(|e| {
                (e.name == "Bash" && grade::is_monogram_cmd(&e.cmd))
                    || e.name.to_lowercase().contains("monogram")
            })
            .count();
        let mono_grep = events
            .iter()
            .filter(|e| {
                ((e.name == "Bash" && grade::is_monogram_cmd(&e.cmd))
                    || e.name.to_lowercase().contains("monogram"))
                    && util::cmd_has_word(&e.cmd, "grep")
            })
            .count();
        let grep = events
            .iter()
            .filter(|e| {
                let is_mono = (e.name == "Bash" && grade::is_monogram_cmd(&e.cmd))
                    || e.name.to_lowercase().contains("monogram");
                !is_mono
                    && (e.name == "Grep"
                        || e.name == "Glob"
                        || (e.name == "Bash"
                            && ["grep", "egrep", "rg", "find", "fd", "ag", "ack"]
                                .iter()
                                .any(|w| util::cmd_has_word(&e.cmd, w))))
            })
            .count();
        println!(
            "\nevents: {} calls · monogram {} · monogram-grep {} · grep/find {}",
            events.len(),
            mono,
            mono_grep,
            grep
        );
        println!("  source: {}", path.display());
    } else {
        println!("\nevents: unavailable");
    }

    let rows = ps_rows();
    let self_pid = std::process::id().to_string();
    let live: Vec<&ProcInfo> = rows
        .iter()
        .filter(|p| {
            p.pid != self_pid
                && p.cmd.contains(&run)
                && !p.cmd.contains(" monobench inspect ")
                && !p.cmd.contains(" inspect ")
        })
        .collect();
    if !live.is_empty() {
        println!("\nprocesses");
        for p in live.iter().take(6) {
            println!(
                "  pid={} ppid={} elapsed={} {}",
                p.pid,
                p.ppid,
                p.etime,
                util::fit_middle(&p.cmd, 150)
            );
        }
    }

    println!("\n[NEXT]");
    println!("  monobench trace {id} {run} 40");
    println!("  monobench export {id} {run}");
    println!("  monobench status {id} --detail");
}

fn integrity_report(root: &Path, id: &str, run_arg: Option<&str>, detail: bool) {
    let d = root.join("results").join(id);
    if !d.is_dir() {
        die("no results for instance (see: monobench status <id>)");
    }
    let stats: HashMap<String, String> = gather_runs(root, id)
        .into_iter()
        .map(|s| (s.label, s.grade))
        .collect();
    let runs = if let Some(run_arg) = run_arg {
        vec![resolve_run_stem(root, id, run_arg)]
    } else {
        result_stems(&d)
            .into_iter()
            .filter(|s| !s.starts_with("_prepare-"))
            .collect()
    };
    if runs.is_empty() {
        die("no recorded runs (see: monobench status <id>)");
    }
    let findings: Vec<integrity::Finding> = runs
        .iter()
        .map(|run| {
            let event = preferred_event_path(root, id, run);
            integrity::scan_run(
                run,
                stats.get(run).map(String::as_str).unwrap_or("?"),
                event.as_deref(),
                &d.join(format!("{run}.index.log")),
                !run_answer_text(root, id, run).trim().is_empty(),
                &d.join(format!("{run}.running")),
            )
        })
        .collect();
    integrity::print_report(id, &findings, detail || run_arg.is_some());
}

fn evidence_index(
    root: &Path,
    id: &str,
    pattern: Option<&str>,
    case_sensitive: bool,
    include_prompt: bool,
    max: usize,
) {
    let d = root.join("results").join(id);
    if !d.is_dir() {
        die("no results for instance (see: monobench status <id>)");
    }
    let stats: HashMap<String, String> = gather_runs(root, id)
        .into_iter()
        .map(|s| (s.label, s.grade))
        .collect();
    let runs: Vec<String> = result_stems(&d)
        .into_iter()
        .filter(|s| !s.starts_with("_prepare-"))
        .collect();
    if runs.is_empty() {
        die("no recorded runs (see: monobench status <id>)");
    }
    let summaries: Vec<evidence::Summary> = runs
        .iter()
        .map(|run| {
            let source = preferred_event_path(root, id, run);
            evidence::summarize(
                run,
                stats.get(run).map(String::as_str).unwrap_or("?"),
                source.as_deref(),
                &run_answer_text(root, id, run),
                &d.join(format!("{run}.index.log")),
                pattern,
                case_sensitive,
                include_prompt,
            )
        })
        .collect();
    evidence::print_index(id, pattern, max, &summaries);
}

// An instance whose grading config still has TODO/empty placeholders grades every run INVALID.
// Surface that once at the instance level so the INVALIDs are not misread as solver failures.
fn warn_if_grading_incomplete(root: &Path, id: &str) {
    let inst_path = root.join(format!("instances/{id}/instance.json"));
    if let Some(reason) = load_inst(&inst_path.to_string_lossy()).invalid {
        println!(
            "{}",
            util::c(
                util::HEAD,
                &format!(
                    "[!] {id}: grading not configured ({reason}). Runs grade INVALID until instance.json grading + ground_truth are completed — grades below are not meaningful yet."
                )
            )
        );
    }
}

fn matrix_state_path(root: &Path) -> PathBuf {
    root.join(".matrix-state")
}

// Best-effort matrix liveness record, sibling to `.matrix-stop`: written once when a matrix starts
// and removed when it ends. Every write ignores errors so it can NEVER disturb the benchmark; the
// reader self-heals a stale file (matrix crashed without cleanup) via a pid liveness check.
fn write_matrix_state(
    root: &Path,
    pid: u32,
    id: &str,
    total: usize,
    jobs: usize,
    cli: &str,
    model: &str,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let v = serde_json::json!({
        "pid": pid, "id": id, "total": total, "jobs": jobs,
        "cli": cli, "model": model, "started_at": now,
    });
    let _ = std::fs::write(matrix_state_path(root), v.to_string());
}

fn clear_matrix_state(root: &Path) {
    let _ = std::fs::remove_file(matrix_state_path(root));
}

fn pid_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// Returns (instance_id, display_line) for a LIVE matrix; clears the file and returns None if the
// recorded pid is gone (crashed matrix left a stale marker).
fn read_matrix_state(root: &Path) -> Option<(String, String)> {
    let txt = std::fs::read_to_string(matrix_state_path(root)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&txt).ok()?;
    let pid = v.get("pid")?.as_u64()? as u32;
    if !pid_alive(pid) {
        let _ = std::fs::remove_file(matrix_state_path(root));
        return None;
    }
    let id = v
        .get("id")
        .and_then(|x| x.as_str())
        .unwrap_or("?")
        .to_string();
    let total = v.get("total").and_then(|x| x.as_u64()).unwrap_or(0);
    let cli = v.get("cli").and_then(|x| x.as_str()).unwrap_or("?");
    let model = v.get("model").and_then(|x| x.as_str()).unwrap_or("?");
    let started = v.get("started_at").and_then(|x| x.as_u64()).unwrap_or(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let elapsed = now.saturating_sub(started);
    let line =
        format!("matrix: {id} · {cli}/{model} · {total} runs · pid {pid} · {elapsed}s (`monobench stop` to halt)");
    Some((id, line))
}

fn die(msg: &str) -> ! {
    eprintln!("monobench: {msg}");
    std::process::exit(1);
}

fn main() {
    let root = find_root();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");
    let a = |i: usize| args.get(i).map(String::as_str);
    if args.iter().any(|s| s == "--live") && matches!(cmd, "status" | "watch") {
        live_loop(&args);
    }

    match cmd {
        "-V" | "--version" | "version" => {
            println!("monobench {}", env!("CARGO_PKG_VERSION"));
        }

        "" | "help" | "-h" | "--help" => {
            let p = root.join("initiate/initiate.md");
            match std::fs::read_to_string(&p) {
                Ok(s) => print!("{s}"),
                Err(_) => println!("monobench (no initiate.md found)"),
            }
        }

        "list" => {
            println!("{:<30} {}", "INSTANCE", "TITLE");
            for id in list_dir_names(&root.join("instances")) {
                if id == "_TEMPLATE" {
                    continue;
                }
                let title = read_json(&root.join(format!("instances/{id}/instance.json")))
                    .get("title")
                    .and_then(|x| x.as_str())
                    .unwrap_or("?")
                    .to_string();
                println!("{:<30} {}", id, title);
            }
            println!("\n[NEXT]");
            println!("  monobench show <id>        # the task given to the solver");
            println!(
                "  monobench status <id>      # run progress  ·  monobench run <id> baseline 1"
            );
        }

        "tools" => {
            println!("{:<14} {}", "TOOL", "DESC");
            for t in list_dir_names(&root.join("harness/tools")) {
                if t == "_TEMPLATE" {
                    continue;
                }
                let tool_json = root.join("harness/tools").join(&t).join("tool.json");
                if !tool_json.is_file() {
                    continue;
                }
                let raw = read_json(&tool_json)
                    .get("desc")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                // Head-truncate with an ellipsis so a long desc reads cleanly (full text in tool.json)
                // instead of being cut off mid-word.
                let desc: String = if raw.chars().count() > 90 {
                    raw.chars().take(89).collect::<String>() + "…"
                } else {
                    raw
                };
                println!("{:<14} {}", t, desc);
            }
            println!("(define your own: cp -r harness/tools/_TEMPLATE harness/tools/<name> && edit tool.json)");
            println!("\n[NEXT]");
            println!(
                "  monobench run <id> <tool> 1                                  # one arm, one run"
            );
            println!("  monobench matrix <id> --tools baseline,monogram --cli claude --model haiku --runs 3");
            println!("  monobench sweep --all --tools baseline --cli claude --model haiku --jobs 3   # many instances; cross-repo parallel, same-repo serialized");
        }

        "status" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench status <id>"));
            let d = root.join("results").join(id);
            println!("status: {id}");
            warn_if_grading_incomplete(&root, id);
            let mut names = list_dir_names(&d);
            // chronological: runs with an embedded -t<ms> start time first-to-last, legacy (none) lead.
            names.sort_by(|a, b| {
                util::label_start_ms(a)
                    .unwrap_or(0)
                    .cmp(&util::label_start_ms(b).unwrap_or(0))
                    .then_with(|| a.cmp(b))
            });
            let detail = args.iter().any(|s| s == "--detail");
            let stats = gather_runs(&root, id);
            let started_at = |label: &str| -> String {
                util::label_start_ms(label)
                    .map(util::fmt_utc_ms)
                    .unwrap_or_else(|| "—".into())
            };
            let time_for = |label: &str| -> String {
                stats
                    .iter()
                    .find(|r| r.label == label && r.time > 0)
                    .map(|r| format!(" · {}s", r.time))
                    .unwrap_or_default()
            };
            for n in &names {
                if n.ends_with(".agy.jsonl") {
                    continue;
                }
                if let Some(b) = n.strip_suffix(".jsonl") {
                    let txt = std::fs::read_to_string(d.join(n)).unwrap_or_default();
                    let st = if txt.contains("\"type\":\"result\"") {
                        format!(
                            "done{}{}",
                            time_for(b),
                            if detail {
                                io_brief(&d, b)
                            } else {
                                String::new()
                            }
                        )
                    } else if let Some(dead) = dead_run_pid(&d, b) {
                        format!(
                            "⚠ crashed (pid {dead} gone, {}c){}",
                            txt.matches("\"type\":\"tool_use\"").count(),
                            io_brief(&d, b)
                        )
                    } else {
                        format!(
                            "running ({}c){}{}",
                            txt.matches("\"type\":\"tool_use\"").count(),
                            idle_tag(&d, b),
                            io_brief(&d, b)
                        )
                    };
                    println!("  {:<34} {:>12} {}", b, started_at(b), st);
                }
            }
            for n in &names {
                if let Some(b) = n.strip_suffix(".answer.txt") {
                    if !names.contains(&format!("{b}.jsonl")) {
                        let source = answer_source_label(&d, &names, b);
                        println!(
                            "  {:<34} {:>12} done ({}){}{}",
                            b,
                            started_at(b),
                            source,
                            time_for(b),
                            if detail {
                                io_brief(&d, b)
                            } else {
                                String::new()
                            }
                        );
                    }
                }
            }
            for n in &names {
                if let Some(b) = n.strip_suffix(".forfeit") {
                    println!("  {:<34} {:>12} FORFEIT", b, started_at(b));
                }
            }
            for n in &names {
                if let Some(b) = n.strip_suffix(".running") {
                    if names.contains(&format!("{b}.jsonl"))
                        || names.contains(&format!("{b}.answer.txt"))
                        || names.contains(&format!("{b}.forfeit"))
                    {
                        continue;
                    }
                    let detail = std::fs::read_to_string(d.join(n))
                        .unwrap_or_default()
                        .lines()
                        .next()
                        .unwrap_or("")
                        .to_string();
                    if let Some(dead) = dead_run_pid(&d, b) {
                        println!(
                            "  {:<34} {:>12} ⚠ crashed (pid {dead} gone, stale marker){}",
                            b,
                            started_at(b),
                            io_brief(&d, b)
                        );
                    } else {
                        println!(
                            "  {:<34} {:>12} running {}{}{}",
                            b,
                            started_at(b),
                            detail,
                            idle_tag(&d, b),
                            io_brief(&d, b)
                        );
                    }
                }
            }
            let rows = ps_rows();
            print_active_runs(&rows, id);
            let work =
                std::env::var("MONOBENCH_WORK").unwrap_or_else(|_| "/tmp/monobench-work".into());
            let wt = std::fs::read_dir(format!("{work}/wt"))
                .into_iter()
                .flatten()
                .flatten()
                .count();
            let (runs, index, claude, codex, agy) = active_counts(&rows);
            println!(
                "  active → runs:{} index:{} claude:{} codex:{} agy:{} worktrees:{}",
                runs, index, claude, codex, agy, wt
            );
            if let Some((sid, line)) = read_matrix_state(&root) {
                if sid == id {
                    println!("  {line}");
                }
            }
            println!("\n[NEXT]");
            println!("  monobench report {id}        # grades + cost/tokens once runs finish");
            println!("  monobench watch --live       # refresh while in flight  ·  monobench run {id} <arm>");
        }

        // Graceful matrix/sweep stop — write the stop files the worker loops poll.
        "stop" => {
            let stopf = root.join(".matrix-stop");
            std::fs::write(&stopf, b"stop").ok();
            std::fs::write(root.join(".sweep-stop"), b"stop").ok(); // sweep has its own poll file
            println!("stop signalled → {}", stopf.display());
            println!("  running matrix/sweep workers finish their CURRENT run and launch no more.");
            println!("  for immediate CPU relief of in-flight runs: pkill -f 'claude -p'  (they will NOT respawn now).");
        }

        // Global overview across ALL instances — no id needed ("뭐가 돌고 있나" 한눈에).
        "watch" => {
            println!(
                "{:<32} {:>3} {:>4} {:>5}  {}",
                "INSTANCE", "RUN", "DONE", "FULL", "running"
            );
            println!("{}", "─".repeat(78));
            let (mut tot_run, mut tot_done) = (0usize, 0usize);
            let rows = ps_rows();
            for id in list_dir_names(&root.join("instances")) {
                if id == "_TEMPLATE" {
                    continue;
                }
                let d = root.join("results").join(&id);
                if !d.is_dir() {
                    continue;
                }
                let names = list_dir_names(&d);
                let (mut running, mut done) = (0usize, vec![]);
                let mut runlist: Vec<String> = vec![];
                for n in &names {
                    if n.ends_with(".agy.jsonl") {
                        continue;
                    }
                    if let Some(b) = n.strip_suffix(".jsonl") {
                        let txt = std::fs::read_to_string(d.join(n)).unwrap_or_default();
                        if txt.contains("\"type\":\"result\"") {
                            done.push(b.to_string());
                        } else {
                            running += 1;
                            runlist.push(format!(
                                "{b}({}c{})",
                                txt.matches("\"type\":\"tool_use\"").count(),
                                io_brief(&d, b)
                            ));
                        }
                    }
                }
                // answer-only completions (codex, agy, via-niia, or other CLI adapters)
                for n in &names {
                    if let Some(b) = n.strip_suffix(".answer.txt") {
                        if !names.contains(&format!("{b}.jsonl")) {
                            done.push(b.to_string());
                        }
                    }
                }
                for n in &names {
                    if let Some(b) = n.strip_suffix(".running") {
                        if names.contains(&format!("{b}.jsonl"))
                            || names.contains(&format!("{b}.answer.txt"))
                            || names.contains(&format!("{b}.forfeit"))
                        {
                            continue;
                        }
                        running += 1;
                        runlist.push(format!("{b}(pre-result{})", io_brief(&d, b)));
                    }
                }
                let procs = active_run_procs(&rows, Some(&id));
                if running == 0 && !procs.is_empty() {
                    running += procs.len();
                    for p in procs {
                        runlist.push(format!("pid={}({})", p.pid, child_phase(&rows, &p.pid)));
                    }
                }
                if running == 0 && done.is_empty() {
                    continue;
                } // no activity → skip
                let full = gather_runs(&root, &id)
                    .iter()
                    .filter(|r| r.grade == "FULL")
                    .count();
                tot_run += running;
                tot_done += done.len();
                let short: String = if id.chars().count() > 31 {
                    id.chars().take(30).chain(std::iter::once('…')).collect()
                } else {
                    id.clone()
                };
                println!(
                    "{:<32} {:>3} {:>4} {:>5}  {}",
                    short,
                    running,
                    done.len(),
                    format!("{full}/{}", done.len()),
                    runlist.join(" ")
                );
            }
            println!("{}", "─".repeat(78));
            let work =
                std::env::var("MONOBENCH_WORK").unwrap_or_else(|_| "/tmp/monobench-work".into());
            let wt = std::fs::read_dir(format!("{work}/wt"))
                .into_iter()
                .flatten()
                .flatten()
                .count();
            let (runs, index, claude, codex, agy) = active_counts(&rows);
            println!("totals: running {tot_run} · done {tot_done}   active → runs:{runs} index:{index} claude:{claude} codex:{codex} agy:{agy} worktrees:{wt}");
            if let Some((_, line)) = read_matrix_state(&root) {
                println!("{line}");
            }
            println!("\n[NEXT]");
            println!("  monobench status <id> --live --every 5   # drill into one instance, live");
            println!("  monobench report <id>                    # grades once a run set finishes");
        }

        "clean" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench clean <id> [arm-prefix]"));
            let d = root.join("results").join(id);
            if !d.is_dir() {
                die("no results for that id");
            }
            let pre = a(2);
            let exts = [
                ".jsonl",
                ".err",
                ".index.log",
                ".codexlog",
                ".agy.log",
                ".agy.jsonl",
                ".forfeit",
                ".answer.txt",
                ".meter.json",
                ".review.json",
                ".running",
                ".md",
            ];
            let before = list_dir_names(&d)
                .iter()
                .filter(|n| n.ends_with(".jsonl") && !n.ends_with(".agy.jsonl"))
                .count();
            for n in list_dir_names(&d) {
                let path = d.join(&n);
                if n.starts_with('_') && path.is_dir() {
                    let _ = std::fs::remove_dir_all(&path);
                    continue;
                }
                let hit = match pre {
                    Some(p) => {
                        (n.starts_with(p) && exts.iter().any(|e| n.ends_with(e)))
                            || (n.starts_with("mcp-") && n.contains(p) && n.ends_with(".json"))
                    }
                    None => {
                        exts.iter().any(|e| n.ends_with(e))
                            || (n.starts_with("mcp-") && n.ends_with(".json"))
                    }
                };
                if hit {
                    let _ = std::fs::remove_file(&path);
                }
            }
            let after: Vec<String> = list_dir_names(&d)
                .into_iter()
                .filter(|n| n.ends_with(".jsonl") && !n.ends_with(".agy.jsonl"))
                .map(|n| n.trim_end_matches(".jsonl").to_string())
                .collect();
            println!(
                "cleaned {} in {id} · runs {} → {} · kept: {}",
                pre.map(|p| format!("arm '{p}*'"))
                    .unwrap_or_else(|| "ALL runs".into()),
                before,
                after.len(),
                after.join(" ")
            );
        }

        "show" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench show <id> [--spoil]"));
            let dir = root.join("instances").join(id);
            if !dir.is_dir() {
                die("no such instance");
            }
            print!(
                "{}",
                std::fs::read_to_string(dir.join("symptom.md")).unwrap_or_default()
            );
            if a(2) == Some("--spoil") {
                println!("\n════ GROUND TRUTH (spoiler) ════");
                print!(
                    "{}",
                    std::fs::read_to_string(dir.join("ground_truth.md")).unwrap_or_default()
                );
            }
            println!("\n[NEXT]");
            println!("  monobench run {id} baseline 1   ·   monobench run {id} monogram 1");
        }

        "run" => {
            let id = a(1).unwrap_or_else(|| {
                die("usage: monobench run <id> <arm> [n|note...] [--cli c] [--model m] [--via direct|niia] [--tag t] [--note text]")
            });
            let arm = a(2).unwrap_or_else(|| die("arm (see: monobench tools)"));
            let (n, positional_note) = run_number_and_positional_note(&args, 3);
            let tag = arg_any_value(&args, &["--tag", "--batch"]);
            let note = arg_any_value(&args, &["--note", "--memo"]).or(positional_note);
            let model = arg_value(&args, "--model")
                .or_else(|| arg_value(&args, "--models"))
                .or_else(|| std::env::var("MONOBENCH_MODEL").ok())
                .unwrap_or_else(|| "opus".into());
            let (cli, via) = axes_for(&model, arg_value(&args, "--cli"), arg_value(&args, "--via"));
            if let Some(effort) = arg_value(&args, "--effort") {
                std::env::set_var("MONOBENCH_EFFORT", effort);
            }
            if args.iter().any(|s| s == "--force") {
                std::env::set_var("MONOBENCH_FORCE", "1");
            }
            std::process::exit(run::run(
                &root,
                id,
                arm,
                &cli,
                &model,
                &via,
                n,
                None,
                tag.as_deref(),
                note.as_deref(),
                false,
                &Mutex::new(()),
            ));
        }

        "prepare" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench prepare <id> [--tools a,b]"));
            let tools = arg_value(&args, "--tools").unwrap_or_else(|| "monogram".into());
            let ts: Vec<String> = tools
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty() && *s != "baseline")
                .map(str::to_string)
                .collect();
            if ts.is_empty() {
                die("prepare needs at least one non-baseline tool");
            }
            std::process::exit(run::prepare(&root, id, &ts, &Mutex::new(())));
        }

        "matrix" => run_matrix(&root, &args),
        "sweep" => run_sweep(&root, &args),

        "grade" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench grade <id> [run]"));
            match a(2) {
                Some(run) => {
                    let run = resolve_run_stem(&root, id, run);
                    let stats = grade_one_run(&root, id, &run);
                    print_grade_with_next(id, &stats);
                }
                None => {
                    let mut runs = gather_runs(&root, id);
                    runs.sort_by(|a, b| a.label.cmp(&b.label));
                    for stats in runs {
                        print_grade_with_next(id, &stats);
                    }
                }
            }
        }

        "judge" => {
            let id = a(1)
                .unwrap_or_else(|| die("usage: monobench judge <id> <run> [--model m] [--write]"));
            let run_arg =
                a(2).unwrap_or_else(|| die("run label, e.g. monogram-codex-gpt-5.4-mini-low-r1"));
            let run = resolve_run_stem(&root, id, run_arg);
            let stats = grade_one_run(&root, id, &run);
            let model = arg_value(&args, "--model").unwrap_or_else(|| "judge-model".into());
            let prompt = judge_prompt(&root, id, &run, &model, &stats);
            if args.iter().any(|s| s == "--write") {
                let p = root
                    .join("results")
                    .join(id)
                    .join(format!("{run}.judge.md"));
                if let Some(parent) = p.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                std::fs::write(&p, prompt)
                    .unwrap_or_else(|e| die(&format!("write judge prompt failed: {e}")));
                println!("wrote {}", p.display());
                println!("[NEXT] monobench review {id} {run} --final <GRADE> --reason \"<reason>\" --judge-model {model}");
            } else {
                print!("{prompt}");
            }
        }

        "review" => {
            let id = a(1).unwrap_or_else(|| {
                die("usage: monobench review <id> <run> --final GRADE --reason TEXT [--judge-model m]")
            });
            let run_arg =
                a(2).unwrap_or_else(|| die("run label, e.g. monogram-codex-gpt-5.4-mini-low-r1"));
            let run = resolve_run_stem(&root, id, run_arg);
            let stats = grade_one_run(&root, id, &run);
            let final_grade = arg_value(&args, "--final")
                .unwrap_or_else(|| {
                    die("--final FULL|NAME_ONLY|DECOY|MISS|NO_RESULT|INVALID|FORFEIT")
                })
                .to_uppercase();
            if !review::is_final_grade(&final_grade) {
                die("invalid --final grade");
            }
            let judge_model = arg_value(&args, "--judge-model");
            let status = arg_value(&args, "--status").unwrap_or_else(|| {
                if judge_model.is_some() {
                    review::REVIEW_JUDGE_DONE.into()
                } else {
                    review::REVIEW_HUMAN_DONE.into()
                }
            });
            let rec = review::ReviewRecord {
                auto_grade: stats.auto_grade,
                final_grade: Some(final_grade),
                review_status: status,
                final_checked: true,
                judge_model,
                judge_at: Some(review::review_now()),
                reason: arg_value(&args, "--reason"),
            };
            if let Err(e) = review::write_review(&root, id, &run, &rec) {
                die(&e);
            }
            println!("wrote {}", review::review_path(&root, id, &run).display());
            let stats = grade_one_run(&root, id, &run);
            print_grade(&stats);
        }

        "trace" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench trace <id> <run> [max]"));
            let run_arg = a(2).unwrap_or_else(|| die("run label, e.g. monogram-haiku-r1"));
            let run = resolve_run_stem(&root, id, run_arg);
            let max: usize = a(3).and_then(|s| s.parse().ok()).unwrap_or(200);
            let p = preferred_event_path(&root, id, &run)
                .unwrap_or_else(|| die("no trace source (see: monobench inspect <id> <run>)"));
            let answer = run_answer_text(&root, id, &run);
            trace::trace_with_answer(
                &p.to_string_lossy(),
                max,
                (!answer.trim().is_empty()).then_some(answer.as_str()),
            );
            println!("\n[NEXT]");
            println!("  monobench evidence {id} {run} --pattern ROOTCAUSE   # matching tool calls + raw lines");
            println!("  monobench export {id} {run}                         # full transcript → results/<id>/<run>.md");
            println!(
                "  monobench integrity {id} {run}                      # contamination-risk view"
            );
        }

        "evidence" => {
            let id = a(1).unwrap_or_else(|| {
                die("usage: monobench evidence <id> [run] [pattern] [--pattern P] [--context N] [--max N] [--case] [--include-prompt]")
            });
            let case_sensitive = args.iter().any(|s| s == "--case");
            let include_prompt = args.iter().any(|s| s == "--include-prompt");
            // No positional run label (or it's a flag) → index mode: scan every run.
            let run_positional = a(2).filter(|s| !s.starts_with("--"));
            if run_positional.is_none() {
                let pattern_arg = arg_value(&args, "--pattern");
                let pattern = pattern_arg.as_deref().filter(|s| !s.is_empty());
                // --max caps index rows (default 40); --context does not apply to the index.
                let max = arg_value(&args, "--max")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(40);
                evidence_index(&root, id, pattern, case_sensitive, include_prompt, max);
                return;
            }
            let run_arg = run_positional.unwrap();
            let run = resolve_run_stem(&root, id, run_arg);
            let source = preferred_event_path(&root, id, &run)
                .unwrap_or_else(|| die("no evidence source (see: monobench inspect <id> <run>)"));
            let d = root.join("results").join(id);
            let grade = if d.join(format!("{run}.jsonl")).is_file()
                || d.join(format!("{run}.answer.txt")).is_file()
                || d.join(format!("{run}.forfeit")).is_file()
            {
                grade_one_run(&root, id, &run).grade
            } else {
                "?".into()
            };
            let pattern_arg = arg_value(&args, "--pattern").or_else(|| {
                let mut parts = vec![];
                let mut i = 3usize;
                while i < args.len() {
                    match args[i].as_str() {
                        "--pattern" | "--context" | "--max" => i += 2,
                        "--case" => i += 1,
                        s if s.starts_with("--") => i += 1,
                        _ => {
                            parts.push(args[i].clone());
                            i += 1;
                        }
                    }
                }
                let joined = parts.join(" ");
                (!joined.trim().is_empty()).then_some(joined)
            });
            let pattern = pattern_arg.as_deref().filter(|s| !s.is_empty());
            let context = arg_value(&args, "--context")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let max = arg_value(&args, "--max")
                .and_then(|s| s.parse().ok())
                .unwrap_or(80);
            evidence::print_evidence(
                id,
                &run,
                &grade,
                &source,
                &run_answer_text(&root, id, &run),
                &root.join(format!("results/{id}/{run}.index.log")),
                pattern,
                context,
                max,
                case_sensitive,
                include_prompt,
            );
        }

        "inspect" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench inspect <id> <run> [--tail N]"));
            let run_arg = a(2).unwrap_or_else(|| die("run label, e.g. monogram-haiku-r1"));
            let tail = arg_value(&args, "--tail")
                .and_then(|s| s.parse().ok())
                .unwrap_or(8);
            inspect_run(&root, id, run_arg, tail);
        }

        "note" | "memo" => {
            let id = a(1).unwrap_or_else(|| {
                die("usage: monobench note <id> <run> [note...] [--tag t] [--note text]")
            });
            let run_arg = a(2).unwrap_or_else(|| die("run label, e.g. monogram-haiku-r1"));
            let run = resolve_run_stem(&root, id, run_arg);
            let d = root.join("results").join(id);
            if !result_stems(&d).contains(&run) {
                die("no such recorded run (see: monobench status <id>)");
            }
            let tag = arg_any_value(&args, &["--tag", "--batch"]);
            let note =
                arg_any_value(&args, &["--note", "--memo"]).or_else(|| positional_note(&args, 3));
            if tag.is_none() && note.is_none() {
                die("nothing to write: provide --tag, --note, or positional note text");
            }
            let p = run_meta::update_note(&root, id, &run, tag.as_deref(), note.as_deref())
                .unwrap_or_else(|e| die(&e));
            println!("wrote {}", p.display());
            println!("[NEXT] monobench inspect {id} {run}");
        }

        "integrity" | "contamination" => {
            let id =
                a(1).unwrap_or_else(|| die("usage: monobench integrity <id> [run] [--detail]"));
            let run_arg = args.iter().skip(2).find(|s| !s.starts_with("--"));
            let detail = args.iter().any(|s| s == "--detail");
            integrity_report(&root, id, run_arg.map(String::as_str), detail);
        }

        "export" => {
            let id = a(1).unwrap_or_else(|| {
                die("usage: monobench export <id> <run>  → results/<id>/<run>.md")
            });
            let run_arg = a(2).unwrap_or_else(|| die("run label, e.g. monogram-discover-haiku-r1"));
            let run = resolve_run_stem(&root, id, run_arg);
            let p = preferred_event_path(&root, id, &run)
                .or_else(|| {
                    run_answer_path(&root, id, &run)
                        .is_file()
                        .then(|| run_answer_path(&root, id, &run))
                })
                .unwrap_or_else(|| die("no such run (see: monobench status <id>)"));
            if !p.is_file() {
                die("no such run (see: monobench status <id>)");
            }
            export::export(&root, id, &run, &p.to_string_lossy());
        }

        "report" => {
            let id =
                a(1).unwrap_or_else(|| die("usage: monobench report <id> [--since 9h|30m|2d]"));
            warn_if_grading_incomplete(&root, id);
            let mut runs = gather_runs(&root, id);
            if let Some(cut) = since_cutoff(&args) {
                let before = runs.len();
                runs.retain(|r| run_since(&root, id, &r.label, cut));
                println!(
                    "[--since {}] {} of {before} runs started within the window\n",
                    arg_value(&args, "--since").unwrap_or_default(),
                    runs.len()
                );
            }
            report::report(&root, id, &runs);
        }

        "summary" => {
            // cross-instance leaderboard: FULL hit-rate per arm × instance
            let cut = since_cutoff(&args);
            let insts: Vec<(String, Vec<RunStats>)> = list_dir_names(&root.join("instances"))
                .into_iter()
                .filter(|id| id != "_TEMPLATE")
                .map(|id| {
                    let mut r = gather_runs(&root, &id);
                    if let Some(c) = cut {
                        r.retain(|rs| run_since(&root, &id, &rs.label, c));
                    }
                    (id, r)
                })
                .collect();
            if cut.is_some() {
                println!(
                    "[--since {}] windowed to runs started within the period\n",
                    arg_value(&args, "--since").unwrap_or_default()
                );
            }
            report::summary(&insts);
            println!("\n[NEXT]");
            println!("  monobench column <arm>                       # one arm's verified grade breakdown + review coverage");
            println!("  monobench report <id>                       # per-CLI/model detail for one instance");
            println!("  monobench evidence <id> --pattern ROOTCAUSE  # scan each run's conclusion");
        }

        "column" => {
            let arm = a(1).unwrap_or_else(|| {
                die("usage: monobench column <arm>   e.g. baseline-codex-gpt-5.4-mini-low (full arm name from `monobench summary`)")
            });
            let cut = since_cutoff(&args);
            let insts: Vec<(String, Vec<RunStats>)> = list_dir_names(&root.join("instances"))
                .into_iter()
                .filter(|id| id != "_TEMPLATE")
                .map(|id| {
                    let mut r = gather_runs(&root, &id);
                    if let Some(c) = cut {
                        r.retain(|rs| run_since(&root, &id, &rs.label, c));
                    }
                    (id, r)
                })
                .collect();
            if cut.is_some() {
                println!(
                    "[--since {}] windowed to runs started within the period",
                    arg_value(&args, "--since").unwrap_or_default()
                );
            }
            report::column(arm, &insts);
        }

        "adoption" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench adoption <id>"));
            adoption::adoption(id, &telemetry_paths(&root.join("results").join(id)));
            println!("\n[NEXT]");
            println!(
                "  monobench report {id}                        # grade × cost × adoption per arm"
            );
            println!("  monobench evidence {id} --pattern monogram   # which runs actually leaned on the tool");
        }

        "monogram-audit" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench monogram-audit <id>"));
            monogram_audit::audit(
                id,
                &telemetry_paths(&root.join("results").join(id)),
                &gather_runs(&root, id),
            );
            println!("\n[NEXT]");
            println!(
                "  monobench evidence {id} --pattern 'region_first_next|success_pattern_next|score-debug|ROOTCAUSE'  # verify maker recommendations"
            );
            println!(
                "  monobench trace {id} <run>                   # classify: path not closed vs closed but uncalibrated"
            );
            println!(
                "  monobench evidence {id} --pattern 'guarded_no_match|database is locked|bad_workdir'"
            );
            println!(
                "  monobench export {id} <run>                  # compare success/failure rails before maker proposal"
            );
        }

        "meter" => {
            let p = a(1).unwrap_or_else(|| die("usage: monobench meter <session.jsonl>"));
            if !Path::new(p).is_file() {
                die("no such session file (expected a model session JSONL, e.g. results/<id>/<run>.jsonl)");
            }
            meter::meter(p);
        }

        "add" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench add <id>"));
            let dst = root.join("instances").join(id);
            if dst.exists() {
                die("instance already exists");
            }
            copy_dir(&root.join("instances/_TEMPLATE"), &dst);
            println!("created instances/{id} — now edit: instance.json, symptom.md (no spoilers), ground_truth.md");
        }

        other => die(&format!("unknown command '{other}' (try: monobench help)")),
    }
}

fn copy_dir(src: &Path, dst: &Path) {
    let _ = std::fs::create_dir_all(dst);
    for e in std::fs::read_dir(src).into_iter().flatten().flatten() {
        let p = e.path();
        let to = dst.join(e.file_name());
        if p.is_dir() {
            copy_dir(&p, &to);
        } else {
            let _ = std::fs::copy(&p, &to);
        }
    }
}

fn run_matrix(root: &Path, args: &[String]) {
    let id = args
        .get(1)
        .filter(|s| !s.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| {
            die("usage: monobench matrix <id> [--tools a,b] [--cli c] [--model x] [--via direct|niia] [--runs N] [--jobs J] [--prepared]")
        });
    let (mut tools, mut model, mut cli_arg, mut via_arg, mut runs, mut jobs) = (
        "baseline,monogram".to_string(),
        std::env::var("MONOBENCH_MODEL").unwrap_or_else(|_| "opus".into()),
        None::<String>,
        None::<String>,
        1usize,
        2usize,
    );
    let mut tag = None::<String>;
    let mut note = None::<String>;
    let mut isolate = "worktree".to_string();
    let mut prepared = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--tools" => {
                tools = args.get(i + 1).cloned().unwrap_or(tools);
                i += 2;
            }
            "--model" | "--models" => {
                model = args.get(i + 1).cloned().unwrap_or(model);
                i += 2;
            }
            "--cli" => {
                cli_arg = args.get(i + 1).cloned();
                i += 2;
            }
            "--via" => {
                via_arg = args.get(i + 1).cloned();
                i += 2;
            }
            "--effort" => {
                if let Some(effort) = args.get(i + 1) {
                    std::env::set_var("MONOBENCH_EFFORT", effort);
                }
                i += 2;
            }
            "--runs" => {
                runs = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(runs);
                i += 2;
            }
            "--jobs" => {
                jobs = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(jobs);
                i += 2;
            }
            "--isolate" => {
                isolate = args.get(i + 1).cloned().unwrap_or(isolate);
                i += 2;
            }
            "--tag" | "--batch" => {
                tag = args.get(i + 1).cloned();
                i += 2;
            }
            "--note" | "--memo" => {
                note = args.get(i + 1).cloned();
                i += 2;
            }
            "--prepared" => {
                prepared = true;
                i += 1;
            }
            _ => i += 1,
        }
    }
    let ts: Vec<String> = tools
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    let ms: Vec<String> = model
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    if ms.len() != 1 {
        die("matrix accepts exactly one model per command; repeat the matrix command once per model")
    }
    let model = ms[0].clone();
    let (cli, via) = axes_for(&model, cli_arg, via_arg);
    if isolate != "worktree" && isolate != "shared" {
        die("--isolate must be worktree or shared");
    }
    if isolate == "shared" && jobs > 1 {
        eprintln!("shared isolation is single-lane; forcing jobs=1");
        jobs = 1;
    }
    let mut combos: Vec<(String, usize)> = vec![];
    for t in &ts {
        for r in 1..=runs {
            combos.push((t.clone(), r));
        }
    }
    let n = combos.len();
    println!("matrix {id} · {{{tools}}} × cli={cli} × model={model} × via={via} × runs={runs} = {n} runs · jobs={jobs} · isolate={isolate} · prepared={prepared}");
    if tag.is_some() || note.is_some() {
        println!(
            "  meta: tag={} note={}",
            tag.as_deref().unwrap_or("-"),
            note.as_deref().unwrap_or("-")
        );
    }

    if args.iter().any(|s| s == "--force") {
        std::env::set_var("MONOBENCH_FORCE", "1");
    }
    if prepared {
        let prep_tools: Vec<String> = ts.iter().filter(|t| *t != "baseline").cloned().collect();
        if !prep_tools.is_empty() {
            let prep_rc = run::prepare(root, &id, &prep_tools, &Mutex::new(()));
            if prep_rc != 0 {
                die("prepare failed");
            }
        }
        std::env::set_var("MONOBENCH_PREPARED", "1");
    } else {
        std::env::remove_var("MONOBENCH_PREPARED");
    }
    if isolate == "worktree" {
        std::env::set_var("MONOBENCH_ISOLATE", "worktree");
    } else {
        std::env::remove_var("MONOBENCH_ISOLATE");
    } // constant across threads ⇒ no race
      // Graceful stop: workers check this file BEFORE pulling each job. Without it,
      // killing the claude workers (e.g. for CPU) just makes each worker pop the next
      // combo and respawn claude — the matrix never relents until the queue drains.
      // `monobench stop` writes this file → workers finish their current run, launch
      // no more, matrix exits cleanly. (Pure std; no signal-handler crate.)
    let stopf = root.join(".matrix-stop");
    let _ = std::fs::remove_file(&stopf); // clear any stale stop from a prior run
    println!("  (pid {} · stop cleanly with `monobench stop` — workers finish current run, launch no more)", std::process::id());
    write_matrix_state(
        root,
        std::process::id(),
        &id,
        combos.len(),
        jobs,
        &cli,
        &model,
    );
    let wtlock = Arc::new(Mutex::new(())); // serializes git-worktree add/remove
    let queue = Arc::new(Mutex::new(
        combos
            .into_iter()
            .collect::<std::collections::VecDeque<_>>(),
    ));
    let root_arc = Arc::new(root.to_path_buf());
    let stop_arc = Arc::new(stopf.clone());
    let mut handles = vec![];
    for _ in 0..jobs.max(1) {
        let q = Arc::clone(&queue);
        let lock = Arc::clone(&wtlock);
        let id2 = id.clone();
        let cli2 = cli.clone();
        let model2 = model.clone();
        let via2 = via.clone();
        let tag2 = tag.clone();
        let note2 = note.clone();
        let r2 = Arc::clone(&root_arc);
        let stop = Arc::clone(&stop_arc);
        handles.push(std::thread::spawn(move || loop {
            if stop.exists() {
                break;
            } // graceful stop — do not pull a new job
            let Some((t, r)) = ({
                let mut g = q.lock().unwrap();
                g.pop_front()
            }) else {
                break;
            };
            run::run(
                &r2,
                &id2,
                &t,
                &cli2,
                &model2,
                &via2,
                r,
                Some(runs),
                tag2.as_deref(),
                note2.as_deref(),
                true,
                &lock,
            ); // quiet=true; matrix prints ✓ + final report
            println!("  ✓ {t} / {cli2} / {model2} r{r}");
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    let stopped = stopf.exists();
    let _ = std::fs::remove_file(&stopf);
    clear_matrix_state(root);
    println!(
        "{}",
        if stopped {
            "── matrix stopped (`monobench stop`) — remaining queue skipped ──"
        } else {
            "── matrix done ──"
        }
    );
    report::report(root, &id, &gather_runs(root, &id));
    adoption::adoption(&id, &telemetry_paths(&root.join("results").join(&id)));
}

// Cross-instance sweep: run MANY instances in ONE process with a per-repo lock map.
// Each repo (bun, cpython, …) gets its own Mutex, so same-repo `git worktree add`s
// serialize on their shared base (safe) while different repos run in parallel — overlapping
// the slow cpython/node first-clones. The worktree lock is an injected `&Mutex` at run::run,
// so this reuses the proven matrix worker path with no change to run.rs (beyond exposing
// repo_basename). Cross-process safety is unneeded: it's one process, like `--jobs`.
fn run_sweep(root: &Path, args: &[String]) {
    let usage = "usage: monobench sweep <id,id,...|--all> [--tools a,b] [--cli c] [--model x] [--via direct|niia] [--runs N] [--jobs J] [--prepared]";
    let first = args.get(1).cloned().unwrap_or_default();
    if first.is_empty() || (first.starts_with("--") && first != "--all") {
        die(usage);
    }
    let (mut tools, mut model, mut cli_arg, mut via_arg, mut runs, mut jobs) = (
        "baseline".to_string(),
        std::env::var("MONOBENCH_MODEL").unwrap_or_else(|_| "haiku".into()),
        None::<String>,
        None::<String>,
        1usize,
        3usize,
    );
    let mut tag = None::<String>;
    let mut note = None::<String>;
    let mut prepared = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--tools" => {
                tools = args.get(i + 1).cloned().unwrap_or(tools);
                i += 2;
            }
            "--model" | "--models" => {
                model = args.get(i + 1).cloned().unwrap_or(model);
                i += 2;
            }
            "--cli" => {
                cli_arg = args.get(i + 1).cloned();
                i += 2;
            }
            "--via" => {
                via_arg = args.get(i + 1).cloned();
                i += 2;
            }
            "--effort" => {
                if let Some(e) = args.get(i + 1) {
                    std::env::set_var("MONOBENCH_EFFORT", e);
                }
                i += 2;
            }
            "--runs" => {
                runs = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(runs);
                i += 2;
            }
            "--jobs" => {
                jobs = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(jobs);
                i += 2;
            }
            "--tag" | "--batch" => {
                tag = args.get(i + 1).cloned();
                i += 2;
            }
            "--note" | "--memo" => {
                note = args.get(i + 1).cloned();
                i += 2;
            }
            "--prepared" => {
                prepared = true;
                i += 1;
            }
            _ => i += 1,
        }
    }
    // instance list: explicit comma list, or every instance dir (skip _TEMPLATE) for --all
    let ids: Vec<String> = if first == "--all" {
        let mut v: Vec<String> = std::fs::read_dir(root.join("instances"))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| !n.starts_with('_'))
            .filter(|n| {
                root.join("instances")
                    .join(n)
                    .join("instance.json")
                    .is_file()
            })
            .collect();
        v.sort();
        v
    } else {
        first
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    };
    if ids.is_empty() {
        die("no instances selected");
    }
    let ts: Vec<String> = tools
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    let ms: Vec<String> = model
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    if ms.len() != 1 {
        die("sweep accepts exactly one model per command; repeat the sweep once per model");
    }
    let model = ms[0].clone();
    let (cli, via) = axes_for(&model, cli_arg, via_arg);

    // Resolve each id's repo and build the per-repo lock map. ids without a repo are skipped.
    // Same base ⇒ same Mutex ⇒ worktree adds serialize; different base ⇒ different Mutex ⇒ parallel.
    let mut repo_of: HashMap<String, String> = HashMap::new();
    let mut locks: HashMap<String, Arc<Mutex<()>>> = HashMap::new();
    let mut runnable: Vec<String> = vec![];
    for id in &ids {
        let ij = read_json(&root.join("instances").join(id).join("instance.json"));
        match ij.get("repo").and_then(serde_json::Value::as_str) {
            Some(repo) if !repo.is_empty() => {
                let base = run::repo_basename(repo);
                locks
                    .entry(base.clone())
                    .or_insert_with(|| Arc::new(Mutex::new(())));
                repo_of.insert(id.clone(), base);
                runnable.push(id.clone());
            }
            _ => eprintln!("  skip {id}: instance.json has no repo"),
        }
    }
    if runnable.is_empty() {
        die("no runnable instances (none had a repo)");
    }

    // Build (id, repo, tool, run) jobs, then interleave round-robin across repos so the worker
    // pool spans different bases first — the cross-repo overlap that hides the slow clones.
    let mut by_repo: std::collections::BTreeMap<
        String,
        std::collections::VecDeque<(String, String, String, usize)>,
    > = Default::default();
    for id in &runnable {
        let base = repo_of[id].clone();
        for t in &ts {
            for r in 1..=runs {
                by_repo.entry(base.clone()).or_default().push_back((
                    id.clone(),
                    base.clone(),
                    t.clone(),
                    r,
                ));
            }
        }
    }
    let mut ordered: Vec<(String, String, String, usize)> = vec![];
    let mut any = true;
    while any {
        any = false;
        for q in by_repo.values_mut() {
            if let Some(j) = q.pop_front() {
                ordered.push(j);
                any = true;
            }
        }
    }
    let n = ordered.len();
    println!(
        "sweep {} instance(s) over {} repo(s) · {{{tools}}} × cli={cli} × model={model} × via={via} × runs={runs} = {n} runs · jobs={jobs}",
        runnable.len(),
        locks.len()
    );
    println!("  per-repo lock: same-repo worktree-adds serialize, different repos run in parallel");

    if prepared {
        let prep_tools: Vec<String> = ts.iter().filter(|t| *t != "baseline").cloned().collect();
        if !prep_tools.is_empty() {
            for id in &runnable {
                if run::prepare(root, id, &prep_tools, &Mutex::new(())) != 0 {
                    die(&format!("prepare failed for {id}"));
                }
            }
        }
        std::env::set_var("MONOBENCH_PREPARED", "1");
    } else {
        std::env::remove_var("MONOBENCH_PREPARED");
    }
    std::env::set_var("MONOBENCH_ISOLATE", "worktree"); // constant across threads ⇒ no race

    // sweep owns a dedicated stop file, kept separate from the matrix's .matrix-stop so a
    // concurrent matrix is never affected (`monobench stop` writes both). Workers poll it
    // before pulling each job — graceful drain, same contract as the matrix.
    let stopf = root.join(".sweep-stop");
    let _ = std::fs::remove_file(&stopf);
    println!(
        "  (pid {} · stop cleanly with `monobench stop` — workers finish current run, launch no more)",
        std::process::id()
    );

    let queue = Arc::new(Mutex::new(
        ordered
            .into_iter()
            .collect::<std::collections::VecDeque<_>>(),
    ));
    let locks = Arc::new(locks);
    let root_arc = Arc::new(root.to_path_buf());
    let stop_arc = Arc::new(stopf.clone());
    let mut handles = vec![];
    for _ in 0..jobs.max(1) {
        let q = Arc::clone(&queue);
        let lk = Arc::clone(&locks);
        let cli2 = cli.clone();
        let model2 = model.clone();
        let via2 = via.clone();
        let tag2 = tag.clone();
        let note2 = note.clone();
        let r2 = Arc::clone(&root_arc);
        let stop = Arc::clone(&stop_arc);
        handles.push(std::thread::spawn(move || loop {
            if stop.exists() {
                break;
            } // graceful stop — do not pull a new job
            let Some((id, base, t, r)) = ({
                let mut g = q.lock().unwrap();
                g.pop_front()
            }) else {
                break;
            };
            let lock: &Mutex<()> = lk.get(&base).expect("per-repo lock built for every job");
            run::run(
                &r2,
                &id,
                &t,
                &cli2,
                &model2,
                &via2,
                r,
                Some(runs),
                tag2.as_deref(),
                note2.as_deref(),
                true,
                lock,
            ); // quiet=true
            println!("  ✓ {id} / {t} / {cli2} / {model2} r{r}");
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    let stopped = stopf.exists();
    let _ = std::fs::remove_file(&stopf);
    println!(
        "{}",
        if stopped {
            "── sweep stopped (`monobench stop`) — remaining queue skipped ──"
        } else {
            "── sweep done ──"
        }
    );
    println!("[NEXT]  monobench summary        # grades + cost/tokens across all swept instances");
}

#[cfg(test)]
mod main_tests {
    use super::*;

    #[test]
    fn idle_label_flags_stall_past_threshold() {
        assert_eq!(idle_label(None), "");
        assert_eq!(idle_label(Some(3)), " · idle 3s");
        // a working run just below the threshold has no warning
        assert_eq!(
            idle_label(Some(IDLE_WARN_SECS - 1)),
            format!(" · idle {}s", IDLE_WARN_SECS - 1)
        );
        // no streaming output for >= the threshold ⇒ flagged as likely hung
        assert!(idle_label(Some(IDLE_WARN_SECS)).contains("⚠ idle"));
        assert!(idle_label(Some(900)).contains("⚠ idle 900s"));
    }

    #[test]
    fn active_counts_ignores_shells_that_merely_mention_solvers() {
        let mk = |cmd: &str| ProcInfo {
            pid: "1".into(),
            ppid: "0".into(),
            etime: "0:05".into(),
            cmd: cmd.into(),
        };
        let rows = vec![
            mk("node /Users/x/bin/codex exec -C /tmp/wt -m gpt-5.3"), // real codex
            mk("/bin/zsh -c 'grep codex exec results/'"),             // shell merely mentioning it
            mk("grep -E codex exec|claude -p|monogram index"),        // a search tool
            mk("monogram index . --ext zig,cpp"),                     // real index
            mk("claude -p --model haiku"),                            // real claude
            mk("agy --print --dangerously-skip-permissions"),         // real agy
        ];
        let (_runs, index, claude, codex, agy) = active_counts(&rows);
        assert_eq!(
            (index, claude, codex, agy),
            (1, 1, 1, 1),
            "count only real solver processes, not shells/greps that mention the strings"
        );
    }

    #[test]
    fn dead_run_pid_flags_stale_marker_from_killed_run() {
        let dir = std::env::temp_dir().join(format!("monobench-dead-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // no marker ⇒ not a crash case
        assert!(dead_run_pid(&dir, "r").is_none());
        // marker with a live pid (ourselves) ⇒ still running, not crashed
        std::fs::write(
            dir.join("r.running"),
            format!("pid={} tool=monogram\n", std::process::id()),
        )
        .unwrap();
        assert!(dead_run_pid(&dir, "r").is_none());
        // marker with a dead pid ⇒ crashed (RAII never ran)
        std::fs::write(
            dir.join("r.running"),
            "pid=999999999 tool=monogram cli=codex\n",
        )
        .unwrap();
        assert_eq!(dead_run_pid(&dir, "r"), Some(999999999));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn live_output_age_reads_freshest_streaming_log() {
        let dir = std::env::temp_dir().join(format!("monobench-idle-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // no logs yet ⇒ None (run hasn't produced output)
        assert!(live_output_age(&dir, "r").is_none());
        std::fs::write(dir.join("r.err"), "stream\n").unwrap();
        // a just-written log ⇒ a small age (freshest output)
        assert!(live_output_age(&dir, "r").unwrap() < 5);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn matrix_state_round_trips_live_and_self_heals_stale() {
        let root = std::env::temp_dir().join(format!("monobench-mstate-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        // A live matrix (our own pid) round-trips with id + cli/model in the line.
        write_matrix_state(
            &root,
            std::process::id(),
            "inst-x",
            5,
            2,
            "codex",
            "gpt-5.3",
        );
        let got = read_matrix_state(&root).expect("live state should read back");
        assert_eq!(got.0, "inst-x");
        assert!(got.1.contains("inst-x") && got.1.contains("codex/gpt-5.3"));
        // A stale state (dead pid) returns None and self-clears the file.
        std::fs::write(
            matrix_state_path(&root),
            serde_json::json!({"pid": 999_999_999u64, "id": "inst-x"}).to_string(),
        )
        .unwrap();
        assert!(read_matrix_state(&root).is_none());
        assert!(
            !matrix_state_path(&root).exists(),
            "stale state must self-clear"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn parse_duration_secs_handles_units_and_bare_seconds() {
        assert_eq!(parse_duration_secs("9h"), Some(9 * 3600));
        assert_eq!(parse_duration_secs("30m"), Some(1800));
        assert_eq!(parse_duration_secs("2d"), Some(2 * 86400));
        assert_eq!(parse_duration_secs("45s"), Some(45));
        assert_eq!(parse_duration_secs("3600"), Some(3600)); // bare = seconds
        assert_eq!(parse_duration_secs(" 9h "), Some(9 * 3600)); // trims
        assert_eq!(parse_duration_secs("h"), None); // no number
        assert_eq!(parse_duration_secs("abc"), None);
        assert_eq!(parse_duration_secs(""), None);
    }

    #[test]
    fn run_start_ms_reads_timestamped_label_only() {
        // Timestamped label: the trailing -t<epoch_ms> is the run's own start time.
        assert_eq!(
            run_start_ms("monogram-codex-gpt-5.3-codex-high-r1-t1779639626850"),
            Some(1779639626850)
        );
        // Legacy label with no -t suffix ⇒ None (caller falls back to file mtime).
        assert_eq!(run_start_ms("baseline-claude-opus-4.1-high-r2"), None);
        // A -t that is not all-digits is not a timestamp.
        assert_eq!(run_start_ms("baseline-claude-test-r1"), None);
    }

    #[test]
    fn run_since_keeps_recent_label_drops_old_one() {
        // Label-based path is pure (no filesystem needed when -t<ms> is present).
        let root = std::env::temp_dir();
        let recent = "monogram-codex-gpt-5.3-high-r1-t2000000000000"; // ~2033, far future
        let ancient = "monogram-codex-gpt-5.3-high-r1-t1000000000000"; // ~2001
        assert!(run_since(&root, "noinst", recent, 1_500_000_000));
        assert!(!run_since(&root, "noinst", ancient, 1_500_000_000));
    }

    #[test]
    fn infers_root_from_result_artifact_path_in_process_command() {
        let cmd = "codex exec -o /tmp/bench-root/results/case/a.answer.txt";
        assert_eq!(
            root_from_results_path(cmd),
            Some(PathBuf::from("/tmp/bench-root"))
        );
    }

    #[test]
    fn active_controllers_include_matrix_processes() {
        let rows = vec![ProcInfo {
            pid: "10".into(),
            ppid: "1".into(),
            etime: "00:01".into(),
            cmd: "./target/debug/monobench matrix bun-case --tools monogram".into(),
        }];
        assert_eq!(active_run_procs(&rows, Some("bun-case")).len(), 1);
        assert_eq!(active_run_procs(&rows, Some("other-case")).len(), 0);
    }

    #[test]
    fn run_positional_text_becomes_note_without_breaking_legacy_number() {
        let args = vec![
            "run".into(),
            "case".into(),
            "monogram".into(),
            "2".into(),
            "lock fix rerun".into(),
            "--cli".into(),
            "codex".into(),
            "--tag".into(),
            "lockfix".into(),
        ];
        let (n, note) = run_number_and_positional_note(&args, 3);
        assert_eq!(n, 2);
        assert_eq!(note.as_deref(), Some("lock fix rerun"));
    }

    #[test]
    fn positional_note_skips_flag_values() {
        let args = vec![
            "note".into(),
            "case".into(),
            "runid".into(),
            "manual analysis".into(),
            "--tag".into(),
            "suspect".into(),
            "after lock fix".into(),
        ];
        assert_eq!(
            positional_note(&args, 3).as_deref(),
            Some("manual analysis after lock fix")
        );
    }

    #[test]
    fn preferred_event_path_preserves_dotted_run_labels() {
        let root = std::env::temp_dir().join(format!(
            "monobench-test-{}-{}",
            std::process::id(),
            "dotted"
        ));
        let id = "case";
        let run = "monogram-agy-gemini-3.5-flash-medium-medium-r2-t1";
        let dir = root.join("results").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        let event = dir.join(format!("{run}.agy.jsonl"));
        std::fs::write(&event, b"").unwrap();
        assert_eq!(preferred_event_path(&root, id, run), Some(event));
        let _ = std::fs::remove_dir_all(root);
    }
}
