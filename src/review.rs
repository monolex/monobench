// monobench — optional per-run final review sidecar.
// Automatic grades remain deterministic; review JSON records the human/LLM final judgement.
use crate::grade::RunStats;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const REVIEW_UNREVIEWED: &str = "unreviewed";
pub const REVIEW_JUDGE_DONE: &str = "judge_done";
pub const REVIEW_HUMAN_DONE: &str = "human_done";

const FINAL_GRADES: [&str; 7] = [
    "FULL",
    "NAME_ONLY",
    "DECOY",
    "MISS",
    "NO_RESULT",
    "INVALID",
    "FORFEIT",
];

#[derive(Clone, Debug)]
pub struct ReviewRecord {
    pub auto_grade: String,
    pub final_grade: Option<String>,
    pub review_status: String,
    pub final_checked: bool,
    pub judge_model: Option<String>,
    pub judge_at: Option<String>,
    pub reason: Option<String>,
}

pub fn is_final_grade(s: &str) -> bool {
    FINAL_GRADES.contains(&s)
}

pub fn review_path(root: &Path, id: &str, run: &str) -> PathBuf {
    root.join("results")
        .join(id)
        .join(format!("{run}.review.json"))
}

pub fn review_now() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{ms}")
}

fn opt_string(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
}

pub fn load_review(root: &Path, id: &str, run: &str) -> Option<ReviewRecord> {
    let p = review_path(root, id, run);
    let v: Value = std::fs::read_to_string(p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())?;
    let auto_grade = opt_string(&v, "auto_grade").unwrap_or_default();
    let final_grade = opt_string(&v, "final_grade");
    let review_status = opt_string(&v, "review_status").unwrap_or_else(|| REVIEW_UNREVIEWED.into());
    let final_checked = v
        .get("final_checked")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Some(ReviewRecord {
        auto_grade,
        final_grade,
        review_status,
        final_checked,
        judge_model: opt_string(&v, "judge_model"),
        judge_at: opt_string(&v, "judge_at"),
        reason: opt_string(&v, "reason"),
    })
}

pub fn write_review(root: &Path, id: &str, run: &str, r: &ReviewRecord) -> Result<(), String> {
    let p = review_path(root, id, run);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let v = serde_json::json!({
        "schema_version": 1,
        "auto_grade": r.auto_grade,
        "final_grade": r.final_grade,
        "review_status": r.review_status,
        "final_checked": r.final_checked,
        "judge_model": r.judge_model,
        "judge_at": r.judge_at,
        "reason": r.reason,
    });
    let text = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
    std::fs::write(&p, format!("{text}\n")).map_err(|e| format!("{}: {e}", p.display()))
}

pub fn apply_review(root: &Path, id: &str, mut stats: RunStats) -> RunStats {
    stats.auto_grade = stats.grade.clone();
    if let Some(r) = load_review(root, id, &stats.label) {
        if !r.auto_grade.is_empty() {
            stats.auto_grade = r.auto_grade;
        }
        stats.final_grade = r.final_grade.clone();
        stats.review_status = r.review_status;
        stats.final_checked = r.final_checked;
        if let Some(final_grade) = r.final_grade {
            stats.grade = final_grade;
        }
    }
    stats
}

pub fn unreviewed_next(id: &str, run: &str) -> String {
    format!("monobench judge {id} {run} --model <judge-model>")
}
