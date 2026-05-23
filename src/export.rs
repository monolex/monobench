// monobench — full verbose transcript of ONE run → Markdown (auditable bench evidence).
// Reuses the stream-json schema parsed in trace.rs: assistant{text|tool_use} · user{tool_result} · result.
use crate::grade::is_monogram_cmd;
use crate::util::load_jsonl;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

fn tool_result_text(b: &Value) -> String {
    match b.get("content") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|x| x.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// Render results/<id>/<run>.jsonl into a readable results/<id>/<run>.md (full verbose).
pub fn export(root: &Path, id: &str, run: &str, jsonl: &str) {
    let evs = load_jsonl(jsonl);
    if evs.is_empty() {
        eprintln!("(no such run / empty: {jsonl})");
        return;
    }

    // tool_use_id -> its output text (results arrive in later `user` events)
    let mut results: HashMap<String, String> = HashMap::new();
    for e in &evs {
        if e.get("type").and_then(Value::as_str) == Some("user") {
            if let Some(ct) = e.pointer("/message/content").and_then(Value::as_array) {
                for b in ct {
                    if b.get("type").and_then(Value::as_str) == Some("tool_result") {
                        if let Some(tid) = b.get("tool_use_id").and_then(Value::as_str) {
                            results.insert(tid.to_string(), tool_result_text(b));
                        }
                    }
                }
            }
        }
    }

    let mut md = String::new();
    md.push_str(&format!(
        "# monobench transcript — {run}\n\n_instance `{id}` · full verbose render of `{run}.jsonl`_\n\n---\n\n"
    ));

    let (mut step, mut mono) = (0usize, 0usize);
    for e in &evs {
        match e.get("type").and_then(Value::as_str) {
            Some("assistant") => {
                let Some(ct) = e.pointer("/message/content").and_then(Value::as_array) else {
                    continue;
                };
                for b in ct {
                    match b.get("type").and_then(Value::as_str) {
                        Some("text") => {
                            let t = b.get("text").and_then(Value::as_str).unwrap_or("").trim();
                            if !t.is_empty() {
                                md.push_str(&format!("> 💭 {}\n\n", t.replace('\n', "\n> ")));
                            }
                        }
                        Some("tool_use") => {
                            step += 1;
                            let name = b.get("name").and_then(Value::as_str).unwrap_or("?");
                            let cmd =
                                b.pointer("/input/command").and_then(Value::as_str).unwrap_or("");
                            let tid = b.get("id").and_then(Value::as_str).unwrap_or("");
                            let is_mono = (name == "Bash" && is_monogram_cmd(cmd))
                                || name.to_lowercase().contains("monogram");
                            if is_mono {
                                mono += 1;
                            }
                            let tag = if is_mono { " · 🔎 monogram" } else { "" };
                            let call = if name == "Bash" && !cmd.is_empty() {
                                cmd.to_string()
                            } else {
                                serde_json::to_string_pretty(b.get("input").unwrap_or(&Value::Null))
                                    .unwrap_or_default()
                            };
                            md.push_str(&format!("### {step}. {name}{tag}\n\n```bash\n{}\n```\n\n", call.trim()));
                            let out = results.get(tid).cloned().unwrap_or_default();
                            if !out.trim().is_empty() {
                                md.push_str(&format!(
                                    "<details><summary>output ({} lines)</summary>\n\n````\n{}\n````\n\n</details>\n\n",
                                    out.lines().count(),
                                    out.trim_end()
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
            Some("result") => {
                let t = e.get("result").and_then(Value::as_str).unwrap_or("");
                if !t.trim().is_empty() {
                    md.push_str(&format!("---\n\n## Final answer\n\n{}\n\n", t.trim()));
                }
            }
            _ => {}
        }
    }
    md.push_str(&format!("---\n\n_total steps: {step} · monogram invocations: {mono}_\n"));

    let outp = root.join(format!("results/{id}/{run}.md"));
    match std::fs::write(&outp, &md) {
        Ok(_) => println!(
            "wrote {} ({} bytes · {step} steps · {mono} monogram calls)",
            outp.display(),
            md.len()
        ),
        Err(e) => eprintln!("write failed: {e}"),
    }
}
