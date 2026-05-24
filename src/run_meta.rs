// monobench — advisory run metadata sidecars.
// File names stay stable; human intent lives in `<run>.meta.json`.
use crate::util::fit_middle;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct StartMeta<'a> {
    pub id: &'a str,
    pub run_id: &'a str,
    pub label: &'a str,
    pub tool: &'a str,
    pub cli: &'a str,
    pub model: &'a str,
    pub effort: &'a str,
    pub via: &'a str,
    pub repeat_index: usize,
    pub repeat_total: Option<usize>,
    pub tag: Option<&'a str>,
    pub note: Option<&'a str>,
    pub started_at_ms: u128,
    pub prepared: bool,
    pub isolate: &'a str,
}

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub fn meta_path(root: &Path, id: &str, run: &str) -> PathBuf {
    root.join("results")
        .join(id)
        .join(format!("{run}.meta.json"))
}

pub fn write_start(out: &Path, meta: StartMeta<'_>) {
    let p = out.join(format!("{}.meta.json", meta.run_id));
    let mut v = json!({
        "schema": 1,
        "id": meta.id,
        "run_id": meta.run_id,
        "label": meta.label,
        "tool": meta.tool,
        "cli": meta.cli,
        "model": meta.model,
        "effort": meta.effort,
        "via": meta.via,
        "repeat_index": meta.repeat_index,
        "started_at_ms": meta.started_at_ms,
        "prepared": meta.prepared,
        "isolate": meta.isolate,
        "created_at_ms": now_ms()
    });
    if let Some(total) = meta.repeat_total {
        v["repeat_total"] = json!(total);
    }
    if let Some(tag) = clean_optional(meta.tag) {
        v["tag"] = json!(tag);
    }
    if let Some(note) = clean_optional(meta.note) {
        v["note"] = json!(note);
    }
    if let Ok(s) = serde_json::to_string_pretty(&v) {
        std::fs::write(p, format!("{s}\n")).ok();
    }
}

pub fn load(root: &Path, id: &str, run: &str) -> Option<Value> {
    let text = std::fs::read_to_string(meta_path(root, id, run)).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn update_note(
    root: &Path,
    id: &str,
    run: &str,
    tag: Option<&str>,
    note: Option<&str>,
) -> Result<PathBuf, String> {
    let p = meta_path(root, id, run);
    let mut obj = std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_else(Map::new);

    obj.entry("schema").or_insert(json!(1));
    obj.insert("id".into(), json!(id));
    obj.insert("run_id".into(), json!(run));
    obj.insert("updated_at_ms".into(), json!(now_ms()));
    if let Some(tag) = clean_optional(tag) {
        obj.insert("tag".into(), json!(tag));
    }
    if let Some(note) = clean_optional(note) {
        obj.insert("note".into(), json!(note));
    }

    let text = serde_json::to_string_pretty(&Value::Object(obj))
        .map_err(|e| format!("serialize meta failed: {e}"))?;
    std::fs::write(&p, format!("{text}\n")).map_err(|e| format!("write {}: {e}", p.display()))?;
    Ok(p)
}

pub fn summary(root: &Path, id: &str, run: &str) -> Option<String> {
    let v = load(root, id, run)?;
    let tag = text_field(&v, "tag");
    let note = text_field(&v, "note");
    match (tag, note) {
        (Some(t), Some(n)) => Some(fit_middle(&format!("[{t}] {n}"), 80)),
        (Some(t), None) => Some(format!("[{t}]")),
        (None, Some(n)) => Some(fit_middle(&n, 80)),
        (None, None) => None,
    }
}

pub fn print(root: &Path, id: &str, run: &str) {
    let Some(v) = load(root, id, run) else {
        println!("  meta: -");
        return;
    };
    if let Some(tag) = text_field(&v, "tag") {
        println!("  tag: {tag}");
    }
    if let Some(note) = text_field(&v, "note") {
        println!("  note: {note}");
    }
    if let Some(idx) = v.get("repeat_index").and_then(Value::as_u64) {
        let total = v
            .get("repeat_total")
            .and_then(Value::as_u64)
            .map(|n| format!("/{n}"))
            .unwrap_or_default();
        println!("  repeat: {idx}{total}");
    }
    if let Some(started) = v.get("started_at_ms").and_then(Value::as_u64) {
        println!("  started_at_ms: {started}");
    }
}

fn text_field(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn clean_optional(s: Option<&str>) -> Option<String> {
    s.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}
