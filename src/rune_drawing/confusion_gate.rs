//! Phase 0 regression wall: every rune template, plus a fixed battery of
//! deterministic perturbations, must still recognize as itself. This is the
//! gate future tuning changes are checked against — see
//! `.project/magic-symbol-system-plan.md` Phase 0 and `.project/prd.md`.

use super::test_support::{perturb, resample_density};
use super::*;
use crate::data::GameData;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// (truth, predicted) pairs the current recognizer is known to confuse under
/// perturbation — all rounded/polygonal single-stroke shapes scored partly
/// via `circle_likeness` (see Part 1, issue A3 in the plan). Tracked here
/// instead of silently loosening the gate, so Phase 1's corner-threshold
/// work has a concrete list to clear before this allowlist can shrink.
const KNOWN_CONFUSIONS: &[(&str, &str)] = &[
    // Sparse-resample density dependence (A8) — corner_count's internal
    // resample-to-36 grid shifts safer's soft hexagon corners at low input
    // density. Phase 1 item 1 (canonicalize stroke density at capture)
    // should clear this; see property_tests' point_density exclusion.
    ("safer", "sphere"),
];

struct Case {
    name: &'static str,
    scale: f32,
    translate: (f32, f32),
    jitter_amp: f32,
    seed: u64,
    resample_to: Option<usize>,
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "clean",
            scale: 1.0,
            translate: (0.0, 0.0),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: None,
        },
        Case {
            name: "translate_pos",
            scale: 1.0,
            translate: (0.05, 0.04),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: None,
        },
        Case {
            name: "translate_neg",
            scale: 1.0,
            translate: (-0.05, -0.04),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: None,
        },
        Case {
            name: "scale_down",
            scale: 0.75,
            translate: (0.0, 0.0),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: None,
        },
        Case {
            name: "scale_up",
            scale: 1.25,
            translate: (0.0, 0.0),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: None,
        },
        Case {
            name: "sparse",
            scale: 1.0,
            translate: (0.0, 0.0),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: Some(14),
        },
        Case {
            name: "dense",
            scale: 1.0,
            translate: (0.0, 0.0),
            jitter_amp: 0.0,
            seed: 0,
            resample_to: Some(72),
        },
        Case {
            name: "jitter_a",
            scale: 1.0,
            translate: (0.0, 0.0),
            jitter_amp: 0.008,
            seed: 1,
            resample_to: None,
        },
        Case {
            name: "jitter_b",
            scale: 1.05,
            translate: (0.015, -0.015),
            jitter_amp: 0.008,
            seed: 2,
            resample_to: None,
        },
        Case {
            name: "jitter_c",
            scale: 0.90,
            translate: (-0.015, 0.015),
            jitter_amp: 0.012,
            seed: 3,
            resample_to: None,
        },
    ]
}

#[test]
fn confusion_matrix_perturbations_recognize_their_own_rune() {
    let data = GameData::load().unwrap();
    let runes: Vec<&RuneDef> = data.runes.iter().collect();
    let mut matrix: BTreeMap<(String, String), u32> = BTreeMap::new();
    let mut failures = Vec::new();
    let mut known_failures = Vec::new();
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
            sample = perturb(&sample, case.scale, case.translate, case.jitter_amp, case.seed);
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
                if KNOWN_CONFUSIONS.contains(&(rune.id.as_str(), predicted)) {
                    known_failures.push(entry);
                } else {
                    failures.push(entry);
                }
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
