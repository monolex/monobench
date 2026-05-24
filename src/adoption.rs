// monobench — per-run tool-call + monogram-subcommand breakdown (+ git-integrity), grouped by CLI/model.
// "Did the agent actually USE monogram — how much, how early, how well; and did it try to cheat via git?"
use crate::telemetry;
use crate::util::*;

const W: usize = 80;

struct Row {
    label: String,
    tool: String,
    cli: String,
    model: String,
    effort: String,
    total: i64,
    mono: i64,
    first: i64,
    failed: i64,
    guarded: i64,
    git: i64,
    git_denied: i64,
    share: i64,
    subs: String,
}

fn meaningless(t: &str) -> bool {
    let body: String = t
        .lines()
        .filter(|l| {
            let s = l.trim_start();
            !l.trim().is_empty() && !s.starts_with("[INFO]") && !s.starts_with("[NEXT]")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let b = body.trim();
    let bl = b.to_lowercase();
    bl.contains("no match")
        || bl.contains("not found")
        || bl.starts_with("0 results")
        || bl.contains("no results")
        || bl.contains("no symbol")
        || b.chars().count() < 3
}

/// A no-result/no-match output that still emits recovery steering (`[NEXT]` or a
/// `[marker: ...]`) is a *guarded* no-match, not a dead one — keep these out of the
/// `failed` tally so the fail count reflects real dead ends, not recovered ones.
fn is_guarded_recovery(t: &str) -> bool {
    t.contains("[NEXT]") || t.contains("[marker:")
}

/// Extract the monogram subcommand verb from a Bash `monogram <sub>` or an MCP `monogram_<sub>` tool.
fn monogram_sub(name: &str, cmd: &str) -> Option<String> {
    if name == "Bash" {
        let idx = cmd_word_pos(cmd, "monogram")?;
        let tok = cmd[idx + 8..].split_whitespace().next().unwrap_or("");
        return Some(if tok.is_empty() {
            "?".into()
        } else {
            tok.into()
        });
    }
    let nlc = name.to_lowercase();
    if let Some(i) = nlc.rfind("monogram") {
        let s = name[i + 8..].trim_start_matches(|c| c == '_' || c == '-');
        return Some(if s.is_empty() { "?".into() } else { s.into() });
    }
    None
}

pub fn adoption(id: &str, files: &[String]) {
    let mut rows: Vec<Row> = vec![];
    for f in files {
        let calls = telemetry::events_from_path(f);
        if calls.is_empty() {
            continue;
        }
        let label = telemetry::label_from_path(f);
        let (mut mono, mut first, mut failed, mut guarded, mut git, mut git_denied) =
            (0i64, -1i64, 0i64, 0i64, 0i64, 0i64);
        let mut subs_ord: Vec<(String, i64)> = vec![]; // insertion order (matches the JS object), tie-stable
        for (i, b) in calls.iter().enumerate() {
            let name = b.name.as_str();
            let cmd = b.cmd.as_str();
            if name == "Bash" && cmd_has_word(cmd, "git") {
                git += 1;
                if b.denied {
                    git_denied += 1;
                }
            }
            if let Some(sub) = monogram_sub(name, cmd) {
                mono += 1;
                if first < 0 {
                    first = i as i64 + 1;
                }
                match subs_ord.iter_mut().find(|(k, _)| *k == sub) {
                    Some(e) => e.1 += 1,
                    None => subs_ord.push((sub, 1)),
                }
                if b.denied {
                    failed += 1;
                } else if meaningless(&b.result) {
                    if is_guarded_recovery(&b.result) {
                        guarded += 1;
                    } else {
                        failed += 1;
                    }
                }
            }
        }
        let mut sarr = subs_ord;
        sarr.sort_by(|a, b| b.1.cmp(&a.1)); // stable ⇒ equal counts keep insertion order
        let mut subs = sarr
            .iter()
            .take(6)
            .map(|(k, v)| format!("{k}×{v}"))
            .collect::<Vec<_>>()
            .join("  ");
        if sarr.len() > 6 {
            subs.push_str("  …");
        }
        let total = calls.len() as i64;
        let share = if total > 0 {
            (100.0 * mono as f64 / total as f64).round() as i64
        } else {
            0
        };
        let a = parse_arm(&label);
        rows.push(Row {
            label,
            tool: a.tool,
            cli: a.cli,
            model: a.model,
            effort: a.effort,
            total,
            mono,
            first,
            failed,
            guarded,
            git,
            git_denied,
            share,
            subs,
        });
    }
    if rows.is_empty() {
        println!("(no tool-call telemetry found — need .jsonl, .err, or .agy.jsonl logs)");
        return;
    }

    println!(
        "{}{}{}",
        c(DIM, "[INFO] adoption · "),
        c("1", id),
        c(DIM, &format!(" · {} runs", rows.len()))
    );
    println!("\n{}", c(HEAD, &"═".repeat(W)));
    println!(
        "{}",
        c(
            HEAD,
            "TOOL ADOPTION  (did the agent USE monogram — how much, how early, how well)"
        )
    );
    println!("{}", c(HEAD, &"═".repeat(W)));

    let mut envs: Vec<(String, String, String, String)> = vec![];
    for r in &rows {
        let name = env_name(&r.cli, &r.model, &r.effort);
        if !envs.iter().any(|(x, _, _, _)| x == &name) {
            envs.push((name, r.cli.clone(), r.model.clone(), r.effort.clone()));
        }
    }
    envs.sort_by_key(|(_, cli, model, effort)| env_ord(cli, model, effort));
    let prow = |l: &str, t: &str, m: &str, sh: &str, fi: &str, fa: &str| {
        format!(
            "  {} {} {} {} {} {}",
            pad_end(l, 24),
            pad_start(t, 5),
            pad_start(m, 5),
            pad_start(sh, 6),
            pad_start(fi, 6),
            pad_start(fa, 6)
        )
    };
    for (env, _, _, _) in &envs {
        println!(
            "\n  {}\n  {}",
            c("1", &env.to_uppercase()),
            c(DIM, &"─".repeat(W - 2))
        );
        println!(
            "{}",
            c(
                DIM,
                &prow("run", "calls", "mono", "share", "first", "fails")
            )
        );
        let mut mrows: Vec<&Row> = rows
            .iter()
            .filter(|r| env_name(&r.cli, &r.model, &r.effort) == *env)
            .collect();
        mrows.sort_by(|a, b| a.label.cmp(&b.label));
        for x in mrows {
            let first = if x.mono > 0 {
                format!("#{}", x.first)
            } else {
                "—".into()
            };
            let fail = if x.mono > 0 {
                if x.failed > 0 {
                    format!("⚠{}", x.failed)
                } else {
                    "0".into()
                }
            } else {
                "—".into()
            };
            println!(
                "{}",
                c(
                    arm_code(&x.tool),
                    &prow(
                        &x.label,
                        &x.total.to_string(),
                        &x.mono.to_string(),
                        &format!("{}%", x.share),
                        &first,
                        &fail
                    )
                )
            );
            let gitnote = if x.git > 0 {
                let st = if x.git_denied == x.git {
                    " (all denied ✓)".to_string()
                } else if x.git_denied > 0 {
                    format!(" ({} denied)", x.git_denied)
                } else {
                    " ⚠ NOT denied".into()
                };
                format!(
                    "   · git {} attempt{}{}",
                    x.git,
                    if x.git > 1 { "s" } else { "" },
                    st
                )
            } else {
                String::new()
            };
            let guardnote = if x.guarded > 0 {
                format!("  · {}g guarded", x.guarded)
            } else {
                String::new()
            };
            let body = if x.mono > 0 {
                format!("        ↳ {}{}{}", x.subs, guardnote, gitnote)
            } else if x.tool == "baseline" {
                format!("        ↳ (control — no tool){}", gitnote)
            } else {
                format!("        ↳ (tool never called){}", gitnote)
            };
            println!("{}", c(DIM, &body));
        }
    }
    println!(
        "{}",
        c(
            DIM,
            "\nfirst = call # of first monogram use (late ⇒ grepped first)"
        )
    );
    println!("{}", c(DIM, "fails = dead no-match (denied or no marker/NEXT) · Ng = guarded no-match (recovered via marker+NEXT) · git = history-access attempts (must be denied)"));
    println!(
        "{}",
        c(
            DIM,
            "low share or late first-use ⇒ the tool was not really tested (SPEC §7)"
        )
    );
}
