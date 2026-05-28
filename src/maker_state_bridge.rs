//! Maker pipeline state bridge — adapts lib-niia-core's SMPC injection primitives to the
//! monogram-audit maker pipeline.
//!
//! Uses the SAME primitive that niia flow's consciousness bridge uses: classify per-layer
//! chaos scores into SMPC states (Chaos / Part / Managed / Simple), look up the next maker
//! action per (layer, state), surface compound multi-layer patterns.
//!
//! Niia flow's domain = monoflow 5 layers (css / documentation / structural / rhythm / temporal).
//! Maker pipeline's domain = 5 layers (below). Same primitive, different mapping table.
//!
//! # Layers
//!
//! - `lexical_semantic_drift` — lexical score ≠ semantic intent
//!     (e.g., cleanup-helper-over-owner, library-noise-over-app, query-echoes-region-name)
//! - `scope_calibration` — reached neighborhood, but calibration of root vs nearby-wrong-root
//! - `discovery_path` — whether the path closed at all for the symptom-aligned query
//! - `proof_locality` — whether proof stays near the same file/symbol/operation anchor
//! - `confidence_amplification` — top_region_lock / score-separated false confidence
//!
//! Each layer's score is normalized 0.0–1.0 (1.0 = max managed/simple, 0.0 = max chaos).
//!
//! # Scoring
//!
//! `compute_maker_state` reads per-run grade stats plus the structured `monogram-audit`
//! pattern summary and returns a 5-tuple of scores.

use lib_niia_core::{
    bridge::{find_payload, matching_compounds, CompoundPattern, LayerStateMapping},
    smpc::{smpc_state, SmpcState},
};

use crate::grade::RunStats;
use crate::monogram_audit::MonogramAuditSummary;

const QUERY_LEXICAL_DRIFT: &str = "boundary connection order";
const QUERY_SCOPE_BUDGET: &str = "fanout boundary chaos formula";
const QUERY_DISCOVERY_PATH: &str = "path close search within";
const QUERY_PROOF_LOCALITY: &str = "proof mirror resonance";
const QUERY_ANCHOR_PRESERVATION: &str = "anchor preservation mirror";
const QUERY_CONFIDENCE_UNCERTAINTY: &str = "unknown confidence chaos notation";
const QUERY_CONFIDENCE_EXPLICIT: &str = "explicit uncertainty notation";
const QUERY_SIMPLE_COMPRESSION: &str = "output compression achieved";
const QUERY_ANTI_OVERFIT: &str = "anti overfit boundary chaos";
const QUERY_SOURCE_EVIDENCE: &str = "source evidence boundary";
const QUERY_TRANSPORT_BRIDGE: &str = "same message bridge query";

const LAYER_LEXICAL_SEMANTIC_DRIFT: &str = "lexical_semantic_drift";
const LAYER_SCOPE_CALIBRATION: &str = "scope_calibration";
const LAYER_DISCOVERY_PATH: &str = "discovery_path";
const LAYER_PROOF_LOCALITY: &str = "proof_locality";
const LAYER_CONFIDENCE_AMPLIFICATION: &str = "confidence_amplification";
const LAYER_QUERY_TRANSPORT: &str = "query_transport";
const LAYER_DELIVERY_PARITY: &str = "delivery_parity";

#[derive(Debug, Clone, Copy)]
struct MakerLayerSpec {
    maker_layer: &'static str,
    monogram_layer: Option<&'static str>,
    role: &'static str,
}

const MAKER_LAYER_SPECS: &[MakerLayerSpec] = &[
    MakerLayerSpec {
        maker_layer: LAYER_LEXICAL_SEMANTIC_DRIFT,
        monogram_layer: None,
        role: "maker-only lexical/domain drift",
    },
    MakerLayerSpec {
        maker_layer: LAYER_SCOPE_CALIBRATION,
        monogram_layer: Some("scope_budget"),
        role: "score layer",
    },
    MakerLayerSpec {
        maker_layer: LAYER_DISCOVERY_PATH,
        monogram_layer: Some("discovery_path"),
        role: "score layer",
    },
    MakerLayerSpec {
        maker_layer: LAYER_PROOF_LOCALITY,
        monogram_layer: Some("proof_locality"),
        role: "score layer",
    },
    MakerLayerSpec {
        maker_layer: LAYER_CONFIDENCE_AMPLIFICATION,
        monogram_layer: Some("confidence_calibration"),
        role: "score layer",
    },
    MakerLayerSpec {
        maker_layer: LAYER_QUERY_TRANSPORT,
        monogram_layer: None,
        role: "diagnostic pressure only",
    },
    MakerLayerSpec {
        maker_layer: LAYER_DELIVERY_PARITY,
        monogram_layer: Some("delivery_parity"),
        role: "diagnostic pressure only",
    },
];

#[derive(Debug, Clone, Copy)]
struct PatternPressureRule {
    key: &'static str,
    maker_layer: &'static str,
    family: &'static str,
    signals: &'static [&'static str],
    weight: f64,
    affects_score: bool,
    query_key: &'static str,
    basis: &'static str,
}

const PATTERN_PRESSURE_RULES: &[PatternPressureRule] = &[
    PatternPressureRule {
        key: "closed_candidate_space",
        maker_layer: LAYER_LEXICAL_SEMANTIC_DRIFT,
        family: "root-selection",
        signals: &[
            "closed_candidate_space_but_wrong_root",
            "region_contrast_lock_unresolved",
        ],
        weight: 0.35,
        affects_score: true,
        query_key: "coupling_boundary",
        basis: "closed candidate space but wrong root maps to lexical/domain drift",
    },
    PatternPressureRule {
        key: "scope_budget_pressure",
        maker_layer: LAYER_SCOPE_CALIBRATION,
        family: "budget",
        signals: &[
            "broad_output_or_fanout_loop",
            "source_promotion_review_required",
        ],
        weight: 0.70,
        affects_score: true,
        query_key: "scope_budget",
        basis: "broad output and source-promotion pressure reduce scope calibration",
    },
    PatternPressureRule {
        key: "guarded_recovery_pressure",
        maker_layer: LAYER_DISCOVERY_PATH,
        family: "discovery",
        signals: &["guarded_no_match_recovery_pressure"],
        weight: 0.60,
        affects_score: true,
        query_key: "discovery_path",
        basis: "guarded no-match recovery means path closure remains partial",
    },
    PatternPressureRule {
        key: "proof_anchor_pressure",
        maker_layer: LAYER_PROOF_LOCALITY,
        family: "proof",
        signals: &[
            "lifecycle_proof_unresolved",
            "region_query_anchor_drift",
            "rootcause_label_guard_pivot",
        ],
        weight: 0.65,
        affects_score: true,
        query_key: "proof_locality",
        basis: "lifecycle proof and anchor drift reduce proof locality",
    },
    PatternPressureRule {
        key: "confidence_pressure",
        maker_layer: LAYER_CONFIDENCE_AMPLIFICATION,
        family: "confidence",
        signals: &[
            "closed_candidate_space_but_wrong_root",
            "region_contrast_lock_unresolved",
            "rootcause_label_guard_ignored",
        ],
        weight: 0.55,
        affects_score: true,
        query_key: "confidence_calibration",
        basis: "wrong-root close candidates and ignored guards reduce confidence calibration",
    },
    PatternPressureRule {
        key: "query_transport_pressure",
        maker_layer: LAYER_QUERY_TRANSPORT,
        family: "transport",
        signals: &[
            "regex_alternation_query",
            "shell_post_filter_pipeline",
            "shell_file_search_fallback",
            "git_denied_fallback",
            "query_pipe_marker",
            "pipe_query_redirect",
        ],
        weight: 0.0,
        affects_score: false,
        query_key: "query_transport",
        basis:
            "query syntax, shell fallback, and denied git fallback are tracked before becoming a score layer",
    },
    PatternPressureRule {
        key: "delivery_parity_pressure",
        maker_layer: LAYER_DELIVERY_PARITY,
        family: "delivery",
        signals: &[
            "json_without_next_hint",
            "oversized_json_without_next_hint",
            "harness_db_mismatch",
        ],
        weight: 0.0,
        affects_score: false,
        query_key: "delivery_parity",
        basis: "text/json/harness drift is diagnostic until parity rules are validated",
    },
];

/// Payload for one (layer, state) maker action.
#[derive(Debug, Clone, Copy)]
pub struct MakerAction {
    /// Optional consciousness query for `niia consciousness "..."` echo (orchestrator-only,
    /// never fed to solver prompts).
    pub consciousness_query: &'static str,
    /// What the maker should do next at this (layer, state).
    pub action_text: &'static str,
    /// Brief description of the audit-gate check to apply to any proposed signal here.
    pub audit_check: &'static str,
}

/// Maker-layer state vector: (lex_drift, scope, discovery, proof, confidence).
/// Each score is in 0.0..=1.0.
pub type MakerStateVec = (f64, f64, f64, f64, f64);

pub const MAKER_NEXT_ACTION_MAP: &[LayerStateMapping<MakerAction>] = &[
    // ─── lexical_semantic_drift ──────────────────────────────────────────────
    LayerStateMapping {
        layer: LAYER_LEXICAL_SEMANTIC_DRIFT,
        state: SmpcState::Chaos,
        payload: MakerAction {
            consciousness_query: QUERY_ANTI_OVERFIT,
            action_text: "Many isolated proximate-vs-root misses without group. Cluster failures by (query-vocab vs region-body-vocab) signature.",
            audit_check: "no benchmark literals in cluster descriptors",
        },
    },
    LayerStateMapping {
        layer: LAYER_LEXICAL_SEMANTIC_DRIFT,
        state: SmpcState::Part,
        payload: MakerAction {
            consciousness_query: QUERY_LEXICAL_DRIFT,
            action_text: "Sub-mechanism partially observed (owner-vs-helper / vendored-noise / identifier-echo). Identify the dominant sub-pattern.",
            audit_check: "sub-pattern names use general predicates, not answer-key symbols",
        },
    },
    LayerStateMapping {
        layer: LAYER_LEXICAL_SEMANTIC_DRIFT,
        state: SmpcState::Managed,
        payload: MakerAction {
            consciousness_query: QUERY_LEXICAL_DRIFT,
            action_text: "Sub-pattern named and recurring across >=2 instances. Design generalizable signal (^on[A-Z] callback / body-cleanup density / vendored path).",
            audit_check: "signal definition contains zero benchmark literals; uses pattern/density only",
        },
    },
    LayerStateMapping {
        layer: LAYER_LEXICAL_SEMANTIC_DRIFT,
        state: SmpcState::Simple,
        payload: MakerAction {
            consciousness_query: QUERY_SIMPLE_COMPRESSION,
            action_text: "Generalizable signal designed + held-out validation passed. Ready to ship as 0.61.x patch.",
            audit_check: "monogram-audit hard gate (literal-free) passes",
        },
    },
    // ─── scope_calibration ───────────────────────────────────────────────────
    LayerStateMapping {
        layer: LAYER_SCOPE_CALIBRATION,
        state: SmpcState::Chaos,
        payload: MakerAction {
            consciousness_query: QUERY_SCOPE_BUDGET,
            action_text: "Broad-fanout / oversized output dominates. Tighten output budgets before any scoring change.",
            audit_check: "budget changes affect all queries equally, not specific instances",
        },
    },
    LayerStateMapping {
        layer: LAYER_SCOPE_CALIBRATION,
        state: SmpcState::Part,
        payload: MakerAction {
            consciousness_query: QUERY_SCOPE_BUDGET,
            action_text: "Correctness is present, but fanout/filter pressure remains. Preserve the chosen file/symbol with --file, bounded context, and monogram grep before widening.",
            audit_check: "fallback narrows by current evidence and avoids shell find/grep/git history",
        },
    },
    LayerStateMapping {
        layer: LAYER_SCOPE_CALIBRATION,
        state: SmpcState::Managed,
        payload: MakerAction {
            consciousness_query: QUERY_SOURCE_EVIDENCE,
            action_text: "Scope calibrated; observe for new edge cases before adding signals.",
            audit_check: "n/a (observation only)",
        },
    },
    LayerStateMapping {
        layer: LAYER_SCOPE_CALIBRATION,
        state: SmpcState::Simple,
        payload: MakerAction {
            consciousness_query: QUERY_SIMPLE_COMPRESSION,
            action_text: "Scope-calibration layer is solved within current pattern set.",
            audit_check: "n/a",
        },
    },
    // ─── discovery_path ──────────────────────────────────────────────────────
    LayerStateMapping {
        layer: LAYER_DISCOVERY_PATH,
        state: SmpcState::Chaos,
        payload: MakerAction {
            consciousness_query: QUERY_DISCOVERY_PATH,
            action_text: "Symptom queries don't reach the answer file at all. Trace the gap: vocabulary, domain bridge, or index-language coverage.",
            audit_check: "no answer-file literals in the bridge logic",
        },
    },
    LayerStateMapping {
        layer: LAYER_DISCOVERY_PATH,
        state: SmpcState::Part,
        payload: MakerAction {
            consciousness_query: QUERY_DISCOVERY_PATH,
            action_text: "Discovery partial — answer surfaced at low rank. Consider symptom→domain vocabulary bridging (synonym/abbrev expansion).",
            audit_check: "vocabulary table is multi-instance, not single-answer-specific",
        },
    },
    LayerStateMapping {
        layer: LAYER_DISCOVERY_PATH,
        state: SmpcState::Managed,
        payload: MakerAction {
            consciousness_query: QUERY_DISCOVERY_PATH,
            action_text: "Path closes reliably for symptom queries. Hold steady.",
            audit_check: "n/a",
        },
    },
    LayerStateMapping {
        layer: LAYER_DISCOVERY_PATH,
        state: SmpcState::Simple,
        payload: MakerAction {
            consciousness_query: QUERY_SIMPLE_COMPRESSION,
            action_text: "Discovery saturated — every symptom query reaches its answer neighborhood.",
            audit_check: "n/a",
        },
    },
    // ─── proof_locality ──────────────────────────────────────────────────────
    LayerStateMapping {
        layer: LAYER_PROOF_LOCALITY,
        state: SmpcState::Chaos,
        payload: MakerAction {
            consciousness_query: QUERY_PROOF_LOCALITY,
            action_text: "Proof anchors are not staying local. Treat lifecycle/region pivots as a blocker before any score boost.",
            audit_check: "proof proposal names generic same-file/same-symbol anchors only",
        },
    },
    LayerStateMapping {
        layer: LAYER_PROOF_LOCALITY,
        state: SmpcState::Part,
        payload: MakerAction {
            consciousness_query: QUERY_ANCHOR_PRESERVATION,
            action_text: "Proof locality is partial. Preserve one symptom, file, operation, or coupling anchor across the next region/context step.",
            audit_check: "anchor preservation is measured from adjacent commands, not answer-key names",
        },
    },
    LayerStateMapping {
        layer: LAYER_PROOF_LOCALITY,
        state: SmpcState::Managed,
        payload: MakerAction {
            consciousness_query: QUERY_PROOF_LOCALITY,
            action_text: "Proof locality mostly holds. Prefer bounded contrast over widening when a guard or lifecycle marker fires.",
            audit_check: "contrast set remains small and traceable",
        },
    },
    LayerStateMapping {
        layer: LAYER_PROOF_LOCALITY,
        state: SmpcState::Simple,
        payload: MakerAction {
            consciousness_query: QUERY_PROOF_LOCALITY,
            action_text: "Proof remains local across guarded pivots. This layer can stay observation-only.",
            audit_check: "n/a",
        },
    },
    // ─── confidence_amplification ────────────────────────────────────────────
    LayerStateMapping {
        layer: LAYER_CONFIDENCE_AMPLIFICATION,
        state: SmpcState::Chaos,
        payload: MakerAction {
            consciousness_query: QUERY_CONFIDENCE_UNCERTAINTY,
            action_text: "top_region_lock firing on decoys without warning. Audit which queries trigger lock-on-decoy; downgrade lock confidence when anchor is identifier-echo.",
            audit_check: "anchor-echo detection uses generic token-overlap, no symbol literals",
        },
    },
    LayerStateMapping {
        layer: LAYER_CONFIDENCE_AMPLIFICATION,
        state: SmpcState::Part,
        payload: MakerAction {
            consciousness_query: QUERY_CONFIDENCE_EXPLICIT,
            action_text: "Contrast-lock partial. When top-2 regions are close, prefer contrast hint over lock.",
            audit_check: "contrast thresholds are score-based, not name-based",
        },
    },
    LayerStateMapping {
        layer: LAYER_CONFIDENCE_AMPLIFICATION,
        state: SmpcState::Managed,
        payload: MakerAction {
            consciousness_query: QUERY_CONFIDENCE_UNCERTAINTY,
            action_text: "Confidence amplification calibrated. Observe.",
            audit_check: "n/a",
        },
    },
    LayerStateMapping {
        layer: LAYER_CONFIDENCE_AMPLIFICATION,
        state: SmpcState::Simple,
        payload: MakerAction {
            consciousness_query: QUERY_SIMPLE_COMPRESSION,
            action_text: "Confidence hints fire only when justified by score-separation AND structural evidence.",
            audit_check: "n/a",
        },
    },
];

pub const MAKER_COMPOUND_PATTERNS: &[CompoundPattern<MakerStateVec, MakerAction>] = &[
    CompoundPattern {
        name: "CLEANUP-HELPER-OVER-OWNER-CLUSTERED",
        condition: |s: &MakerStateVec| {
            let (lex, _, _, _, _) = *s;
            lex < 0.3
        },
        payload: MakerAction {
            consciousness_query: QUERY_LEXICAL_DRIFT,
            action_text: "Multiple cases of cleanup-helper-over-owner. Prototype callback_owner_bonus + cleanup_helper_penalty as paired addition.",
            audit_check: "^on[A-Z]/^handle[A-Z] pattern is generic, not answer-shaped",
        },
    },
    CompoundPattern {
        name: "MULTI-MECHANISM-NEAR-MISS",
        condition: |s: &MakerStateVec| {
            let (lex, _, _, _, conf) = *s;
            lex < 0.5 && conf < 0.5
        },
        payload: MakerAction {
            consciousness_query: QUERY_ANTI_OVERFIT,
            action_text: "Lexical drift + confidence amplification both active. Codex's lifecycle_owner_contrast addresses this combo — observe outcome before adding more signals.",
            audit_check: "n/a (no new signal proposed)",
        },
    },
    CompoundPattern {
        name: "SIMPLE-ALL-LAYERS",
        condition: |s: &MakerStateVec| {
            let (lex, scope, disc, proof, conf) = *s;
            [lex, scope, disc, proof, conf]
                .iter()
                .filter(|&&v| v > 0.7)
                .count()
                >= 4
        },
        payload: MakerAction {
            consciousness_query: QUERY_SIMPLE_COMPRESSION,
            action_text: "Three+ maker layers in Simple state. Held-out + audit pass → ship.",
            audit_check: "monogram-audit hard gate confirms no literal regressions",
        },
    },
    CompoundPattern {
        name: "ANCHOR-DRIFT-PROOF-GAP",
        condition: |s: &MakerStateVec| {
            let (_, scope, _, proof, conf) = *s;
            proof < 0.5 && (scope < 0.6 || conf < 0.6)
        },
        payload: MakerAction {
            consciousness_query: QUERY_ANCHOR_PRESERVATION,
            action_text: "Anchor drift is active with scope/confidence pressure. Add a bounded proof-locality check before changing ranking.",
            audit_check: "adjacent commands preserve at least one non-generic anchor or mark an explicit contrast pivot",
        },
    },
];

/// Compute the 5-layer maker state vector from run grades plus audit pattern pressure.
pub fn compute_maker_state(
    stats: &[RunStats],
    audit: Option<&MonogramAuditSummary>,
) -> MakerStateVec {
    if stats.is_empty() {
        return (0.5, 0.5, 0.5, 0.5, 0.5);
    }

    let gradeable: Vec<&RunStats> = stats
        .iter()
        .filter(|s| matches!(s.grade.as_str(), "FULL" | "MISS" | "DECOY"))
        .collect();

    let total = gradeable.len() as f64;
    if total == 0.0 {
        return (0.5, 0.5, 0.5, 0.5, 0.5);
    }

    let full = gradeable.iter().filter(|s| s.grade == "FULL").count() as f64;
    let decoy = gradeable.iter().filter(|s| s.grade == "DECOY").count() as f64;

    let base = full / total;
    let decoy_rate = decoy / total;
    let Some(audit) = audit else {
        let lex_drift = (base - decoy_rate * 0.3).clamp(0.0, 1.0);
        let confidence = (base - decoy_rate * 0.5).clamp(0.0, 1.0);
        return (lex_drift, base, base, base, confidence);
    };

    let lex_drift = layer_score(
        base,
        decoy_rate * 0.3 + score_penalty(audit, LAYER_LEXICAL_SEMANTIC_DRIFT),
    );
    let scope = layer_score(base, score_penalty(audit, LAYER_SCOPE_CALIBRATION));
    let discovery = layer_score(base, score_penalty(audit, LAYER_DISCOVERY_PATH));
    let proof = layer_score(base, score_penalty(audit, LAYER_PROOF_LOCALITY));
    let confidence = layer_score(
        base,
        decoy_rate * 0.5 + score_penalty(audit, LAYER_CONFIDENCE_AMPLIFICATION),
    );

    (lex_drift, scope, discovery, proof, confidence)
}

fn signal_pressure(audit: &MonogramAuditSummary, keys: &[&str]) -> f64 {
    let count: usize = keys
        .iter()
        .map(|key| {
            audit.recommendation_signals.get(*key).copied().unwrap_or(0)
                + audit.patterns.get(*key).copied().unwrap_or(0)
                + audit.kinds.get(*key).copied().unwrap_or(0)
        })
        .sum();
    if count == 0 {
        return 0.0;
    }

    let runs = audit.runs.max(1) as f64;
    (count as f64 / runs).min(1.0)
}

fn layer_score(base: f64, penalty: f64) -> f64 {
    (base - penalty).clamp(0.0, 1.0)
}

fn score_penalty(audit: &MonogramAuditSummary, maker_layer: &str) -> f64 {
    PATTERN_PRESSURE_RULES
        .iter()
        .filter(|rule| rule.affects_score && rule.maker_layer == maker_layer)
        .map(|rule| signal_pressure(audit, rule.signals) * rule.weight)
        .sum::<f64>()
        .clamp(0.0, 1.0)
}

fn layer_spec(layer: &str) -> Option<MakerLayerSpec> {
    MAKER_LAYER_SPECS
        .iter()
        .copied()
        .find(|spec| spec.maker_layer == layer)
}

fn layer_spec_json(layer: &str) -> serde_json::Value {
    let spec = layer_spec(layer);
    serde_json::json!({
        "maker_layer": layer,
        "monogram_layer": spec.and_then(|s| s.monogram_layer),
        "role": spec.map(|s| s.role).unwrap_or("unregistered"),
    })
}

fn diagnostic_pressure_rows(audit: &MonogramAuditSummary) -> Vec<serde_json::Value> {
    PATTERN_PRESSURE_RULES
        .iter()
        .filter(|rule| !rule.affects_score)
        .map(|rule| {
            let raw_pressure = signal_pressure(audit, rule.signals);
            serde_json::json!({
                "key": rule.key,
                "maker_layer": rule.maker_layer,
                "monogram_layer": layer_spec(rule.maker_layer).and_then(|s| s.monogram_layer),
                "family": rule.family,
                "signals": rule.signals,
                "pressure": raw_pressure,
                "weight": rule.weight,
                "affects_score": rule.affects_score,
                "query_key": rule.query_key,
                "basis": rule.basis,
            })
        })
        .collect()
}

fn query_provenance(query: &str) -> (&'static str, &'static str) {
    match query {
        QUERY_LEXICAL_DRIFT => (
            "coupling_boundary",
            "07-consciousness-vocabulary-expansion: boundary connection order scored high for relation/order bridging",
        ),
        QUERY_SCOPE_BUDGET => (
            "scope_budget",
            "07-consciousness-vocabulary-expansion: fanout boundary chaos formula matched fanout/boundary pressure",
        ),
        QUERY_DISCOVERY_PATH => (
            "discovery_path",
            "07-consciousness-vocabulary-expansion: path close search within matches monogram's close-path-then-search rule",
        ),
        QUERY_PROOF_LOCALITY => (
            "proof_locality",
            "07-consciousness-vocabulary-expansion: proof mirror resonance maps proof to local mirrored evidence",
        ),
        QUERY_ANCHOR_PRESERVATION => (
            "anchor_preservation",
            "07-consciousness-vocabulary-expansion: anchor preservation mirror maps query drift to stable anchors",
        ),
        QUERY_CONFIDENCE_UNCERTAINTY => (
            "confidence_calibration",
            "07-consciousness-vocabulary-expansion: unknown confidence chaos notation was the strongest uncertainty query",
        ),
        QUERY_CONFIDENCE_EXPLICIT => (
            "explicit_uncertainty",
            "07-consciousness-vocabulary-expansion: explicit uncertainty notation maps close candidates to declared uncertainty",
        ),
        QUERY_SIMPLE_COMPRESSION => (
            "simple_compression",
            "07-consciousness-vocabulary-expansion: output compression achieved maps solved layers to compressed guidance",
        ),
        QUERY_ANTI_OVERFIT => (
            "anti_overfit_gate",
            "07-consciousness-vocabulary-expansion: anti overfit boundary chaos maps source-promotion risk to a gate",
        ),
        QUERY_SOURCE_EVIDENCE => (
            "source_evidence",
            "07-consciousness-vocabulary-expansion: source evidence boundary maps observation-only states to source proof",
        ),
        QUERY_TRANSPORT_BRIDGE => (
            "query_transport",
            "07-consciousness-vocabulary-expansion: same message bridge query maps query transport to bridge parity",
        ),
        _ => ("unclassified", "legacy query retained without explicit provenance"),
    }
}

fn print_query_metadata(action: &MakerAction) {
    let (key, basis) = query_provenance(action.consciousness_query);
    println!(
        "    @query=\"{}\" @query_key={} @basis=\"{}\"",
        action.consciousness_query, key, basis
    );
}

fn action_json(action: &MakerAction) -> serde_json::Value {
    let (query_key, basis) = query_provenance(action.consciousness_query);
    serde_json::json!({
        "action_text": action.action_text,
        "audit_check": action.audit_check,
        "consciousness_query": action.consciousness_query,
        "query_key": query_key,
        "query_basis": basis,
    })
}

pub fn maker_state_json(
    stats: &[RunStats],
    audit: Option<&MonogramAuditSummary>,
) -> serde_json::Value {
    let state_vec = compute_maker_state(stats, audit);
    let (lex, scope, disc, proof, conf) = state_vec;
    let layers = [
        (LAYER_LEXICAL_SEMANTIC_DRIFT, lex),
        (LAYER_SCOPE_CALIBRATION, scope),
        (LAYER_DISCOVERY_PATH, disc),
        (LAYER_PROOF_LOCALITY, proof),
        (LAYER_CONFIDENCE_AMPLIFICATION, conf),
    ];

    let layer_values = layers
        .iter()
        .map(|(name, score)| {
            let state = smpc_state(*score);
            let action = find_payload(name, state, MAKER_NEXT_ACTION_MAP);
            serde_json::json!({
                "layer": name,
                "layer_spec": layer_spec_json(name),
                "score": score,
                "state": state.as_str(),
                "action": action.map(action_json),
            })
        })
        .collect::<Vec<_>>();

    let compounds = matching_compounds(&state_vec, MAKER_COMPOUND_PATTERNS)
        .into_iter()
        .map(action_json)
        .collect::<Vec<_>>();

    serde_json::json!({
        "source": "lib-niia-core bridge",
        "layer_specs": MAKER_LAYER_SPECS.iter().map(|spec| {
            serde_json::json!({
                "maker_layer": spec.maker_layer,
                "monogram_layer": spec.monogram_layer,
                "role": spec.role,
            })
        }).collect::<Vec<_>>(),
        "diagnostic_pressures": audit.map(diagnostic_pressure_rows).unwrap_or_default(),
        "layers": layer_values,
        "compounds": compounds,
    })
}

/// Print the maker state report — the niia-style "(layer, state) → action" injection.
///
/// Called from monogram-audit at the end of its output.
pub fn print_maker_state_report(stats: &[RunStats], audit: Option<&MonogramAuditSummary>) {
    let state_vec = compute_maker_state(stats, audit);
    let (lex, scope, disc, proof, conf) = state_vec;

    println!();
    println!("MAKER STATE ANALYSIS (lib-niia-core bridge)");
    println!("{}", "=".repeat(70));

    let layers = [
        (LAYER_LEXICAL_SEMANTIC_DRIFT, lex),
        (LAYER_SCOPE_CALIBRATION, scope),
        (LAYER_DISCOVERY_PATH, disc),
        (LAYER_PROOF_LOCALITY, proof),
        (LAYER_CONFIDENCE_AMPLIFICATION, conf),
    ];

    for (name, score) in &layers {
        let state = smpc_state(*score);
        let filled = ((*score) * 15.0) as usize;
        let bar = "█".repeat(filled.min(15)) + &"░".repeat(15 - filled.min(15));
        let alias = layer_spec(name)
            .and_then(|spec| spec.monogram_layer)
            .map(|layer| format!(" -> {}", layer))
            .unwrap_or_default();
        println!("  {:<28} [{}] {:.3}  {}{}", name, bar, score, state, alias);

        if let Some(action) = find_payload(name, state, MAKER_NEXT_ACTION_MAP) {
            println!("    → {}", action.action_text);
            print_query_metadata(action);
        }
    }

    let compounds = matching_compounds(&state_vec, MAKER_COMPOUND_PATTERNS);
    if !compounds.is_empty() {
        println!();
        println!("COMPOUND PATTERNS:");
        for action in compounds {
            println!("  ◆ {}", action.action_text);
            print_query_metadata(action);
            println!("    audit: {}", action.audit_check);
        }
    }

    if let Some(audit) = audit {
        let diagnostics = diagnostic_pressure_rows(audit)
            .into_iter()
            .filter(|row| row["pressure"].as_f64().unwrap_or(0.0) > 0.0)
            .collect::<Vec<_>>();
        if !diagnostics.is_empty() {
            println!();
            println!("DIAGNOSTIC PRESSURES (non-scoring, adjustable)");
            for row in diagnostics {
                println!(
                    "  {:<24} pressure={:.3} query_key={} basis={}",
                    row["maker_layer"].as_str().unwrap_or("-"),
                    row["pressure"].as_f64().unwrap_or(0.0),
                    row["query_key"].as_str().unwrap_or("-"),
                    row["basis"].as_str().unwrap_or("-")
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn researched_queries_have_provenance_keys() {
        assert_eq!(query_provenance(QUERY_SCOPE_BUDGET).0, "scope_budget");
        assert_eq!(
            query_provenance(QUERY_CONFIDENCE_UNCERTAINTY).0,
            "confidence_calibration"
        );
        assert_eq!(query_provenance(QUERY_ANTI_OVERFIT).0, "anti_overfit_gate");
    }

    #[test]
    fn maker_state_json_carries_query_provenance() {
        let value = maker_state_json(&[], None);
        let first_action = &value["layers"][0]["action"];
        assert!(first_action["consciousness_query"].is_string());
        assert!(first_action["query_key"].is_string());
        assert!(first_action["query_basis"].is_string());
    }

    #[test]
    fn layer_specs_expose_monogram_aliases_and_diagnostics() {
        assert_eq!(
            layer_spec(LAYER_SCOPE_CALIBRATION).unwrap().monogram_layer,
            Some("scope_budget")
        );
        assert_eq!(
            layer_spec(LAYER_CONFIDENCE_AMPLIFICATION)
                .unwrap()
                .monogram_layer,
            Some("confidence_calibration")
        );
        assert!(
            !PATTERN_PRESSURE_RULES
                .iter()
                .find(|rule| rule.maker_layer == LAYER_QUERY_TRANSPORT)
                .unwrap()
                .affects_score
        );
        assert!(
            !PATTERN_PRESSURE_RULES
                .iter()
                .find(|rule| rule.maker_layer == LAYER_DELIVERY_PARITY)
                .unwrap()
                .affects_score
        );
    }
}
