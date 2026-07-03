//! Phase 0 regression wall: every rune template, plus a fixed battery of
//! deterministic perturbations, must still recognize as itself. This is the
//! gate future tuning changes are checked against — see
//! `.project/magic-symbol-system-plan.md` Phase 0 and `.project/prd.md`.

use super::test_support::{perturb, resample_density};
use super::*;
use crate::data::GameData;
use std::collections::BTreeMap;
use std::fmt::Write as _;

// (truth, predicted) pairs the current recognizer is known to confuse under
// perturbation are declared in the template data itself (`confusable_with`
// in assets/data/rune_templates.json) and read via
// `templates::known_confusions()` — tracked there instead of silently
// loosening the gate, so tuning work has a concrete list to clear before
// the allowlist can shrink. Current entries: safer→sphere (sparse-resample
// density dependence, A8 — corner_count's internal resample-to-36 grid
// shifts safer's soft hexagon corners at low input density; see
// property_tests' point_density exclusion).

struct Case {
    name: &'static str,
    scale: f32,
    translate: (f32, f32),
    jitter_amp: f32,
    seed: u64,
    resample_to: Option<usize>,
    /// Minimum winner-vs-runner-up `score_gap` this case must keep against
    /// every other rune (plan Phase 0 item 2, "margin ≥ threshold"). Clean
    /// geometry must hold the full acceptance margin; jittered hands are
    /// allowed a narrower but still positive separation.
    min_margin: f32,
}

/// Clean/affine cases must beat every other rune by at least the commission
/// acceptance margin; perturbed cases by at least half of it.
const CLEAN_MARGIN: f32 = MIN_RECOGNITION_MARGIN;
const PERTURBED_MARGIN: f32 = MIN_RECOGNITION_MARGIN * 0.5;

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "clean",
            scale: 1.0,
            translate: (0.0, 0.0),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: None,
            min_margin: CLEAN_MARGIN,
        },
        Case {
            name: "translate_pos",
            scale: 1.0,
            translate: (0.05, 0.04),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: None,
            min_margin: CLEAN_MARGIN,
        },
        Case {
            name: "translate_neg",
            scale: 1.0,
            translate: (-0.05, -0.04),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: None,
            min_margin: CLEAN_MARGIN,
        },
        // The plan asks for the full 0.5–2.0× band. Down-scale is exercised
        // directly at half size; up-scale requests are clamped per template
        // to the largest factor that still fits the slate (most templates
        // already span most of it — doubling them in place would slam their
        // points into the [0,1] clamp and distort the shape instead of
        // testing scale). See `fitted_scale`.
        Case {
            name: "scale_half",
            scale: 0.5,
            translate: (0.0, 0.0),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: None,
            min_margin: CLEAN_MARGIN,
        },
        Case {
            name: "scale_double",
            scale: 2.0,
            translate: (0.0, 0.0),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: None,
            min_margin: CLEAN_MARGIN,
        },
        Case {
            name: "scale_down",
            scale: 0.75,
            translate: (0.0, 0.0),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: None,
            min_margin: CLEAN_MARGIN,
        },
        Case {
            name: "scale_up",
            scale: 1.25,
            translate: (0.0, 0.0),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: None,
            min_margin: CLEAN_MARGIN,
        },
        Case {
            name: "sparse",
            scale: 1.0,
            translate: (0.0, 0.0),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: Some(14),
            min_margin: PERTURBED_MARGIN,
        },
        Case {
            name: "dense",
            scale: 1.0,
            translate: (0.0, 0.0),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: Some(72),
            min_margin: PERTURBED_MARGIN,
        },
        Case {
            name: "jitter_a",
            scale: 1.0,
            translate: (0.0, 0.0),
            jitter_amp: 0.008,
            seed: 1,
            resample_to: None,
            min_margin: PERTURBED_MARGIN,
        },
        Case {
            name: "jitter_b",
            scale: 1.05,
            translate: (0.015, -0.015),
            jitter_amp: 0.008,
            seed: 2,
            resample_to: None,
            min_margin: PERTURBED_MARGIN,
        },
        Case {
            name: "jitter_c",
            scale: 0.90,
            translate: (-0.015, 0.015),
            jitter_amp: 0.012,
            seed: 3,
            resample_to: None,
            min_margin: PERTURBED_MARGIN,
        },
    ]
}

/// The largest factor (≤ requested) this template can grow by and still fit
/// the slate with a small safety margin — so a 2.0× request exercises real
/// up-scaling as far as the unit slate physically allows instead of letting
/// `StrokePoint::new`'s [0,1] clamp silently distort the shape.
fn fitted_scale(template: &[DrawnStroke], requested: f32) -> f32 {
    let (min_x, max_x, min_y, max_y) = super::test_support::bounding_box(template);
    let span = (max_x - min_x).max(max_y - min_y).max(0.001);
    requested.min(0.94 / span)
}

#[test]
fn confusion_matrix_perturbations_recognize_their_own_rune() {
    let data = GameData::load().unwrap();
    let runes: Vec<&RuneDef> = data.runes.iter().collect();
    let known_confusions = super::templates::known_confusions();
    let mut matrix: BTreeMap<(String, String), u32> = BTreeMap::new();
    let mut failures = Vec::new();
    let mut known_failures = Vec::new();
    let mut margin_failures = Vec::new();
    let mut total = 0u32;

    for rune in &runes {
        let Some(template) = template_strokes_for_rune(&rune.id) else {
            continue;
        };
        for case in cases() {
            let mut sample = template.clone();
            if let Some(target_count) = case.resample_to {
                sample = resample_density(&sample, target_count);
            }
            sample = perturb(
                &sample,
                fitted_scale(&sample, case.scale),
                case.translate,
                case.jitter_amp,
                case.seed,
            );
            total += 1;

            let outcome = recognize_rune(&sample, runes.iter().copied());
            let predicted = outcome
                .as_ref()
                .map(|result| result.rune_id.clone())
                .unwrap_or_else(|| "<none>".to_string());
            *matrix.entry((rune.id.clone(), predicted)).or_default() += 1;

            let ok = outcome
                .as_ref()
                .is_some_and(|result| result.rune_id == rune.id && result.accepted);
            if !ok {
                let predicted = outcome
                    .as_ref()
                    .map(|result| result.rune_id.as_str())
                    .unwrap_or("<none>");
                let entry = format!("{} case={} -> {:?}", rune.id, case.name, outcome);
                if known_confusions.contains(&(rune.id.as_str(), predicted)) {
                    known_failures.push(entry);
                } else {
                    failures.push(entry);
                }
                continue;
            }

            // Margin gate (plan Phase 0 item 2): recognizing itself is not
            // enough — the winner must beat the best *other* rune by the
            // case's margin, or a hair of extra jitter could flip the read.
            // Pairs in KNOWN_CONFUSIONS are exempt from the margin (their
            // separation is the tracked problem), never from identity.
            let result = outcome.as_ref().unwrap();
            let runner_up = result
                .alternatives
                .first()
                .map(|candidate| candidate.rune_id.as_str())
                .unwrap_or("<none>");
            let margin_exempt = known_confusions.contains(&(rune.id.as_str(), runner_up));
            if result.score_gap < case.min_margin && !margin_exempt {
                margin_failures.push(format!(
                    "{} case={} gap={:.4} < {:.4} (runner-up {})",
                    rune.id, case.name, result.score_gap, case.min_margin, runner_up
                ));
            }
        }
    }

    if !known_failures.is_empty() {
        eprintln!(
            "{} known-confusion misreads (tracked in KNOWN_CONFUSIONS, not gating):\n{}",
            known_failures.len(),
            known_failures.join("\n")
        );
    }

    if !failures.is_empty() {
        eprintln!("{}", render_matrix(&runes, &matrix));
        panic!(
            "{}/{} perturbed samples misread their own rune as something new \
             (not in KNOWN_CONFUSIONS):\n{}",
            failures.len(),
            total,
            failures.join("\n")
        );
    }

    if !margin_failures.is_empty() {
        eprintln!("{}", render_matrix(&runes, &matrix));
        panic!(
            "{}/{} samples recognized correctly but with too little margin \
             over the runner-up:\n{}",
            margin_failures.len(),
            total,
            margin_failures.join("\n")
        );
    }
}

fn render_matrix(runes: &[&RuneDef], matrix: &BTreeMap<(String, String), u32>) -> String {
    let ids: Vec<&str> = runes.iter().map(|rune| rune.id.as_str()).collect();
    let mut out = String::from("Confusion matrix (rows=truth, cols=predicted, dot=0):\n");
    let _ = write!(out, "{:>14}", "");
    for id in &ids {
        let _ = write!(out, "{:>6}", &id[..id.len().min(5)]);
    }
    out.push('\n');
    for truth in &ids {
        let _ = write!(out, "{:>14}", truth);
        for predicted in &ids {
            let count = matrix
                .get(&(truth.to_string(), predicted.to_string()))
                .copied()
                .unwrap_or(0);
            if count > 0 {
                let _ = write!(out, "{:>6}", count);
            } else {
                let _ = write!(out, "{:>6}", ".");
            }
        }
        out.push('\n');
    }
    out
}
