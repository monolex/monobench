// monobench — diagnose monogram CLI usage inside solver telemetry.
// This looks past "was monogram called?" and classifies the command/result failure mode.
use crate::grade::RunStats;
use crate::telemetry;
use crate::util::{cmd_has_unquoted_pipe, cmd_word_pos};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
struct Row {
    label: String,
    grade: String,
    calls: usize,
    issues: usize,
    oversized: usize,
    help: usize,
    next_lines: usize,
    json_next_hints: usize,
    subs: BTreeMap<String, usize>,
    kinds: BTreeMap<String, usize>,
    patterns: BTreeMap<String, usize>,
    examples: Vec<(String, String, String)>,
}

struct Oversized {
    label: String,
    grade: String,
    sub: String,
    kind: String,
    bytes: usize,
    lines: usize,
    next: usize,
    json_next: bool,
    cmd: String,
    signal: String,
}

struct MakerRecommendation {
    signal: &'static str,
    count: usize,
    why: &'static str,
    avoid: &'static str,
    prefer: &'static str,
    validate: &'static str,
}

fn monogram_sub(cmd: &str) -> Option<String> {
    let idx = cmd_word_pos(cmd, "monogram")?;
    let tok = cmd[idx + 8..].split_whitespace().next().unwrap_or("");
    Some(if tok.is_empty() {
        "help".into()
    } else if matches!(tok, "help" | "--help" | "-h") {
        "help".into()
    } else if tok == "|" || tok == "&&" || tok == ";" || tok.starts_with('>') || tok.contains(">&")
    {
        "help".into()
    } else {
        tok.into()
    })
}

fn is_help_output(result: &str) -> bool {
    let low = result.to_lowercase();
    low.contains("monogram") && (low.contains("usage") || low.contains("commands"))
}

fn first_status_line(result: &str) -> String {
    result
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_lowercase()
}

fn status_exited_nonzero(status: &str) -> bool {
    status.starts_with("exited ") || status.contains(" exited ")
}

/// Downgrade a no-match issue to `guarded_no_match` when monogram still emitted
/// recovery steering (`[NEXT]` / `[marker: ...]`), so the audit separates dead
/// no-matches from guarded recoveries.
fn guarded_or(kind: &'static str, result: &str) -> &'static str {
    if result.contains("[NEXT]") || result.contains("[marker:") {
        "guarded_no_match"
    } else {
        kind
    }
}

fn issue_kind(sub: &str, result: &str) -> Option<&'static str> {
    let low = result.to_lowercase();
    let status = first_status_line(result);
    if sub == "help" && status_exited_nonzero(&status) && !status.contains("exited 0 ") {
        if is_help_output(result) {
            None
        } else {
            Some("help_exit_nonzero")
        }
    } else if sub == "help" && low.contains("error") && !is_help_output(result) {
        Some("help_exit_nonzero")
    } else if low.contains("zsh:cd:") && low.contains("no such file or directory") {
        Some("bad_workdir_path")
    } else if low.contains("sqlite failure") && low.contains("database is locked") {
        Some("sqlite_locked")
    } else if low.contains("no bindings indexed") {
        Some(guarded_or("no_bindings_indexed", result))
    } else if low.contains("unexpected argument")
        || low.contains("unrecognized")
        || low.contains("invalid value")
        || low.contains("path required")
    {
        Some("bad_invocation")
    } else if low.contains("no symbol matches") || low.contains("no symbol docstring matches") {
        Some(guarded_or("no_symbol", result))
    } else if low.contains("no results found") || low.contains("0 results") {
        Some(guarded_or("no_results", result))
    } else if status_exited_nonzero(&status)
        && !status.contains("exited 0 ")
        && !low.contains("no results found")
    {
        Some("nonzero_other")
    } else {
        None
    }
}

fn first_signal(result: &str) -> String {
    result
        .lines()
        .map(str::trim)
        .find(|l| {
            !l.is_empty()
                && !l.starts_with("[INFO]")
                && !l.starts_with("════")
                && !l.starts_with("────")
        })
        .unwrap_or("")
        .chars()
        .take(140)
        .collect()
}

fn value_has_next_hint(v: &Value) -> bool {
    match v {
        Value::Object(map) => {
            map.contains_key("next_hint")
                || map.contains_key("ownership_next_hint")
                || map.values().any(value_has_next_hint)
        }
        Value::Array(items) => items.iter().any(value_has_next_hint),
        _ => false,
    }
}

fn result_has_json_next_hint(result: &str) -> bool {
    if !result.contains("\"next_hint\"") && !result.contains("\"ownership_next_hint\"") {
        return false;
    }
    let trimmed = result.trim();
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        return value_has_next_hint(&v);
    }
    if let Some(pos) = trimmed.find('{') {
        if let Ok(v) = serde_json::from_str::<Value>(&trimmed[pos..]) {
            return value_has_next_hint(&v);
        }
    }
    true
}

fn flag_value_usize(cmd: &str, flag: &str) -> Option<usize> {
    let mut it = cmd.split_whitespace();
    while let Some(tok) = it.next() {
        if tok == flag {
            return it.next().and_then(|v| v.parse().ok());
        }
    }
    None
}

fn has_flag(cmd: &str, flag: &str) -> bool {
    cmd.split_whitespace().any(|tok| tok == flag)
}

fn likely_generic_query(cmd: &str) -> bool {
    [
        "\"String\"",
        "\"PathLike\"",
        "\"intern\"",
        "\"ref\"",
        "\"deref\"",
        "\"Object\"",
        "\"Value\"",
        "\"Error\"",
    ]
    .iter()
    .any(|needle| cmd.contains(needle))
}

fn has_post_filter_pipeline(cmd: &str) -> bool {
    cmd_has_unquoted_pipe(cmd)
}

fn has_query_pipe_marker(cmd: &str) -> bool {
    if cmd_has_unquoted_pipe(cmd) {
        return false;
    }
    let bytes = cmd.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if *b != b'|' {
            continue;
        }
        let prev = i.checked_sub(1).and_then(|j| bytes.get(j)).copied();
        let next = bytes.get(i + 1).copied();
        if prev == Some(b'\\') {
            return true;
        }
        if prev.is_some_and(|c| !c.is_ascii_whitespace())
            && next.is_some_and(|c| !c.is_ascii_whitespace())
        {
            return true;
        }
    }
    false
}

fn has_line_range_file_filter(cmd: &str) -> bool {
    let mut it = cmd.split_whitespace();
    while let Some(tok) = it.next() {
        if tok == "--file" {
            if let Some(value) = it.next() {
                let value = value.trim_matches(|c| c == '"' || c == '\'');
                if !value.contains('/') && !value.contains('.') {
                    let mut parts = value.split('-');
                    let Some(start) = parts.next() else {
                        continue;
                    };
                    let end = parts.next();
                    if parts.next().is_none()
                        && start.parse::<usize>().map_or(false, |n| n > 0)
                        && end.map_or(true, |n| n.parse::<usize>().map_or(false, |n| n > 0))
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn has_low_value_context_callee(cmd: &str) -> bool {
    if !cmd.contains("monogram context ") {
        return false;
    }
    [
        "allocator",
        "argument",
        "assert",
        "core",
        "create",
        "DECLARE_THROW_SCOPE",
        "defaultGlobalObject",
        "get",
        "getTreeHasParents",
        "getVM",
        "identifier",
        "idx",
        "idleAdd",
        "init",
        "isOnline",
        "isUndefined",
        "new",
        "newInstance",
        "notifyByPspec",
        "promiseStructure",
        "private",
        "ptr",
        "remove",
        "RETURN_IF_EXCEPTION",
        "scriptExecutionContext",
        "set",
        "setChild",
        "validateBoolean",
        "validateObject",
        "warn",
        "wrapped",
    ]
    .iter()
    .any(|name| {
        cmd.contains(&format!("monogram context {}", name))
            || cmd.contains(&format!("monogram context \"{}\"", name))
            || cmd.contains(&format!("monogram context '{}'", name))
    })
}

fn classify_patterns(sub: &str, cmd: &str, result: &str, has_json_next: bool) -> Vec<&'static str> {
    let mut out = vec![];
    let is_json = has_flag(cmd, "--json");
    let depth = flag_value_usize(cmd, "--depth");
    let code_lines = flag_value_usize(cmd, "--code");
    let n_limit = flag_value_usize(cmd, "-n")
        .or_else(|| flag_value_usize(cmd, "--n"))
        .or_else(|| flag_value_usize(cmd, "--limit"));

    if has_json_next {
        out.push("json_next_hint_present");
    }
    if is_json && !has_json_next {
        out.push("json_without_next_hint");
    }
    if is_json && result.len() > 50_000 && !has_json_next {
        out.push("oversized_json_without_next_hint");
    }
    if sub == "search" && has_flag(cmd, "--explain") {
        out.push("search_explain");
    }
    if sub == "region" && has_flag(cmd, "--score-debug") {
        out.push("region_score_debug");
    }
    if sub == "search" && has_flag(cmd, "--explain") && result.len() > 50_000 {
        out.push("oversized_search_explain");
    }
    if sub == "search"
        && has_flag(cmd, "--explain")
        && result.to_lowercase().contains("no results found")
    {
        out.push("search_explain_no_results");
    }
    if sub == "search" && has_flag(cmd, "--explain") && n_limit.is_some_and(|n| n >= 20) {
        out.push("search_explain_high_limit");
    }
    if sub == "chain" && depth.is_none() {
        out.push("chain_default_depth_3");
    }
    if sub == "chain" && depth.is_some_and(|d| d >= 3) {
        out.push("chain_depth_ge_3");
    }
    if sub == "chain" && has_flag(cmd, "--callers") && depth.unwrap_or(3) >= 3 {
        out.push("chain_callers_depth_ge_3");
    }
    if sub == "context" && code_lines.is_some_and(|n| n >= 100) {
        out.push("context_code_ge_100");
    }
    if sub == "context" && result.len() > 50_000 {
        out.push("oversized_context_bundle");
    }
    if likely_generic_query(cmd) {
        out.push("generic_symbol_or_query");
    }
    if has_post_filter_pipeline(cmd) {
        out.push("shell_post_filter_pipeline");
    }
    if has_query_pipe_marker(cmd) {
        if sub == "grep" {
            out.push("regex_alternation_query");
        } else {
            out.push("query_pipe_marker");
        }
    }
    if has_line_range_file_filter(cmd) || result.contains("bad_file_filter_line_range") {
        out.push("bad_file_filter_line_range");
    }
    if sub == "context" && has_low_value_context_callee(cmd) {
        out.push("low_value_context_callee");
    }
    if result.contains("free_site_triage") {
        out.push("free_site_triage");
    }
    if result.contains("systems_lifecycle_next") {
        out.push("systems_lifecycle_next");
    }
    if result.contains("lifecycle_owner_contrast") {
        out.push("lifecycle_owner_contrast");
    }
    if result.contains("lifecycle_owner_helper_contrast") {
        out.push("lifecycle_owner_helper_contrast");
    }
    if result.contains("broad_lifecycle_or_redirect") {
        out.push("broad_lifecycle_or_redirect");
    }
    if result.contains("answer_not_ready") {
        out.push("answer_not_ready");
    }
    if result.contains("lifecycle_file_probe") {
        out.push("lifecycle_file_probe");
    }
    if result.contains("region_contrast_lock") {
        out.push("region_contrast_lock");
    }
    if result.contains("guarded_anchor_preserve") {
        out.push("guarded_anchor_preserve");
    }
    if result.contains("bounded_contrast_only") {
        out.push("bounded_contrast_only");
    }
    if result.contains("ui_render_timing_next") {
        out.push("ui_render_timing_next");
    }
    if result.contains("ui_lifecycle_free_site_redirect") {
        out.push("ui_lifecycle_free_site_redirect");
    }
    if result.contains("context_signature_symbol_redirect") {
        out.push("context_signature_symbol_redirect");
    }
    if result.contains("fanout_preflight") {
        out.push("fanout_preflight");
    }
    if result.contains("blocked_inline_output") {
        out.push("blocked_inline_output");
    }
    if result.contains("compact_json") {
        out.push("compact_json");
    }
    if result.contains("_json_compacted") {
        out.push("json_compacted_any");
    }
    if result.contains("chain_json_compacted") {
        out.push("chain_json_compacted");
    }
    if result.contains("tree_json_compacted") {
        out.push("tree_json_compacted");
    }
    if result.contains("context_json_compacted") {
        out.push("context_json_compacted");
    }
    if result.contains("budget_truncated") {
        out.push("budget_truncated");
    }
    if result.contains("broad_explain_compacted") {
        out.push("broad_explain_compacted");
    }
    if result.contains("context_code_capped") {
        out.push("context_code_capped");
    }
    if result.contains("region_first_next") {
        out.push("region_first_next");
    }
    if result.contains("rootcause_label_guard") {
        out.push("rootcause_label_guard");
    }
    if result.contains("staged_depth_next") {
        out.push("staged_depth_next");
    }
    if result.contains("success_pattern_next") {
        out.push("success_pattern_next");
    }
    if result.contains("ownership_verb_redirect") {
        out.push("ownership_verb_redirect");
    }
    if result.contains("generic_symbol_redirect") {
        out.push("generic_symbol_redirect");
    }
    if result.contains("pipe_query_redirect") {
        out.push("pipe_query_redirect");
    }
    out
}

fn region_query_terms(cmd: &str) -> Option<Vec<String>> {
    monogram_query_terms(cmd, &["region", "locate", "discover"])
}

fn monogram_query_terms(cmd: &str, allowed_subcommands: &[&str]) -> Option<Vec<String>> {
    let idx = cmd_word_pos(cmd, "monogram")?;
    let rest = cmd[idx + "monogram".len()..].trim_start();
    let mut parts = rest.splitn(2, char::is_whitespace);
    let sub = parts.next().unwrap_or("");
    if !allowed_subcommands.contains(&sub) {
        return None;
    }
    let query = first_shell_arg(parts.next().unwrap_or("").trim_start())?;
    let terms = normalized_audit_terms(&query)
        .into_iter()
        .filter(|term| !is_region_drift_generic_term(term))
        .collect::<Vec<_>>();
    if terms.is_empty() {
        None
    } else {
        Some(terms)
    }
}

fn first_shell_arg(s: &str) -> Option<String> {
    let mut chars = s.chars();
    let first = chars.next()?;
    if first == '"' || first == '\'' {
        let quote = first;
        let mut out = String::new();
        let mut escaped = false;
        for ch in chars {
            if escaped {
                out.push(ch);
                escaped = false;
            } else if ch == '\\' && quote == '"' {
                escaped = true;
            } else if ch == quote {
                break;
            } else {
                out.push(ch);
            }
        }
        return Some(out);
    }
    Some(
        std::iter::once(first)
            .chain(chars)
            .take_while(|ch| !ch.is_whitespace())
            .collect(),
    )
}

fn normalized_audit_terms(query: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in query.split(|c: char| c.is_whitespace() || c == '|') {
        add_audit_term(&mut out, raw);
        for part in raw.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
            add_audit_term(&mut out, part);
        }
    }
    out
}

fn add_audit_term(out: &mut Vec<String>, raw: &str) {
    let term: String = raw
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .flat_map(|c| c.to_lowercase())
        .collect();
    if term.len() < 3 || out.contains(&term) {
        return;
    }
    out.push(term);
}

fn is_region_drift_generic_term(term: &str) -> bool {
    matches!(
        term,
        "ownership"
            | "owner"
            | "owned"
            | "boundary"
            | "boundaries"
            | "ref"
            | "refs"
            | "deref"
            | "derefs"
            | "retain"
            | "retains"
            | "release"
            | "releases"
            | "free"
            | "frees"
            | "string"
            | "thread"
            | "worker"
            | "memory"
            | "safety"
    )
}

fn region_queries_drift(prev: &[String], next: &[String]) -> bool {
    if prev.len() < 2 || next.len() < 2 {
        return false;
    }
    let overlap = prev.iter().filter(|term| next.contains(*term)).count();
    overlap == 0
}

fn backtick_spans(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(start) = rest.find('`') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        let label = after_start[..end].trim();
        if !label.is_empty() && !out.iter().any(|v| v == label) {
            out.push(label.to_string());
        }
        rest = &after_start[end + 1..];
    }
    out
}

fn guard_forbidden_labels(result: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in result.lines() {
        let low = line.to_ascii_lowercase();
        let guard_start = ["do not label", "do not replace it with"]
            .iter()
            .filter_map(|needle| low.find(needle))
            .min();
        let Some(start) = guard_start else {
            continue;
        };
        for label in backtick_spans(&line[start..]) {
            if !out.iter().any(|v| v == &label) {
                out.push(label);
            }
        }
    }
    out
}

fn rootcause_contains_label(rootcause: &str, label: &str) -> bool {
    let label_terms = normalized_audit_terms(label);
    if label_terms.is_empty() {
        return false;
    }
    let root_terms = normalized_audit_terms(rootcause);
    label_terms.iter().any(|term| root_terms.contains(term))
}

fn oversized_kind(sub: &str, result: &str, cmd: &str) -> String {
    if cmd.contains("--json") {
        "json_dump".into()
    } else if result.contains("MONOGRAM CONTEXT") {
        "context_bundle".into()
    } else if sub == "grep" && result.contains("↳ in ") {
        "grep_chain".into()
    } else if sub == "chain" {
        "chain_graph".into()
    } else if result.contains("MONOGRAM Search") {
        "search_results".into()
    } else if result.contains("SYMBOLS:") {
        "symbols_listing".into()
    } else {
        sub.into()
    }
}

fn inc(map: &mut BTreeMap<String, usize>, key: impl Into<String>) {
    *map.entry(key.into()).or_default() += 1;
}

fn map_count(map: &BTreeMap<String, usize>, key: &str) -> usize {
    map.get(key).copied().unwrap_or(0)
}

fn is_failure_grade(grade: &str) -> bool {
    matches!(
        grade,
        "MISS" | "DECOY" | "NAME" | "NAME_ONLY" | "NO_RESULT" | "INVALID" | "FORFEIT"
    )
}

fn maker_recommendations(
    rows: &[Row],
    oversized: &[Oversized],
    total: &Row,
) -> Vec<MakerRecommendation> {
    let mut out = vec![];

    let closed_but_wrong = rows
        .iter()
        .filter(|r| {
            is_failure_grade(&r.grade)
                && r.calls >= 10
                && (map_count(&r.patterns, "region_first_next") > 0
                    || map_count(&r.patterns, "success_pattern_next") > 0
                    || map_count(&r.patterns, "region_score_debug") > 0
                    || map_count(&r.patterns, "systems_lifecycle_next") > 0
                    || map_count(&r.patterns, "lifecycle_file_probe") > 0
                    || map_count(&r.patterns, "region_contrast_lock") > 0)
        })
        .count();
    if closed_but_wrong > 0 {
        out.push(MakerRecommendation {
            signal: "closed_candidate_space_but_wrong_root",
            count: closed_but_wrong,
            why: "failure runs used region/NEXT/score-debug steering but still ended on the wrong root",
            avoid: "copying observed trace terms into generic classifiers, scoring branches, or fixed NEXT strings",
            prefer: "scoreable evidence such as facet coverage, anchor coverage, graph reachability, coupling endpoints, and proof markers",
            validate: "rerun the exposing failure, a prior FULL case, and an unrelated holdout; inspect trace shape, not grade alone",
        });
    }

    let broad_output_pressure = map_count(&total.patterns, "context_code_ge_100")
        + map_count(&total.patterns, "chain_depth_ge_3")
        + map_count(&total.patterns, "chain_callers_depth_ge_3")
        + map_count(&total.patterns, "search_explain_high_limit")
        + map_count(&total.patterns, "fanout_preflight")
        + map_count(&total.patterns, "oversized_context_bundle")
        + map_count(&total.patterns, "oversized_search_explain")
        + oversized.len();
    if broad_output_pressure > 0 {
        out.push(MakerRecommendation {
            signal: "broad_output_or_fanout_loop",
            count: broad_output_pressure,
            why: "large context/search/chain shapes still appear and can keep the solver circling nearby evidence",
            avoid: "raising raw limits or allowing full dumps as the default recovery path",
            prefer: "budgeted preflight, staged depth, compact summaries, and region-first narrowing before expanded context",
            validate: "compare output bytes, NEXT adherence, and root-cause cone width before and after the change",
        });
    }

    let guarded_recovery = map_count(&total.kinds, "guarded_no_match");
    if guarded_recovery > 0 {
        out.push(MakerRecommendation {
            signal: "guarded_no_match_recovery_pressure",
            count: guarded_recovery,
            why: "monogram avoided a dead no-match, but the recovery path still needs ranking and narrowing evidence",
            avoid: "treating guarded recovery as success by itself",
            prefer: "rewrite the recovery into region/query-facet candidates with explicit uncertainty and next proof steps",
            validate: "trace whether the run moves from guarded recovery to a smaller verified candidate set",
        });
    }

    let lifecycle_proof_unresolved = rows
        .iter()
        .filter(|r| {
            is_failure_grade(&r.grade)
                && r.calls >= 10
                && (map_count(&r.patterns, "systems_lifecycle_next") > 0
                    || map_count(&r.patterns, "lifecycle_file_probe") > 0
                    || map_count(&r.patterns, "free_site_triage") > 0)
        })
        .count();
    if lifecycle_proof_unresolved > 0 {
        out.push(MakerRecommendation {
            signal: "lifecycle_proof_unresolved",
            count: lifecycle_proof_unresolved,
            why: "failure runs received lifecycle/root proof guidance but still widened or chose a nearby helper",
            avoid: "adding more generic lifecycle prose without measuring whether the next command preserves the same file/object anchor",
            prefer: "make lifecycle proof output compare the same-file operation, helper, and teardown candidates as a bounded contrast set",
            validate: "trace the first command after systems_lifecycle_next or lifecycle_file_probe and confirm whether it stays on the guarded anchor",
        });
    }

    let contrast_lock_unresolved = rows
        .iter()
        .filter(|r| {
            is_failure_grade(&r.grade)
                && r.calls >= 10
                && map_count(&r.patterns, "region_contrast_lock") > 0
        })
        .count();
    if contrast_lock_unresolved > 0 {
        out.push(MakerRecommendation {
            signal: "region_contrast_lock_unresolved",
            count: contrast_lock_unresolved,
            why: "failure runs saw a close-candidate contrast lock but did not resolve the bounded cone",
            avoid: "treating close top regions as a reason to widen the query or raise output limits",
            prefer: "rank the locked candidates with same-file context, depth-1 callers, and explicit operation-boundary evidence",
            validate: "compare the commands after region_contrast_lock against the final ROOTCAUSE cone",
        });
    }

    let region_query_drift = map_count(&total.patterns, "region_query_anchor_drift");
    if region_query_drift > 0 {
        out.push(MakerRecommendation {
            signal: "region_query_anchor_drift",
            count: region_query_drift,
            why: "a run changed non-generic region query anchors between bounded region calls",
            avoid: "widening from the current candidate cone into unrelated subsystem terms without carrying a stable anchor",
            prefer: "keep at least one concrete symptom, operation, file, or coupling anchor when rerunning region",
            validate: "compare adjacent region calls in trace; the later query should preserve a non-generic anchor or explicitly mark the pivot as contrast evidence",
        });
    }

    let label_guard_pivot = map_count(&total.patterns, "rootcause_label_guard_pivot");
    if label_guard_pivot > 0 {
        out.push(MakerRecommendation {
            signal: "rootcause_label_guard_pivot",
            count: label_guard_pivot,
            why: "a run received a root-cause label guard, then pivoted to disjoint query anchors",
            avoid: "treating guard markers as decorative output while widening into another subsystem",
            prefer: "preserve the guarded query anchor or make the next command a bounded contrast check",
            validate: "trace the command immediately after [marker: rootcause_label_guard] and confirm whether it keeps an anchor",
        });
    }

    let label_guard_ignored = map_count(&total.patterns, "rootcause_label_guard_ignored");
    if label_guard_ignored > 0 {
        out.push(MakerRecommendation {
            signal: "rootcause_label_guard_ignored",
            count: label_guard_ignored,
            why: "a failure run received a guard that forbade a wrapper/mechanism label, but the final ROOTCAUSE used that forbidden label",
            avoid: "adding more decorative guard text without measuring whether the solver obeys it",
            prefer: "make guard violations auditable first, then improve ranking/proof output that locks the owner candidate",
            validate: "inspect the guard line and final ROOTCAUSE; the forbidden label should not be a substring of the owner label",
        });
    }

    let static_steering_review = rows
        .iter()
        .filter(|r| {
            is_failure_grade(&r.grade)
                && map_count(&r.patterns, "success_pattern_next") > 0
                && map_count(&r.patterns, "ownership_verb_redirect") > 0
        })
        .count();
    if static_steering_review > 0 {
        out.push(MakerRecommendation {
            signal: "source_promotion_review_required",
            count: static_steering_review,
            why: "failure traces show fixed success-pattern steering and ownership redirects in the same loop",
            avoid: "promoting benchmark-observed names into source-level query guards, route selection, or canned commands",
            prefer: "derive NEXT commands from current top-region evidence kinds, current-file facets, and measured broadness",
            validate: "review source diffs for literal promotion, then rerun with at least one unrelated benchmark",
        });
    }

    out
}

fn print_maker_recommendations(recs: &[MakerRecommendation]) {
    if recs.is_empty() {
        return;
    }
    println!("\nMAKER RECOMMENDATIONS");
    for r in recs.iter().take(8) {
        println!("  {}  count={}", r.signal, r.count);
        println!("    why: {}", r.why);
        println!("    avoid: {}", r.avoid);
        println!("    prefer: {}", r.prefer);
        println!("    validate: {}", r.validate);
    }
}

pub fn audit(id: &str, files: &[String], stats: &[RunStats]) {
    let grades: HashMap<String, String> = stats
        .iter()
        .map(|s| (s.label.clone(), s.grade.clone()))
        .collect();
    let rootcauses: HashMap<String, String> = stats
        .iter()
        .map(|s| (s.label.clone(), s.rootcause.clone()))
        .collect();
    let mut rows: Vec<Row> = vec![];
    let mut total = Row::default();
    let mut oversized: Vec<Oversized> = vec![];
    let mut oversized_kinds: BTreeMap<String, usize> = BTreeMap::new();
    for f in files {
        let calls = telemetry::events_from_path(f);
        if calls.is_empty() {
            continue;
        }
        let label = telemetry::label_from_path(f);
        let mut row = Row {
            label: label.clone(),
            grade: grades.get(&label).cloned().unwrap_or_else(|| "?".into()),
            ..Row::default()
        };
        let mut last_region_terms: Option<Vec<String>> = None;
        let mut last_label_guard_terms: Option<Vec<String>> = None;
        let mut forbidden_rootcause_labels: Vec<String> = Vec::new();
        for ev in calls {
            if ev.name != "Bash" {
                continue;
            }
            let Some(sub) = monogram_sub(&ev.cmd) else {
                continue;
            };
            row.calls += 1;
            inc(&mut row.subs, sub.clone());
            if sub == "help" {
                row.help += 1;
            }
            let next_count = ev
                .result
                .lines()
                .filter(|l| l.trim_start().starts_with("[NEXT]"))
                .count();
            row.next_lines += next_count;
            let has_json_next = result_has_json_next_hint(&ev.result);
            if has_json_next {
                row.json_next_hints += 1;
            }
            let query_terms = monogram_query_terms(
                &ev.cmd,
                &["search", "grep", "refgrep", "context", "symbols", "chain"],
            );
            if let (Some(prev), Some(next)) = (&last_label_guard_terms, &query_terms) {
                if region_queries_drift(prev, next) {
                    inc(&mut row.patterns, "rootcause_label_guard_pivot");
                    last_label_guard_terms = None;
                }
            }
            for pattern in classify_patterns(&sub, &ev.cmd, &ev.result, has_json_next) {
                inc(&mut row.patterns, pattern);
            }
            if ev.result.contains("rootcause_label_guard") {
                for label in guard_forbidden_labels(&ev.result) {
                    if !forbidden_rootcause_labels.iter().any(|v| v == &label) {
                        forbidden_rootcause_labels.push(label);
                    }
                }
                if let Some(terms) = query_terms.clone() {
                    last_label_guard_terms = Some(terms);
                }
            }
            if sub == "region" {
                if let Some(terms) = region_query_terms(&ev.cmd) {
                    if let Some(prev) = &last_region_terms {
                        if region_queries_drift(prev, &terms) {
                            inc(&mut row.patterns, "region_query_anchor_drift");
                        }
                    }
                    last_region_terms = Some(terms);
                }
            }
            if ev.result.len() > 50_000 {
                row.oversized += 1;
                let kind = oversized_kind(&sub, &ev.result, &ev.cmd);
                inc(&mut oversized_kinds, kind.clone());
                oversized.push(Oversized {
                    label: label.clone(),
                    grade: row.grade.clone(),
                    sub: sub.clone(),
                    kind,
                    bytes: ev.result.len(),
                    lines: ev.result.lines().count(),
                    next: next_count,
                    json_next: has_json_next,
                    cmd: ev.cmd.clone(),
                    signal: first_signal(&ev.result),
                });
            }
            if let Some(kind) = issue_kind(&sub, &ev.result) {
                row.issues += 1;
                inc(&mut row.kinds, kind);
                if row.examples.len() < 3 {
                    row.examples
                        .push((kind.into(), ev.cmd.clone(), first_signal(&ev.result)));
                }
            }
        }
        if is_failure_grade(&row.grade) {
            if let Some(rootcause) = rootcauses.get(&label) {
                if forbidden_rootcause_labels
                    .iter()
                    .any(|label| rootcause_contains_label(rootcause, label))
                {
                    inc(&mut row.patterns, "rootcause_label_guard_ignored");
                }
            }
        }
        if row.calls == 0 {
            continue;
        }
        total.calls += row.calls;
        total.issues += row.issues;
        total.oversized += row.oversized;
        total.help += row.help;
        total.next_lines += row.next_lines;
        total.json_next_hints += row.json_next_hints;
        for (k, v) in &row.subs {
            *total.subs.entry(k.clone()).or_default() += v;
        }
        for (k, v) in &row.kinds {
            *total.kinds.entry(k.clone()).or_default() += v;
        }
        for (k, v) in &row.patterns {
            *total.patterns.entry(k.clone()).or_default() += v;
        }
        rows.push(row);
    }

    println!("MONOGRAM AUDIT  {id}");
    println!(
        "runs={} calls={} issues={} oversized={} help={} next-lines={} json-next={}",
        rows.len(),
        total.calls,
        total.issues,
        total.oversized,
        total.help,
        total.next_lines,
        total.json_next_hints
    );
    print_map("issues", &total.kinds);
    print_map("subcommands", &total.subs);
    print_map("patterns", &total.patterns);
    print_map("oversized-kinds", &oversized_kinds);
    let recs = maker_recommendations(&rows, &oversized, &total);
    print_maker_recommendations(&recs);

    rows.sort_by(|a, b| {
        b.issues
            .cmp(&a.issues)
            .then_with(|| b.oversized.cmp(&a.oversized))
            .then_with(|| a.label.cmp(&b.label))
    });
    println!("\nRUNS");
    for r in rows.iter().take(20) {
        println!(
            "  {:<44} grade={:<9} calls={:<4} issues={:<3} oversized={:<3} help={:<2}",
            r.label, r.grade, r.calls, r.issues, r.oversized, r.help
        );
        if !r.kinds.is_empty() {
            print!("    kinds:");
            for (k, v) in &r.kinds {
                print!(" {k}×{v}");
            }
            println!();
        }
        if !r.patterns.is_empty() {
            let mut pairs: Vec<(&String, &usize)> = r.patterns.iter().collect();
            pairs.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
            print!("    patterns:");
            for (k, v) in pairs.into_iter().take(8) {
                print!(" {k}×{v}");
            }
            println!();
        }
        for (kind, cmd, signal) in &r.examples {
            println!(
                "    - {kind}: {}",
                cmd.chars().take(110).collect::<String>()
            );
            if !signal.is_empty() {
                println!("      {}", signal);
            }
        }
    }
    if !oversized.is_empty() {
        oversized.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.label.cmp(&b.label)));
        println!("\nOVERSIZED OUTPUTS  (>50KB, largest first)");
        for o in oversized.iter().take(16) {
            println!(
                "  {:<44} grade={:<9} {:<15} {:>7}B {:>5} lines next={:<3} jsonNext={:<5} sub={}",
                o.label, o.grade, o.kind, o.bytes, o.lines, o.next, o.json_next, o.sub
            );
            println!("    {}", o.cmd.chars().take(130).collect::<String>());
            if !o.signal.is_empty() {
                println!("    {}", o.signal);
            }
        }
    }
}

fn print_map(title: &str, map: &BTreeMap<String, usize>) {
    if map.is_empty() {
        return;
    }
    println!("\n{title}");
    let mut pairs: Vec<(&String, &usize)> = map.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (k, v) in pairs.into_iter().take(24) {
        println!("  {:<24} {}", k, v);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_patterns, guard_forbidden_labels, issue_kind, maker_recommendations,
        monogram_query_terms, monogram_sub, region_queries_drift, region_query_terms,
        rootcause_contains_label, Row,
    };
    use std::collections::BTreeMap;

    #[test]
    fn monogram_redirection_probe_is_help_not_redirection_subcommand() {
        assert_eq!(
            monogram_sub("monogram 2>&1 | head -100").as_deref(),
            Some("help")
        );
        assert_eq!(monogram_sub("niia && monogram").as_deref(), Some("help"));
    }

    #[test]
    fn issue_kind_uses_first_status_line_only() {
        let result = "succeeded in 182ms:\nreal output\n\n exited 1 in 5903ms:\nother command";
        assert_eq!(issue_kind("context", result), None);
        assert_eq!(
            issue_kind("context", "exited 1 in 5ms:\nactual failure"),
            Some("nonzero_other")
        );
    }

    #[test]
    fn region_score_debug_is_a_command_shape_pattern() {
        let patterns = classify_patterns(
            "region",
            "monogram region \"ownership boundary\" -n 5 --score-debug",
            "",
            false,
        );
        assert!(patterns.contains(&"region_score_debug"));
    }

    #[test]
    fn rootcause_label_guard_is_a_command_shape_pattern() {
        let patterns = classify_patterns(
            "grep",
            "monogram grep \"switch.*tag\" --chain --depth 1",
            "[marker: rootcause_label_guard]\n",
            false,
        );
        assert!(patterns.contains(&"rootcause_label_guard"));
    }

    #[test]
    fn lifecycle_and_contrast_markers_are_command_shape_patterns() {
        let patterns = classify_patterns(
            "region",
            "monogram region \"session teardown\" -n 5 --score-debug",
            "[marker: systems_lifecycle_next]\n[marker: lifecycle_file_probe]\n[marker: lifecycle_owner_contrast]\n[marker: lifecycle_owner_helper_contrast]\n[marker: broad_lifecycle_or_redirect]\n[marker: answer_not_ready]\n[marker: region_contrast_lock]\n[marker: guarded_anchor_preserve]\n[marker: bounded_contrast_only]\n",
            false,
        );
        assert!(patterns.contains(&"systems_lifecycle_next"));
        assert!(patterns.contains(&"lifecycle_file_probe"));
        assert!(patterns.contains(&"lifecycle_owner_contrast"));
        assert!(patterns.contains(&"lifecycle_owner_helper_contrast"));
        assert!(patterns.contains(&"broad_lifecycle_or_redirect"));
        assert!(patterns.contains(&"answer_not_ready"));
        assert!(patterns.contains(&"region_contrast_lock"));
        assert!(patterns.contains(&"guarded_anchor_preserve"));
        assert!(patterns.contains(&"bounded_contrast_only"));
    }

    #[test]
    fn rootcause_label_guard_forbidden_label_matches_wrapper_not_owner() {
        let result = "[WARN] ffi_owner_rootcause_hint: `toThreadSafe` delegates across the C-ABI boundary into `BunString__toThreadSafe`.\n  ROOTCAUSE label guard: if `BunString__toThreadSafe` owns the imbalance, answer `ROOTCAUSE: <owner-file>::BunString__toThreadSafe`; do not label the thin wrapper `toThreadSafe` unless the owner is disproven.";
        assert_eq!(guard_forbidden_labels(result), vec!["toThreadSafe"]);
        assert!(rootcause_contains_label(
            "ROOTCAUSE: src/string.zig::String::toThreadSafe",
            "toThreadSafe"
        ));
        assert!(!rootcause_contains_label(
            "ROOTCAUSE: src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe",
            "toThreadSafe"
        ));
    }

    #[test]
    fn label_guard_followup_terms_detect_disjoint_pivot() {
        let guard = monogram_query_terms(
            "monogram grep \"switch.*tag\" --chain --depth 1 -n 10",
            &["grep", "search"],
        )
        .unwrap();
        let pivot = monogram_query_terms(
            "monogram search \"N-API string napi\" -n 5 --explain",
            &["grep", "search"],
        )
        .unwrap();
        assert!(region_queries_drift(&guard, &pivot));
    }

    #[test]
    fn region_query_anchor_drift_detects_disjoint_specific_terms() {
        let first = region_query_terms(
            "monogram region \"ownership boundary ref deref leakRef isolatedCopy\" -n 5 --score-debug",
        )
        .unwrap();
        let second = region_query_terms(
            "monogram region \"String thread worker transferToJS fromJS thread transfer\" -n 5 --score-debug",
        )
        .unwrap();

        assert!(region_queries_drift(&first, &second));
    }

    #[test]
    fn region_query_anchor_drift_preserves_shared_specific_anchor() {
        let first = region_query_terms(
            "monogram region \"ownership boundary ref deref leakRef isolatedCopy\" -n 5 --score-debug",
        )
        .unwrap();
        let second = region_query_terms(
            "monogram region \"leakRef thread transfer evidence\" -n 5 --score-debug",
        )
        .unwrap();

        assert!(!region_queries_drift(&first, &second));
    }

    #[test]
    fn source_promotion_recommendation_requires_steering_shape_not_literals() {
        let literal_only = Row {
            label: "r1".into(),
            grade: "MISS".into(),
            calls: 20,
            ..Row::default()
        };
        let total = Row::default();
        let recs = maker_recommendations(&[literal_only], &[], &total);
        assert!(!recs
            .iter()
            .any(|r| r.signal == "source_promotion_review_required"));

        let mut patterns = BTreeMap::new();
        patterns.insert("success_pattern_next".into(), 1);
        patterns.insert("ownership_verb_redirect".into(), 1);
        let steering_shape = Row {
            label: "r2".into(),
            grade: "MISS".into(),
            calls: 20,
            patterns,
            ..Row::default()
        };
        let recs = maker_recommendations(&[steering_shape], &[], &total);
        assert!(recs
            .iter()
            .any(|r| r.signal == "source_promotion_review_required"));
    }
}
