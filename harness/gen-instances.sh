#!/usr/bin/env bash
# monobench instance scaffolder.
# Prefills the MECHANICAL fields of an instance from a merged GitHub fix-PR and leaves the JUDGMENT
# fields (root_cause_fn, decoy, grading keys, the de-keyworded symptom) as TODO. This makes
# "expand the test count" a one-row edit to instances/backlog.tsv.
#
#   usage: harness/gen-instances.sh [manifest]        # default: instances/backlog.tsv
#   manifest rows (pipe-delimited):  id | owner/repo | pr | langs(csv) | category
#
# Already-built instances (those whose instance.json exists) are skipped, never clobbered.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="${1:-$ROOT/backlog.tsv}"
command -v jq >/dev/null 2>&1 || { echo "FATAL: jq required"; exit 1; }
command -v gh >/dev/null 2>&1 || { echo "FATAL: gh required"; exit 1; }
gen=0; skip=0; fail=0
while IFS='|' read -r id repo pr langs category _; do
  id="${id// /}"
  [[ -z "$id" || "$id" == \#* ]] && continue
  dir="$ROOT/instances/$id"
  if [[ -e "$dir/instance.json" ]]; then echo "skip   $id (exists)"; skip=$((skip+1)); continue; fi
  meta=$(gh api "repos/$repo/pulls/$pr" 2>/dev/null)
  merge=$(jq -r '.merge_commit_sha // empty' <<<"${meta:-}" 2>/dev/null)
  if [[ -z "$merge" || "$merge" == "null" ]]; then echo "FAIL   $id ($repo#$pr not merged / not found)"; fail=$((fail+1)); continue; fi
  base=$(gh api "repos/$repo/commits/$merge" --jq '.parents[0].sha' 2>/dev/null)
  title=$(jq -r '.title' <<<"$meta")
  body=$(jq -r '.body // ""' <<<"$meta")
  merged=$(jq -r '(.merged_at // "")[0:10]' <<<"$meta")
  files=$(gh api "repos/$repo/pulls/$pr/files" --paginate --jq '[.[]|select(.filename|test("(test|spec|fixture|__tests__|\\.test\\.|\\.spec\\.)";"i")|not)|.filename]' 2>/dev/null)
  [[ -z "$files" || "$files" == "[]" ]] && files=$(gh api "repos/$repo/pulls/$pr/files" --jq '[.[].filename]' 2>/dev/null)
  [[ -z "$files" ]] && files="[]"
  rootfile=$(jq -r '.[0] // "TODO"' <<<"$files" 2>/dev/null)
  langjson=$(jq -cn --arg l "$langs" '$l|split(",")')
  mkdir -p "$dir"
  if ! jq -n --arg id "$id" --arg title "$title" --arg repo "https://github.com/$repo" \
        --arg base "${base:0:12}" --argjson langs "$langjson" --arg cat "$category" \
        --argjson files "$files" --arg rootfile "$rootfile" \
        --argjson fixpr "$pr" --arg fix "${merge:0:12}" --arg merged "$merged" --arg repofull "$repo" '
    {id:$id,title:$title,repo:$repo,tag:$base,base_commit:$base,languages:$langs,category:$cat,
     crash_site:"TODO — where the crash is OBSERVED (must be a DIFFERENT fn than the cause)",
     buggy_files_to_verify:$files,
     ground_truth:{root_cause_file:$rootfile,root_cause_fn:"TODO",
       decoy_fn:"TODO — adjacent plausible-but-wrong fn (often the crash site itself)",
       fix_pr:$fixpr,fix_commit:$fix,fix_summary:"TODO — see ground_truth.md"},
     grading:{full_must_name:["TODO"],mechanism_keywords:["TODO"],decoy_markers:["TODO"]},
     admission:{note:"PROVISIONAL scaffold. Author TODO before running: hide the API in symptom.md, verify crash-fn≠cause-fn, fill grading keys, run the thin arm.",baseline_full_hit_rate_max:0.5},
     provenance:("https://github.com/"+$repofull+"/pull/"+($fixpr|tostring)+" — "+$title+" (merged "+$merged+"); base "+$base+" = merge first-parent.")}' \
    > "$dir/instance.json"; then echo "FAIL   $id (json build)"; fail=$((fail+1)); continue; fi
  { echo "TODO — author the symptom. RULE: do NOT name the buggy subsystem/API (it greps straight to the cause).";
    echo "Give only the crash trace + observable behavior; make the agent infer the subsystem.";
    echo; echo "(reference, do NOT paste verbatim — PR title: $title)"; echo;
    echo "End your reply with exactly:"; echo "ROOTCAUSE: <file path>::<function>"; echo "FIX: <one sentence>"; } > "$dir/symptom.md"
  { echo "# Ground truth — SPOILER (the real fix PR — author the instance FROM this, never feed to the solver)";
    echo; echo "**$repo PR #$pr** — $title"; echo;
    echo "**fix commit:** ${merge:0:12} · **base (merge^):** ${base:0:12} · merged $merged"; echo;
    echo "## Changed source files (test/fixture files filtered out)"; jq -r '.[]' <<<"$files" 2>/dev/null | sed 's/^/- /'; echo;
    echo "## PR body"; echo; echo "$body"; } > "$dir/ground_truth.md"
  echo "gen    $id  ->  root_file=$rootfile  base=${base:0:12}"; gen=$((gen+1))
done < "$MANIFEST"
echo "---- generated=$gen  skipped=$skip  failed=$fail ----"
