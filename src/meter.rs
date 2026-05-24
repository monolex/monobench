// monobench — per-run token + CACHE breakdown for a niia-driven model CLI run (reads a claude session
// JSONL, sums usage incl. cache). Emits the JSON shape grade_text_file consumes.
use serde_json::Value;

/// Parse an ISO-8601 timestamp (YYYY-MM-DDThh:mm:ss…) to epoch seconds (civil-days algorithm; no dep).
fn iso_to_secs(s: &str) -> Option<i64> {
    if s.len() < 19 {
        return None;
    }
    let p = |a: usize, z: usize| -> Option<i64> { s.get(a..z)?.parse().ok() };
    let (mut y, mo, d, h, mi, se) = (
        p(0, 4)?,
        p(5, 7)?,
        p(8, 10)?,
        p(11, 13)?,
        p(14, 16)?,
        p(17, 19)?,
    );
    if mo <= 2 {
        y -= 1;
    }
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if mo > 2 { mo - 3 } else { mo + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days * 86400 + h * 3600 + mi * 60 + se)
}

pub fn meter_json(path: &str) -> Value {
    let sid = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".jsonl");
    let (mut input, mut output, mut cr, mut cc, mut msgs) = (0i64, 0i64, 0i64, 0i64, 0i64);
    let mut model = String::new();
    let (mut tmin, mut tmax) = (i64::MAX, i64::MIN);
    if let Ok(s) = std::fs::read_to_string(path) {
        for l in s.lines().filter(|l| !l.trim().is_empty()) {
            let Ok(j) = serde_json::from_str::<Value>(l) else {
                continue;
            };
            if let Some(u) = j.pointer("/message/usage").or_else(|| j.get("usage")) {
                msgs += 1;
                let gi = |k: &str| u.get(k).and_then(Value::as_i64).unwrap_or(0);
                input += gi("input_tokens");
                output += gi("output_tokens");
                cr += gi("cache_read_input_tokens");
                cc += gi("cache_creation_input_tokens");
            }
            if let Some(m) = j.pointer("/message/model").and_then(Value::as_str) {
                model = m.into();
            }
            if let Some(ts) = j.get("timestamp").and_then(Value::as_str) {
                if let Some(e) = iso_to_secs(ts) {
                    tmin = tmin.min(e);
                    tmax = tmax.max(e);
                }
            }
        }
    }
    let total = input + output + cr + cc;
    let denom = cr + input + cc;
    let cache_hit = if denom > 0 {
        (1000.0 * cr as f64 / denom as f64).round() / 10.0
    } else {
        0.0
    };
    let dur: Option<i64> = if tmax > tmin { Some(tmax - tmin) } else { None };
    serde_json::json!({
        "session_id": sid, "model": model, "msgs": msgs,
        "tokens": total, "input": input, "output": output, "cache_read": cr, "cache_creation": cc,
        "cache_hit_pct": cache_hit, "duration_s": dur
    })
}

pub fn meter(path: &str) {
    println!("{}", meter_json(path));
}
