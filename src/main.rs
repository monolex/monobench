// monobench — code-intelligence-tool benchmark (mono-series CLI, Rust core).
// Management, analysis, run, and matrix orchestration are native Rust.
mod util; mod grade; mod report; mod adoption; mod trace; mod meter; mod run; mod export; mod niia_runner;
use grade::{load_inst, grade_jsonl, grade_text_file, print_grade, RunStats};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

fn find_root() -> PathBuf {
    if let Ok(r) = std::env::var("MONOBENCH_ROOT") { return PathBuf::from(r); }
    let mut cands: Vec<PathBuf> = vec![];
    if let Ok(exe) = std::env::current_exe() {
        let mut p = exe.parent().map(Path::to_path_buf);
        for _ in 0..4 { if let Some(pp) = p { cands.push(pp.clone()); p = pp.parent().map(Path::to_path_buf); } }
    }
    if let Ok(cwd) = std::env::current_dir() { cands.push(cwd); }
    for c in &cands { if c.join("harness").is_dir() && c.join("instances").is_dir() { return c.clone(); } }
    // No on-disk problem set (installed / standalone binary): use the build-time-embedded set,
    // extracted once to a writable cache. Dev runs from the repo never reach here — the on-disk
    // search above wins, so the live files are always used in-tree.
    if let Some(root) = embedded::extract_root() { return root; }
    std::env::current_dir().unwrap_or_default()
}

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
    use std::path::PathBuf;
    /// Extract the embedded problem set to ~/.monobench/<ver>-<build_id> once; return it as the root.
    pub fn extract_root() -> Option<PathBuf> {
        if EMBEDDED.is_empty() { return None; }
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).ok()?;
        let root = PathBuf::from(home).join(".monobench")
            .join(format!("{}-{}", env!("CARGO_PKG_VERSION"), BUILD_ID));
        let marker = root.join(".extracted");
        if !marker.exists() {
            for (rel, bytes) in EMBEDDED {
                let dst = root.join(rel);
                if let Some(p) = dst.parent() { let _ = std::fs::create_dir_all(p); }
                let _ = std::fs::write(&dst, bytes);
            }
            let _ = std::fs::create_dir_all(&root);
            let _ = std::fs::write(&marker, b"1");
        }
        if root.join("harness").is_dir() && root.join("instances").is_dir() { Some(root) } else { None }
    }
}

fn read_json(path: &Path) -> serde_json::Value {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::Value::Null)
}

fn list_dir_names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).into_iter().flatten().flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

/// Gather every recorded run of an instance as typed RunStats (jsonl + niia/codex answer + forfeit).
fn gather_runs(root: &Path, id: &str) -> Vec<RunStats> {
    let inst = load_inst(&root.join(format!("instances/{id}/instance.json")).to_string_lossy());
    let d = root.join("results").join(id);
    let mut runs = vec![];
    let names = list_dir_names(&d);
    for n in &names {
        if let Some(stem) = n.strip_suffix(".jsonl") {
            let _ = stem;
            runs.push(grade_jsonl(&inst, &d.join(n).to_string_lossy()));
        }
    }
    for n in &names {
        if let Some(stem) = n.strip_suffix(".answer.txt") {
            if names.contains(&format!("{stem}.jsonl")) { continue; }
            runs.push(grade_text_file(&inst, &d.join(n).to_string_lossy(), &d.join(format!("{stem}.meter.json")).to_string_lossy()));
        }
    }
    for n in &names {
        if let Some(stem) = n.strip_suffix(".forfeit") {
            runs.push(RunStats { label: stem.into(), grade: "FORFEIT".into(), cost: 0.0, tok: 0, calls: None, adopt: 0, time: 0, rootcause: String::new() });
        }
    }
    runs
}

fn jsonl_paths(d: &Path) -> Vec<String> {
    let mut v: Vec<String> = list_dir_names(d).into_iter().filter(|n| n.ends_with(".jsonl"))
        .map(|n| d.join(n).to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

fn pgrep_count(pat: &str) -> usize {
    Command::new("pgrep").arg("-f").arg(pat).output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count()).unwrap_or(0)
}

fn die(msg: &str) -> ! { eprintln!("monobench: {msg}"); std::process::exit(1); }

fn main() {
    let root = find_root();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");
    let a = |i: usize| args.get(i).map(String::as_str);

    match cmd {
        "-V" | "--version" | "version" => {
            println!("monobench {}", env!("CARGO_PKG_VERSION"));
        }

        "" | "help" | "-h" | "--help" => {
            let p = root.join("initiate/initiate.md");
            match std::fs::read_to_string(&p) { Ok(s) => print!("{s}"), Err(_) => println!("monobench (no initiate.md found)") }
        }

        "list" => {
            println!("{:<30} {}", "INSTANCE", "TITLE");
            for id in list_dir_names(&root.join("instances")) {
                if id == "_TEMPLATE" { continue; }
                let title = read_json(&root.join(format!("instances/{id}/instance.json"))).get("title").and_then(|x| x.as_str()).unwrap_or("?").to_string();
                println!("{:<30} {}", id, title);
            }
        }

        "tools" => {
            println!("{:<14} {}", "TOOL", "DESC");
            for t in list_dir_names(&root.join("harness/tools")) {
                if t == "_TEMPLATE" { continue; }
                let tool_json = root.join("harness/tools").join(&t).join("tool.json");
                if !tool_json.is_file() { continue; }
                let desc = read_json(&tool_json).get("desc").and_then(|x| x.as_str()).unwrap_or("").chars().take(90).collect::<String>();
                println!("{:<14} {}", t, desc);
            }
            println!("(define your own: cp -r harness/tools/_TEMPLATE harness/tools/<name> && edit tool.json)");
        }

        "status" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench status <id>"));
            let d = root.join("results").join(id);
            println!("status: {id}");
            let names = list_dir_names(&d);
            for n in &names { if let Some(b) = n.strip_suffix(".jsonl") {
                let txt = std::fs::read_to_string(d.join(n)).unwrap_or_default();
                let st = if txt.contains("\"type\":\"result\"") { "done".to_string() }
                    else { format!("running ({}c)", txt.matches("\"type\":\"tool_use\"").count()) };
                println!("  {:<34} {}", b, st);
            } }
            for n in &names { if let Some(b) = n.strip_suffix(".answer.txt") {
                if !names.contains(&format!("{b}.jsonl")) { println!("  {:<34} done (niia/codex)", b); }
            } }
            for n in &names { if let Some(b) = n.strip_suffix(".forfeit") { println!("  {:<34} FORFEIT", b); } }
            let work = std::env::var("MONOBENCH_WORK").unwrap_or_else(|_| format!("{}/.monobench-work", std::env::var("HOME").unwrap_or_default()));
            let wt = std::fs::read_dir(format!("{work}/wt")).into_iter().flatten().flatten().count();
            println!("  active → claude:{} codex:{} worktrees:{}", pgrep_count("claude -p"), pgrep_count("codex exec"), wt);
        }

        // Graceful matrix stop — write the stop file the worker loop polls.
        "stop" => {
            let stopf = root.join(".matrix-stop");
            std::fs::write(&stopf, b"stop").ok();
            println!("stop signalled → {}", stopf.display());
            println!("  running matrix workers finish their CURRENT run and launch no more.");
            println!("  for immediate CPU relief of in-flight runs: pkill -f 'claude -p'  (they will NOT respawn now).");
        }

        // Global overview across ALL instances — no id needed ("뭐가 돌고 있나" 한눈에).
        "watch" => {
            println!("{:<32} {:>3} {:>4} {:>5}  {}", "INSTANCE", "RUN", "DONE", "FULL", "running");
            println!("{}", "─".repeat(78));
            let (mut tot_run, mut tot_done) = (0usize, 0usize);
            for id in list_dir_names(&root.join("instances")) {
                if id == "_TEMPLATE" { continue; }
                let d = root.join("results").join(&id);
                if !d.is_dir() { continue; }
                let names = list_dir_names(&d);
                let (mut running, mut done) = (0usize, vec![]);
                let mut runlist: Vec<String> = vec![];
                for n in &names { if let Some(b) = n.strip_suffix(".jsonl") {
                    let txt = std::fs::read_to_string(d.join(n)).unwrap_or_default();
                    if txt.contains("\"type\":\"result\"") { done.push(b.to_string()); }
                    else { running += 1; runlist.push(format!("{b}({}c)", txt.matches("\"type\":\"tool_use\"").count())); }
                } }
                // niia/codex answer-only completions
                for n in &names { if let Some(b) = n.strip_suffix(".answer.txt") {
                    if !names.contains(&format!("{b}.jsonl")) { done.push(b.to_string()); }
                } }
                if running == 0 && done.is_empty() { continue; }  // no activity → skip
                let full = gather_runs(&root, &id).iter().filter(|r| r.grade == "FULL").count();
                tot_run += running; tot_done += done.len();
                let short: String = if id.chars().count() > 31 { id.chars().take(30).chain(std::iter::once('…')).collect() } else { id.clone() };
                println!("{:<32} {:>3} {:>4} {:>5}  {}", short, running, done.len(), format!("{full}/{}", done.len()), runlist.join(" "));
            }
            println!("{}", "─".repeat(78));
            let work = std::env::var("MONOBENCH_WORK").unwrap_or_else(|_| format!("{}/.monobench-work", std::env::var("HOME").unwrap_or_default()));
            let wt = std::fs::read_dir(format!("{work}/wt")).into_iter().flatten().flatten().count();
            println!("totals: running {tot_run} · done {tot_done}   active → claude:{} codex:{} worktrees:{}",
                pgrep_count("claude -p"), pgrep_count("codex exec"), wt);
        }

        "clean" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench clean <id> [arm-prefix]"));
            let d = root.join("results").join(id);
            if !d.is_dir() { die("no results for that id"); }
            let pre = a(2);
            let exts = [".jsonl", ".err", ".forfeit", ".answer.txt", ".meter.json"];
            let before = list_dir_names(&d).iter().filter(|n| n.ends_with(".jsonl")).count();
            for n in list_dir_names(&d) {
                let path = d.join(&n);
                if n.starts_with('_') && path.is_dir() { let _ = std::fs::remove_dir_all(&path); continue; }
                let hit = match pre {
                    Some(p) => (n.starts_with(p) && exts.iter().any(|e| n.ends_with(e)))
                        || (n.starts_with("mcp-") && n.contains(p) && n.ends_with(".json")),
                    None => exts.iter().any(|e| n.ends_with(e)) || (n.starts_with("mcp-") && n.ends_with(".json")),
                };
                if hit { let _ = std::fs::remove_file(&path); }
            }
            let after: Vec<String> = list_dir_names(&d).into_iter().filter(|n| n.ends_with(".jsonl")).map(|n| n.trim_end_matches(".jsonl").to_string()).collect();
            println!("cleaned {} in {id} · runs {} → {} · kept: {}", pre.map(|p| format!("arm '{p}*'")).unwrap_or_else(|| "ALL runs".into()), before, after.len(), after.join(" "));
        }

        "show" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench show <id> [--spoil]"));
            let dir = root.join("instances").join(id);
            if !dir.is_dir() { die("no such instance"); }
            print!("{}", std::fs::read_to_string(dir.join("symptom.md")).unwrap_or_default());
            if a(2) == Some("--spoil") {
                println!("\n════ GROUND TRUTH (spoiler) ════");
                print!("{}", std::fs::read_to_string(dir.join("ground_truth.md")).unwrap_or_default());
            }
        }

        "run" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench run <id> <arm> [n]"));
            let arm = a(2).unwrap_or_else(|| die("arm (see: monobench tools)"));
            let n: usize = a(3).and_then(|s| s.parse().ok()).unwrap_or(1);
            let model = std::env::var("MONOBENCH_MODEL").unwrap_or_else(|_| "opus".into());
            if args.iter().any(|s| s == "--force") { std::env::set_var("MONOBENCH_FORCE", "1"); }
            std::process::exit(run::run(&root, id, arm, &model, n, false, &Mutex::new(())));
        }

        "matrix" => run_matrix(&root, &args),

        "grade" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench grade <id> [run]"));
            let inst = load_inst(&root.join(format!("instances/{id}/instance.json")).to_string_lossy());
            match a(2) {
                Some(run) => {  // auto-detect: claude-p jsonl, else niia/codex answer.txt + meter.json
                    let jsonl = root.join(format!("results/{id}/{run}.jsonl"));
                    let stats = if jsonl.is_file() { grade_jsonl(&inst, &jsonl.to_string_lossy()) }
                        else { grade_text_file(&inst,
                            &root.join(format!("results/{id}/{run}.answer.txt")).to_string_lossy(),
                            &root.join(format!("results/{id}/{run}.meter.json")).to_string_lossy()) };
                    print_grade(&stats);
                }
                None => for p in jsonl_paths(&root.join("results").join(id)) { print_grade(&grade_jsonl(&inst, &p)); }
            }
        }

        "trace" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench trace <id> <run> [max]"));
            let run = a(2).unwrap_or_else(|| die("run label, e.g. monogram-haiku-r1"));
            let max: usize = a(3).and_then(|s| s.parse().ok()).unwrap_or(200);
            let p = root.join(format!("results/{id}/{run}.jsonl"));
            if !p.is_file() { die("no such run (see: monobench status <id>)"); }
            trace::trace(&p.to_string_lossy(), max);
        }

        "export" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench export <id> <run>  → results/<id>/<run>.md"));
            let run = a(2).unwrap_or_else(|| die("run label, e.g. monogram-discover-haiku-r1"));
            let p = root.join(format!("results/{id}/{run}.jsonl"));
            if !p.is_file() { die("no such run (see: monobench status <id>)"); }
            export::export(&root, id, run, &p.to_string_lossy());
        }

        "report" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench report <id>"));
            report::report(id, &gather_runs(&root, id));
        }

        "summary" => {  // cross-instance leaderboard: FULL hit-rate per arm × instance
            let insts: Vec<(String, Vec<RunStats>)> = list_dir_names(&root.join("instances")).into_iter()
                .filter(|id| id != "_TEMPLATE")
                .map(|id| { let r = gather_runs(&root, &id); (id, r) }).collect();
            report::summary(&insts);
        }

        "adoption" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench adoption <id>"));
            adoption::adoption(id, &jsonl_paths(&root.join("results").join(id)));
        }

        "meter" => {
            let p = a(1).unwrap_or_else(|| die("usage: monobench meter <session.jsonl>"));
            meter::meter(p);
        }

        "add" => {
            let id = a(1).unwrap_or_else(|| die("usage: monobench add <id>"));
            let dst = root.join("instances").join(id);
            if dst.exists() { die("instance already exists"); }
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
        if p.is_dir() { copy_dir(&p, &to); } else { let _ = std::fs::copy(&p, &to); }
    }
}

fn run_matrix(root: &Path, args: &[String]) {
    let id = args.get(1).filter(|s| !s.starts_with("--")).cloned().unwrap_or_else(|| die("usage: monobench matrix <id> [--tools a,b] [--models x,y] [--runs N] [--jobs J]"));
    let (mut tools, mut models, mut runs, mut jobs) = ("baseline,monogram".to_string(), "opus".to_string(), 1usize, 2usize);
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--tools" => { tools = args.get(i + 1).cloned().unwrap_or(tools); i += 2; }
            "--models" => { models = args.get(i + 1).cloned().unwrap_or(models); i += 2; }
            "--runs" => { runs = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(runs); i += 2; }
            "--jobs" => { jobs = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(jobs); i += 2; }
            _ => i += 1,
        }
    }
    let ts: Vec<String> = tools.split(',').map(str::to_string).collect();
    let ms: Vec<String> = models.split(',').map(str::to_string).collect();
    let mut combos: Vec<(String, String, usize)> = vec![];
    for t in &ts { for m in &ms { for r in 1..=runs { combos.push((t.clone(), m.clone(), r)); } } }
    let n = combos.len();
    println!("matrix {id} · {{{tools}}} × {{{models}}} × runs={runs} = {n} runs · jobs={jobs} · git-worktree isolated");

    if args.iter().any(|s| s == "--force") { std::env::set_var("MONOBENCH_FORCE", "1"); }
    std::env::set_var("MONOBENCH_ISOLATE", "worktree");        // constant across threads ⇒ no race
    // Graceful stop: workers check this file BEFORE pulling each job. Without it,
    // killing the claude workers (e.g. for CPU) just makes each worker pop the next
    // combo and respawn claude — the matrix never relents until the queue drains.
    // `monobench stop` writes this file → workers finish their current run, launch
    // no more, matrix exits cleanly. (Pure std; no signal-handler crate.)
    let stopf = root.join(".matrix-stop");
    let _ = std::fs::remove_file(&stopf);   // clear any stale stop from a prior run
    println!("  (pid {} · stop cleanly with `monobench stop` — workers finish current run, launch no more)", std::process::id());
    let wtlock = Arc::new(Mutex::new(()));                     // serializes git-worktree add/remove
    let queue = Arc::new(Mutex::new(combos.into_iter().collect::<std::collections::VecDeque<_>>()));
    let root_arc = Arc::new(root.to_path_buf());
    let stop_arc = Arc::new(stopf.clone());
    let mut handles = vec![];
    for _ in 0..jobs.max(1) {
        let q = Arc::clone(&queue);
        let lock = Arc::clone(&wtlock);
        let id2 = id.clone();
        let r2 = Arc::clone(&root_arc);
        let stop = Arc::clone(&stop_arc);
        handles.push(std::thread::spawn(move || loop {
            if stop.exists() { break; }   // graceful stop — do not pull a new job
            let Some((t, m, r)) = ({ let mut g = q.lock().unwrap(); g.pop_front() }) else { break };
            run::run(&r2, &id2, &t, &m, r, true, &lock);   // quiet=true; matrix prints ✓ + final report
            println!("  ✓ {t} / {m} r{r}");
        }));
    }
    for h in handles { let _ = h.join(); }
    let stopped = stopf.exists();
    let _ = std::fs::remove_file(&stopf);
    println!("{}", if stopped { "── matrix stopped (`monobench stop`) — remaining queue skipped ──" } else { "── matrix done ──" });
    report::report(&id, &gather_runs(root, &id));
    adoption::adoption(&id, &jsonl_paths(&root.join("results").join(&id)));
}
