// monobench — native runner. Runs ONE instance under ONE tool adapter:
// clone/worktree → index (FORFEIT if it can't) → assemble the docs-in-prompt → invoke the model
// (claude-p / codex / niia native) → grade. Parallel-safe via a worktree lock.
use crate::grade::{grade_jsonl, grade_text_file, load_inst, print_grade};
use crate::util::read_json;
use serde_json::Value;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

const STRIP_ENV: [&str; 6] = ["CLAUDECODE", "CLAUDE_CODE_ENTRYPOINT", "CLAUDE_CODE_SESSION_ID", "CLAUDE_EFFORT", "AI_AGENT", "CLAUDE_CODE_EXECPATH"];

fn repo_basename(url: &str) -> String {
    let last = url.rsplit('/').next().unwrap_or(url);
    last.strip_suffix(".git").unwrap_or(last).to_string()
}

/// worktree cleanup on scope exit (replaces the bash `trap cleanup EXIT`).
struct Worktree<'a> { base: PathBuf, wt: PathBuf, lock: &'a Mutex<()> }
impl Drop for Worktree<'_> {
    fn drop(&mut self) {
        let _g = self.lock.lock().unwrap();
        Command::new("git").arg("-C").arg(&self.base).args(["worktree", "remove", "--force"]).arg(&self.wt).output().ok();
        std::fs::remove_dir_all(&self.wt).ok();
    }
}

// `model` is a parameter (not env) because the matrix runs threads in-process and each needs its own.
// `quiet` suppresses the per-run ▶/grade lines (matrix prints its own ✓ + a final report instead).
pub fn run(root: &Path, id: &str, arm: &str, model: &str, run_no: usize, quiet: bool, wtlock: &Mutex<()>) -> i32 {
    let inst_dir = root.join("instances").join(id);
    if !inst_dir.is_dir() { eprintln!("no instance '{id}'"); return 1; }
    let tooldir = root.join("harness/tools").join(arm);
    if !tooldir.join("tool.json").is_file() { eprintln!("no tool adapter '{arm}'"); return 1; }
    let inst_json = read_json(&inst_dir.join("instance.json"));
    let repo_url = inst_json.get("repo").and_then(Value::as_str).unwrap_or("").to_string();
    let tag = inst_json.get("tag").and_then(Value::as_str).unwrap_or("").to_string();
    let tj = read_json(&tooldir.join("tool.json"));
    let field = |k: &str| tj.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let (skill, deliver, fgrep) = (field("skill"), field("deliver"), field("forfeit_grep"));

    let env = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.into());
    let work = env("MONOBENCH_WORK", "/tmp/monobench-work");
    std::fs::create_dir_all(&work).ok();
    let codegraph = env("MONOBENCH_CODEGRAPH", "codegraph");
    let out = root.join("results").join(id);
    std::fs::create_dir_all(&out).ok();
    let effort = std::env::var("MONOBENCH_EFFORT").unwrap_or_default();
    let runner = env("MONOBENCH_RUNNER", "claude-p");
    let cap = env("MONOBENCH_CAP", "6");

    let mut label = arm.to_string();
    if model != "opus" { label = format!("{label}-{model}"); }
    if !effort.is_empty() { label = format!("{label}-{effort}"); }
    let runid = format!("{label}-r{run_no}");

    // Unique-key pre-flight: runid (<arm>-<model>-rN) is the ONLY key for results — a colliding
    // transcript would be silently truncated/overwritten. Skip BEFORE the expensive worktree+index
    // unless forced. (The atomic O_EXCL guard at write-time below is the concurrent-race backstop.)
    let force = std::env::var("MONOBENCH_FORCE").map(|v| v == "1").unwrap_or(false);
    if !force && out.join(format!("{runid}.jsonl")).exists() {
        if !quiet { eprintln!("skip {runid}: results exist — use --force to overwrite"); }
        return 0;
    }

    // 1. repo: worktree-isolated (parallel-safe) or shared clone
    let isolate = std::env::var("MONOBENCH_ISOLATE").unwrap_or_default() == "worktree";
    let _wt_guard;
    let clone: PathBuf;
    if isolate {
        let base = PathBuf::from(format!("{work}/{}-base", repo_basename(&repo_url)));
        { let _g = wtlock.lock().unwrap();
          if !base.join(".git").is_dir() { Command::new("git").args(["clone", "--filter=blob:none", "--quiet", &repo_url]).arg(&base).status().ok(); }
          Command::new("git").arg("-C").arg(&base).args(["worktree", "prune"]).output().ok();
        }
        let wt = PathBuf::from(format!("{work}/wt/{runid}-{}", std::process::id()));
        std::fs::remove_dir_all(&wt).ok();
        std::fs::create_dir_all(format!("{work}/wt")).ok();
        { let _g = wtlock.lock().unwrap();
          Command::new("git").arg("-C").arg(&base).args(["worktree", "add", "--quiet", "--force", "--detach"]).arg(&wt).arg(&tag).output().ok();
        }
        clone = wt.clone();   // ${REPO} is substituted from `clone` directly (no process-global env — matrix runs threads)
        _wt_guard = Some(Worktree { base, wt, lock: wtlock });
    } else {
        let c = PathBuf::from(format!("{work}/{}", repo_basename(&repo_url)));
        if !c.join(".git").is_dir() { Command::new("git").args(["clone", "--filter=blob:none", "--quiet", &repo_url]).arg(&c).status().ok(); }
        Command::new("git").arg("-C").arg(&c).args(["checkout", "--quiet", &tag]).output().ok();
        Command::new("git").arg("-C").arg(&c).args(["checkout", "--", "."]).output().ok();
        clone = c;
        _wt_guard = None;
    }

    if !quiet { println!("▶ {id} / {label} r{run_no}  (deliver={}, runner={runner}, isolate={})",
        if deliver.is_empty() { "none" } else { &deliver }, if isolate { "worktree" } else { "shared" }); }

    // 2. index for the tool (+ FORFEIT if it can't)
    let log = match run_index(&tj, &clone, &clone, &codegraph) {
        Ok(log) => log,
        Err(e) => {
            eprintln!("index failed for '{arm}': {e}");
            return 1;
        }
    };
    if !fgrep.is_empty() {
        let ll = log.to_lowercase();
        if fgrep.split('|').any(|p| !p.is_empty() && ll.contains(&p.to_lowercase())) {
            let msg = format!("  FORFEIT — '{arm}' could not index this repo");
            println!("{msg}");
            std::fs::write(out.join(format!("{runid}.forfeit")), msg).ok();
            return 0;
        }
    }

    // 3. prompt preamble: lead.md + initiate.md + skill.md + depth.md (docs shoved into the -p prompt)
    let cat = |p: PathBuf| std::fs::read_to_string(p).unwrap_or_default();
    let mut sys = cat(root.join("harness/prompts/depth.md"));
    if !skill.is_empty() && tooldir.join(&skill).is_file() { sys = format!("{}\n\n{}", cat(tooldir.join(&skill)), sys); }
    if tooldir.join("initiate.md").is_file() { sys = format!("{}\n\n{}", cat(tooldir.join("initiate.md")), sys); }
    if tooldir.join("lead.md").is_file() { sys = format!("{}\n\n{}", cat(tooldir.join("lead.md")), sys); }

    // 4. MCP config (per-run filename, parallel-safe)
    let mcpcfg = if deliver == "mcp" {
        let p = out.join(format!("mcp-{runid}.json"));
        let mcp = tj.get("mcp").cloned().unwrap_or(Value::Null);
        let raw_command = mcp.get("command").and_then(Value::as_str).unwrap_or("");
        let raw_args: Vec<String> = mcp.get("args").and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
        let (command, args) = match command_and_args(raw_command, &raw_args, &clone, &codegraph) {
            Ok(x) => x,
            Err(e) => { eprintln!("invalid mcp config for '{arm}': {e}"); return 1; }
        };
        let cfg = serde_json::json!({ "mcpServers": { arm: { "command": command, "args": args } } });
        std::fs::write(&p, cfg.to_string()).ok();
        p
    } else {
        let p = out.join(format!("mcp-empty-{runid}.json"));
        std::fs::write(&p, "{\"mcpServers\":{}}").ok();
        p
    };

    let q = cat(inst_dir.join("symptom.md"));
    let inst = load_inst(&inst_dir.join("instance.json").to_string_lossy());

    match runner.as_str() {
        "niia" => {
            if let Err(e) = crate::niia_runner::run(&clone, &format!("{sys}\n\n{q}\n"), "ROOTCAUSE", &out.join(&runid), &effort) {
                eprintln!("niia runner failed: {e}");
                return 1;
            }
            if !quiet { print_grade(&grade_text_file(&inst, &out.join(format!("{runid}.answer.txt")).to_string_lossy(), &out.join(format!("{runid}.meter.json")).to_string_lossy())); }
        }
        "codex" => {
            let pf = std::env::temp_dir().join(format!("mb-pf-{runid}"));
            std::fs::write(&pf, format!("{sys}\n\n{q}\n")).ok();
            let ans = out.join(format!("{runid}.answer.txt"));
            let t0 = std::time::Instant::now();
            let mut cmd = Command::new("codex");
            cmd.arg("exec").arg("-C").arg(&clone).args(["--skip-git-repo-check", "--dangerously-bypass-approvals-and-sandbox"]);
            if let Ok(m) = std::env::var("MONOBENCH_CODEX_MODEL") { if !m.is_empty() { cmd.arg("-m").arg(m); } }
            cmd.arg("-c").arg(format!("model_reasoning_effort={}", if effort.is_empty() { "high".into() } else { effort.clone() }));
            cmd.arg("-o").arg(&ans);
            for e in STRIP_ENV { cmd.env_remove(e); }
            cmd.stdin(File::open(&pf).unwrap()).stdout(File::create(out.join(format!("{runid}.codexlog"))).unwrap()).stderr(File::create(out.join(format!("{runid}.err"))).unwrap());
            cmd.status().ok();
            let dur = t0.elapsed().as_secs();
            Command::new("monometer").args(["daemon", "recompute"]).output().ok();
            std::thread::sleep(std::time::Duration::from_secs(1));
            let meter = codex_meter(dur);
            std::fs::write(out.join(format!("{runid}.meter.json")), meter).ok();
            if !quiet { print_grade(&grade_text_file(&inst, &ans.to_string_lossy(), &out.join(format!("{runid}.meter.json")).to_string_lossy())); }
        }
        _ => { // claude -p
            let f = out.join(format!("{runid}.jsonl"));
            // Unique-key guard: runid (<arm>-<model>-rN) is the ONLY key for results, so a
            // colliding run would silently truncate (sequential) or interleave-corrupt
            // (concurrent) this transcript. Claim it atomically (O_EXCL) unless MONOBENCH_FORCE=1.
            let opened = if force {
                File::create(&f)
            } else {
                std::fs::OpenOptions::new().write(true).create_new(true).open(&f)
            };
            let jsonl_file = match opened {
                Ok(fh) => fh,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if !quiet { eprintln!("  skip {runid}: results exist — use --force to overwrite"); }
                    return 0;
                }
                Err(e) => { eprintln!("create {runid}.jsonl failed: {e}"); return 1; }
            };
            let prompt = format!("{sys}\n\n{}\n# YOUR TASK\n{q}", "═".repeat(80));
            let mut cmd = Command::new("claude");
            cmd.current_dir(&clone)
                .arg("-p").arg(&prompt)
                .args(["--output-format", "stream-json", "--verbose", "--permission-mode", "bypassPermissions", "--model", &model]);
            if !effort.is_empty() { cmd.arg("--effort").arg(&effort); }
            cmd.args(["--max-budget-usd", &cap, "--setting-sources", "", "--disable-slash-commands", "--strict-mcp-config"])
                .arg("--mcp-config").arg(&mcpcfg)
                .args(["--disallowedTools", "Bash(git:*)"]);  // anti-contamination: no reading the fix from git history
            for e in STRIP_ENV { cmd.env_remove(e); }
            cmd.stdout(jsonl_file).stderr(File::create(out.join(format!("{runid}.err"))).unwrap());
            cmd.status().ok();
            if !quiet { print_grade(&grade_jsonl(&inst, &f.to_string_lossy())); }
        }
    }
    0
}

fn sub_vars(s: &str, repo: &Path, codegraph: &str) -> String {
    s.replace("${REPO}", &repo.to_string_lossy()).replace("${CODEGRAPH}", codegraph)
}

fn split_words(s: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut esc = false;
    for ch in s.chars() {
        if esc {
            cur.push(ch);
            esc = false;
            continue;
        }
        if ch == '\\' {
            esc = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q { quote = None; } else { cur.push(ch); }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            c if c.is_whitespace() => {
                if !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(ch),
        }
    }
    if esc { cur.push('\\'); }
    if quote.is_some() { return Err("unterminated quote".into()); }
    if !cur.is_empty() { words.push(cur); }
    Ok(words)
}

fn command_and_args(command: &str, args: &[String], repo: &Path, codegraph: &str) -> Result<(String, Vec<String>), String> {
    let expanded = sub_vars(command, repo, codegraph);
    let mut words = split_words(&expanded)?;
    if words.is_empty() { return Err("missing command".into()); }
    let exe = words.remove(0);
    for arg in args {
        words.push(sub_vars(arg, repo, codegraph));
    }
    Ok((exe, words))
}

fn run_argv(command: &str, args: &[String], cwd: &Path) -> Result<String, String> {
    let out = Command::new(command).args(args).current_dir(cwd).output()
        .map_err(|e| format!("{command}: {e}"))?;
    let mut log = String::new();
    log.push_str(&String::from_utf8_lossy(&out.stdout));
    log.push_str(&String::from_utf8_lossy(&out.stderr));
    if !out.status.success() {
        log.push_str(&format!("\n[exit {}]\n", out.status.code().unwrap_or(-1)));
    }
    Ok(log)
}

fn run_index_step(step: &Value, cwd: &Path, repo: &Path, codegraph: &str) -> Result<String, String> {
    let command = step.get("command").and_then(Value::as_str).unwrap_or("");
    let args: Vec<String> = step.get("args").and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
    let quiet = step.get("quiet").and_then(Value::as_bool).unwrap_or(false);
    let (exe, argv) = command_and_args(command, &args, repo, codegraph)?;
    let log = run_argv(&exe, &argv, cwd)?;
    if quiet { Ok(String::new()) } else { Ok(log) }
}

fn run_legacy_index(index: &str, cwd: &Path, repo: &Path, codegraph: &str) -> Result<String, String> {
    if index.chars().any(|c| matches!(c, ';' | '|' | '&' | '<' | '>' | '`')) {
        return Err("legacy index contains shell operators; convert it to index_steps".into());
    }
    let expanded = sub_vars(index, repo, codegraph);
    let mut words = split_words(&expanded)?;
    if words.is_empty() { return Ok(String::new()); }
    let exe = words.remove(0);
    run_argv(&exe, &words, cwd)
}

fn run_index(tool_json: &Value, cwd: &Path, repo: &Path, codegraph: &str) -> Result<String, String> {
    if let Some(steps) = tool_json.get("index_steps").and_then(Value::as_array) {
        let mut log = String::new();
        for step in steps {
            log.push_str(&run_index_step(step, cwd, repo, codegraph)?);
        }
        return Ok(log);
    }
    let index = tool_json.get("index").and_then(Value::as_str).unwrap_or("").trim();
    if index.is_empty() { Ok(String::new()) } else { run_legacy_index(index, cwd, repo, codegraph) }
}

/// Parse `monometer sessions --provider codex` → the meter.json shape (tokens/cost/duration/model).
fn codex_meter(dur: u64) -> String {
    let out = Command::new("monometer").args(["sessions", "--provider", "codex", "--recent", "3", "--json"]).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let v: Value = serde_json::from_str(&out).unwrap_or(Value::Null);
    let x = v.get(0).cloned().unwrap_or(Value::Null);
    let model = x.get("models").and_then(Value::as_array).and_then(|a| a.first()).and_then(Value::as_str).unwrap_or("codex");
    serde_json::json!({
        "tokens": x.get("total_tokens").and_then(Value::as_i64),
        "cost_usd": x.get("cost_usd").and_then(Value::as_f64),
        "duration_s": dur,
        "model": model
    }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_repo_and_codegraph() {
        let repo = Path::new("/tmp/repo");
        assert_eq!(sub_vars("${CODEGRAPH} --path ${REPO}", repo, "node cg.js"), "node cg.js --path /tmp/repo");
    }

    #[test]
    fn expands_command_with_prefix_args() {
        let repo = Path::new("/tmp/repo");
        let args = vec!["serve".to_string(), "--path".to_string(), "${REPO}".to_string()];
        let (cmd, argv) = command_and_args("${CODEGRAPH}", &args, repo, "node /opt/codegraph.js").unwrap();
        assert_eq!(cmd, "node");
        assert_eq!(argv, vec!["/opt/codegraph.js", "serve", "--path", "/tmp/repo"]);
    }

    #[test]
    fn rejects_legacy_shell_operators() {
        let err = run_legacy_index("a; b", Path::new("."), Path::new("."), "codegraph").unwrap_err();
        assert!(err.contains("index_steps"));
    }
}
