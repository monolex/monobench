// monobench — native runner. Runs ONE instance under ONE tool adapter:
// clone/worktree → index (FORFEIT if it can't) → assemble the docs-in-prompt → invoke the model
// (direct claude/codex/agy or via niia) → grade. Parallel-safe via a worktree lock.
use crate::grade::{grade_jsonl, grade_text_file, load_inst, print_grade};
use crate::run_meta::{self, StartMeta};
use crate::util::{full_arm_name, read_json};
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::fs::{File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const STRIP_ENV: [&str; 6] = [
    "CLAUDECODE",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_EFFORT",
    "AI_AGENT",
    "CLAUDE_CODE_EXECPATH",
];

pub(crate) fn repo_basename(url: &str) -> String {
    let last = url.rsplit('/').next().unwrap_or(url);
    last.strip_suffix(".git").unwrap_or(last).to_string()
}

/// worktree cleanup on scope exit (replaces the bash `trap cleanup EXIT`).
struct Worktree<'a> {
    base: PathBuf,
    wt: PathBuf,
    base_lock: PathBuf,
    lock: &'a Mutex<()>,
}
impl Drop for Worktree<'_> {
    fn drop(&mut self) {
        // cross-process base guard (Option A): `git worktree remove` mutates the shared base's
        // worktree metadata, so serialize it against another process's add/remove on the same base.
        let _flk = acquire_file_lock(&self.base_lock, 600, Duration::from_millis(100));
        let _g = self.lock.lock().unwrap();
        Command::new("git")
            .arg("-C")
            .arg(&self.base)
            .args(["worktree", "remove", "--force"])
            .arg(&self.wt)
            .output()
            .ok();
        std::fs::remove_dir_all(&self.wt).ok();
    }
}

struct RunningMarker {
    path: PathBuf,
}

impl Drop for RunningMarker {
    fn drop(&mut self) {
        std::fs::remove_file(&self.path).ok();
    }
}

impl RunningMarker {
    fn set(&self, phase: &str, detail: &str) {
        std::fs::write(&self.path, format!("{phase} {detail}\n")).ok();
    }
}

fn result_exists(out: &Path, runid: &str) -> bool {
    [
        ".jsonl",
        ".answer.txt",
        ".agy.jsonl",
        ".forfeit",
        ".meter.json",
    ]
    .iter()
    .any(|ext| out.join(format!("{runid}{ext}")).exists())
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn unique_runid(out: &Path, label: &str, run_no: usize) -> String {
    let start = unix_ms();
    for offset in 0..1000u128 {
        let runid = format!("{label}-r{run_no}-t{}", start + offset);
        if !result_exists(out, &runid) && !out.join(format!("{runid}.running")).exists() {
            return runid;
        }
    }
    format!("{label}-r{run_no}-t{}", unix_ms() + 1000)
}

fn path_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "instance".into()
    } else {
        out
    }
}

fn worktree_run_dir(work: &str, id: &str, runid: &str) -> PathBuf {
    Path::new(work)
        .join("wt")
        .join(path_component(id))
        .join(format!("{runid}-{}", std::process::id()))
}

fn runid_timestamp(runid: &str) -> u128 {
    runid
        .rsplit("-t")
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(run_meta::now_ms)
}

fn shared_clone(
    repo_url: &str,
    tag: &str,
    work: &str,
    wtlock: &Mutex<()>,
) -> Result<PathBuf, String> {
    let c = PathBuf::from(format!("{work}/{}", repo_basename(repo_url)));
    // cross-process guard (Option A): serialize clone/checkout of the shared dir across processes.
    let base_lock = PathBuf::from(format!("{work}/{}.lock", repo_basename(repo_url)));
    let _flk = acquire_file_lock(&base_lock, 6000, Duration::from_millis(100));
    let _g = wtlock.lock().unwrap();
    if !c.join(".git").is_dir() {
        Command::new("git")
            .args(["clone", "--filter=blob:none", "--quiet", repo_url])
            .arg(&c)
            .status()
            .map_err(|e| format!("git clone: {e}"))?;
    }
    // defensive: a killed prior prepare can leave a stale index.lock that blocks every index op
    let _ = std::fs::remove_file(c.join(".git").join("index.lock"));
    let co = Command::new("git")
        .arg("-C")
        .arg(&c)
        .args(["checkout", "--quiet", "--force", tag])
        .output()
        .map_err(|e| format!("git checkout: {e}"))?;
    if !co.status.success() {
        return Err(format!(
            "git checkout {tag} failed (corrupt/partial clone of {}): {}",
            c.display(),
            String::from_utf8_lossy(&co.stderr).trim()
        ));
    }
    let _ = Command::new("git")
        .arg("-C")
        .arg(&c)
        .args(["checkout", "--", "."])
        .output();
    // verify the working tree actually populated. A silent empty checkout (blob:none corruption,
    // interrupted fetch, or stale lock) otherwise yields a 0-file index that quietly degrades the
    // tool arm to bare grep and produces misleading grades — node-* hit exactly this. Fail loud.
    let tracked = Command::new("git")
        .arg("-C")
        .arg(&c)
        .args(["ls-files"])
        .output()
        .map(|o| o.stdout.iter().filter(|&&b| b == b'\n').count())
        .unwrap_or(0);
    if tracked == 0 {
        return Err(format!(
            "clone working tree empty after checkout of {tag} (corrupt/partial clone — `rm -rf {}` to force a fresh clone)",
            c.display()
        ));
    }
    Ok(c)
}

fn monolex_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".monolex")
}

fn monogram_project_db_path(project_root: &Path) -> PathBuf {
    let canonical = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    let name = canonical
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("default"))
        .to_string_lossy();

    let mut hasher = DefaultHasher::new();
    canonical.to_string_lossy().as_ref().hash(&mut hasher);
    let hash = format!("{:x}", hasher.finish());
    let short_hash = &hash[..6.min(hash.len())];

    monolex_home()
        .join("monogram")
        .join(format!("{name}-{short_hash}.db"))
}

fn sqlite_sidecars(base: &Path) -> [PathBuf; 3] {
    [
        base.to_path_buf(),
        base.with_extension("db-wal"),
        base.with_extension("db-shm"),
    ]
}

fn copy_sqlite_snapshot(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.is_file() {
        return Err(format!("prepared DB not found: {}", src.display()));
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    for p in sqlite_sidecars(dst) {
        std::fs::remove_file(p).ok();
    }
    for (from, to) in sqlite_sidecars(src).into_iter().zip(sqlite_sidecars(dst)) {
        if from.exists() {
            std::fs::copy(&from, &to)
                .map_err(|e| format!("copy {} -> {}: {e}", from.display(), to.display()))?;
        }
    }
    Ok(())
}

fn remove_sqlite_db(path: &Path) {
    for sidecar in sqlite_sidecars(path) {
        std::fs::remove_file(sidecar).ok();
    }
}

struct FileLock {
    path: PathBuf,
}

impl Drop for FileLock {
    fn drop(&mut self) {
        std::fs::remove_file(&self.path).ok();
    }
}

fn acquire_file_lock(path: &Path, attempts: usize, sleep: Duration) -> Option<FileLock> {
    for _ in 0..attempts {
        match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(mut f) => {
                let _ = writeln!(f, "pid={}", std::process::id());
                return Some(FileLock {
                    path: path.to_path_buf(),
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                std::thread::sleep(sleep);
            }
            Err(_) => return None,
        }
    }
    None
}

fn sql_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn prefix_update_sql(table: &str, col: &str, from: &str, to: &str) -> String {
    let from_q = sql_quote(from);
    let slash_q = sql_quote(&format!("{from}/"));
    let to_q = sql_quote(to);
    format!(
        "UPDATE {table}
         SET {col} = CASE
             WHEN {col} = {from_q} THEN {to_q}
             ELSE {to_q} || substr({col}, length({from_q}) + 1)
         END
         WHERE {col} IS NOT NULL
           AND ({col} = {from_q} OR substr({col}, 1, length({slash_q})) = {slash_q});"
    )
}

fn rewrite_prepared_paths(
    db_path: &Path,
    source_root: &Path,
    target_root: &Path,
    log_path: &Path,
) -> Result<(), String> {
    let from = source_root
        .canonicalize()
        .unwrap_or_else(|_| source_root.to_path_buf())
        .to_string_lossy()
        .to_string();
    let to = target_root
        .canonicalize()
        .unwrap_or_else(|_| target_root.to_path_buf())
        .to_string_lossy()
        .to_string();
    if from == to {
        return Ok(());
    }
    let sql = format!(
        "PRAGMA busy_timeout=5000;
         BEGIN IMMEDIATE;
         {}
         {}
         {}
         COMMIT;
         PRAGMA wal_checkpoint(TRUNCATE);",
        prefix_update_sql("files", "path", &from, &to),
        prefix_update_sql("relations", "resolved_path", &from, &to),
        prefix_update_sql("import_bindings", "resolved_path", &from, &to),
    );
    append_log(
        log_path,
        &format!("[prepared] rewriting monogram DB paths\n  from: {from}\n  to:   {to}\n"),
    );
    let out = match Command::new("sqlite3").arg(db_path).arg(sql).output() {
        Ok(out) => out,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            append_log(
                log_path,
                "[prepared] sqlite3 not found; skipped absolute path rewrite\n",
            );
            return Ok(());
        }
        Err(e) => return Err(format!("sqlite3 path rewrite launch failed: {e}")),
    };
    if !out.status.success() {
        append_log(log_path, &String::from_utf8_lossy(&out.stdout));
        append_log(log_path, &String::from_utf8_lossy(&out.stderr));
        return Err(format!(
            "sqlite3 path rewrite failed for {}",
            db_path.display()
        ));
    }
    Ok(())
}

fn sqlite3_query_lines(db_path: &Path, sql: &str) -> Result<Vec<String>, String> {
    let out = Command::new("sqlite3")
        .arg("-batch")
        .arg(db_path)
        .arg(sql)
        .output()
        .map_err(|e| format!("sqlite3 query launch failed: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "sqlite3 query failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect())
}

fn sqlite3_exec_stdin(db_path: &Path, sql: &str) -> Result<(), String> {
    let mut child = Command::new("sqlite3")
        .arg("-batch")
        .arg(db_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("sqlite3 exec launch failed: {e}"))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| "sqlite3 stdin unavailable".to_string())?
        .write_all(sql.as_bytes())
        .map_err(|e| format!("sqlite3 stdin write failed: {e}"))?;
    let out = child
        .wait_with_output()
        .map_err(|e| format!("sqlite3 exec wait failed: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "sqlite3 exec failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}

fn refresh_prepared_mtimes(
    db_path: &Path,
    target_root: &Path,
    log_path: &Path,
) -> Result<(), String> {
    let paths = match sqlite3_query_lines(db_path, "SELECT path FROM files;") {
        Ok(paths) => paths,
        Err(e) if e.contains("No such file or directory") => {
            append_log(
                log_path,
                "[prepared] sqlite3 not found; skipped mtime refresh\n",
            );
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    let mut updated = 0usize;
    let mut missing = 0usize;
    let mut sql = String::from("PRAGMA busy_timeout=5000;\nBEGIN IMMEDIATE;\n");
    for path in paths {
        let path_ref = Path::new(&path);
        let fs_path = if path_ref.is_absolute() {
            path_ref.to_path_buf()
        } else {
            let rel = path.strip_prefix("./").unwrap_or(path.as_str());
            target_root.join(rel)
        };
        match std::fs::metadata(&fs_path).and_then(|m| m.modified()) {
            Ok(modified) => {
                let mtime = modified
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                sql.push_str(&format!(
                    "UPDATE files SET indexed_at = {mtime} WHERE path = {};\n",
                    sql_quote(&path)
                ));
                updated += 1;
            }
            Err(_) => missing += 1,
        }
    }
    sql.push_str("COMMIT;\nPRAGMA wal_checkpoint(TRUNCATE);\n");
    sqlite3_exec_stdin(db_path, &sql)?;
    append_log(
        log_path,
        &format!("[prepared] refreshed monogram mtimes updated={updated} missing={missing}\n"),
    );
    Ok(())
}

fn tool_uses_monogram_index(tool_json: &Value) -> bool {
    if let Some(steps) = tool_json.get("index_steps").and_then(Value::as_array) {
        return steps.iter().any(|step| {
            let command = step.get("command").and_then(Value::as_str).unwrap_or("");
            let args: Vec<&str> = step
                .get("args")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            command.split_whitespace().next() == Some("monogram")
                && args.first().copied() == Some("index")
        });
    }
    tool_json
        .get("index")
        .and_then(Value::as_str)
        .map(|s| {
            let words = split_words(s).unwrap_or_default();
            words.first().map(String::as_str) == Some("monogram")
                && words.get(1).map(String::as_str) == Some("index")
        })
        .unwrap_or(false)
}

fn tool_version_from_json(tool_json: &Value) -> String {
    let vb = tool_json
        .get("version_bin")
        .and_then(Value::as_str)
        .unwrap_or("");
    if vb.is_empty() {
        String::new()
    } else {
        crate::util::capture_semver(vb)
    }
}

fn prepared_snapshot_dir(out: &Path, tool: &str) -> PathBuf {
    out.join("_prepared").join(tool)
}

fn prepared_manifest_value(snap_dir: &Path, key: &str) -> Option<String> {
    let prefix = format!("{key}\t");
    std::fs::read_to_string(snap_dir.join("manifest.tsv"))
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix(&prefix).map(str::to_string))
}

fn prepared_snapshot_version_matches(snap_dir: &Path, expected_tool_version: &str) -> bool {
    if expected_tool_version.is_empty() {
        return true;
    }
    prepared_manifest_value(snap_dir, "tool_version").as_deref() == Some(expected_tool_version)
}

fn save_prepared_monogram_snapshot(
    out: &Path,
    tool: &str,
    repo: &Path,
    tool_version: &str,
    log_path: &Path,
) -> Result<(), String> {
    let src_db = monogram_project_db_path(repo);

    // Checkpoint the WAL into the main DB so the snapshot is a single self-
    // contained file. Without this, `-wal` is copied as a sidecar; readers
    // would think pending state exists and refuse multi-reader sharing on
    // the symlink fast path (Change 6). Silently OK if sqlite3 missing.
    let _ = Command::new("sqlite3")
        .arg(&src_db)
        .arg("PRAGMA wal_checkpoint(TRUNCATE);")
        .output();

    // Detect whether the indexer wrote root-relative paths (lib-monogram
    // schema v11+). A single non-null row tells us; rows beginning with
    // `/` indicate the legacy absolute scheme, which still works through
    // the install-side copy+rewrite fallback (Change 6).
    let portable = sqlite3_query_lines(
        &src_db,
        "SELECT resolved_path FROM relations WHERE resolved_path IS NOT NULL LIMIT 1;",
    )
    .ok()
    .and_then(|rows| rows.into_iter().next())
    .map(|v| !v.starts_with('/'))
    .unwrap_or(false);

    let snap_dir = prepared_snapshot_dir(out, tool);
    std::fs::remove_dir_all(&snap_dir).ok();
    std::fs::create_dir_all(&snap_dir).map_err(|e| format!("{}: {e}", snap_dir.display()))?;
    let snap_db = snap_dir.join("monogram.db");
    copy_sqlite_snapshot(&src_db, &snap_db)?;
    let source_root = repo
        .canonicalize()
        .unwrap_or_else(|_| repo.to_path_buf())
        .to_string_lossy()
        .to_string();
    let manifest = format!(
        "source_root\t{}\nsource_db\t{}\nsnapshot_db\t{}\ntool_version\t{}\ncreated_ms\t{}\nportable\t{}\n",
        source_root,
        src_db.display(),
        snap_db.display(),
        tool_version,
        unix_ms(),
        portable
    );
    std::fs::write(snap_dir.join("manifest.tsv"), manifest)
        .map_err(|e| format!("write prepared manifest: {e}"))?;
    append_log(
        log_path,
        &format!(
            "[prepared] saved monogram snapshot {} -> {} (portable={})\n",
            src_db.display(),
            snap_db.display(),
            portable
        ),
    );
    Ok(())
}

fn prepared_monogram_snapshot_ready(out: &Path, tool: &str, expected_tool_version: &str) -> bool {
    let snap_dir = prepared_snapshot_dir(out, tool);
    let db = snap_dir.join("monogram.db");
    if !db.is_file() || !snap_dir.join("manifest.tsv").is_file() {
        return false;
    }
    if !prepared_snapshot_version_matches(&snap_dir, expected_tool_version) {
        return false;
    }
    // verify the snapshot is NON-EMPTY. A snapshot prepared before the shared clone finished
    // indexing copies an empty source DB (0 files); reusing it silently degrades the tool arm to
    // bare grep (node-*/threadpool/freeparser hit this). Re-prepare instead of reusing empty.
    let files = Command::new("sqlite3")
        .arg(&db)
        .arg("SELECT count(*) FROM files;")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(0);
    files > 0
}

fn refresh_prepared_requested() -> bool {
    matches!(
        std::env::var("MONOBENCH_REFRESH_PREPARED")
            .or_else(|_| std::env::var("MONOBENCH_PREPARE_REFRESH"))
            .as_deref(),
        Ok("1") | Ok("true") | Ok("yes") | Ok("force")
    )
}

fn prepared_source_root(snap_dir: &Path) -> Result<PathBuf, String> {
    let manifest = std::fs::read_to_string(snap_dir.join("manifest.tsv"))
        .map_err(|e| format!("read prepared manifest: {e}"))?;
    manifest
        .lines()
        .find_map(|line| line.strip_prefix("source_root\t"))
        .map(PathBuf::from)
        .ok_or_else(|| "prepared manifest missing source_root".into())
}

fn register_monogram_project(project_root: &Path, db_path: &Path) {
    let Some(db_file) = db_path.file_name().and_then(|s| s.to_str()) else {
        return;
    };
    let canonical = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    let registry_dir = monolex_home().join("monogram");
    if std::fs::create_dir_all(&registry_dir).is_err() {
        return;
    }
    let registry_path = registry_dir.join(".registry");
    let lock_path = registry_dir.join(".registry.lock");
    let Some(_lock) = acquire_file_lock(&lock_path, 500, Duration::from_millis(10)) else {
        return;
    };
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(registry_path)
    {
        let _ = writeln!(f, "{}\t{}", canonical.to_string_lossy(), db_file);
    }
}

fn install_prepared_monogram_snapshot(
    out: &Path,
    tool: &str,
    repo: &Path,
    log_path: &Path,
    marker: &RunningMarker,
) -> Result<String, String> {
    std::fs::remove_file(log_path).ok();
    let snap_dir = prepared_snapshot_dir(out, tool);
    let snap_db = snap_dir.join("monogram.db");
    let dst_db = monogram_project_db_path(repo);

    // Fast path: portable templates (lib-monogram schema v11+) store all
    // paths as repo-root-relative `./...`, so every worktree with the same
    // file layout reads the same rows. Symlink — zero copy, zero rewrite.
    let portable = prepared_manifest_value(&snap_dir, "portable").as_deref() == Some("true");
    if portable {
        marker.set(
            "index",
            &format!("prepared-link log={}", log_path.display()),
        );
        if let Some(parent) = dst_db.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        // The worktree is fresh; remove any pre-existing destination
        // (including its WAL/SHM sidecars) before linking.
        for sidecar in sqlite_sidecars(&dst_db) {
            std::fs::remove_file(sidecar).ok();
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&snap_db, &dst_db).map_err(|e| {
                format!("symlink {} -> {}: {e}", snap_db.display(), dst_db.display())
            })?;
        }
        #[cfg(not(unix))]
        {
            // Fall back to copy on non-Unix platforms — symlinking on
            // Windows needs SeCreateSymbolicLinkPrivilege which monobench
            // can't assume. The portable DB still avoids the path-rewrite
            // step, so we save the SQL UPDATEs even when we copy.
            copy_sqlite_snapshot(&snap_db, &dst_db)?;
        }
        register_monogram_project(repo, &dst_db);
        append_log(
            log_path,
            &format!(
                "[prepared] linked portable monogram snapshot {} -> {}\n",
                snap_db.display(),
                dst_db.display()
            ),
        );
        append_log(
            log_path,
            "[prepared] monogram snapshot ready (portable; no copy, no path rewrite)\n",
        );
        return Ok(std::fs::read_to_string(log_path).unwrap_or_default());
    }

    // Legacy path: absolute-path DBs (schema v10 and earlier) still need a
    // private copy + SQL rewrite so the canonical paths inside the DB
    // match this run's worktree. Kept so older prepared snapshots keep
    // working until refreshed.
    marker.set(
        "index",
        &format!("prepared-copy log={}", log_path.display()),
    );
    append_log(
        log_path,
        &format!(
            "[prepared] installing legacy monogram snapshot {} -> {}\n",
            snap_db.display(),
            dst_db.display()
        ),
    );
    copy_sqlite_snapshot(&snap_db, &dst_db)?;
    register_monogram_project(repo, &dst_db);
    let source_root = prepared_source_root(&snap_dir)?;
    rewrite_prepared_paths(&dst_db, &source_root, repo, log_path)?;
    refresh_prepared_mtimes(&dst_db, repo, log_path)?;
    append_log(
        log_path,
        "[prepared] monogram snapshot ready; per-run index skipped\n",
    );
    Ok(std::fs::read_to_string(log_path).unwrap_or_default())
}

pub fn prepare(root: &Path, id: &str, tools: &[String], wtlock: &Mutex<()>) -> i32 {
    let inst_dir = root.join("instances").join(id);
    if !inst_dir.is_dir() {
        eprintln!("no instance '{id}'");
        return 1;
    }
    let inst = load_inst(&inst_dir.join("instance.json").to_string_lossy());
    if let Some(reason) = &inst.invalid {
        eprintln!("invalid instance '{id}': {reason}");
        return 1;
    }
    let inst_json = read_json(&inst_dir.join("instance.json"));
    let repo_url = inst_json
        .get("repo")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let repo_tag = inst_json
        .get("tag")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let env = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.into());
    let work = env("MONOBENCH_WORK", "/tmp/monobench-work");
    std::fs::create_dir_all(&work).ok();
    let codegraph = env("MONOBENCH_CODEGRAPH", "codegraph");
    let out = root.join("results").join(id);
    std::fs::create_dir_all(&out).ok();
    let clone = match shared_clone(&repo_url, &repo_tag, &work, wtlock) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("prepare clone failed: {e}");
            return 1;
        }
    };
    for tool in tools {
        let tooldir = root.join("harness/tools").join(tool);
        if !tooldir.join("tool.json").is_file() {
            eprintln!("no tool adapter '{tool}'");
            return 1;
        }
        let tj = read_json(&tooldir.join("tool.json"));
        let runid = format!("_prepare-{tool}-t{}", unix_ms());
        let marker_path = out.join(format!("{runid}.running"));
        let marker = RunningMarker { path: marker_path };
        marker.set("prepare", &format!("tool={tool} repo={}", clone.display()));
        let log_path = out.join(format!("{runid}.index.log"));
        println!(
            "prepare {id}/{tool} on {} -> {}",
            clone.display(),
            log_path.display()
        );
        let uses_monogram_index = tool_uses_monogram_index(&tj);
        let tool_version = tool_version_from_json(&tj);
        if uses_monogram_index
            && prepared_monogram_snapshot_ready(&out, tool, &tool_version)
            && !refresh_prepared_requested()
        {
            append_log(
                &log_path,
                &format!(
                    "[prepared] reused existing monogram snapshot version={}; set MONOBENCH_REFRESH_PREPARED=1 to rebuild\n",
                    if tool_version.is_empty() { "-" } else { &tool_version }
                ),
            );
            println!("  prepared {tool} (reused)");
            continue;
        }
        if uses_monogram_index {
            remove_sqlite_db(&monogram_project_db_path(&clone));
            append_log(
                &log_path,
                "[prepared] cleared existing monogram DB before snapshot index\n",
            );
        }
        if let Err(e) = run_index(&tj, &clone, &clone, &codegraph, &log_path, &marker) {
            eprintln!("prepare index failed for '{tool}': {e}");
            return 1;
        }
        if uses_monogram_index {
            if let Err(e) =
                save_prepared_monogram_snapshot(&out, tool, &clone, &tool_version, &log_path)
            {
                eprintln!("prepare snapshot failed for '{tool}': {e}");
                return 1;
            }
        }
        println!("  prepared {tool}");
    }
    0
}

// `cli` and `model` are parameters (not env) because matrix runs threads in-process and each needs its own.
// `quiet` suppresses the per-run ▶/grade lines (matrix prints its own ✓ + a final report instead).
pub fn run(
    root: &Path,
    id: &str,
    arm: &str,
    cli: &str,
    model: &str,
    via: &str,
    run_no: usize,
    repeat_total: Option<usize>,
    tag: Option<&str>,
    note: Option<&str>,
    quiet: bool,
    wtlock: &Mutex<()>,
) -> i32 {
    let inst_dir = root.join("instances").join(id);
    if !inst_dir.is_dir() {
        eprintln!("no instance '{id}'");
        return 1;
    }
    let inst = load_inst(&inst_dir.join("instance.json").to_string_lossy());
    if let Some(reason) = &inst.invalid {
        eprintln!("invalid instance '{id}': {reason}");
        eprintln!(
            "refusing to run because provisional/TODO grading would corrupt benchmark results"
        );
        return 1;
    }
    let tooldir = root.join("harness/tools").join(arm);
    if !tooldir.join("tool.json").is_file() {
        eprintln!("no tool adapter '{arm}'");
        return 1;
    }
    let inst_json = read_json(&inst_dir.join("instance.json"));
    let repo_url = inst_json
        .get("repo")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let repo_tag = inst_json
        .get("tag")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
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
    let cap = env("MONOBENCH_CAP", "6");
    let cli = cli.to_lowercase();
    let via = via.to_lowercase();
    let prepared = std::env::var("MONOBENCH_PREPARED")
        .map(|v| v == "1")
        .unwrap_or(false);

    // Capture the tool's version (e.g. monogram semver) so runs from different builds form DISTINCT
    // arms instead of silently averaging together. `version_bin` is declared per tool.json; baseline
    // omits it → no version segment, identical to legacy labels. Empty when not OpenCLIs-installed.
    let tool_version = tool_version_from_json(&tj);
    let label = full_arm_name(arm, &tool_version, &cli, model, &effort);
    let runid = unique_runid(&out, &label, run_no);

    // Unique-key pre-flight: runid (<tool>-<cli>-<model>-rN-t<start_ms>) is the ONLY key for
    // results. The timestamp keeps repeated rN experiments distinct without cleaning old files.
    // Claim it BEFORE expensive worktree+index so status can see pre-model phases.
    let force = std::env::var("MONOBENCH_FORCE")
        .map(|v| v == "1")
        .unwrap_or(false);
    let running_path = out.join(format!("{runid}.running"));
    if !force && (result_exists(&out, &runid) || running_path.exists()) {
        if !quiet {
            eprintln!("skip {runid}: results or active marker exist — use --force to overwrite");
        }
        return 0;
    }
    let marker = format!(
        "pid={} tool={} cli={} model={} via={} effort={} run={} tag={} note={}\n",
        std::process::id(),
        arm,
        cli,
        model,
        via,
        if effort.is_empty() { "-" } else { &effort },
        run_no,
        tag.unwrap_or("-"),
        note.unwrap_or("-")
    );
    let marker_file = if force {
        File::create(&running_path)
    } else {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&running_path)
    };
    let mut marker_file = match marker_file {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if !quiet {
                eprintln!("skip {runid}: active marker exists — use --force to overwrite");
            }
            return 0;
        }
        Err(e) => {
            eprintln!("create {runid}.running failed: {e}");
            return 1;
        }
    };
    let _ = marker_file.write_all(marker.as_bytes());
    drop(marker_file);
    let running_guard = RunningMarker { path: running_path };
    let isolate = std::env::var("MONOBENCH_ISOLATE").unwrap_or_default() == "worktree";
    run_meta::write_start(
        &out,
        StartMeta {
            id,
            run_id: &runid,
            label: &label,
            tool: arm,
            cli: &cli,
            model,
            effort: &effort,
            via: &via,
            monogram_version: if tool_version.is_empty() {
                None
            } else {
                Some(&tool_version)
            },
            repeat_index: run_no,
            repeat_total,
            tag,
            note,
            started_at_ms: runid_timestamp(&runid),
            prepared,
            isolate: if isolate { "worktree" } else { "shared" },
        },
    );

    // 1. repo: worktree-isolated (parallel-safe) or shared clone
    let _wt_guard;
    let clone: PathBuf;
    // Repo base clone, kept in scope at the per-CLI call sites so the sandbox jail can
    // re-allow it (git-worktree's `.git` file points back to the base). Some in isolate
    // mode, None in shared-clone mode (which doesn't use git-worktree).
    let repo_base: Option<PathBuf>;
    if isolate {
        running_guard.set("clone", "worktree");
        let base = PathBuf::from(format!("{work}/{}-base", repo_basename(&repo_url)));
        let base_lock = PathBuf::from(format!("{work}/{}-base.lock", repo_basename(&repo_url)));
        // Cross-process base guard (Option A): separate monobench processes (e.g. a `sweep` and a
        // `matrix`) sharing this repo base serialize their clone + worktree add. The in-process
        // `wtlock` only orders threads within ONE process; this O_EXCL lock orders them ACROSS
        // processes. Held only over clone/add (NOT the run); different repos ⇒ different lock files
        // ⇒ still parallel. `flk.is_none()` (budget exhausted) degrades to the in-process lock.
        let flk = acquire_file_lock(&base_lock, 6000, Duration::from_millis(100));
        if flk.is_none() {
            eprintln!(
                "  ⚠ {}-base.lock not acquired in 600s — proceeding (in-process lock only)",
                repo_basename(&repo_url)
            );
        }
        {
            let _g = wtlock.lock().unwrap();
            if !base.join(".git").is_dir() {
                Command::new("git")
                    .args(["clone", "--filter=blob:none", "--quiet", &repo_url])
                    .arg(&base)
                    .status()
                    .ok();
            }
            Command::new("git")
                .arg("-C")
                .arg(&base)
                .args(["worktree", "prune"])
                .output()
                .ok();
        }
        let wt = worktree_run_dir(&work, id, &runid);
        std::fs::remove_dir_all(&wt).ok();
        if let Some(parent) = wt.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        {
            let _g = wtlock.lock().unwrap();
            Command::new("git")
                .arg("-C")
                .arg(&base)
                .args(["worktree", "add", "--quiet", "--force", "--detach"])
                .arg(&wt)
                .arg(&repo_tag)
                .output()
                .ok();
        }
        drop(flk); // release cross-process base lock before the (long) run
        clone = wt.clone(); // ${REPO} is substituted from `clone` directly (no process-global env — matrix runs threads)
        // Capture base BEFORE it moves into _wt_guard, so agy_read_jail_profile can re-allow
        // reads on the per-repo base clone (per-repo allow ⇒ still structurally safe, since
        // a netty agent gets netty-base only, not php-src-base).
        repo_base = Some(base.clone());
        _wt_guard = Some(Worktree {
            base,
            wt,
            base_lock,
            lock: wtlock,
        });
    } else {
        running_guard.set("clone", "shared");
        clone = match shared_clone(&repo_url, &repo_tag, &work, wtlock) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("shared clone failed: {e}");
                return 1;
            }
        };
        // Shared-clone mode doesn't use git-worktree, so there's no separate base-clone
        // path to re-allow in the jail.
        repo_base = None;
        _wt_guard = None;
    }

    if !quiet {
        println!(
            "▶ {id} / {runid}  (deliver={}, cli={cli}, model={model}, via={via}, isolate={})",
            if deliver.is_empty() { "none" } else { &deliver },
            if isolate { "worktree" } else { "shared" }
        );
    }

    // 2. index for the tool (+ FORFEIT if it can't)
    let index_log = out.join(format!("{runid}.index.log"));
    let log = if prepared && isolate && tool_uses_monogram_index(&tj) {
        match install_prepared_monogram_snapshot(&out, arm, &clone, &index_log, &running_guard) {
            Ok(log) => log,
            Err(e) => {
                eprintln!("prepared index install failed for '{arm}': {e}");
                return 1;
            }
        }
    } else if prepared && !isolate {
        running_guard.set(
            "index",
            &format!("prepared-skip log={}", index_log.display()),
        );
        append_log(
            &index_log,
            "[prepared] skipped per-run index; using stable shared clone index\n",
        );
        String::new()
    } else {
        if prepared && isolate {
            append_log(
                &index_log,
                "[prepared] no reusable snapshot for this tool; running adapter index\n",
            );
        }
        running_guard.set("index", &format!("log={}", index_log.display()));
        match run_index(&tj, &clone, &clone, &codegraph, &index_log, &running_guard) {
            Ok(log) => log,
            Err(e) => {
                eprintln!("index failed for '{arm}': {e}");
                return 1;
            }
        }
    };
    if !fgrep.is_empty() {
        let ll = log.to_lowercase();
        if fgrep
            .split('|')
            .any(|p| !p.is_empty() && ll.contains(&p.to_lowercase()))
        {
            let msg = format!("  FORFEIT — '{arm}' could not index this repo");
            println!("{msg}");
            std::fs::write(out.join(format!("{runid}.forfeit")), msg).ok();
            return 0;
        }
    }

    // 3. prompt preamble: lead.md + initiate.md + skill.md + depth.md (docs shoved into the -p prompt)
    let cat = |p: PathBuf| std::fs::read_to_string(p).unwrap_or_default();
    let mut sys = cat(root.join("harness/prompts/depth.md"));
    if !skill.is_empty() && tooldir.join(&skill).is_file() {
        sys = format!("{}\n\n{}", cat(tooldir.join(&skill)), sys);
    }
    if tooldir.join("initiate.md").is_file() {
        sys = format!("{}\n\n{}", cat(tooldir.join("initiate.md")), sys);
    }
    if tooldir.join("lead.md").is_file() {
        sys = format!("{}\n\n{}", cat(tooldir.join("lead.md")), sys);
    }

    // 4. MCP config (per-run filename, parallel-safe)
    let mcpcfg = if deliver == "mcp" {
        let p = out.join(format!("mcp-{runid}.json"));
        let mcp = tj.get("mcp").cloned().unwrap_or(Value::Null);
        let raw_command = mcp.get("command").and_then(Value::as_str).unwrap_or("");
        let raw_args: Vec<String> = mcp
            .get("args")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let (command, args) = match command_and_args(raw_command, &raw_args, &clone, &codegraph) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("invalid mcp config for '{arm}': {e}");
                return 1;
            }
        };
        let cfg =
            serde_json::json!({ "mcpServers": { arm: { "command": command, "args": args } } });
        std::fs::write(&p, cfg.to_string()).ok();
        p
    } else {
        let p = out.join(format!("mcp-empty-{runid}.json"));
        std::fs::write(&p, "{\"mcpServers\":{}}").ok();
        p
    };

    let q = cat(inst_dir.join("symptom.md"));
    if q.to_lowercase().contains("todo") {
        eprintln!("invalid instance '{id}': symptom.md still contains TODO");
        eprintln!("refusing to run because provisional symptoms would corrupt benchmark results");
        return 1;
    }

    // agy model preflight (mirrors the codex model-mismatch guard above): `agy --print` has no
    // --model flag, so the model is fixed by ~/.gemini/antigravity-cli/settings.json. Rather than
    // silently record a run under the wrong label, refuse unless the configured model matches the
    // requested --model. This keeps the agy model axis honest — one model per agy settings.
    if cli == "agy" {
        match agy_settings_model() {
            Some(actual) if agy_model_norm(&actual) == agy_model_norm(model) => {}
            Some(actual) => {
                eprintln!(
                    "agy model mismatch: --model is '{model}' but agy is configured for '{actual}'"
                );
                eprintln!(
                    "agy --print has no --model flag; set the model in ~/.gemini/antigravity-cli/settings.json to match, then run one matrix command per model"
                );
                return 1;
            }
            None => {
                eprintln!(
                    "agy preflight: cannot read model from ~/.gemini/antigravity-cli/settings.json — refusing rather than record an unverified model"
                );
                return 1;
            }
        }
    }

    running_guard.set("solver", &format!("cli={cli} via={via} model={model}"));
    let prepared_monogram_guard = prepared && tool_uses_monogram_index(&tj);
    match (via.as_str(), cli.as_str()) {
        ("niia", _) => {
            if let Err(e) = crate::niia_runner::run(
                root,
                &clone,
                &format!("{sys}\n\n{q}\n"),
                "ROOTCAUSE",
                &out.join(&runid),
                &effort,
                &cli,
                model,
                prepared_monogram_guard,
            ) {
                eprintln!("niia runner failed: {e}");
                return 1;
            }
            if !quiet {
                print_grade(&grade_text_file(
                    &inst,
                    &out.join(format!("{runid}.answer.txt")).to_string_lossy(),
                    &out.join(format!("{runid}.meter.json")).to_string_lossy(),
                ));
            }
        }
        ("direct", "codex") => {
            let pf = std::env::temp_dir().join(format!("mb-pf-{runid}"));
            std::fs::write(&pf, format!("{sys}\n\n{q}\n")).ok();
            let ans = out.join(format!("{runid}.answer.txt"));
            let err = out.join(format!("{runid}.err"));
            let solver_deny = install_solver_deny_wrapper(&runid, prepared_monogram_guard);
            // kernel-level git block (+ answer-key/.git read-jail) — codex has no usable native
            // command-deny and runs with --dangerously-bypass-…-sandbox, so the PATH shim alone
            // is bypassable. Wrap in sandbox-exec like agy.
            let t0 = std::time::Instant::now();
            let mut cmd =
                jailed_solver_command("codex", root, &runid, &clone, repo_base.as_deref());
            cmd.arg("exec").arg("-C").arg(&clone).args([
                "--skip-git-repo-check",
                "--dangerously-bypass-approvals-and-sandbox",
            ]);
            let codex_model_override = std::env::var("MONOBENCH_CODEX_MODEL")
                .ok()
                .filter(|m| !m.is_empty());
            if let Some(m) = &codex_model_override {
                if m != model {
                    eprintln!(
                        "codex model mismatch: --model/MONOBENCH_MODEL is '{model}', but MONOBENCH_CODEX_MODEL is '{m}'"
                    );
                    eprintln!(
                        "run one matrix command per model and keep the label and actual Codex model identical"
                    );
                    return 1;
                }
            }
            if codex_model_override.is_none() && model == "opus" {
                eprintln!(
                    "codex runner requires --model/MONOBENCH_MODEL with a Codex/GPT model id"
                );
                return 1;
            }
            let codex_model = model.to_string();
            cmd.arg("-m").arg(&codex_model);
            cmd.arg("-c").arg(format!(
                "model_reasoning_effort={}",
                if effort.is_empty() {
                    "high".into()
                } else {
                    effort.clone()
                }
            ));
            cmd.arg("-o").arg(&ans);
            for e in STRIP_ENV {
                cmd.env_remove(e);
            }
            if prepared_monogram_guard {
                cmd.env("MONOGRAM_PREPARED_INDEX", "1");
            }
            if let Some(dir) = &solver_deny {
                prepend_path(&mut cmd, dir);
            }
            cmd.stdin(File::open(&pf).unwrap())
                .stdout(File::create(out.join(format!("{runid}.codexlog"))).unwrap())
                .stderr(File::create(&err).unwrap());
            cmd.status().ok();
            let dur = t0.elapsed().as_secs();
            Command::new("monometer")
                .args(["daemon", "recompute"])
                .output()
                .ok();
            std::thread::sleep(std::time::Duration::from_secs(1));
            let meter = codex_meter(dur, &err, &codex_model);
            std::fs::write(out.join(format!("{runid}.meter.json")), meter).ok();
            if !quiet {
                print_grade(&grade_text_file(
                    &inst,
                    &ans.to_string_lossy(),
                    &out.join(format!("{runid}.meter.json")).to_string_lossy(),
                ));
            }
        }
        ("direct", "agy") => {
            let prompt = format!("{sys}\n\n{q}\n");
            let ans = out.join(format!("{runid}.answer.txt"));
            let err = out.join(format!("{runid}.err"));
            let log = out.join(format!("{runid}.agy.log"));
            let solver_deny = install_solver_deny_wrapper(&runid, prepared_monogram_guard);
            let t0 = std::time::Instant::now();
            // agy ignores the process cwd (it runs in ~/.gemini/antigravity-cli/scratch), so the
            // repo under test must be passed as a workspace dir (--add-dir) or agy indexes 0 files
            // and roams the filesystem. The sandbox-exec read-jail then stops it reading the
            // benchmark's own answer files it would otherwise find (instance.json / ground_truth.md).
            let mut cmd =
                jailed_solver_command("agy", root, &runid, &clone, repo_base.as_deref());
            cmd.current_dir(&clone)
                .arg("--add-dir")
                .arg(&clone)
                .arg("--log-file")
                .arg(&log)
                .arg("--print-timeout")
                .arg(std::env::var("MONOBENCH_AGY_TIMEOUT").unwrap_or_else(|_| "20m".into()))
                .arg("--dangerously-skip-permissions")
                .arg("--print")
                .arg(prompt);
            for e in STRIP_ENV {
                cmd.env_remove(e);
            }
            if prepared_monogram_guard {
                cmd.env("MONOGRAM_PREPARED_INDEX", "1");
            }
            if let Some(dir) = &solver_deny {
                prepend_path(&mut cmd, dir);
            }
            cmd.stdout(File::create(&ans).unwrap())
                .stderr(File::create(&err).unwrap());
            let status = cmd.status();
            let dur = t0.elapsed().as_secs();
            if let Some(cid) = parse_agy_conversation_id(&log) {
                if let Some(src) = wait_for_agy_transcript(&cid) {
                    let _ = std::fs::copy(src, out.join(format!("{runid}.agy.jsonl")));
                }
            }
            let observed_model = parse_agy_observed_model(&log);
            let (exit_status, exit_success, runner_error) = match status {
                Ok(s) => (
                    s.code(),
                    s.success(),
                    if s.success() {
                        None
                    } else {
                        Some(format!("agy exited with {s}"))
                    },
                ),
                Err(e) => (None, false, Some(format!("failed to run agy: {e}"))),
            };
            // Verified, not enforced: preflight already refused on a settings≠label mismatch;
            // here we confirm the model agy actually logged matches the requested label.
            let model_verified = observed_model
                .as_deref()
                .map(|o| agy_model_norm(o) == agy_model_norm(model))
                .unwrap_or(false);
            let meter = serde_json::json!({
                "runner": "agy",
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
                "meter_error": "agy usage/cost telemetry unavailable",
                "duration_s": dur,
                "exit_status": exit_status,
                "exit_success": exit_success,
                "runner_error": runner_error,
                "transcript_available": out.join(format!("{runid}.agy.jsonl")).is_file()
            })
            .to_string();
            std::fs::write(out.join(format!("{runid}.meter.json")), meter).ok();
            if !quiet {
                print_grade(&grade_text_file(
                    &inst,
                    &ans.to_string_lossy(),
                    &out.join(format!("{runid}.meter.json")).to_string_lossy(),
                ));
            }
        }
        ("direct", "claude") => {
            // claude -p
            let f = out.join(format!("{runid}.jsonl"));
            // Unique-key guard: runid (<arm>-<cli>-<model>-rN-t<start_ms>) is the ONLY key for results, so a
            // colliding run would silently truncate (sequential) or interleave-corrupt
            // (concurrent) this transcript. Claim it atomically (O_EXCL) unless MONOBENCH_FORCE=1.
            let opened = if force {
                File::create(&f)
            } else {
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&f)
            };
            let jsonl_file = match opened {
                Ok(fh) => fh,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if !quiet {
                        eprintln!("  skip {runid}: results exist — use --force to overwrite");
                    }
                    return 0;
                }
                Err(e) => {
                    eprintln!("create {runid}.jsonl failed: {e}");
                    return 1;
                }
            };
            let prompt = format!("{sys}\n\n{}\n# YOUR TASK\n{q}", "═".repeat(80));
            let solver_deny = install_solver_deny_wrapper(&runid, prepared_monogram_guard);
            // kernel-level git block (+ read-jail) in addition to --disallowedTools "Bash(git:*)":
            // the matcher misses compound/absolute git (`cd x && /usr/bin/git log`); sandbox-exec
            // closes that hole for every invocation.
            let mut cmd =
                jailed_solver_command("claude", root, &runid, &clone, repo_base.as_deref());
            cmd.current_dir(&clone).arg("-p").arg(&prompt).args([
                "--output-format",
                "stream-json",
                "--verbose",
                "--permission-mode",
                "bypassPermissions",
                "--model",
                &model,
            ]);
            if !effort.is_empty() {
                cmd.arg("--effort").arg(&effort);
            }
            cmd.args([
                "--max-budget-usd",
                &cap,
                "--setting-sources",
                "",
                "--disable-slash-commands",
                "--strict-mcp-config",
            ])
            .arg("--mcp-config")
            .arg(&mcpcfg)
            .args(["--disallowedTools", "Bash(git:*)"]); // anti-contamination: no reading the fix from git history
            for e in STRIP_ENV {
                cmd.env_remove(e);
            }
            if prepared_monogram_guard {
                cmd.env("MONOGRAM_PREPARED_INDEX", "1");
            }
            // anti-contamination: PATH-shadow `git` (exit 126) so bare-git invocations the claude
            // --disallowedTools "Bash(git:*)" matcher misses (e.g. `cd x && git log`) also fail.
            if let Some(dir) = &solver_deny {
                prepend_path(&mut cmd, dir);
            }
            cmd.stdout(jsonl_file)
                .stderr(File::create(out.join(format!("{runid}.err"))).unwrap());
            cmd.status().ok();
            if !quiet {
                print_grade(&grade_jsonl(&inst, &f.to_string_lossy()));
            }
        }
        ("direct", "grok") => {
            // grok -p <prompt> --output-format json → {text, stopReason, sessionId, requestId, thought}.
            // No per-turn token split or cost (OAuth subscription, single model grok-build); telemetry
            // comes from the session's signals.json, located via the returned sessionId.
            let prompt = format!("{sys}\n\n{q}\n");
            let ans = out.join(format!("{runid}.answer.txt"));
            let envelope = out.join(format!("{runid}.grok.json"));
            let err = out.join(format!("{runid}.err"));
            let grok_model = if model.is_empty() {
                "grok-build"
            } else {
                model
            };
            let solver_deny = install_solver_deny_wrapper(&runid, prepared_monogram_guard);
            // kernel-level git block (+ read-jail); grok runs --always-approve so the PATH shim
            // alone is bypassable. Wrap in sandbox-exec like agy.
            let t0 = std::time::Instant::now();
            let mut cmd =
                jailed_solver_command("grok", root, &runid, &clone, repo_base.as_deref());
            cmd.current_dir(&clone)
                .arg("-p")
                .arg(&prompt)
                .arg("--cwd")
                .arg(&clone)
                .arg("--model")
                .arg(grok_model)
                .args([
                    "--output-format",
                    "json",
                    "--always-approve",
                    "--no-subagents",
                ]);
            if !effort.is_empty() {
                cmd.arg("--effort").arg(&effort);
            }
            for e in STRIP_ENV {
                cmd.env_remove(e);
            }
            if prepared_monogram_guard {
                cmd.env("MONOGRAM_PREPARED_INDEX", "1");
            }
            if let Some(dir) = &solver_deny {
                prepend_path(&mut cmd, dir);
            }
            cmd.stdout(File::create(&envelope).unwrap())
                .stderr(File::create(&err).unwrap());
            let status = cmd.status();
            let dur = t0.elapsed().as_secs();
            // Envelope → answer text for grading; keep sessionId to find signals.json.
            let (answer, session_id) = parse_grok_envelope(&envelope);
            std::fs::write(&ans, &answer).ok();
            let (exit_status, exit_success, runner_error) = match status {
                Ok(s) => (
                    s.code(),
                    s.success(),
                    if s.success() {
                        None
                    } else {
                        Some(format!("grok exited with {s}"))
                    },
                ),
                Err(e) => (None, false, Some(format!("failed to run grok: {e}"))),
            };
            let meter = grok_meter(
                dur,
                session_id.as_deref(),
                grok_model,
                &effort,
                exit_status,
                exit_success,
                runner_error,
            );
            std::fs::write(out.join(format!("{runid}.meter.json")), meter).ok();
            if !quiet {
                print_grade(&grade_text_file(
                    &inst,
                    &ans.to_string_lossy(),
                    &out.join(format!("{runid}.meter.json")).to_string_lossy(),
                ));
            }
        }
        ("direct", "opencode") => {
            // opencode run <prompt> --format json → streams events to stdout, exits when the
            // session goes idle. Model format is `provider/model` (e.g. `anthropic/claude-haiku-4-5`).
            // Like grok, opencode is a subscription/API-key CLI without per-run cost in its
            // stdout — we capture duration + activity, not billed tokens.
            let prompt = format!("{sys}\n\n{q}\n");
            let ans = out.join(format!("{runid}.answer.txt"));
            let envelope = out.join(format!("{runid}.opencode.json"));
            let err = out.join(format!("{runid}.err"));
            let opencode_model = if model.is_empty() {
                eprintln!(
                    "opencode runner requires --model/MONOBENCH_MODEL in 'provider/model' format \
                     (e.g. 'anthropic/claude-haiku-4-5')"
                );
                return 1;
            } else {
                model
            };
            let solver_deny = install_solver_deny_wrapper(&runid, prepared_monogram_guard);
            // kernel-level git block (+ read-jail). opencode walks the filesystem to find project
            // markers; sandbox-exec confines reads to the assigned worktree like the other CLIs.
            let t0 = std::time::Instant::now();
            let mut cmd =
                jailed_solver_command("opencode", root, &runid, &clone, repo_base.as_deref());
            // opencode run <prompt> [project] -m provider/model --format json
            //   --dangerously-skip-permissions: opencode otherwise auto-rejects every tool call
            //     in non-interactive mode (read/edit/glob/grep/list/bash/external_directory all
            //     prompt-gated). The KERNEL sandbox (sandbox-exec) is doing the actual confinement
            //     — opencode's app-level prompt would just freeze the run since stdin is a pipe.
            //     Same flag name and semantics as claude's `--dangerously-skip-permissions`.
            // The positional `project` sets opencode's project root to our assigned worktree;
            // current_dir matches it so libc cwd resolution lands inside the allowed subpath.
            cmd.current_dir(&clone)
                .arg("run")
                .arg(&prompt)
                .arg(&clone)
                .arg("-m")
                .arg(opencode_model)
                .arg("--dangerously-skip-permissions")
                .args(["--format", "json"]);
            for e in STRIP_ENV {
                cmd.env_remove(e);
            }
            if prepared_monogram_guard {
                cmd.env("MONOGRAM_PREPARED_INDEX", "1");
            }
            if let Some(dir) = &solver_deny {
                prepend_path(&mut cmd, dir);
            }
            cmd.stdout(File::create(&envelope).unwrap())
                .stderr(File::create(&err).unwrap());
            let status = cmd.status();
            let dur = t0.elapsed().as_secs();
            // Envelope is a stream of JSON events; extract the agent's final text for grading.
            let answer = parse_opencode_envelope(&envelope);
            std::fs::write(&ans, &answer).ok();
            let (exit_status, exit_success, runner_error) = match status {
                Ok(s) => (
                    s.code(),
                    s.success(),
                    if s.success() {
                        None
                    } else {
                        Some(format!("opencode exited with {s}"))
                    },
                ),
                Err(e) => (None, false, Some(format!("failed to run opencode: {e}"))),
            };
            let meter = opencode_meter(
                dur,
                opencode_model,
                &effort,
                exit_status,
                exit_success,
                runner_error,
            );
            std::fs::write(out.join(format!("{runid}.meter.json")), meter).ok();
            if !quiet {
                print_grade(&grade_text_file(
                    &inst,
                    &ans.to_string_lossy(),
                    &out.join(format!("{runid}.meter.json")).to_string_lossy(),
                ));
            }
        }
        ("direct", other) => {
            eprintln!(
                "unsupported direct cli '{other}' (supported: claude, codex, agy, grok, opencode; use --via niia for other CLIs)"
            );
            return 1;
        }
        (other, _) => {
            eprintln!("unsupported --via '{other}' (supported: direct, niia)");
            return 1;
        }
    }
    0
}

fn sub_vars(s: &str, repo: &Path, codegraph: &str) -> String {
    s.replace("${REPO}", &repo.to_string_lossy())
        .replace("${CODEGRAPH}", codegraph)
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
            if ch == q {
                quote = None;
            } else {
                cur.push(ch);
            }
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
    if esc {
        cur.push('\\');
    }
    if quote.is_some() {
        return Err("unterminated quote".into());
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    Ok(words)
}

fn command_and_args(
    command: &str,
    args: &[String],
    repo: &Path,
    codegraph: &str,
) -> Result<(String, Vec<String>), String> {
    let expanded = sub_vars(command, repo, codegraph);
    let mut words = split_words(&expanded)?;
    if words.is_empty() {
        return Err("missing command".into());
    }
    let exe = words.remove(0);
    for arg in args {
        words.push(sub_vars(arg, repo, codegraph));
    }
    Ok((exe, words))
}

fn append_log(path: &Path, text: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(text.as_bytes());
    }
}

fn run_argv(command: &str, args: &[String], cwd: &Path, log_path: &Path) -> Result<String, String> {
    append_log(
        log_path,
        &format!(
            "\n$ {} {}\n",
            command,
            args.iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(" ")
        ),
    );
    let out = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|e| format!("{}: {e}", log_path.display()))?;
    let err = out
        .try_clone()
        .map_err(|e| format!("{}: {e}", log_path.display()))?;
    let status = Command::new(command)
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .status()
        .map_err(|e| format!("{command}: {e}"))?;
    if !status.success() {
        append_log(
            log_path,
            &format!("\n[exit {}]\n", status.code().unwrap_or(-1)),
        );
    }
    Ok(std::fs::read_to_string(log_path).unwrap_or_default())
}

fn run_index_step(
    step: &Value,
    cwd: &Path,
    repo: &Path,
    codegraph: &str,
    log_path: &Path,
    marker: &RunningMarker,
) -> Result<String, String> {
    let command = step.get("command").and_then(Value::as_str).unwrap_or("");
    let args: Vec<String> = step
        .get("args")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let quiet = step.get("quiet").and_then(Value::as_bool).unwrap_or(false);
    let (exe, argv) = command_and_args(command, &args, repo, codegraph)?;
    marker.set("index", &format!("cmd={} log={}", exe, log_path.display()));
    let log = run_argv(&exe, &argv, cwd, log_path)?;
    if quiet {
        Ok(String::new())
    } else {
        Ok(log)
    }
}

fn run_legacy_index(
    index: &str,
    cwd: &Path,
    repo: &Path,
    codegraph: &str,
    log_path: &Path,
    marker: &RunningMarker,
) -> Result<String, String> {
    if index
        .chars()
        .any(|c| matches!(c, ';' | '|' | '&' | '<' | '>' | '`'))
    {
        return Err("legacy index contains shell operators; convert it to index_steps".into());
    }
    let expanded = sub_vars(index, repo, codegraph);
    let mut words = split_words(&expanded)?;
    if words.is_empty() {
        return Ok(String::new());
    }
    let exe = words.remove(0);
    marker.set("index", &format!("cmd={} log={}", exe, log_path.display()));
    run_argv(&exe, &words, cwd, log_path)
}

fn run_index(
    tool_json: &Value,
    cwd: &Path,
    repo: &Path,
    codegraph: &str,
    log_path: &Path,
    marker: &RunningMarker,
) -> Result<String, String> {
    std::fs::remove_file(log_path).ok();
    if let Some(steps) = tool_json.get("index_steps").and_then(Value::as_array) {
        let mut log = String::new();
        for step in steps {
            log.push_str(&run_index_step(
                step, cwd, repo, codegraph, log_path, marker,
            )?);
        }
        return Ok(log);
    }
    let index = tool_json
        .get("index")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if index.is_empty() {
        Ok(String::new())
    } else {
        run_legacy_index(index, cwd, repo, codegraph, log_path, marker)
    }
}

pub(crate) fn install_solver_deny_wrapper(
    runid: &str,
    block_monogram_reindex: bool,
) -> Option<PathBuf> {
    let dir = std::env::temp_dir().join(format!(
        "monobench-solver-deny-{runid}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).ok()?;
    let git = dir.join("git");
    std::fs::write(&git, "#!/bin/sh\necho 'monobench: git is disabled during solver runs (anti-contamination)' >&2\nexit 126\n").ok()?;
    if block_monogram_reindex {
        let monogram = dir.join("monogram");
        let real = resolve_executable("monogram").unwrap_or_else(|| PathBuf::from("monogram"));
        std::fs::write(&monogram, monogram_prepared_guard_script(&real)).ok()?;
    }
    #[cfg(unix)]
    {
        for exe in ["git", "monogram"] {
            let path = dir.join(exe);
            if !path.exists() {
                continue;
            }
            let mut perm = std::fs::metadata(&path).ok()?.permissions();
            perm.set_mode(0o755);
            std::fs::set_permissions(&path, perm).ok()?;
        }
    }
    Some(dir)
}

fn resolve_executable(name: &str) -> Option<PathBuf> {
    if name.contains(std::path::MAIN_SEPARATOR) {
        let path = PathBuf::from(name);
        return path.is_file().then_some(path);
    }
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn monogram_prepared_guard_script(real_monogram: &Path) -> String {
    format!(
        r#"#!/bin/sh
case "${{1:-}}" in
  index|i|reindex|prune)
    echo 'monobench: prepared monogram index is already installed; solver-side monogram index mutation is disabled.' >&2
    echo '[NEXT]' >&2
    echo '  monogram stats' >&2
    echo '  monogram search "<symptom words>" -n 8' >&2
    echo '  monogram region "<implementation intent>" -n 5 --score-debug' >&2
    exit 126
    ;;
esac
case "${{1:-}} ${{2:-}}" in
  "boot init"|"b init")
    echo 'monobench: prepared monogram index is already installed; solver-side boot init is disabled.' >&2
    echo '[NEXT]' >&2
    echo '  monogram stats' >&2
    echo '  monogram search "<symptom words>" -n 8' >&2
    echo '  monogram region "<implementation intent>" -n 5 --score-debug' >&2
    exit 126
    ;;
esac
for arg in "$@"; do
  case "$arg" in
    -r|--reindex)
      echo 'monobench: prepared monogram index is already installed; solver-side -r/--reindex is disabled.' >&2
      echo '[NEXT]' >&2
      echo '  monogram stats' >&2
      echo '  monogram search "<symptom words>" -n 8' >&2
      echo '  monogram region "<implementation intent>" -n 5 --score-debug' >&2
      exit 126
      ;;
  esac
done
export MONOGRAM_PREPARED_INDEX=1
exec {} "$@"
"#,
        sh_quote(&real_monogram.to_string_lossy())
    )
}

fn prepend_path(cmd: &mut Command, dir: &Path) {
    let old = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![dir.to_path_buf()];
    paths.extend(std::env::split_paths(&old));
    if let Ok(joined) = std::env::join_paths(paths) {
        cmd.env("PATH", joined);
    }
}

/// macOS `sandbox-exec` read-jail for every solver (claude/agy/codex/grok), used by both the
/// direct and niia runners. The profile starts from `(allow default)` (industry pattern — pure
/// deny-default breaks language toolchains and package managers) and layers targeted denies
/// plus a per-run re-allow:
///
///   - DENIES git exec + `.git` reads → the agent can't read the fix-commit from history
///   - DENIES `<root>/instances` + `<root>/research` → can't read the benchmark's answer keys
///   - DENIES `/tmp/monobench-work/` reads → STRUCTURAL: sibling worktrees become physically
///     invisible to the agent regardless of how it asks (`find`, absolute path, anything)
///   - RE-ALLOWS the assigned worktree (more-specific subpath wins over the broader deny under
///     SBPL precedence — verified empirically) → the agent has full read access to ITS OWN
///     worktree
///   - RE-ALLOWS the repo's base clone if known → git-worktree's `.git` file points back to
///     the base repo's `.git/worktrees/<wt>/`; without this allow, the agent can't follow that
///     pointer for legitimate ops. Per-REPO not per-instance, so cross-instance contamination
///     stays blocked (netty agent reads netty-base, not php-src-base).
///
/// `repo_base = None` (shared-clone mode or niia path without base in scope) yields a profile
/// without the base re-allow. Shared-clone mode doesn't use git-worktree so this is correct;
/// the niia path accepts a slightly tighter policy as a known limitation.
///
/// Returns the profile path, or `None` on non-macOS / write failure, in which case the caller
/// runs the solver unwrapped rather than failing the run (degrade, don't break).
pub(crate) fn agy_read_jail_profile(
    root: &Path,
    tag: &str,
    assigned_worktree: &Path,
    repo_base: Option<&Path>,
) -> Option<PathBuf> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let canon = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let bases: Vec<PathBuf> = if canon == root {
        vec![canon]
    } else {
        vec![canon, root.to_path_buf()]
    };
    let mut profile = String::from(
        "(version 1)\n(allow default)\n\
         ; anti-contamination: block reading any .git data — this is the real anti-cheat. Even if\n\
         ; a solver invokes git (absolute path, login shell, env), git itself can't read .git, so\n\
         ; `git log` exits with \"not a git repository\" and the fix-in-history vector is closed.\n\
         ;\n\
         ; NB: we intentionally do NOT deny process-exec* on `git`. That used to be defense-in-depth\n\
         ; but it breaks opencode, which spawns git at session-init for VCS detection — Bun's\n\
         ; posix_spawn EPERM throws synchronously and crashes opencode before it can fall through\n\
         ; to its \"not a git repo\" branch (Effect.catch in core/src/git.ts:run wraps proc.run\n\
         ; failure → exitCode 1 → vcs undefined). With process-exec allowed and .git reads denied,\n\
         ; opencode detects no-vcs gracefully (snapshot+vcs both skipped via `vcs !== \"git\"`\n\
         ; guards in 15+ callsites), while every other solver still cannot read git history.\n\
         (deny file-read* (regex #\"(^|/)\\.git(/|$)\"))\n",
    );
    for base in &bases {
        for sub in ["instances", "research"] {
            profile.push_str(&format!(
                "(deny file-read* (subpath {:?}))\n",
                base.join(sub).to_string_lossy()
            ));
        }
    }
    // STRUCTURAL cross-instance contamination guard: deny all reads under monobench-work, then
    // re-allow only OUR assigned worktree. SBPL precedence: more-specific allow beats broader
    // deny. Verified by `sandbox-exec` precedence test on macOS 15.4 (2026-05-28).
    profile.push_str(
        "; structural cross-instance contamination guard:\n\
         (deny file-read* (subpath \"/private/tmp/monobench-work\"))\n\
         (deny file-read* (subpath \"/tmp/monobench-work\"))\n",
    );
    let assigned_canon = std::fs::canonicalize(assigned_worktree)
        .unwrap_or_else(|_| assigned_worktree.to_path_buf());
    profile.push_str(&format!(
        "(allow file-read* (subpath {:?}))\n",
        assigned_canon.to_string_lossy()
    ));
    if let Some(rb) = repo_base {
        let rb_canon = std::fs::canonicalize(rb).unwrap_or_else(|_| rb.to_path_buf());
        profile.push_str(&format!(
            "(allow file-read* (subpath {:?}))\n",
            rb_canon.to_string_lossy()
        ));
    }
    // Ancestor path-traversal allows (metadata only). Many CLIs (grok empirically confirmed
    // on macOS 15.4 + grok 0.2.3; others likely benefit too) need to stat their cwd's ancestor
    // dirs at startup — the cwd is INSIDE the assigned worktree but the libc / Go runtime / etc.
    // resolves it by traversing up, which requires metadata access on each ancestor. The broader
    // file-read* deny above blocks even the stat.
    //
    // Adding `(allow file-read-metadata (literal "<ancestor>"))` for each ancestor under
    // /tmp/monobench-work unblocks the traversal without compromising content-read protection:
    //
    //   - `metadata` allows stat/access/readdir, NOT content reads (open RDONLY data)
    //   - `literal` matches just that exact path, not its subpaths
    //
    // Net effect: agent can stat `wt/`, `wt/<own-instance>/` (and see sibling instance NAMES
    // via readdir on `wt/` — public benchmark identifiers, no content leak), but the file-read*
    // subpath deny still blocks any CONTENT read of anything under sibling instances.
    let mut ancestor = assigned_canon.parent();
    while let Some(p) = ancestor {
        let path_str = p.to_string_lossy().to_string();
        if path_str.starts_with("/private/tmp/monobench-work")
            || path_str.starts_with("/tmp/monobench-work")
        {
            profile.push_str(&format!(
                "(allow file-read-metadata (literal {:?}))\n",
                path_str
            ));
            ancestor = p.parent();
        } else {
            break;
        }
    }
    let path = std::env::temp_dir().join(format!("monobench-agy-jail-{tag}.sb"));
    std::fs::write(&path, profile).ok()?;
    Some(path)
}

/// Linux equivalent of `agy_read_jail_profile` — returns bubblewrap arg list (without the
/// trailing solver binary). Built with the same allow-default + targeted-deny model as the
/// macOS SBPL profile:
///
///   1. `--ro-bind / /` exposes the host read-only as a baseline (so the solver finds libc,
///      git, its own config dirs, etc.). `--dev /dev` / `--proc /proc` / `--tmpfs /tmp`
///      reinstate the special filesystems bwrap can't pass through verbatim.
///   2. The benchmark answer-key dirs (`<root>/instances`, `<root>/research`) are masked with
///      `--tmpfs` so the solver cannot peek at gold patches even though the read-only root
///      bind would otherwise expose them.
///   3. `--tmpfs /tmp/monobench-work` (and `/private/tmp/...` for macOS hosts running these
///      args under Linux containers) hides sibling worktrees — the *next* arg re-binds ONLY
///      our assigned worktree, which means siblings simply do not exist inside the sandbox.
///      `--bind` (writable) lets the solver edit its own worktree.
///   4. `--tmpfs <worktree>/.git` (and `<repo_base>/.git` if a repo base is allowed) is the
///      real anti-cheat: tmpfs over the worktree's `.git` marker file replaces it with an
///      empty directory, which git treats as "not a git repository" — `git log` exits 128 and
///      the fix-in-history vector is closed even though the git binary itself can still run.
///   5. `--share-net` keeps the LLM API reachable; `--die-with-parent --new-session` make the
///      sandbox die cleanly when monobench exits.
///
/// Returns `None` on non-Linux. The wrapper `jailed_solver_command` chooses which OS path to
/// use at runtime so callers don't need cfg gates.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn agy_read_jail_bwrap_args(
    root: &Path,
    assigned_worktree: &Path,
    repo_base: Option<&Path>,
) -> Option<Vec<String>> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let canon_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let bases: Vec<PathBuf> = if canon_root == root {
        vec![canon_root]
    } else {
        vec![canon_root, root.to_path_buf()]
    };
    let wt = std::fs::canonicalize(assigned_worktree)
        .unwrap_or_else(|_| assigned_worktree.to_path_buf());
    let mut args: Vec<String> = Vec::new();
    // Base layer: host read-only.
    args.extend([
        "--ro-bind".into(),
        "/".into(),
        "/".into(),
        "--dev".into(),
        "/dev".into(),
        "--proc".into(),
        "/proc".into(),
        "--tmpfs".into(),
        "/tmp".into(),
        "--share-net".into(),
        "--die-with-parent".into(),
        "--new-session".into(),
    ]);
    // Mask the benchmark answer-key dirs even though `/` was bound read-only above.
    for base in &bases {
        for sub in ["instances", "research"] {
            let p = base.join(sub);
            args.extend(["--tmpfs".into(), p.to_string_lossy().into_owned()]);
        }
    }
    // Hide every monobench-work tree (cross-instance + cross-process protection), then
    // re-expose ONLY our assigned worktree as writable.
    for masked in ["/tmp/monobench-work", "/private/tmp/monobench-work"] {
        args.extend(["--tmpfs".into(), masked.into()]);
    }
    args.extend([
        "--bind".into(),
        wt.to_string_lossy().into_owned(),
        wt.to_string_lossy().into_owned(),
    ]);
    // Anti-cheat: mask .git in the worktree (worktree's .git is a file marker; tmpfs replaces
    // it with an empty directory which is indistinguishable from "no git repo" to git itself).
    args.extend([
        "--tmpfs".into(),
        wt.join(".git").to_string_lossy().into_owned(),
    ]);
    if let Some(rb) = repo_base {
        let rb_canon = std::fs::canonicalize(rb).unwrap_or_else(|_| rb.to_path_buf());
        args.extend([
            "--ro-bind".into(),
            rb_canon.to_string_lossy().into_owned(),
            rb_canon.to_string_lossy().into_owned(),
        ]);
        args.extend([
            "--tmpfs".into(),
            rb_canon.join(".git").to_string_lossy().into_owned(),
        ]);
    }
    Some(args)
}

/// Cross-platform dispatcher: returns a `Command` that runs `inner` (a solver binary like
/// "claude" / "codex" / "agy" / "grok" / "opencode") wrapped in the OS-appropriate sandbox.
/// On macOS, wraps with `sandbox-exec -f <SBPL profile>`. On Linux, wraps with `bwrap <args> --`.
/// On other OSes or if sandbox generation fails, returns `Command::new(inner)` unwrapped —
/// the integrity scanner is the second line of defense in that case.
pub(crate) fn jailed_solver_command(
    inner: &str,
    root: &Path,
    tag: &str,
    assigned_worktree: &Path,
    repo_base: Option<&Path>,
) -> Command {
    #[cfg(target_os = "macos")]
    {
        if let Some(p) = agy_read_jail_profile(root, tag, assigned_worktree, repo_base) {
            let mut c = Command::new("sandbox-exec");
            c.arg("-f").arg(p).arg(inner);
            return c;
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(args) = agy_read_jail_bwrap_args(root, assigned_worktree, repo_base) {
            let mut c = Command::new("bwrap");
            c.args(&args);
            c.arg("--");
            c.arg(inner);
            return c;
        }
    }
    // Silences unused-param warnings on non-macOS/non-Linux targets where neither branch fires.
    let _ = (root, tag);
    Command::new(inner)
}

/// Shell-string variant of [`jailed_solver_command`] for callers that build a shell command
/// rather than spawn a `Command` directly (niia_runner). Returns an empty string if no
/// sandbox is available on this OS (caller runs the solver unwrapped). The returned prefix
/// always ends with a trailing space so it can be string-concatenated with the rest of the
/// command without extra padding logic.
pub(crate) fn jailed_solver_shell_prefix(
    root: &Path,
    tag: &str,
    assigned_worktree: &Path,
    repo_base: Option<&Path>,
) -> String {
    #[cfg(target_os = "macos")]
    {
        if let Some(p) = agy_read_jail_profile(root, tag, assigned_worktree, repo_base) {
            return format!(
                "sandbox-exec -f {} ",
                crate::niia_runner::shell_quote(&p.to_string_lossy())
            );
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(args) = agy_read_jail_bwrap_args(root, assigned_worktree, repo_base) {
            let quoted: Vec<String> = args
                .iter()
                .map(|a| crate::niia_runner::shell_quote(a))
                .collect();
            return format!("bwrap {} -- ", quoted.join(" "));
        }
    }
    let _ = (root, tag, assigned_worktree, repo_base);
    String::new()
}

/// The model agy will actually use in `--print` mode. Print mode has NO `--model` flag (verified:
/// `agy --model=…` → "flags provided but not defined"), so the model is whatever the GLOBAL
/// settings file says — monobench cannot set it per-run via CLI, and editing that one shared file
/// would race parallel runs. So we read it and verify instead of pretending `--model` controls it.
pub(crate) fn agy_settings_model() -> Option<String> {
    let home = std::env::var_os("HOME")?;
    let p = PathBuf::from(home).join(".gemini/antigravity-cli/settings.json");
    read_json(&p)
        .get("model")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

/// Compare model identifiers loosely: lowercased, alphanumerics only. So agy's display names
/// "Gemini 3.5 Flash (Medium)" / "Gemini 3.5 Flash (Low)" and the labels
/// "gemini-3.5-flash-medium" / "gemini-3.5-flash-low" match while still covering the model and
/// reasoning suffix together.
pub(crate) fn agy_model_norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
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

fn agy_transcript_path(cid: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let p = PathBuf::from(home)
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
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    None
}

fn parse_codex_session_id(err_path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(err_path).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("session id:") {
            let sid = rest.trim();
            if !sid.is_empty() {
                return Some(sid.to_string());
            }
        }
    }
    None
}

fn session_model(x: &Value) -> &str {
    x.get("models")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(Value::as_str)
        .unwrap_or("codex")
}

fn meter_from_session(x: &Value, dur: u64) -> String {
    let cache_write_5m = x.get("cache_write_5m").and_then(Value::as_i64).unwrap_or(0);
    let cache_write_1h = x.get("cache_write_1h").and_then(Value::as_i64).unwrap_or(0);
    serde_json::json!({
        "session_id": x.get("session_id").and_then(Value::as_str),
        "tokens": x.get("total_tokens").and_then(Value::as_i64),
        "input": x.get("input_tokens").and_then(Value::as_i64),
        "output": x.get("output_tokens").and_then(Value::as_i64),
        "cache_read": x.get("cache_read").and_then(Value::as_i64),
        "cache_creation": cache_write_5m + cache_write_1h,
        "cache_write_5m": cache_write_5m,
        "cache_write_1h": cache_write_1h,
        "cost_usd": x.get("cost_usd").and_then(Value::as_f64),
        "duration_s": dur,
        "model": session_model(x)
    })
    .to_string()
}

/// Parse grok's `--output-format json` envelope `{text, stopReason, sessionId, requestId, thought}`.
/// Returns (answer_text, session_id); falls back to the raw file contents if it isn't valid JSON.
fn parse_grok_envelope(path: &Path) -> (String, Option<String>) {
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(v) => {
            let text = v
                .get("text")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let sid = v
                .get("sessionId")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string());
            // No "text" field → hand grading the raw envelope rather than an empty string.
            if text.is_empty() {
                (raw, sid)
            } else {
                (text, sid)
            }
        }
        Err(_) => (raw, None),
    }
}

/// Locate `~/.grok/sessions/<urlenc-cwd>/<session_id>/signals.json` by unique session id
/// (avoids reconstructing grok's URL-encoded cwd directory name).
fn grok_signals_path(session_id: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let base = PathBuf::from(home).join(".grok/sessions");
    for entry in std::fs::read_dir(&base).ok()?.flatten() {
        let p = entry.path().join(session_id).join("signals.json");
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// Build a grok meter. grok has no per-turn token split and no cost (OAuth subscription, single
/// model grok-build), so `tokens`/`cost_usd` are null — never faked. Honest session metrics
/// (context size, turns, tool calls, duration, TTFT) come from the session's signals.json.
#[allow(clippy::too_many_arguments)]
fn grok_meter(
    dur: u64,
    session_id: Option<&str>,
    model_hint: &str,
    effort: &str,
    exit_status: Option<i32>,
    exit_success: bool,
    runner_error: Option<String>,
) -> String {
    let signals: Option<serde_json::Value> = session_id
        .and_then(grok_signals_path)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok());
    let g = |k: &str| -> serde_json::Value {
        signals
            .as_ref()
            .and_then(|s| s.get(k).cloned())
            .unwrap_or(serde_json::Value::Null)
    };
    let observed_model = signals
        .as_ref()
        .and_then(|s| s.get("primaryModelId"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let model_enforced = observed_model.as_deref() == Some(model_hint);
    serde_json::json!({
        "runner": "grok",
        "model": model_hint,
        "requested_model": model_hint,
        "requested_effort": effort,
        "observed_model": observed_model,
        "model_enforced": model_enforced,
        "effort_enforced": false, // grok-build: supports_reasoning_effort=false
        "tokens": null,
        "cost_usd": null,
        "tokens_available": false,
        "cost_available": false,
        "meter_error": "grok exposes session metrics only; no per-turn token split or cost",
        "signals_available": signals.is_some(),
        "session_id": session_id,
        "context_tokens_used": g("contextTokensUsed"),
        "context_window_tokens": g("contextWindowTokens"),
        "turns": g("turnCount"),
        "tool_calls": g("toolCallCount"),
        "tools_used": g("toolsUsed"),
        "session_duration_s": g("sessionDurationSeconds"),
        "avg_ttft_ms": g("avgTimeToFirstTokenMs"),
        "duration_s": dur,
        "exit_status": exit_status,
        "exit_success": exit_success,
        "runner_error": runner_error
    })
    .to_string()
}

/// Parse opencode's `--format json` stream-of-events envelope to extract the agent's final text.
///
/// opencode emits NDJSON events (`message.part.updated`, `session.idle`, `session.error`, …).
/// Text the user sees comes from `message.part.updated` events whose `part.type == "text"`.
/// We collect all assistant text parts in order and concatenate; if the envelope isn't NDJSON
/// or has no recognizable text parts, return the raw envelope so grading still sees something.
fn parse_opencode_envelope(path: &Path) -> String {
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let mut texts: Vec<String> = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('{') {
            continue;
        }
        let Ok(ev) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            continue;
        };
        // Two shapes to handle: outer SDK event `{type, properties: {part: {...}}}` or already-
        // unwrapped `{type, part: {...}}`. Take the `part` from either.
        let part = ev
            .pointer("/properties/part")
            .or_else(|| ev.pointer("/part"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        if part.get("type").and_then(|x| x.as_str()) == Some("text") {
            if let Some(t) = part.get("text").and_then(|x| x.as_str()) {
                if !t.is_empty() {
                    texts.push(t.to_string());
                }
            }
        }
    }
    if texts.is_empty() {
        raw
    } else {
        // De-dup consecutive identical entries (incremental updates may repeat the running text).
        let mut out = String::new();
        for t in &texts {
            if !out.ends_with(t.as_str()) {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(t);
            }
        }
        out
    }
}

/// Build an opencode meter. Like grok, opencode uses a subscription/API key and doesn't emit
/// reliable per-turn token/cost in its `--format json` stream — we record activity + duration,
/// never fake tokens or cost. (Per-session usage is available via `opencode stats` separately.)
fn opencode_meter(
    dur: u64,
    model_hint: &str,
    effort: &str,
    exit_status: Option<i32>,
    exit_success: bool,
    runner_error: Option<String>,
) -> String {
    serde_json::json!({
        "runner": "opencode",
        "model": model_hint,
        "requested_model": model_hint,
        "requested_effort": effort,
        "tokens": null,
        "cost_usd": null,
        "tokens_available": false,
        "cost_available": false,
        "meter_error": "opencode --format json doesn't expose per-turn token split or cost; use `opencode stats` for session totals",
        "duration_s": dur,
        "exit_status": exit_status,
        "exit_success": exit_success,
        "runner_error": runner_error
    })
    .to_string()
}

fn empty_codex_meter(dur: u64, session_id: Option<&str>, model_hint: &str, reason: &str) -> String {
    serde_json::json!({
        "session_id": session_id,
        "tokens": null,
        "input": null,
        "output": null,
        "cache_read": null,
        "cache_creation": null,
        "cost_usd": null,
        "duration_s": dur,
        "model": model_hint,
        "meter_error": reason
    })
    .to_string()
}

/// Parse `monometer sessions --provider codex` → the meter.json shape.
/// Prefer the exact session id printed by `codex exec`; never attach a different
/// recent session to a run, because that corrupts per-arm benchmark cost.
fn codex_meter(dur: u64, err_path: &Path, model_hint: &str) -> String {
    let wanted_sid = parse_codex_session_id(err_path);
    for _ in 0..5 {
        let out = Command::new("monometer")
            .args([
                "sessions",
                "--provider",
                "codex",
                "--recent",
                "120",
                "--json",
            ])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        let v: Value = serde_json::from_str(&out).unwrap_or(Value::Null);
        let Some(arr) = v.as_array() else {
            return empty_codex_meter(
                dur,
                wanted_sid.as_deref(),
                model_hint,
                "monometer sessions returned non-array json",
            );
        };
        if let Some(sid) = wanted_sid.as_deref() {
            if let Some(x) = arr
                .iter()
                .find(|x| x.get("session_id").and_then(Value::as_str) == Some(sid))
            {
                return meter_from_session(x, dur);
            }
        } else {
            return empty_codex_meter(
                dur,
                None,
                model_hint,
                "codex session id not found in stderr",
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    empty_codex_meter(
        dur,
        wanted_sid.as_deref(),
        model_hint,
        "codex session not found in monometer",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_path_includes_instance_id() {
        let runid = "monogram-0.61.31-claude-haiku-r1-t123";
        let a = worktree_run_dir(
            "/tmp/monobench-work",
            "redis-14929-restorecmd-meta-uaf",
            runid,
        );
        let b = worktree_run_dir("/tmp/monobench-work", "ktor-5626-readchannel-close", runid);

        assert_ne!(a, b);
        assert!(a
            .to_string_lossy()
            .contains("redis-14929-restorecmd-meta-uaf"));
        assert!(b.to_string_lossy().contains("ktor-5626-readchannel-close"));
    }

    #[test]
    fn substitutes_repo_and_codegraph() {
        let repo = Path::new("/tmp/repo");
        assert_eq!(
            sub_vars("${CODEGRAPH} --path ${REPO}", repo, "node cg.js"),
            "node cg.js --path /tmp/repo"
        );
    }

    #[test]
    fn expands_command_with_prefix_args() {
        let repo = Path::new("/tmp/repo");
        let args = vec![
            "serve".to_string(),
            "--path".to_string(),
            "${REPO}".to_string(),
        ];
        let (cmd, argv) =
            command_and_args("${CODEGRAPH}", &args, repo, "node /opt/codegraph.js").unwrap();
        assert_eq!(cmd, "node");
        assert_eq!(
            argv,
            vec!["/opt/codegraph.js", "serve", "--path", "/tmp/repo"]
        );
    }

    #[test]
    fn rejects_legacy_shell_operators() {
        let marker = RunningMarker {
            path: std::env::temp_dir()
                .join(format!("monobench-test-{}.running", std::process::id())),
        };
        let log =
            std::env::temp_dir().join(format!("monobench-test-{}.index.log", std::process::id()));
        let err = run_legacy_index(
            "a; b",
            Path::new("."),
            Path::new("."),
            "codegraph",
            &log,
            &marker,
        )
        .unwrap_err();
        assert!(err.contains("index_steps"));
        let _ = std::fs::remove_file(log);
    }

    #[test]
    #[cfg(unix)]
    fn solver_wrapper_blocks_prepared_monogram_reindex() {
        let dir = install_solver_deny_wrapper("prepared-guard-test", true).unwrap();
        let out = Command::new(dir.join("monogram"))
            .args(["index", ".", "-r"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(126));
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("prepared monogram index is already installed"));
        assert!(stderr.contains("[NEXT]"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    #[cfg(unix)]
    fn solver_wrapper_blocks_prepared_monogram_mutation_commands() {
        let dir = install_solver_deny_wrapper("prepared-mutation-guard-test", true).unwrap();
        for args in [["prune", "--force"], ["boot", "init"]] {
            let out = Command::new(dir.join("monogram"))
                .args(args)
                .output()
                .unwrap();
            assert_eq!(out.status.code(), Some(126));
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(stderr.contains("prepared monogram index is already installed"));
            assert!(stderr.contains("[NEXT]"));
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rewrites_prepared_sqlite_paths() {
        if Command::new("sqlite3").arg("-version").output().is_err() {
            return;
        }
        let tmp = std::env::temp_dir().join(format!(
            "monobench-rewrite-test-{}-{}",
            std::process::id(),
            unix_ms()
        ));
        let from = tmp.join("prepared");
        let to = tmp.join("run");
        std::fs::create_dir_all(from.join("src")).unwrap();
        std::fs::create_dir_all(to.join("src")).unwrap();
        let db = tmp.join("test.db");
        let from_file = from
            .canonicalize()
            .unwrap()
            .join("src/lib.rs")
            .to_string_lossy()
            .to_string();
        let sql = format!(
            "CREATE TABLE files(path TEXT);
             CREATE TABLE relations(resolved_path TEXT);
             CREATE TABLE import_bindings(resolved_path TEXT);
             INSERT INTO files VALUES ('{}');
             INSERT INTO files VALUES ('./relative.rs');
             INSERT INTO relations VALUES ('{}');
             INSERT INTO import_bindings VALUES ('{}');",
            from_file, from_file, from_file
        );
        let status = Command::new("sqlite3").arg(&db).arg(sql).status().unwrap();
        assert!(status.success());
        let log = tmp.join("rewrite.log");
        rewrite_prepared_paths(&db, &from, &to, &log).unwrap();
        let out = Command::new("sqlite3")
            .arg(&db)
            .arg("SELECT path FROM files ORDER BY path;")
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.contains(
            &to.canonicalize()
                .unwrap()
                .join("src/lib.rs")
                .to_string_lossy()
                .to_string()
        ));
        assert!(text.contains("./relative.rs"));
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    #[cfg(unix)]
    fn install_prepared_portable_uses_symlink() {
        // Build a fake _prepared/monogram/ template marked portable=true with a
        // minimal sqlite-shaped file (the install path doesn't read it). Then
        // call install and assert the destination is a symlink, not a copy.
        let tmp = std::env::temp_dir().join(format!(
            "monobench-portable-link-{}-{}",
            std::process::id(),
            unix_ms()
        ));
        let out = tmp.join("results").join("inst-x");
        let repo = tmp.join("worktree");
        std::fs::create_dir_all(&repo).unwrap();
        let snap_dir = prepared_snapshot_dir(&out, "monogram");
        std::fs::create_dir_all(&snap_dir).unwrap();
        let snap_db = snap_dir.join("monogram.db");
        std::fs::write(&snap_db, b"SQLite-snapshot-stub").unwrap();
        std::fs::write(
            snap_dir.join("manifest.tsv"),
            "source_root\t/tmp/source\nsource_db\t/tmp/source.db\nsnapshot_db\t/tmp/snap.db\ntool_version\t0.62.0\ncreated_ms\t1\nportable\ttrue\n",
        )
        .unwrap();
        let log_path = tmp.join("install.log");
        let marker_path = tmp.join("running.marker");
        std::fs::write(&marker_path, "").unwrap();
        let marker = RunningMarker { path: marker_path };
        let dst_db = monogram_project_db_path(&repo);
        // Cleanup any prior link/file at dst_db so this test is self-contained.
        for sidecar in sqlite_sidecars(&dst_db) {
            std::fs::remove_file(sidecar).ok();
        }
        install_prepared_monogram_snapshot(&out, "monogram", &repo, &log_path, &marker).unwrap();
        let meta = std::fs::symlink_metadata(&dst_db).unwrap();
        assert!(
            meta.file_type().is_symlink(),
            "dst_db must be a symlink, got {:?}",
            meta.file_type()
        );
        let target = std::fs::read_link(&dst_db).unwrap();
        assert_eq!(target, snap_db, "symlink target mismatch");
        // Confirm the log narrates the portable path, not the legacy one.
        let log = std::fs::read_to_string(&log_path).unwrap_or_default();
        assert!(
            log.contains("portable"),
            "log missing portable marker: {log}"
        );
        assert!(
            !log.contains("rewriting monogram DB paths"),
            "portable install must NOT rewrite paths: {log}"
        );
        // Cleanup: remove the dst symlink + sidecars + tmp.
        for sidecar in sqlite_sidecars(&dst_db) {
            std::fs::remove_file(sidecar).ok();
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn prepared_snapshot_requires_matching_tool_version() {
        let tmp = std::env::temp_dir().join(format!(
            "monobench-prepared-version-test-{}-{}",
            std::process::id(),
            unix_ms()
        ));
        let snap = tmp.join("_prepared").join("monogram");
        std::fs::create_dir_all(&snap).unwrap();
        std::fs::write(
            snap.join("manifest.tsv"),
            "source_root\t/tmp/repo\nsource_db\t/tmp/a.db\nsnapshot_db\t/tmp/b.db\ntool_version\t0.61.22\ncreated_ms\t1\n",
        )
        .unwrap();
        assert!(prepared_snapshot_version_matches(&snap, "0.61.22"));
        assert!(!prepared_snapshot_version_matches(&snap, "0.61.23"));
        assert!(prepared_snapshot_version_matches(&snap, ""));
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn parses_codex_session_id_from_stderr() {
        let p = std::env::temp_dir().join(format!("monobench-codex-err-{}", std::process::id()));
        std::fs::write(&p, "model: gpt-5.4-mini\nsession id: 019e5455-abc\n").unwrap();
        assert_eq!(parse_codex_session_id(&p).as_deref(), Some("019e5455-abc"));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn codex_meter_preserves_cache_breakdown() {
        let session = serde_json::json!({
            "session_id": "sid-1",
            "total_tokens": 1050,
            "input_tokens": 200,
            "output_tokens": 50,
            "cache_read": 800,
            "cache_write_5m": 0,
            "cache_write_1h": 0,
            "cost_usd": 0.123,
            "models": ["gpt-5.4-mini"]
        });
        let meter: Value = serde_json::from_str(&meter_from_session(&session, 7)).unwrap();
        assert_eq!(
            meter.get("session_id").and_then(Value::as_str),
            Some("sid-1")
        );
        assert_eq!(meter.get("input").and_then(Value::as_i64), Some(200));
        assert_eq!(meter.get("cache_read").and_then(Value::as_i64), Some(800));
        assert_eq!(meter.get("tokens").and_then(Value::as_i64), Some(1050));
    }

    #[test]
    fn parses_agy_conversation_id_from_log() {
        let p = std::env::temp_dir().join(format!("monobench-agy-log-{}", std::process::id()));
        std::fs::write(&p, "I printmode.go:130] Print mode: conversation=51ccc7c7-3534-4386-aee1-e47b64cd2666, sending message\n").unwrap();
        assert_eq!(
            parse_agy_conversation_id(&p).as_deref(),
            Some("51ccc7c7-3534-4386-aee1-e47b64cd2666")
        );
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn agy_model_norm_matches_flash_low_and_medium_display_labels() {
        assert_eq!(
            agy_model_norm("Gemini 3.5 Flash (Low)"),
            agy_model_norm("gemini-3.5-flash-low")
        );
        assert_eq!(
            agy_model_norm("Gemini 3.5 Flash (Medium)"),
            agy_model_norm("gemini-3.5-flash-medium")
        );
        assert_ne!(
            agy_model_norm("Gemini 3.5 Flash (Low)"),
            agy_model_norm("gemini-3.5-flash-medium")
        );
    }

    #[test]
    fn parses_grok_envelope_text_and_session_id() {
        let p = std::env::temp_dir().join(format!("monobench-grok-env-{}", std::process::id()));
        std::fs::write(
            &p,
            r#"{"text":"ok","stopReason":"EndTurn","sessionId":"019e62dc-aaaa","requestId":"r1","thought":"t"}"#,
        )
        .unwrap();
        let (answer, sid) = super::parse_grok_envelope(&p);
        assert_eq!(answer, "ok");
        assert_eq!(sid.as_deref(), Some("019e62dc-aaaa"));
        // non-JSON stdout falls back to raw text with no session id (grading still sees something)
        std::fs::write(&p, "raw non-json output").unwrap();
        let (answer2, sid2) = super::parse_grok_envelope(&p);
        assert_eq!(answer2, "raw non-json output");
        assert!(sid2.is_none());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn grok_meter_never_fakes_tokens_or_cost() {
        // No session id → signals unavailable; the meter must still never fake tokens/cost.
        let meter: Value = serde_json::from_str(&super::grok_meter(
            12,
            None,
            "grok-build",
            "low",
            Some(0),
            true,
            None,
        ))
        .unwrap();
        assert_eq!(meter.get("runner").and_then(Value::as_str), Some("grok"));
        assert!(meter.get("tokens").unwrap().is_null());
        assert!(meter.get("cost_usd").unwrap().is_null());
        assert_eq!(
            meter.get("tokens_available").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            meter.get("cost_available").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            meter.get("signals_available").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(meter.get("duration_s").and_then(Value::as_u64), Some(12));
        assert_eq!(
            meter.get("requested_model").and_then(Value::as_str),
            Some("grok-build")
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Structural cross-instance sandbox guard tests (added 2026-05-28)
    //
    // Pin the new agy_read_jail_profile behavior so a future edit can't silently
    // regress the deny-on-monobench-work + per-instance allow pattern.
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn agy_jail_profile_denies_monobench_work_subpaths() {
        if !cfg!(target_os = "macos") {
            return; // function returns None on other platforms
        }
        let root = std::env::temp_dir().join(format!("mb-jail-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).ok();
        let wt = std::env::temp_dir()
            .join("monobench-work/wt/some-instance/baseline-r1-t1");
        let base = std::env::temp_dir().join("monobench-work/some-repo-base");
        let p = super::agy_read_jail_profile(&root, "test-deny-mw", &wt, Some(&base))
            .expect("profile should be created on macOS");
        let body = std::fs::read_to_string(&p).expect("profile readable");
        assert!(
            body.contains("(deny file-read* (subpath \"/private/tmp/monobench-work\"))"),
            "expected /private/tmp/monobench-work deny, got:\n{body}"
        );
        assert!(
            body.contains("(deny file-read* (subpath \"/tmp/monobench-work\"))"),
            "expected /tmp/monobench-work deny, got:\n{body}"
        );
        assert!(
            body.contains("(allow file-read* (subpath"),
            "expected at least one allow file-read* line, got:\n{body}"
        );
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn agy_jail_profile_skips_repo_base_allow_when_none() {
        if !cfg!(target_os = "macos") {
            return;
        }
        let root = std::env::temp_dir().join(format!("mb-jail-test2-{}", std::process::id()));
        std::fs::create_dir_all(&root).ok();
        let wt = std::env::temp_dir().join("monobench-work/wt/x/r-t1");
        let p = super::agy_read_jail_profile(&root, "test-no-base", &wt, None)
            .expect("profile should be created");
        let body = std::fs::read_to_string(&p).expect("profile readable");
        // Exactly ONE allow file-read* (subpath ...) line (the assigned worktree).
        // No allow file-read* on a base. Ancestor traversal allows use `file-read-metadata`
        // not `file-read*`, so they don't match this pattern.
        let allow_subpath_count = body.matches("(allow file-read* (subpath").count();
        assert_eq!(
            allow_subpath_count, 1,
            "expected 1 allow file-read* (subpath ...) line, got {allow_subpath_count}:\n{body}"
        );
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn agy_jail_profile_emits_ancestor_metadata_allows_for_grok_path_traversal() {
        if !cfg!(target_os = "macos") {
            return;
        }
        let root = std::env::temp_dir().join(format!("mb-jail-test-anc-{}", std::process::id()));
        std::fs::create_dir_all(&root).ok();
        // Simulate the typical layout: /tmp/monobench-work/wt/<instance>/<runid>-<pid>/
        let wt = PathBuf::from("/tmp/monobench-work/wt/my-instance/baseline-r1-t1-PID");
        let p = super::agy_read_jail_profile(&root, "test-ancestors", &wt, None)
            .expect("profile should be created");
        let body = std::fs::read_to_string(&p).expect("profile readable");
        // Each ancestor under monobench-work should appear as a metadata-only allow.
        // Stop at the first ancestor that's NOT under monobench-work.
        for expected_ancestor in [
            "/tmp/monobench-work/wt/my-instance",
            "/tmp/monobench-work/wt",
            "/tmp/monobench-work",
        ] {
            let line = format!(
                "(allow file-read-metadata (literal \"{}\"))",
                expected_ancestor
            );
            assert!(
                body.contains(&line),
                "expected ancestor metadata allow line {line:?}, got:\n{body}"
            );
        }
        // Sanity: should NOT walk above monobench-work (so no allow for /tmp or / itself).
        assert!(
            !body.contains("(allow file-read-metadata (literal \"/tmp\"))"),
            "must not allow metadata on /tmp itself, got:\n{body}"
        );
        assert!(
            !body.contains("(allow file-read-metadata (literal \"/\"))"),
            "must not allow metadata on / itself, got:\n{body}"
        );
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn agy_jail_profile_keeps_existing_git_and_answer_key_denies() {
        if !cfg!(target_os = "macos") {
            return;
        }
        let root = std::env::temp_dir().join(format!("mb-jail-test3-{}", std::process::id()));
        std::fs::create_dir_all(&root).ok();
        let wt = std::env::temp_dir().join("monobench-work/wt/y/r-t1");
        let p = super::agy_read_jail_profile(&root, "test-existing-denies", &wt, None)
            .expect("profile should be created");
        let body = std::fs::read_to_string(&p).expect("profile readable");
        // file-read deny on .git is THE anti-cheat — must remain.
        assert!(
            body.contains("(deny file-read* (regex #\"(^|/)\\.git(/|$)\"))"),
            ".git deny missing:\n{body}"
        );
        // process-exec deny on git was removed for opencode compatibility (Bun's posix_spawn
        // EPERM crashes opencode synchronously before Effect.catch can degrade vcs to undefined).
        // Anti-cheat is preserved by the file-read .git deny above — git can spawn but cannot
        // read .git, so `git log` exits with "not a git repository".
        assert!(
            !body.contains("(deny process-exec* (regex #\"(^|/)git$\"))"),
            "process-exec git deny must NOT be present (breaks opencode):\n{body}"
        );
        assert!(body.contains("/instances"), "instances deny missing:\n{body}");
        assert!(body.contains("/research"), "research deny missing:\n{body}");
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn bwrap_args_linux_enforce_anti_cheat_and_sibling_isolation() {
        // The bwrap path only fires on Linux (the function returns None elsewhere). On macOS
        // we can still pin the *expected* arg shape so a future edit can't silently regress
        // the Linux side — directly call the inner builder ignoring the cfg gate by working
        // around it with a helper closure that mirrors the function body. Here we instead
        // assert the cross-platform dispatcher returns the macOS form on macOS, and the
        // standalone bwrap fn only returns Some on Linux.
        if cfg!(target_os = "linux") {
            let root = std::env::temp_dir().join(format!("mb-bwrap-test-{}", std::process::id()));
            std::fs::create_dir_all(&root).ok();
            let wt = std::env::temp_dir().join("monobench-work/wt/inst42/r-t1");
            let base = std::env::temp_dir().join("monobench-work/inst42-base");
            let args = super::agy_read_jail_bwrap_args(&root, &wt, Some(&base))
                .expect("Linux must return Some");
            let joined = args.join(" ");
            // Network must stay open for LLM API calls.
            assert!(joined.contains("--share-net"), "share-net missing: {joined}");
            // Anti-cheat: .git inside the worktree masked with tmpfs.
            let wt_git = wt.join(".git").to_string_lossy().into_owned();
            assert!(
                joined.contains(&format!("--tmpfs {wt_git}")),
                ".git tmpfs mask missing: {joined}"
            );
            // Structural sibling isolation: monobench-work tree masked before our worktree
            // bind. Both /tmp and /private/tmp variants present so macOS hosts that bind-mount
            // the same path are also covered.
            assert!(joined.contains("--tmpfs /tmp/monobench-work"), "{joined}");
            // Answer-key dirs masked.
            assert!(joined.contains("/instances"), "{joined}");
            assert!(joined.contains("/research"), "{joined}");
            // Worktree is re-bound writable AFTER the monobench-work tmpfs mask so our path
            // wins. The dispatcher concatenates inner cmd after `--`.
            assert!(
                joined.contains(&format!(
                    "--bind {} {}",
                    wt.to_string_lossy(),
                    wt.to_string_lossy()
                )),
                "worktree writable bind missing: {joined}"
            );
        } else {
            // On non-Linux, the function must return None so dispatcher falls back gracefully.
            let wt = std::env::temp_dir().join("monobench-work/wt/inst42/r-t1");
            assert!(super::agy_read_jail_bwrap_args(&std::env::temp_dir(), &wt, None).is_none());
        }
    }

    #[test]
    fn jailed_solver_command_dispatches_per_os() {
        let root = std::env::temp_dir();
        let wt = std::env::temp_dir().join("monobench-work/wt/inst99/r-t1");
        std::fs::create_dir_all(&wt).ok();
        let cmd = super::jailed_solver_command("claude", &root, "test-dispatch", &wt, None);
        let program = cmd.get_program().to_string_lossy().into_owned();
        if cfg!(target_os = "macos") {
            assert_eq!(program, "sandbox-exec", "macOS must wrap with sandbox-exec");
            let args: Vec<String> = cmd
                .get_args()
                .map(|s| s.to_string_lossy().into_owned())
                .collect();
            assert_eq!(args.first().map(String::as_str), Some("-f"));
            assert_eq!(args.last().map(String::as_str), Some("claude"));
        } else if cfg!(target_os = "linux") {
            assert_eq!(program, "bwrap", "Linux must wrap with bwrap");
            let args: Vec<String> = cmd
                .get_args()
                .map(|s| s.to_string_lossy().into_owned())
                .collect();
            assert_eq!(args.last().map(String::as_str), Some("claude"));
            // The bwrap arg list ends with `--` then the inner command.
            assert!(args.contains(&"--".to_string()), "bwrap separator missing");
        } else {
            // Unsupported OS: dispatcher returns Command::new("claude") unwrapped.
            assert_eq!(program, "claude");
        }
        let _ = std::fs::remove_dir_all(&wt);
    }
}
