// monobench — per-CLI/model comparison grid (monogram visual language: [INFO] · ═/─ rules · CAPS sections
// · (N) counts · [NEXT] footer). Failures pulled to a separate section. Palette yellow→orange→red.
use crate::grade::RunStats;
use crate::run_meta;
use crate::util::*;
use std::path::Path;

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
fn prow(
    label_w: usize,
    lbl: &str,
    start: &str,
    g: &str,
    cost: &str,
    tm: &str,
    calls: &str,
    mono: &str,
) -> String {
    format!(
        "  {} {} {} {} {} {} {}",
        pad_end_fit(lbl, label_w),
        pad_start_fit(start, 12),
        pad_end_fit(g, 9),
        pad_start_fit(cost, 7),
        pad_start_fit(tm, 6),
        pad_start_fit(calls, 6),
        pad_start_fit(mono, 5)
    )
}

/// UTC start time for a run label ("MM-DD HH:MMZ"); "—" for legacy labels with no embedded time.
fn start_utc(label: &str) -> String {
    match label_start_ms(label) {
        Some(ms) => fmt_utc_ms(ms),
        None => "—".into(),
    }
}
fn arow(
    arm_w: usize,
    a: &str,
    n: &str,
    full: &str,
    d: &str,
    tok: &str,
    tm: &str,
    calls: &str,
    mono: &str,
) -> String {
    format!(
        "  {} {} {} {} {} {} {} {}",
        pad_end_fit(a, arm_w),
        pad_start_fit(n, 2),
        pad_start_fit(full, 5),
        pad_start_fit(d, 7),
        pad_start_fit(tok, 6),
        pad_start_fit(tm, 7),
        pad_start_fit(calls, 17),
        pad_start_fit(mono, 6)
    )
}

/// Arm name as shown in reports: the tool, with ` @<version>` appended when a version was captured.
/// Different versions render as different arm rows so their medians never blend together.
fn arm_display(a: &Arm) -> String {
    if a.version.is_empty() {
        a.tool.clone()
    } else {
        format!("{} @{}", a.tool, a.version)
    }
}

fn envs_in_order(runs: &[&RunStats]) -> Vec<String> {
    let mut envs: Vec<(String, String, String, String)> = vec![];
    for r in runs {
        let a = parse_arm(&r.label);
        let name = env_name(&a.cli, &a.model, &a.effort);
        if !envs.iter().any(|(x, _, _, _)| x == &name) {
            envs.push((name, a.cli, a.model, a.effort));
        }
    }
    envs.sort_by_key(|(_, cli, model, effort)| env_ord(cli, model, effort));
    envs.into_iter().map(|(name, _, _, _)| name).collect()
}

fn report_dims(runs: &[&RunStats]) -> (usize, usize, usize) {
    let max_run = runs
        .iter()
        .map(|r| visible_len(&r.label))
        .max()
        .unwrap_or(3)
        .max(3);
    let mut max_arm = visible_len("arm");
    for r in runs {
        let a = parse_arm(&r.label);
        max_arm = max_arm.max(visible_len(&arm_display(&a)));
    }
    let label_w = max_run.max(28).min(MAX_W - 53);
    let arm_w = max_arm.max(20).min(MAX_W - 59);
    let width = MIN_W.max(label_w + 53).max(arm_w + 59).min(MAX_W);
    (width, label_w, arm_w)
}

pub fn report(root: &Path, id: &str, runs: &[RunStats]) {
    if runs.is_empty() {
        println!("(no runs yet — `monobench run <id> <arm>` or `monobench matrix <id>`)");
        return;
    }
    let failure_grade = |g: &str| matches!(g, "FORFEIT" | "NO_RESULT" | "INVALID");
    let fail: Vec<&RunStats> = runs.iter().filter(|x| failure_grade(&x.grade)).collect();
    let ok: Vec<&RunStats> = runs.iter().filter(|x| !failure_grade(&x.grade)).collect();
    let pending_review = runs.iter().filter(|x| !x.final_checked).count();
    let envs = envs_in_order(&ok);
    let (width, run_label_w, arm_w) = report_dims(&ok);
    let mut arms_set: Vec<String> = vec![];
    for r in &ok {
        let a = parse_arm(&r.label).arm;
        if !arms_set.contains(&a) {
            arms_set.push(a);
        }
    }

    println!(
        "{}{}{}",
        c(DIM, "[INFO] "),
        c("1", id),
        c(
            DIM,
            &format!(
                " · {} runs · {} envs · {} arms · {} failed · {} pending-review",
                ok.len(),
                envs.len(),
                arms_set.len(),
                fail.len(),
                pending_review
            )
        )
    );

    // ── per run, grouped by CLI/model/effort ──
    major(
        "PER-RUN RESULTS  (every run, grouped by CLI/MODEL — consistency)",
        width,
    );
    for env in &envs {
        let mut rows: Vec<&&RunStats> = ok
            .iter()
            .filter(|r| {
                let a = parse_arm(&r.label);
                env_name(&a.cli, &a.model, &a.effort) == *env
            })
            .collect();
        rows.sort_by(|a, b| {
            label_start_ms(&a.label)
                .unwrap_or(0)
                .cmp(&label_start_ms(&b.label).unwrap_or(0))
                .then_with(|| a.label.cmp(&b.label))
        });
        sub(&format!("{}  ({})", env.to_uppercase(), rows.len()), width);
        println!(
            "{}",
            c(
                DIM,
                &prow(
                    run_label_w,
                    "run",
                    "started",
                    "grade",
                    "cost$",
                    "time",
                    "calls",
                    "mono"
                )
            )
        );
        for r in rows {
            let tool = parse_arm(&r.label).tool;
            let calls = r
                .calls
                .map(|c| format!("{c}c"))
                .unwrap_or_else(|| "—".into());
            let mut row = prow(
                run_label_w,
                &r.label,
                &start_utc(&r.label),
                &r.grade,
                &if r.cost_available {
                    format!("${:.2}", r.cost)
                } else {
                    "—".into()
                },
                &format!("{}s", r.time),
                &calls,
                &format!("·{}", r.adopt),
            );
            row.push_str(&c(
                DIM,
                &format!(
                    "  {}",
                    if r.final_checked {
                        "reviewed"
                    } else {
                        "auto-only"
                    }
                ),
            ));
            if let Some(meta) = run_meta::summary(root, id, &r.label) {
                row.push_str(&c(DIM, &format!("  {meta}")));
            }
            println!("{}", c(arm_code(&tool), &row));
        }
    }

    // ── aggregate, grouped by CLI/model/effort (median per arm) ──
    major("AGGREGATE  (median per arm — failures excluded)", width);
    println!(
        "{}",
        c(
            DIM,
            &arow(
                arm_w,
                "arm",
                "n",
                "FULL",
                "med$",
                "medTok",
                "medTime",
                "calls min–med–max",
                "mono%"
            )
        )
    );
    for env in &envs {
        sub(&env.to_uppercase(), width);
        // tool arms within this CLI/model/effort environment
        let mut arm_keys: Vec<String> = vec![];
        for r in &ok {
            let a = parse_arm(&r.label);
            let key = arm_display(&a);
            if env_name(&a.cli, &a.model, &a.effort) == *env && !arm_keys.contains(&key) {
                arm_keys.push(key);
            }
        }
        arm_keys.sort_by(|a, b| {
            let ba = if a.starts_with("baseline") { 0 } else { 1 };
            let bb = if b.starts_with("baseline") { 0 } else { 1 };
            ba.cmp(&bb).then(a.cmp(b))
        });
        for arm in arm_keys {
            let v: Vec<&&RunStats> = ok
                .iter()
                .filter(|r| {
                    let a = parse_arm(&r.label);
                    env_name(&a.cli, &a.model, &a.effort) == *env && arm_display(&a) == arm
                })
                .collect();
            let a0 = parse_arm(&v[0].label);
            let is_tool = a0.tool != "baseline";
            let name = arm_display(&a0);
            let measured: Vec<&&RunStats> =
                v.iter().filter(|r| r.calls.is_some()).copied().collect();
            let unused = if is_tool {
                measured.iter().filter(|r| r.adopt == 0).count()
            } else {
                0
            };
            let full = v.iter().filter(|r| r.grade == "FULL").count();
            let cs: Vec<i64> = measured.iter().map(|r| r.calls.unwrap()).collect();
            let callstr = if cs.is_empty() {
                "—".to_string()
            } else {
                format!(
                    "{}–{}–{}",
                    cs.iter().min().unwrap(),
                    median_i(&cs),
                    cs.iter().max().unwrap()
                )
            };
            let suma: i64 = measured.iter().map(|r| r.adopt).sum();
            let sumc: i64 = cs.iter().sum();
            let monostr = if !is_tool {
                "0%".into()
            } else {
                format!(
                    "{}%",
                    if sumc > 0 {
                        (100.0 * suma as f64 / sumc as f64).round() as i64
                    } else {
                        0
                    }
                )
            };
            let costs = v
                .iter()
                .filter(|r| r.cost_available)
                .map(|r| r.cost)
                .collect::<Vec<_>>();
            let toks = v
                .iter()
                .filter(|r| r.tokens_available)
                .map(|r| r.tok)
                .collect::<Vec<_>>();
            let medc = if costs.is_empty() {
                "—".into()
            } else {
                format!("${:.2}", median_f(&costs))
            };
            let medt = if toks.is_empty() {
                "—".into()
            } else {
                hum_tok(median_i(&toks))
            };
            let medtime = median_i(&v.iter().map(|r| r.time).collect::<Vec<_>>());
            let mut row = arow(
                arm_w,
                &name,
                &v.len().to_string(),
                &format!("{}/{}", full, v.len()),
                &medc,
                &medt,
                &format!("{}s", medtime),
                &callstr,
                &monostr,
            );
            if unused > 0 {
                row.push_str(&c(DIM, &format!("  ⚠{} never called the tool", unused)));
            }
            println!("{}", c(arm_code(&a0.tool), &row));
        }
    }

    // ── failures ──
    if !fail.is_empty() {
        major("FAILURES & INCOMPLETE  (excluded from medians)", width);
        let mut fs = fail.clone();
        fs.sort_by(|a, b| {
            label_start_ms(&a.label)
                .unwrap_or(0)
                .cmp(&label_start_ms(&b.label).unwrap_or(0))
                .then_with(|| a.label.cmp(&b.label))
        });
        for r in fs {
            let why = match r.grade.as_str() {
                "FORFEIT" => "could not index repo (OOM)",
                "INVALID" => "instance still has TODO/provisional grading metadata",
                _ => "incomplete / over budget",
            };
            let tool = parse_arm(&r.label).tool;
            println!(
                "{}{}",
                c(
                    arm_code(&tool),
                    &format!(
                        "  {} {} {}",
                        pad_end_fit(&r.label, run_label_w),
                        pad_start_fit(&start_utc(&r.label), 12),
                        pad_end_fit(&r.grade, 10)
                    )
                ),
                c(DIM, &format!("— {}", why))
            );
        }
    }

    println!("\n{}", c(HEAD, "[NEXT]"));
    if pending_review > 0 {
        println!(
            "{}",
            c(
                DIM,
                &format!("  {pending_review} run(s) need final review: monobench judge {id} <run> --model <judge-model>")
            )
        );
    }
    println!(
        "{}",
        c(
            DIM,
            &format!("  monobench adoption {id}   # tool-call breakdown")
        )
    );
    println!(
        "{}",
        c(
            DIM,
            &format!("  monobench show {id} --spoil   # answer key")
        )
    );
    println!(
        "{}",
        c(
            DIM,
            "\nmono% = monogram share of tool calls · sampling n=1 → 3 → 5/9 (SPEC §6)"
        )
    );
}

/// Cross-instance leaderboard: root-cause FULL hit-rate per arm × instance (failures excluded).
pub fn summary(insts: &[(String, Vec<RunStats>)]) {
    let heavy = "═".repeat(MIN_W);
    println!("\n{}", c(HEAD, &heavy));
    println!(
        "{}",
        c(
            HEAD,
            "SUMMARY  ·  root-cause Hit-rate (FULL) per arm × instance  (failures excluded)"
        )
    );
    println!("{}", c(HEAD, &heavy));
    if insts.iter().all(|(_, r)| r.is_empty()) {
        println!("(no runs in any instance — run `monobench matrix <id> …`)");
        return;
    }
    let pending_review: usize = insts
        .iter()
        .map(|(_, runs)| runs.iter().filter(|r| !r.final_checked).count())
        .sum();
    let gradeable =
        |r: &&RunStats| r.grade != "FORFEIT" && r.grade != "NO_RESULT" && r.grade != "INVALID";

    // unique arms across all instances, ordered (baseline-first, then CLI/model order, then alpha)
    let mut arms: Vec<(String, String, String, String, String)> = vec![]; // (display, tool, cli, model, effort)
    for (_, runs) in insts {
        for r in runs.iter().filter(gradeable) {
            let a = parse_arm(&r.label);
            let d = full_arm_name(&a.tool, &a.version, &a.cli, &a.model, &a.effort);
            if !arms.iter().any(|(x, _, _, _, _)| x == &d) {
                arms.push((d, a.tool, a.cli, a.model, a.effort));
            }
        }
    }
    arms.sort_by(|a, b| {
        (if a.1 == "baseline" { 0 } else { 1 })
            .cmp(&(if b.1 == "baseline" { 0 } else { 1 }))
            .then(env_ord(&a.2, &a.3, &a.4).cmp(&env_ord(&b.2, &b.3, &b.4)))
            .then(a.0.cmp(&b.0))
    });

    let cellv = |disp: &str, runs: &[RunStats]| -> (usize, usize) {
        let mut full = 0;
        let mut n = 0;
        for r in runs.iter().filter(gradeable) {
            let a = parse_arm(&r.label);
            if full_arm_name(&a.tool, &a.version, &a.cli, &a.model, &a.effort) == disp {
                n += 1;
                if r.grade == "FULL" {
                    full += 1;
                }
            }
        }
        (full, n)
    };
    let cell_times = |disp: &str, runs: &[RunStats]| -> Vec<i64> {
        runs.iter()
            .filter(gradeable)
            .filter_map(|r| {
                let a = parse_arm(&r.label);
                if full_arm_name(&a.tool, &a.version, &a.cli, &a.model, &a.effort) == disp
                    && r.time > 0
                {
                    Some(r.time)
                } else {
                    None
                }
            })
            .collect()
    };

    let mut hdr = format!("  {}", pad_end("arm", 20));
    for (id, _) in insts {
        hdr.push_str(&format!(
            " {}",
            pad_start(&id.chars().take(13).collect::<String>(), 13)
        ));
    }
    hdr.push_str(&format!(" {}", pad_start("overall", 9)));
    println!("{}", c(DIM, &hdr));
    for (disp, tool, _, _, _) in &arms {
        let mut row = format!("  {}", pad_end(disp, 20));
        let (mut tf, mut tn) = (0, 0);
        for (_, runs) in insts {
            let (f, n) = cellv(disp, runs);
            tf += f;
            tn += n;
            row.push_str(&format!(
                " {}",
                pad_start(
                    &(if n == 0 {
                        "–".into()
                    } else {
                        format!("{f}/{n}")
                    }),
                    13
                )
            ));
        }
        row.push_str(&format!(" {}", pad_start(&format!("{tf}/{tn}"), 9)));
        println!("{}", c(arm_code(tool), &row));
    }

    println!(
        "\n{}",
        c(
            HEAD,
            "TIMING  ·  median wall time per arm × instance  (seconds)"
        )
    );
    println!("{}", c(DIM, &hdr));
    for (disp, tool, _, _, _) in &arms {
        let mut row = format!("  {}", pad_end(disp, 20));
        let mut all_times: Vec<i64> = vec![];
        for (_, runs) in insts {
            let times = cell_times(disp, runs);
            all_times.extend(times.iter().copied());
            row.push_str(&format!(
                " {}",
                pad_start(
                    &(if times.is_empty() {
                        "–".into()
                    } else {
                        format!("{}s", median_i(&times))
                    }),
                    13
                )
            ));
        }
        row.push_str(&format!(
            " {}",
            pad_start(
                &(if all_times.is_empty() {
                    "–".into()
                } else {
                    format!("{}s", median_i(&all_times))
                }),
                9
            )
        ));
        println!("{}", c(arm_code(tool), &row));
    }
    println!(
        "{}",
        c(
            DIM,
            &format!(
                "\ninstances: {}",
                insts
                    .iter()
                    .map(|(id, _)| id.as_str())
                    .collect::<Vec<_>>()
                    .join(" · ")
            )
        )
    );
    println!("{}", c(DIM, "per-instance detail: monobench report <id>"));
    if pending_review > 0 {
        println!(
            "{}",
            c(
                DIM,
                &format!(
                    "review: {pending_review} run(s) are auto-only; final leaderboard should use judged runs"
                )
            )
        );
    }
}

/// Short fixed-width grade label for the `column` table header.
fn short_grade(g: &str) -> &str {
    match g {
        "NAME_ONLY" => "NAME",
        "INVALID" => "INVL",
        "NO_RESULT" => "NORS",
        "FORFEIT" => "FRFT",
        x => x,
    }
}

/// One arm's verified grade column across every instance — the judged detail behind `summary`.
/// Disk-truth via the same gradeable gather as `report` (answer-less crashes are excluded, so
/// `n` here is answered runs; the FULL/MISS/DECOY/NAME_ONLY/INVALID split + review coverage are
/// exactly the hand tally this replaces). `arm` is a full arm name, e.g.
/// `baseline-codex-gpt-5.4-mini-low`.
pub fn column(arm_query: &str, insts: &[(String, Vec<RunStats>)]) {
    let order = [
        "FULL",
        "NAME_ONLY",
        "DECOY",
        "MISS",
        "INVALID",
        "NO_RESULT",
        "FORFEIT",
    ];

    // single pass: per-instance counts per grade, plus n and judged
    let mut per: Vec<(String, [usize; 7], usize, usize)> = vec![];
    let mut tot = [0usize; 7];
    let (mut tn, mut tjudged) = (0usize, 0usize);
    let mut arm_tool = String::new();
    for (id, runs) in insts {
        let mut cs = [0usize; 7];
        let (mut n, mut judged) = (0usize, 0usize);
        for r in runs {
            let a = parse_arm(&r.label);
            if a.arm != arm_query {
                continue;
            }
            n += 1;
            if r.final_checked {
                judged += 1;
            }
            if arm_tool.is_empty() {
                arm_tool = a.tool.clone();
            }
            if let Some(i) = order.iter().position(|g| *g == r.grade) {
                cs[i] += 1;
                tot[i] += 1;
            }
        }
        if n > 0 {
            per.push((id.clone(), cs, n, judged));
            tn += n;
            tjudged += judged;
        }
    }

    let heavy = "═".repeat(MIN_W);
    println!("\n{}", c(HEAD, &heavy));
    println!(
        "{}",
        c(
            HEAD,
            &format!("COLUMN  ·  {arm_query}  ·  verified grade × instance")
        )
    );
    println!("{}", c(HEAD, &heavy));

    if tn == 0 {
        println!("(no gradeable runs match arm '{arm_query}')");
        let mut arms: Vec<String> = vec![];
        for (_, runs) in insts {
            for r in runs {
                let a = parse_arm(&r.label).arm;
                if !arms.contains(&a) {
                    arms.push(a);
                }
            }
        }
        arms.sort();
        if !arms.is_empty() {
            println!("  known arms:");
            for a in &arms {
                println!("    {a}");
            }
        }
        println!("\n[NEXT]");
        println!("  monobench summary                     # all arms × instance leaderboard");
        return;
    }

    let cols: Vec<usize> = (0..order.len()).filter(|&i| tot[i] > 0).collect();
    let mut hdr = format!("  {}", pad_end("instance", 30));
    for &i in &cols {
        hdr.push_str(&format!(" {}", pad_start(short_grade(order[i]), 5)));
    }
    hdr.push_str(&format!(
        " {} {}",
        pad_start("n", 3),
        pad_start("judged", 7)
    ));
    println!("{}", c(DIM, &hdr));

    for (id, cs, n, judged) in &per {
        let mut row = format!(
            "  {}",
            pad_end(&id.chars().take(30).collect::<String>(), 30)
        );
        for &i in &cols {
            let v = cs[i];
            row.push_str(&format!(
                " {}",
                pad_start(&(if v == 0 { ".".into() } else { v.to_string() }), 5)
            ));
        }
        row.push_str(&format!(
            " {} {}",
            pad_start(&n.to_string(), 3),
            pad_start(&format!("{judged}/{n}"), 7)
        ));
        println!("{}", c(arm_code(&arm_tool), &row));
    }

    println!("  {}", c(DIM, &"─".repeat(MIN_W - 2)));
    let mut trow = format!("  {}", pad_end("TOTAL", 30));
    for &i in &cols {
        trow.push_str(&format!(" {}", pad_start(&tot[i].to_string(), 5)));
    }
    trow.push_str(&format!(
        " {} {}",
        pad_start(&tn.to_string(), 3),
        pad_start(&format!("{tjudged}/{tn}"), 7)
    ));
    println!("{}", c(HEAD, &trow));

    let full = tot[0];
    let failures = tot[4] + tot[5] + tot[6]; // INVALID + NO_RESULT + FORFEIT
    let gradeable = tn - failures;
    let pct = if gradeable > 0 {
        full * 100 / gradeable
    } else {
        0
    };
    println!();
    println!(
        "  root-cause FULL hit-rate: {full}/{gradeable} = {pct}%   (failures excluded: {failures})"
    );
    println!(
        "  review coverage: {tjudged}/{tn} judged · {} unreviewed   (answer-less crashes excluded; see `monobench integrity`)",
        tn - tjudged
    );
    println!("\n[NEXT]");
    println!("  monobench report <id>                 # all arms for one instance");
    println!("  monobench judge <id> <run>            # re-judge one run in-context (you decide; --judge-model is just a provenance label)");
    println!("  monobench summary                     # full arm × instance leaderboard");
}
