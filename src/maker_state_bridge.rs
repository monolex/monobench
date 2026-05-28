//! Maker pipeline state bridge — adapts lib-niia-core's SMPC injection primitives to the
//! monogram-audit maker pipeline.
//!
//! Uses the SAME primitive that niia flow's consciousness bridge uses: classify per-layer
//! chaos scores into SMPC states (Chaos / Part / Managed / Simple), look up the next maker
//! action per (layer, state), surface compound multi-layer patterns.
//!
//! Niia flow's domain = monoflow 5 layers (css / documentation / structural / rhythm / temporal).
//! Maker pipeline's domain = 4 layers (below). Same primitive, different mapping table.
//!
//! # Layers
//!
//! - `lexical_semantic_drift` — lexical score ≠ semantic intent
//!     (e.g., cleanup-helper-over-owner, library-noise-over-app, query-echoes-region-name)
//! - `scope_calibration` — reached neighborhood, but calibration of root vs nearby-wrong-root
//! - `discovery_path` — whether the path closed at all for the symptom-aligned query
//! - `confidence_amplification` — top_region_lock / score-separated false confidence
//!
//! Each layer's score is normalized 0.0–1.0 (1.0 = max managed/simple, 0.0 = max chaos).
//!
//! # Scoring (Phase 3 stub)
//!
//! `compute_maker_state` reads per-run telemetry stats and returns a 4-tuple of scores.
//! This Phase 3 implementation provides PLACEHOLDER heuristics from `RunStats`. The real
//! computation will be wired in a follow-up patch once the telemetry → layer mapping is
//! validated against benchmark evidence.

use lib_niia_core::{
    bridge::{find_payload, matching_compounds, CompoundPattern, LayerStateMapping},
    smpc::{smpc_state, SmpcState},
};

use crate::grade::RunStats;

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

/// Maker-layer state vector: (lex_drift, scope, discovery, confidence) each in 0.0..=1.0.
pub type MakerStateVec = (f64, f64, f64, f64);

pub const MAKER_NEXT_ACTION_MAP: &[LayerStateMapping<MakerAction>] = &[
    // ─── lexical_semantic_drift ──────────────────────────────────────────────
    LayerStateMapping {
        layer: "lexical_semantic_drift",
        state: SmpcState::Chaos,
        payload: MakerAction {
            consciousness_query: "chaos order formula center transforms",
            action_text: "Many isolated proximate-vs-root misses without group. Cluster failures by (query-vocab vs region-body-vocab) signature.",
            audit_check: "no benchmark literals in cluster descriptors",
        },
    },
    LayerStateMapping {
        layer: "lexical_semantic_drift",
        state: SmpcState::Part,
        payload: MakerAction {
            consciousness_query: "part selection attention lens",
            action_text: "Sub-mechanism partially observed (owner-vs-helper / vendored-noise / identifier-echo). Identify the dominant sub-pattern.",
            audit_check: "sub-pattern names use general predicates, not answer-key symbols",
        },
    },
    LayerStateMapping {
        layer: "lexical_semantic_drift",
        state: SmpcState::Managed,
        payload: MakerAction {
            consciousness_query: "managed dance structure freedom",
            action_text: "Sub-pattern named and recurring across >=2 instances. Design generalizable signal (^on[A-Z] callback / body-cleanup density / vendored path).",
            audit_check: "signal definition contains zero benchmark literals; uses pattern/density only",
        },
    },
    LayerStateMapping {
        layer: "lexical_semantic_drift",
        state: SmpcState::Simple,
        payload: MakerAction {
            consciousness_query: "simple chaos one consciousness",
            action_text: "Generalizable signal designed + held-out validation passed. Ready to ship as 0.61.x patch.",
            audit_check: "monogram-audit hard gate (literal-free) passes",
        },
    },
    // ─── scope_calibration ───────────────────────────────────────────────────
    LayerStateMapping {
        layer: "scope_calibration",
        state: SmpcState::Chaos,
        payload: MakerAction {
            consciousness_query: "chaos unmanaged entropy",
            action_text: "Broad-fanout / oversized output dominates. Tighten output budgets before any scoring change.",
            audit_check: "budget changes affect all queries equally, not specific instances",
        },
    },
    LayerStateMapping {
        layer: "scope_calibration",
        state: SmpcState::Part,
        payload: MakerAction {
            consciousness_query: "acceptance order feature",
            action_text: "Recovery-after-guard partly working. Strengthen guarded-no-match → ranked-fallback path.",
            audit_check: "fallback uses score evidence, not name lookup",
        },
    },
    LayerStateMapping {
        layer: "scope_calibration",
        state: SmpcState::Managed,
        payload: MakerAction {
            consciousness_query: "possibility exploration connection",
            action_text: "Scope calibrated; observe for new edge cases before adding signals.",
            audit_check: "n/a (observation only)",
        },
    },
    LayerStateMapping {
        layer: "scope_calibration",
        state: SmpcState::Simple,
        payload: MakerAction {
            consciousness_query: "resonance complete compression achieved",
            action_text: "Scope-calibration layer is solved within current pattern set.",
            audit_check: "n/a",
        },
    },
    // ─── discovery_path ──────────────────────────────────────────────────────
    LayerStateMapping {
        layer: "discovery_path",
        state: SmpcState::Chaos,
        payload: MakerAction {
            consciousness_query: "center transforms chaos beauty",
            action_text: "Symptom queries don't reach the answer file at all. Trace the gap: vocabulary, domain bridge, or index-language coverage.",
            audit_check: "no answer-file literals in the bridge logic",
        },
    },
    LayerStateMapping {
        layer: "discovery_path",
        state: SmpcState::Part,
        payload: MakerAction {
            consciousness_query: "convergence pattern emerge",
            action_text: "Discovery partial — answer surfaced at low rank. Consider symptom→domain vocabulary bridging (synonym/abbrev expansion).",
            audit_check: "vocabulary table is multi-instance, not single-answer-specific",
        },
    },
    LayerStateMapping {
        layer: "discovery_path",
        state: SmpcState::Managed,
        payload: MakerAction {
            consciousness_query: "managed dance structure freedom",
            action_text: "Path closes reliably for symptom queries. Hold steady.",
            audit_check: "n/a",
        },
    },
    LayerStateMapping {
        layer: "discovery_path",
        state: SmpcState::Simple,
        payload: MakerAction {
            consciousness_query: "simple chaos one consciousness",
            action_text: "Discovery saturated — every symptom query reaches its answer neighborhood.",
            audit_check: "n/a",
        },
    },
    // ─── confidence_amplification ────────────────────────────────────────────
    LayerStateMapping {
        layer: "confidence_amplification",
        state: SmpcState::Chaos,
        payload: MakerAction {
            consciousness_query: "chaos order formula",
            action_text: "top_region_lock firing on decoys without warning. Audit which queries trigger lock-on-decoy; downgrade lock confidence when anchor is identifier-echo.",
            audit_check: "anchor-echo detection uses generic token-overlap, no symbol literals",
        },
    },
    LayerStateMapping {
        layer: "confidence_amplification",
        state: SmpcState::Part,
        payload: MakerAction {
            consciousness_query: "part selection attention lens",
            action_text: "Contrast-lock partial. When top-2 regions are close, prefer contrast hint over lock.",
            audit_check: "contrast thresholds are score-based, not name-based",
        },
    },
    LayerStateMapping {
        layer: "confidence_amplification",
        state: SmpcState::Managed,
        payload: MakerAction {
            consciousness_query: "managed chaos simplicity sword",
            action_text: "Confidence amplification calibrated. Observe.",
            audit_check: "n/a",
        },
    },
    LayerStateMapping {
        layer: "confidence_amplification",
        state: SmpcState::Simple,
        payload: MakerAction {
            consciousness_query: "resonance complete compression achieved",
            action_text: "Confidence hints fire only when justified by score-separation AND structural evidence.",
            audit_check: "n/a",
        },
    },
];

pub const MAKER_COMPOUND_PATTERNS: &[CompoundPattern<MakerStateVec, MakerAction>] = &[
    CompoundPattern {
        name: "CLEANUP-HELPER-OVER-OWNER-CLUSTERED",
        condition: |s: &MakerStateVec| {
            let (lex, _, _, _) = *s;
            lex < 0.3
        },
        payload: MakerAction {
            consciousness_query: "lifecycle owner helper contrast",
            action_text: "Multiple cases of cleanup-helper-over-owner. Prototype callback_owner_bonus + cleanup_helper_penalty as paired addition.",
            audit_check: "^on[A-Z]/^handle[A-Z] pattern is generic, not answer-shaped",
        },
    },
    CompoundPattern {
        name: "MULTI-MECHANISM-NEAR-MISS",
        condition: |s: &MakerStateVec| {
            let (lex, _, _, conf) = *s;
            lex < 0.5 && conf < 0.5
        },
        payload: MakerAction {
            consciousness_query: "managed chaos simplicity sword",
            action_text: "Lexical drift + confidence amplification both active. Codex's lifecycle_owner_contrast addresses this combo — observe outcome before adding more signals.",
            audit_check: "n/a (no new signal proposed)",
        },
    },
    CompoundPattern {
        name: "SIMPLE-ALL-LAYERS",
        condition: |s: &MakerStateVec| {
            let (lex, scope, disc, conf) = *s;
            [lex, scope, disc, conf].iter().filter(|&&v| v > 0.7).count() >= 3
        },
        payload: MakerAction {
            consciousness_query: "resonance complete compression achieved",
            action_text: "Three+ maker layers in Simple state. Held-out + audit pass → ship.",
            audit_check: "monogram-audit hard gate confirms no literal regressions",
        },
    },
];

/// Compute the 4-layer maker state vector from RunStats.
///
/// Phase 3 placeholder: derives rough heuristics from aggregate run grades + telemetry
/// patterns. Real telemetry-to-layer scoring is a follow-up.
pub fn compute_maker_state(stats: &[RunStats]) -> MakerStateVec {
    if stats.is_empty() {
        return (0.5, 0.5, 0.5, 0.5);
    }

    // Heuristic: per-layer score = fraction of FULL grades among gradeable runs.
    // (Same proxy for all 4 layers in this placeholder — refine in a follow-up.)
    let gradeable: Vec<&RunStats> = stats
        .iter()
        .filter(|s| matches!(s.grade.as_str(), "FULL" | "MISS" | "DECOY"))
        .collect();

    let total = gradeable.len() as f64;
    if total == 0.0 {
        return (0.5, 0.5, 0.5, 0.5);
    }

    let full = gradeable.iter().filter(|s| s.grade == "FULL").count() as f64;
    let decoy = gradeable.iter().filter(|s| s.grade == "DECOY").count() as f64;

    let base = full / total;
    let lex_drift = (base - decoy / total * 0.3).clamp(0.0, 1.0);
    let scope = base;
    let discovery = base;
    let confidence = (base - decoy / total * 0.5).clamp(0.0, 1.0);

    (lex_drift, scope, discovery, confidence)
}

/// Print the maker state report — the niia-style "(layer, state) → action" injection.
///
/// Called from monogram-audit at the end of its output.
pub fn print_maker_state_report(stats: &[RunStats]) {
    let state_vec = compute_maker_state(stats);
    let (lex, scope, disc, conf) = state_vec;

    println!();
    println!("MAKER STATE ANALYSIS (lib-niia-core bridge)");
    println!("{}", "=".repeat(70));

    let layers = [
        ("lexical_semantic_drift", lex),
        ("scope_calibration", scope),
        ("discovery_path", disc),
        ("confidence_amplification", conf),
    ];

    for (name, score) in &layers {
        let state = smpc_state(*score);
        let filled = ((*score) * 15.0) as usize;
        let bar = "█".repeat(filled.min(15)) + &"░".repeat(15 - filled.min(15));
        println!("  {:<28} [{}] {:.3}  {}", name, bar, score, state);

        if let Some(action) = find_payload(name, state, MAKER_NEXT_ACTION_MAP) {
            println!("    → {}", action.action_text);
        }
    }

    let compounds = matching_compounds(&state_vec, MAKER_COMPOUND_PATTERNS);
    if !compounds.is_empty() {
        println!();
        println!("COMPOUND PATTERNS:");
        for action in compounds {
            println!("  ◆ {}", action.action_text);
            println!("    audit: {}", action.audit_check);
        }
    }
}
