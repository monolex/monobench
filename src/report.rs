// monobench — per-model comparison grid (monogram visual language: [INFO] · ═/─ rules · CAPS sections
// · (N) counts · [NEXT] footer). Failures pulled to a separate section. Palette yellow→orange→red.
use crate::grade::RunStats;
use crate::util::*;

const MIN_W: usize = 80;
const MAX_W: usize = 120;

fn major(t: &str, w: usize) {
    let heavy = "═".repeat(w);
    println!("\n{}", c(HEAD, &heavy));
    println!("{}", c(HEAD, t));
    println!("{}", c(HEAD, &heavy));
}
fn sub(t: &str, w: usize) {
    println!("\n  {}\n  {}", c("1", t), c(DIM, &"─".repeat(w - 2)));
}
fn prow(label_w: usize, lbl: &str, g: &str, cost: &str, tm: &str, calls: &str, mono: &str) -> String {
    format!("  {} {} {} {} {} {}", pad_end_fit(lbl, label_w), pad_end_fit(g, 9), pad_start_fit(cost, 7), pad_start_fit(tm, 6), pad_start_fit(calls, 6), pad_start_fit(mono, 5))
}
fn arow(arm_w: usize, a: &str, n: &str, full: &str, d: &str, tok: &str, tm: &str, calls: &str, mono: &str) -> String {
    format!("  {} {} {} {} {} {} {} {}", pad_end_fit(a, arm_w), pad_start_fit(n, 2), pad_start_fit(full, 5), pad_start_fit(d, 7), pad_start_fit(tok, 6), pad_start_fit(tm, 7), pad_start_fit(calls, 17), pad_start_fit(mono, 6))
}

fn models_in_order(runs: &[&RunStats]) -> Vec<String> {
    let mut ms: Vec<String> = vec![];
    for r in runs {
        let m = parse_arm(&r.label).model;
        if !ms.contains(&m) { ms.push(m); }
    }
    ms.sort_by_key(|m| model_ord(m));
    ms
}

fn report_dims(runs: &[&RunStats]) -> (usize, usize, usize) {
    let max_run = runs.iter().map(|r| visible_len(&r.label)).max().unwrap_or(3).max(3);
    let mut max_arm = visible_len("arm");
    for r in runs {
        let a = parse_arm(&r.label);
        max_arm = max_arm.max(visible_len(&disp_name(&a.tool, &a.model, &a.effort)));
    }
    let label_w = max_run.max(28).min(MAX_W - 40);
    let arm_w = max_arm.max(20).min(MAX_W - 59);
    let width = MIN_W.max(label_w + 40).max(arm_w + 59).min(MAX_W);
    (width, label_w, arm_w)
}

pub fn report(id: &str, runs: &[RunStats]) {
    if runs.is_empty() {
        println!("(no runs yet — `monobench run <id> <arm>` or `monobench matrix <id>`)");
        return;
    }
    let fail: Vec<&RunStats> = runs.iter().filter(|x| x.grade == "FORFEIT" || x.grade == "NO_RESULT").collect();
    let ok: Vec<&RunStats> = runs.iter().filter(|x| x.grade != "FORFEIT" && x.grade != "NO_RESULT").collect();
    let models = models_in_order(&ok);
    let (width, run_label_w, arm_w) = report_dims(&ok);
    let mut arms_set: Vec<String> = vec![];
    for r in &ok { let a = parse_arm(&r.label).arm; if !arms_set.contains(&a) { arms_set.push(a); } }

    println!("{}{}{}", c(DIM, "[INFO] "), c("1", id),
        c(DIM, &format!(" · {} runs · {} models · {} arms · {} failed", ok.len(), models.len(), arms_set.len(), fail.len())));

    // ── per run, grouped by model ──
    major("PER-RUN RESULTS  (every run, grouped by model — consistency)", width);
    for m in &models {
        let mut rows: Vec<&&RunStats> = ok.iter().filter(|r| &parse_arm(&r.label).model == m).collect();
        rows.sort_by(|a, b| a.label.cmp(&b.label));
        sub(&format!("{}  ({})", m.to_uppercase(), rows.len()), width);
        println!("{}", c(DIM, &prow(run_label_w, "run", "grade", "cost$", "time", "calls", "mono")));
        for r in rows {
            let tool = parse_arm(&r.label).tool;
            let calls = r.calls.map(|c| format!("{c}c")).unwrap_or_else(|| "—".into());
            let row = prow(run_label_w, &r.label, &r.grade, &format!("${:.2}", r.cost), &format!("{}s", r.time), &calls, &format!("·{}", r.adopt));
            println!("{}", c(arm_code(&tool), &row));
        }
    }

    // ── aggregate, grouped by model (median per arm) ──
    major("AGGREGATE  (median per arm — failures excluded)", width);
    println!("{}", c(DIM, &arow(arm_w, "arm", "n", "FULL", "med$", "medTok", "medTime", "calls min–med–max", "mono%")));
    for m in &models {
        sub(&m.to_uppercase(), width);
        // arms within this model
        let mut arm_keys: Vec<String> = vec![];
        for r in &ok {
            let a = parse_arm(&r.label);
            if &a.model == m && !arm_keys.contains(&a.arm) { arm_keys.push(a.arm); }
        }
        arm_keys.sort_by(|a, b| {
            let ba = if a.starts_with("baseline") { 0 } else { 1 };
            let bb = if b.starts_with("baseline") { 0 } else { 1 };
            ba.cmp(&bb).then(a.cmp(b))
        });
        for arm in arm_keys {
            let v: Vec<&&RunStats> = ok.iter().filter(|r| parse_arm(&r.label).arm == arm).collect();
            let a0 = parse_arm(&v[0].label);
            let is_tool = a0.tool != "baseline";
            let name = disp_name(&a0.tool, &a0.model, &a0.effort);
            let measured: Vec<&&RunStats> = v.iter().filter(|r| r.calls.is_some()).copied().collect();
            let unused = if is_tool { measured.iter().filter(|r| r.adopt == 0).count() } else { 0 };
            let full = v.iter().filter(|r| r.grade == "FULL").count();
            let cs: Vec<i64> = measured.iter().map(|r| r.calls.unwrap()).collect();
            let callstr = if cs.is_empty() { "—".to_string() }
                else { format!("{}–{}–{}", cs.iter().min().unwrap(), median_i(&cs), cs.iter().max().unwrap()) };
            let suma: i64 = measured.iter().map(|r| r.adopt).sum();
            let sumc: i64 = cs.iter().sum();
            let monostr = if !is_tool { "0%".into() }
                else { format!("{}%", if sumc > 0 { (100.0 * suma as f64 / sumc as f64).round() as i64 } else { 0 }) };
            let medc = median_f(&v.iter().map(|r| r.cost).collect::<Vec<_>>());
            let medt = median_i(&v.iter().map(|r| r.tok).collect::<Vec<_>>());
            let medtime = median_i(&v.iter().map(|r| r.time).collect::<Vec<_>>());
            let mut row = arow(arm_w, &name, &v.len().to_string(), &format!("{}/{}", full, v.len()),
                &format!("${:.2}", medc), &hum_tok(medt), &format!("{}s", medtime), &callstr, &monostr);
            if unused > 0 { row.push_str(&c(DIM, &format!("  ⚠{} never called the tool", unused))); }
            println!("{}", c(arm_code(&a0.tool), &row));
        }
    }

    // ── failures ──
    if !fail.is_empty() {
        major("FAILURES & INCOMPLETE  (excluded from medians)", width);
        let mut fs = fail.clone();
        fs.sort_by(|a, b| a.label.cmp(&b.label));
        for r in fs {
            let why = if r.grade == "FORFEIT" { "could not index repo (OOM)" } else { "incomplete / over budget" };
            let tool = parse_arm(&r.label).tool;
            println!("{}{}", c(arm_code(&tool), &format!("  {} {}", pad_end_fit(&r.label, run_label_w), pad_end_fit(&r.grade, 10))), c(DIM, &format!("— {}", why)));
        }
    }

    println!("\n{}", c(HEAD, "[NEXT]"));
    println!("{}", c(DIM, &format!("  monobench adoption {id}   # tool-call breakdown")));
    println!("{}", c(DIM, &format!("  monobench show {id} --spoil   # answer key")));
    println!("{}", c(DIM, "\nmono% = monogram share of tool calls · sampling n=1 → 3 → 5/9 (SPEC §6)"));
}

/// Cross-instance leaderboard: root-cause FULL hit-rate per arm × instance (failures excluded).
pub fn summary(insts: &[(String, Vec<RunStats>)]) {
    let heavy = "═".repeat(MIN_W);
    println!("\n{}", c(HEAD, &heavy));
    println!("{}", c(HEAD, "SUMMARY  ·  root-cause Hit-rate (FULL) per arm × instance  (failures excluded)"));
    println!("{}", c(HEAD, &heavy));
    if insts.iter().all(|(_, r)| r.is_empty()) { println!("(no runs in any instance — run `monobench matrix <id> …`)"); return; }
    let gradeable = |r: &&RunStats| r.grade != "FORFEIT" && r.grade != "NO_RESULT";

    // unique arms across all instances, ordered (baseline-first, then model order, then alpha)
    let mut arms: Vec<(String, String, String)> = vec![]; // (display, tool, model)
    for (_, runs) in insts { for r in runs.iter().filter(gradeable) {
        let a = parse_arm(&r.label); let d = disp_name(&a.tool, &a.model, &a.effort);
        if !arms.iter().any(|(x, _, _)| x == &d) { arms.push((d, a.tool, a.model)); }
    } }
    arms.sort_by(|a, b| (if a.1 == "baseline" { 0 } else { 1 }).cmp(&(if b.1 == "baseline" { 0 } else { 1 }))
        .then(model_ord(&a.2).cmp(&model_ord(&b.2))).then(a.0.cmp(&b.0)));

    let cellv = |disp: &str, runs: &[RunStats]| -> (usize, usize) {
        let mut full = 0; let mut n = 0;
        for r in runs.iter().filter(gradeable) {
            let a = parse_arm(&r.label);
            if disp_name(&a.tool, &a.model, &a.effort) == disp { n += 1; if r.grade == "FULL" { full += 1; } }
        }
        (full, n)
    };

    let mut hdr = format!("  {}", pad_end("arm", 20));
    for (id, _) in insts { hdr.push_str(&format!(" {}", pad_start(&id.chars().take(13).collect::<String>(), 13))); }
    hdr.push_str(&format!(" {}", pad_start("overall", 9)));
    println!("{}", c(DIM, &hdr));
    for (disp, tool, _) in &arms {
        let mut row = format!("  {}", pad_end(disp, 20));
        let (mut tf, mut tn) = (0, 0);
        for (_, runs) in insts {
            let (f, n) = cellv(disp, runs); tf += f; tn += n;
            row.push_str(&format!(" {}", pad_start(&(if n == 0 { "–".into() } else { format!("{f}/{n}") }), 13)));
        }
        row.push_str(&format!(" {}", pad_start(&format!("{tf}/{tn}"), 9)));
        println!("{}", c(arm_code(tool), &row));
    }
    println!("{}", c(DIM, &format!("\ninstances: {}", insts.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>().join(" · "))));
    println!("{}", c(DIM, "per-instance detail: monobench report <id>"));
}
