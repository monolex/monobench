// Embed the problem set (instances/) + harness adapters (harness/) into the binary so an INSTALLED
// monobench is self-contained: `monobench list` works from any cwd with no separate data download.
// Dev runs from the repo still prefer the live on-disk dirs (see find_root in main.rs); this embedded
// copy is the fallback, extracted to ~/.monobench/<ver>-<build_id> for installed / standalone binaries.
use std::{env, fs, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut entries = String::new();
    for dir in ["instances", "harness", "initiate"] {
        let root = Path::new(&manifest).join(dir);
        if root.is_dir() { walk(&root, Path::new(&manifest), &mut entries); }
        println!("cargo:rerun-if-changed={dir}");
    }
    println!("cargo:rerun-if-env-changed=MONOBENCH_BUILD_ID");
    let build_id = env::var("MONOBENCH_BUILD_ID").ok().and_then(|s| s.parse().ok())
        .unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
    let code = format!(
        "pub const BUILD_ID: u64 = {build_id};\npub static EMBEDDED: &[(&str, &[u8])] = &[\n{entries}];\n"
    );
    fs::write(Path::new(&out_dir).join("embedded.rs"), code).unwrap();
}

fn walk(dir: &Path, base: &Path, out: &mut String) {
    let mut items: Vec<PathBuf> = fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
    items.sort();
    for path in items {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
        // Skip OS cruft and regenerated run artifacts — ship only the problem definitions + adapters.
        if name == ".DS_Store" || name == "results" || name.ends_with(".jsonl") || name.ends_with(".err") { continue; }
        if path.is_dir() {
            walk(&path, base, out);
        } else {
            let rel = path.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/");
            out.push_str(&format!("    ({:?}, include_bytes!({:?})),\n", rel, path.to_string_lossy()));
        }
    }
}
