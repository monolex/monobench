use crate::meter;
use serde_json::Value;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

fn niia() -> String {
    std::env::var("MONOBENCH_NIIA_BIN").unwrap_or_else(|_| "niia".into())
}

fn run_text(args: &[&str]) -> Result<String, String> {
    let out = Command::new(niia())
        .args(args)
        .output()
        .map_err(|e| format!("niia {}: {e}", args.join(" ")))?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn run_status(args: &[&str]) -> Result<(), String> {
    let status = Command::new(niia())
        .args(args)
        .status()
        .map_err(|e| format!("niia {}: {e}", args.join(" ")))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("niia {} exited with {status}", args.join(" ")))
    }
}

fn live_session() -> Result<String, String> {
    if let Ok(s) = std::env::var("MONOBENCH_NIIA_SESSION") {
        if !s.trim().is_empty() {
            return Ok(s);
        }
    }
    let list = run_text(&["serve", "--list"])?;
    let candidates = parse_session_candidates(&list);
    if candidates.is_empty() {
        return Err("no niia headless sessions - start one with: niia serve".into());
    }
    // `serve --list` reports EVERY registered session, but most are detached zombies that silently
    // drop writes (get-answer → "NOT_ATTACHED: no live instance renders it"). The old code took the
    // first id, which usually hit a zombie — the command went into a void and the run produced an
    // empty answer. Probe and return the first session a live instance actually renders.
    for s in &candidates {
        if session_attached(s) {
            return Ok(s.clone());
        }
    }
    Err(format!(
        "no ATTACHED niia session among {} registered (all detached) - collapse with: niia serve --stop && niia serve",
        candidates.len()
    ))
}

fn parse_session_candidates(list: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in list.lines() {
        for word in line.split_whitespace() {
            let w = word.trim_matches(|c: char| c == ',' || c == ':' || c == ';');
            if w.starts_with("niia-") && !out.iter().any(|x| x == w) {
                out.push(w.to_string());
            }
        }
    }
    out
}

// A detached session answers get-answer with "NOT_ATTACHED"; a live one returns its screen (or
// "No match found"). Read-only probe — no writes, safe to run against any candidate. The
// NOT_ATTACHED warning is logged to STDERR, so we must inspect BOTH streams (run_text captures
// only stdout, which is why the first cut of this silently picked dead sessions).
fn session_attached(session: &str) -> bool {
    match Command::new(niia())
        .args(["get-answer", "--session", session, "MONOBENCH_PROBE"])
        .output()
    {
        Ok(o) => {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            !combined.contains("NOT_ATTACHED")
        }
        Err(_) => false,
    }
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
    words
        .iter()
        .map(|w| {
            if w.chars().all(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/' | '.' | '=' | ':')
            }) {
                w.clone()
            } else {
                shell_quote(w)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_words(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}

fn has_arg(words: &[String], short: &str, long: &str) -> bool {
    words.iter().any(|w| w == short || w == long)
}

fn command_words(cli: &str, model: &str, effort: &str) -> Vec<String> {
    let override_cmd = std::env::var("MONOBENCH_CLI")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let mut words = override_cmd
        .as_deref()
        .map(split_words)
        .unwrap_or_else(|| vec![cli.to_string()]);
    if words.is_empty() {
        words.push(cli.to_string());
    }
    match words.first().map(String::as_str) {
        Some("claude") => {
            if !model.is_empty() && !has_arg(&words, "-m", "--model") {
                words.push("--model".into());
                words.push(model.into());
            }
            if !effort.is_empty() {
                words.push("--effort".into());
                words.push(effort.into());
            }
        }
        Some("codex") => {
            if !model.is_empty() && !has_arg(&words, "-m", "--model") {
                words.push("-m".into());
                words.push(model.into());
            }
            if !effort.is_empty() {
                words.push("-c".into());
                words.push(format!("model_reasoning_effort={effort}"));
            }
        }
        Some("agy") => {
            if !words.iter().any(|w| w == "--dangerously-skip-permissions") {
                words.push("--dangerously-skip-permissions".into());
            }
        }
        _ => {}
    }
    words
}

fn spawn_command(cli: &str, model: &str, effort: &str) -> String {
    shell_join(&command_words(cli, model, effort))
}

fn result_artifact(out_prefix: &Path, suffix: &str) -> PathBuf {
    let file_name = out_prefix
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("run");
    out_prefix.with_file_name(format!("{file_name}.{suffix}"))
}

fn has_option(words: &[String], opt: &str) -> bool {
    words
        .iter()
        .any(|w| w == opt || w.starts_with(&format!("{opt}=")))
}

fn agy_timeout() -> Duration {
    parse_go_duration(&std::env::var("MONOBENCH_AGY_TIMEOUT").unwrap_or_else(|_| "20m".into()))
        .unwrap_or_else(|| Duration::from_secs(20 * 60))
}

// Parse the simple Go-style durations monobench uses for --print-timeout ("20m", "15m", "90s",
// "1h", or bare seconds) into a Duration for the niia completion-poll ceiling. Best-effort: an
// unrecognized value falls back to the caller's default.
fn parse_go_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if let Ok(n) = s.parse::<u64>() {
        return Some(Duration::from_secs(n));
    }
    let (num, mult) = if let Some(n) = s.strip_suffix('h') {
        (n, 3600)
    } else if let Some(n) = s.strip_suffix('m') {
        (n, 60)
    } else if let Some(n) = s.strip_suffix('s') {
        (n, 1)
    } else {
        return None;
    };
    num.trim()
        .parse::<u64>()
        .ok()
        .map(|n| Duration::from_secs(n * mult))
}

fn agy_print_command(
    prompt_file: &Path,
    out_prefix: &Path,
    repo: &Path,
    jail: Option<&Path>,
    model: &str,
    effort: &str,
) -> String {
    let mut words = command_words("agy", model, effort);
    // agy ignores the shell's cwd (it runs in ~/.gemini/antigravity-cli/scratch), so the repo
    // under test must be handed to it explicitly or it sees 0 files and roams the filesystem.
    if !has_option(&words, "--add-dir") {
        words.push("--add-dir".into());
        words.push(repo.to_string_lossy().into_owned());
    }
    if !has_option(&words, "--log-file") {
        words.push("--log-file".into());
        words.push(
            result_artifact(out_prefix, "agy.log")
                .to_string_lossy()
                .into_owned(),
        );
    }
    if !has_option(&words, "--print-timeout") {
        words.push("--print-timeout".into());
        words.push(std::env::var("MONOBENCH_AGY_TIMEOUT").unwrap_or_else(|_| "20m".into()));
    }
    // macOS read-jail prefix: agy still runs + reads the repo, but cannot read the benchmark
    // answer files (root/instances|research). None ⇒ run unwrapped (non-macOS / profile failed).
    let prefix = match jail {
        Some(p) => format!("sandbox-exec -f {} ", shell_quote(&p.to_string_lossy())),
        None => String::new(),
    };
    if !has_arg(&words, "-p", "--print") && !has_option(&words, "--prompt") {
        let mut cmd = format!("{prefix}{}", shell_join(&words));
        cmd.push_str(" --print ");
        cmd.push_str(&format!(
            "\"$(cat {})\"",
            shell_quote(&prompt_file.to_string_lossy())
        ));
        return cmd;
    }
    format!("{prefix}{}", shell_join(&words))
}

fn agy_prompt_file(out_prefix: &Path) -> PathBuf {
    std::env::temp_dir().join(format!("monobench-niia-agy-prompt-{}.txt", slug(out_prefix)))
}

// Filesystem-safe slug from a run's out_prefix file name (the runid). Shared by the capture
// marker, the agy completion sentinel, and the sandbox-jail profile filename.
fn slug(out_prefix: &Path) -> String {
    out_prefix
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("run")
        .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
}

fn install_git_deny_wrapper(tag: &str) -> Option<PathBuf> {
    let dir = std::env::temp_dir().join(format!(
        "monobench-niia-no-git-{tag}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).ok()?;
    let git = dir.join("git");
    std::fs::write(&git, "#!/bin/sh\necho 'monobench: git is disabled during solver runs (anti-contamination)' >&2\nexit 126\n").ok()?;
    #[cfg(unix)]
    {
        let mut perm = std::fs::metadata(&git).ok()?.permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&git, perm).ok()?;
    }
    Some(dir)
}

fn flatten_prompt(prompt: &str) -> String {
    prompt.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn filtered_answer(raw: &str) -> String {
    raw.lines()
        .filter(|line| {
            let lc = line.to_lowercase();
            !line.trim_start().starts_with("[INFO]")
                && !line.contains("MONOBENCH_CAPTURE_")
                && !lc.contains("next")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

fn collect_jsonl(dir: &Path, out: &mut Vec<(PathBuf, SystemTime)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
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
    files
        .iter()
        .rev()
        .find(|(_, modified)| *modified >= since)
        .or_else(|| files.last())
        .map(|(path, _)| path.clone())
}

fn parse_agy_conversation_id(log_path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(log_path).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.split("Print mode: conversation=").nth(1) {
            let cid = rest
                .split(|c: char| c == ',' || c.is_whitespace())
                .next()
                .unwrap_or("")
                .trim();
            if !cid.is_empty() {
                return Some(cid.to_string());
            }
        }
    }
    None
}

fn parse_agy_observed_model(log_path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(log_path).ok()?;
    for line in text.lines().rev() {
        let Some(rest) = line
            .split("Propagating selected model override to backend: label=")
            .nth(1)
        else {
            continue;
        };
        let rest = rest.trim();
        let model = if let Some(quoted) = rest.strip_prefix('"') {
            quoted.split('"').next().unwrap_or("").trim()
        } else {
            rest.split_whitespace().next().unwrap_or("").trim()
        };
        if !model.is_empty() {
            return Some(model.to_string());
        }
    }
    None
}

// Extract agy's answer from its structured transcript (the per-CLI "via-system" source). The
// agent's output is a stream of `PLANNER_RESPONSE` events; concatenating their `content` in order
// reproduces the same narration+analysis the direct runner captures from agy's stdout. None ⇒ no
// transcript yet / no response, so the caller falls back to a terminal scrape.
fn agy_answer_from_transcript(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut parts = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(o) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if o.get("type").and_then(Value::as_str) == Some("PLANNER_RESPONSE") {
            if let Some(c) = o.get("content").and_then(Value::as_str) {
                if !c.trim().is_empty() {
                    parts.push(c.to_string());
                }
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

fn agy_transcript_path(cid: &str) -> Option<PathBuf> {
    let p = home_dir()?
        .join(".gemini/antigravity-cli/brain")
        .join(cid)
        .join(".system_generated/logs/transcript_full.jsonl");
    if p.is_file() {
        Some(p)
    } else {
        None
    }
}

fn wait_for_agy_transcript(cid: &str) -> Option<PathBuf> {
    for _ in 0..20 {
        if let Some(p) = agy_transcript_path(cid) {
            return Some(p);
        }
        thread::sleep(Duration::from_millis(500));
    }
    None
}

fn enrich_cost(meter_json: &mut Value) {
    let sid = meter_json
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if sid.is_empty() {
        return;
    }
    Command::new("monometer")
        .args(["daemon", "recompute"])
        .output()
        .ok();
    thread::sleep(Duration::from_secs(2));
    let out = Command::new("monometer")
        .args(["sessions", "--recent", "40", "--json"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let Ok(v) = serde_json::from_str::<Value>(&out) else {
        return;
    };
    let Some(arr) = v.as_array() else { return };
    let Some(cost) = arr
        .iter()
        .find(|x| x.get("session_id").and_then(Value::as_str) == Some(&sid))
        .and_then(|x| x.get("cost_usd").and_then(Value::as_f64))
    else {
        return;
    };
    meter_json["cost_usd"] = serde_json::json!(cost);
}

pub fn run(
    root: &Path,
    repo: &Path,
    prompt: &str,
    marker: &str,
    out_prefix: &Path,
    effort: &str,
    cli: &str,
    model: &str,
) -> Result<(), String> {
    let session = live_session()?;
    let run_slug = slug(out_prefix);
    let agy_prompt = if cli == "agy" {
        let path = agy_prompt_file(out_prefix);
        std::fs::write(&path, prompt).map_err(|e| format!("write agy prompt: {e}"))?;
        Some(path)
    } else {
        None
    };
    // Once the niia path actually waits for agy (below), agy runs for real and — like the direct
    // path — would otherwise roam and read the answer files. Jail its reads the same way.
    let agy_jail = if cli == "agy" {
        crate::run::agy_read_jail_profile(root, &run_slug)
    } else {
        None
    };
    let spawn = if cli == "agy" {
        agy_print_command(
            agy_prompt.as_deref().unwrap(),
            out_prefix,
            repo,
            agy_jail.as_deref(),
            model,
            effort,
        )
    } else {
        spawn_command(cli, model, effort)
    };
    println!(
        "▶ niia runner · session={session} · spawn='{spawn}' · repo={}",
        repo.display()
    );
    let t0 = Instant::now();

    write(
        &session,
        &format!("cd {}\r", shell_quote(&repo.to_string_lossy())),
    )?;
    wait_idle(&session)?;
    let since = SystemTime::now();
    let git_deny = install_git_deny_wrapper(
        out_prefix
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("run"),
    );
    if let Some(dir) = &git_deny {
        write(
            &session,
            &format!(
                "__MONOBENCH_OLD_PATH=\"$PATH\"; export PATH={}:\"$PATH\"\r",
                shell_quote(&dir.to_string_lossy())
            ),
        )?;
        wait_idle(&session)?;
    }

    let capture_marker = format!("MONOBENCH_CAPTURE_{run_slug}");
    write(&session, &format!("echo {capture_marker}\r"))?;
    wait_idle(&session)?;

    if cli == "agy" {
        // `agy --print` is BLOCKING and non-streaming: it sits silent for minutes while the model
        // reasons server-side, then prints the answer and exits. The old fixed `sleep(1)` +
        // wait_idle fired during that initial silence and captured an empty screen (the 3s /
        // 0-byte bug). Append a shell command that touches a sentinel FILE only after agy exits,
        // and poll the filesystem for it — robust against terminal scroll (a screen marker can
        // scroll off on long output and never be seen). Same-machine session ⇒ same filesystem.
        let done_file = std::env::temp_dir().join(format!("monobench-niia-done-{run_slug}"));
        let _ = std::fs::remove_file(&done_file);
        write(
            &session,
            &format!(
                "{spawn}; : > {}\r",
                shell_quote(&done_file.to_string_lossy())
            ),
        )?;
        let ceiling = agy_timeout() + Duration::from_secs(90);
        let start = Instant::now();
        loop {
            if done_file.exists() || start.elapsed() >= ceiling {
                break;
            }
            thread::sleep(Duration::from_secs(3));
        }
        let _ = std::fs::remove_file(&done_file);
        wait_idle(&session)?;
    } else {
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
    }

    // Answer extraction is PER-CLI, from each CLI's own structured output — NOT a generic terminal
    // screen-scrape. `agy --print` renders TUI-style control output to a PTY that get-answer can't
    // reconstruct (it captured only the typed command line), but agy writes the real analysis to
    // its transcript. This mirrors the direct runner, which already reads each CLI's structured
    // output (claude stream-json, codex -o file, agy stdout). get-answer remains the fallback.
    if cli == "agy" {
        let transcript = result_artifact(out_prefix, "agy.jsonl");
        if let Some(cid) = parse_agy_conversation_id(&result_artifact(out_prefix, "agy.log")) {
            if let Some(src) = wait_for_agy_transcript(&cid) {
                let _ = std::fs::copy(src, &transcript);
            }
        }
        let (answer, answer_source) = match agy_answer_from_transcript(&transcript) {
            Some(a) => (a, "transcript"),
            None => (
                run_text(&["get-answer", "--session", &session, &capture_marker])
                    .unwrap_or_default(),
                "screen",
            ),
        };
        std::fs::write(
            result_artifact(out_prefix, "answer.txt"),
            filtered_answer(&answer),
        )
        .map_err(|e| format!("write answer: {e}"))?;

        let observed_model = parse_agy_observed_model(&result_artifact(out_prefix, "agy.log"));
        // Verified against the requested label (preflight already refused settings≠label mismatch).
        let model_verified = observed_model
            .as_deref()
            .map(|o| crate::run::agy_model_norm(o) == crate::run::agy_model_norm(model))
            .unwrap_or(false);
        let m = serde_json::json!({
            "runner": "niia",
            "cli": cli,
            "model": model,
            "requested_model": model,
            "requested_effort": effort,
            "observed_model": observed_model,
            "model_enforced": model_verified,
            "effort_enforced": false,
            "tokens": null,
            "cost_usd": null,
            "tokens_available": false,
            "cost_available": false,
            "duration_s": t0.elapsed().as_secs(),
            "answer_source": answer_source,
            "meter_error": "agy via-niia token/cost telemetry unavailable; answer from transcript"
        });
        std::fs::write(result_artifact(out_prefix, "meter.json"), m.to_string())
            .map_err(|e| format!("write meter: {e}"))?;
    } else {
        let raw = run_text(&["get-answer", "--session", &session, &capture_marker])
            .or_else(|_| run_text(&["get-answer", "--session", &session, marker]))?;
        std::fs::write(
            result_artifact(out_prefix, "answer.txt"),
            filtered_answer(&raw),
        )
        .map_err(|e| format!("write answer: {e}"))?;
        if let Some(jsonl) = newest_claude_jsonl(since) {
            let mut m = meter::meter_json(&jsonl.to_string_lossy());
            enrich_cost(&mut m);
            std::fs::write(result_artifact(out_prefix, "meter.json"), m.to_string())
                .map_err(|e| format!("write meter: {e}"))?;
        }
    }

    write(&session, "\x03").ok();
    thread::sleep(Duration::from_secs(1));
    write(&session, "\x03").ok();
    write(&session, "if [ -n \"$__MONOBENCH_OLD_PATH\" ]; then export PATH=\"$__MONOBENCH_OLD_PATH\"; unset __MONOBENCH_OLD_PATH; fi\r").ok();
    if let Some(path) = agy_prompt {
        std::fs::remove_file(path).ok();
    }
    println!("  answer -> {}.answer.txt", out_prefix.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_session_candidates() {
        // Collect every registered session id (across [ws] and [disconnected] lines), de-duped,
        // in list order — live_session() then probes them for the attached one.
        let s = "PID 1 [ws]  session: niia-123, niia-456\nPID 2 [disconnected] session: niia-789, niia-123";
        assert_eq!(
            parse_session_candidates(s),
            vec!["niia-123", "niia-456", "niia-789"]
        );
    }

    #[test]
    fn filters_answer_noise() {
        let raw = "[INFO] start\nROOTCAUSE: x\nNEXT do y\nFIX: z";
        assert_eq!(filtered_answer(raw), "ROOTCAUSE: x\nFIX: z");
    }

    #[test]
    fn builds_spawn_command() {
        std::env::set_var("MONOBENCH_CLI", "codex");
        assert_eq!(
            spawn_command("codex", "gpt-5.4-mini", "low"),
            "codex -m gpt-5.4-mini -c model_reasoning_effort=low"
        );
        std::env::remove_var("MONOBENCH_CLI");
    }

    #[test]
    fn agy_spawn_skips_permission_prompts() {
        std::env::remove_var("MONOBENCH_CLI");
        assert_eq!(
            spawn_command("agy", "gemini-3.5-flash-medium", "medium"),
            "agy --dangerously-skip-permissions"
        );

        std::env::set_var("MONOBENCH_CLI", "agy --dangerously-skip-permissions");
        assert_eq!(
            spawn_command("agy", "gemini-3.5-flash-medium", "medium"),
            "agy --dangerously-skip-permissions"
        );
        std::env::remove_var("MONOBENCH_CLI");
    }

    #[test]
    fn result_artifact_preserves_dotted_run_ids() {
        let p = Path::new("/tmp/monogram-agy-gemini-3.5-flash-medium-r1-t123");
        assert_eq!(
            result_artifact(p, "answer.txt"),
            PathBuf::from("/tmp/monogram-agy-gemini-3.5-flash-medium-r1-t123.answer.txt")
        );
    }

    #[test]
    fn agy_print_command_preserves_prompt_and_artifacts() {
        std::env::remove_var("MONOBENCH_CLI");
        std::env::remove_var("MONOBENCH_AGY_TIMEOUT");
        let p = Path::new("/tmp/monogram-agy-gemini-3.5-r1-t123");
        let prompt_file = Path::new("/tmp/prompt-file.txt");
        let cmd = agy_print_command(prompt_file, p, Path::new("/tmp/clone"), None, "gemini-3.5", "medium");
        assert!(cmd.contains("--dangerously-skip-permissions"));
        assert!(cmd.contains("--add-dir /tmp/clone"));
        assert!(cmd.contains("--log-file /tmp/monogram-agy-gemini-3.5-r1-t123.agy.log"));
        assert!(cmd.contains("--print-timeout 20m"));
        assert!(cmd.contains("--print \"$(cat '/tmp/prompt-file.txt')\""));
        // No jail ⇒ no sandbox-exec prefix.
        assert!(!cmd.contains("sandbox-exec"));
    }

    #[test]
    fn agy_print_command_jails_reads_when_profile_present() {
        std::env::remove_var("MONOBENCH_CLI");
        let p = Path::new("/tmp/monogram-agy-x-r1-t1");
        let prompt_file = Path::new("/tmp/pf.txt");
        let jail = Path::new("/tmp/jail.sb");
        let cmd = agy_print_command(
            prompt_file,
            p,
            Path::new("/tmp/clone"),
            Some(jail),
            "g",
            "low",
        );
        // Jail wraps the whole agy invocation; --add-dir still hands agy the repo.
        assert!(cmd.starts_with("sandbox-exec -f '/tmp/jail.sb' agy"));
        assert!(cmd.contains("--add-dir /tmp/clone"));
        assert!(cmd.contains("--print \"$(cat '/tmp/pf.txt')\""));
    }

    #[test]
    fn parses_print_timeout_durations() {
        assert_eq!(parse_go_duration("20m"), Some(Duration::from_secs(1200)));
        assert_eq!(parse_go_duration("15m"), Some(Duration::from_secs(900)));
        assert_eq!(parse_go_duration("90s"), Some(Duration::from_secs(90)));
        assert_eq!(parse_go_duration("1h"), Some(Duration::from_secs(3600)));
        assert_eq!(parse_go_duration("120"), Some(Duration::from_secs(120)));
        assert_eq!(parse_go_duration("bogus"), None);
    }

    #[test]
    fn extracts_agy_answer_from_transcript() {
        let p = std::env::temp_dir().join(format!("mb-agy-tx-{}.jsonl", std::process::id()));
        std::fs::write(
            &p,
            "{\"type\":\"USER_INPUT\",\"content\":\"task\"}\n\
             {\"type\":\"RUN_COMMAND\",\"content\":\"ls -la\"}\n\
             {\"type\":\"PLANNER_RESPONSE\",\"content\":\"First I will inspect the code.\"}\n\
             this-line-is-not-json\n\
             {\"type\":\"PLANNER_RESPONSE\",\"content\":\"### Root cause: smb2_session_logoff\"}\n",
        )
        .unwrap();
        let a = agy_answer_from_transcript(&p).expect("should extract planner responses");
        // Concatenates PLANNER_RESPONSE content in order; ignores tool steps and non-json lines.
        assert!(a.contains("First I will inspect the code."));
        assert!(a.contains("smb2_session_logoff"));
        assert!(!a.contains("ls -la"));
        std::fs::remove_file(&p).ok();
        // Missing/empty transcript ⇒ None (caller falls back to screen scrape).
        assert!(agy_answer_from_transcript(Path::new("/no/such/transcript.jsonl")).is_none());
    }
}
