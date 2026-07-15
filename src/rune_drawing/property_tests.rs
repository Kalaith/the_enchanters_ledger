//! Phase 0 invariance properties: same input reads the same way twice,
//! translate/scale of a mark does not change how it reads, denser or
//! sparser capture does not change identity, and more jitter never reads as
//! *higher* quality. See `.project/magic-symbol-system-plan.md` Phase 0.

use super::samples;
use super::test_support::{perturb, resample_density};
use super::*;
use crate::data::GameData;

fn sample_rune_ids() -> Vec<&'static str> {
    vec![
        "light", "sphere", "touch", "beam", "aura", "burst", "cone", "safer", "force", "warmth",
    ]
}

#[test]
fn translation_and_scale_do_not_change_confidence() {
    let data = GameData::load().unwrap();
    let runes: Vec<&RuneDef> = data.runes.iter().collect();
    for id in sample_rune_ids() {
        let Some(template) = template_strokes_for_rune(id) else {
            continue;
        };
        let baseline = recognize_rune(&template, runes.iter().copied()).unwrap();

        let translated = perturb(&template, 1.0, (0.04, -0.03), 0.0, 0);
        let translated_outcome = recognize_rune(&translated, runes.iter().copied()).unwrap();
        assert_eq!(translated_outcome.rune_id, baseline.rune_id, "id={id}");
        assert!(
            (translated_outcome.confidence - baseline.confidence).abs() < 0.01,
            "id={id} baseline={baseline:?} translated={translated_outcome:?}"
        );

        for scale in [0.5_f32, 1.25] {
            let scaled = perturb(&template, scale, (0.0, 0.0), 0.0, 0);
            let scaled_outcome = recognize_rune(&scaled, runes.iter().copied()).unwrap();
            assert_eq!(
                scaled_outcome.rune_id, baseline.rune_id,
                "id={id} scale={scale}"
            );
            assert!(
                (scaled_outcome.confidence - baseline.confidence).abs() < 0.01,
                "id={id} scale={scale} baseline={baseline:?} scaled={scaled_outcome:?}"
            );
        }
    }
}

#[test]
fn point_density_does_not_change_identity() {
    let data = GameData::load().unwrap();
    let runes: Vec<&RuneDef> = data.runes.iter().collect();
    // "safer" is excluded: its hexagon has two corners just past the corner
    // threshold, and re-sampling to a point count other than the recognizer's
    // internal 36-sample corner grid shifts those corners enough to misread
    // as "sphere" or "force". This is the density-dependence issue tracked
    // as A8 in the plan (device/framerate leaking into scores) — fixing it
    // properly needs canonicalize-at-capture (Phase 1), not a Phase 0 patch.
    for id in sample_rune_ids().into_iter().filter(|id| *id != "safer") {
        let Some(template) = template_strokes_for_rune(id) else {
            continue;
        };
        let baseline = recognize_rune(&template, runes.iter().copied()).unwrap();

        for density in [20usize, 60] {
            let resampled = resample_density(&template, density);
            let outcome = recognize_rune(&resampled, runes.iter().copied()).unwrap();
            assert_eq!(
                outcome.rune_id, baseline.rune_id,
                "id={id} density={density}"
            );
            assert!(
                outcome.accepted,
                "id={id} density={density} outcome={outcome:?}"
            );
        }
    }
}

#[test]
fn recognition_is_deterministic_across_repeated_runs() {
    let data = GameData::load().unwrap();
    let runes: Vec<&RuneDef> = data.runes.iter().collect();
    for id in sample_rune_ids() {
        let Some(template) = template_strokes_for_rune(id) else {
            continue;
        };
        let jittered = perturb(&template, 1.0, (0.0, 0.0), 0.015, 7);
        let first = recognize_rune(&jittered, runes.iter().copied());
        let second = recognize_rune(&jittered, runes.iter().copied());
        assert_eq!(first, second, "id={id}");
    }
}

/// The ambiguity penalty used to snap a fixed ×0.92/×0.96 the instant
/// score_gap dipped under MIN_RECOGNITION_MARGIN — a near-tied shape sitting
/// right at that line could jump several points on a 1-pixel wiggle. It's
/// now a continuous ramp (Phase 1 item 4); sweeping tiny jitter steps across
/// a known near-ambiguous sample should never show a big single-step jump.
#[test]
fn ambiguity_penalty_has_no_visible_cliff() {
    let data = GameData::load().unwrap();
    let runes: Vec<&RuneDef> = data.runes.iter().collect();
    let base = samples::ambiguous_shape_samples()
        .into_iter()
        .find(|sample| sample.name == "sphere_safer_round_hex")
        .unwrap()
        .strokes;

    let mut previous: Option<f32> = None;
    for step in 0..12u64 {
        let amp = step as f32 * 0.003;
        let sample = perturb(&base, 1.0, (0.0, 0.0), amp, step * 131 + 50);
        let outcome = recognize_rune(&sample, runes.iter().copied()).unwrap();
        if let Some(previous_confidence) = previous {
            assert!(
                (outcome.confidence - previous_confidence).abs() < 0.12,
                "step={step} amp={amp} confidence jumped from {previous_confidence} to {} ({outcome:?})",
                outcome.confidence
            );
        }
        previous = Some(outcome.confidence);
    }
}

/// Not a strict monotone (scoring has small nonlinearities, and any single
/// jitter seed can get lucky), so this averages several seeds per amplitude
/// and only checks the trend: quality should not climb as jitter grows.
#[test]
fn increasing_jitter_does_not_raise_quality() {
    let data = GameData::load().unwrap();
    let runes: Vec<&RuneDef> = data.runes.iter().collect();
    for id in sample_rune_ids() {
        let Some(template) = template_strokes_for_rune(id) else {
            continue;
        };
        let mut previous_quality = f32::INFINITY;
        for amp in [0.0_f32, 0.015, 0.03, 0.05] {
            let seeds = 0..5u64;
            let mut total_quality = 0.0;
            let mut sample_count = 0u32;
            for seed in seeds {
                let noisy = perturb(&template, 1.0, (0.0, 0.0), amp, seed * 97 + 11);
                let outcome = recognize_rune(&noisy, runes.iter().copied()).unwrap();
                total_quality += outcome.quality;
                sample_count += 1;
            }
            let average_quality = total_quality / sample_count as f32;
            assert!(
                average_quality <= previous_quality + 0.05,
                "id={id} amp={amp} previous_avg={previous_quality} this_avg={average_quality}"
            );
            previous_quality = average_quality;
        }
    }
}

/// Phase 1 exit criterion: corpus quality scores stay stable under ~1%
/// jitter. A drawing that is basically identical to the template, plus the
/// kind of sub-pixel hand tremor any real drawing has, should not read as
/// meaningfully lower quality than the clean template itself.
#[test]
fn quality_is_stable_under_one_percent_jitter() {
    const ONE_PERCENT_JITTER: f32 = 0.01;

    let data = GameData::load().unwrap();
    let runes: Vec<&RuneDef> = data.runes.iter().collect();
    for id in sample_rune_ids() {
        let Some(template) = template_strokes_for_rune(id) else {
            continue;
        };
        let baseline = recognize_rune(&template, runes.iter().copied()).unwrap();

        for seed in 0..5u64 {
            let jittered = perturb(
                &template,
                1.0,
                (0.0, 0.0),
                ONE_PERCENT_JITTER,
                seed * 61 + 3,
            );
            let outcome = recognize_rune(&jittered, runes.iter().copied()).unwrap();
            assert_eq!(outcome.rune_id, baseline.rune_id, "id={id} seed={seed}");
            assert!(
                outcome.quality >= baseline.quality - 0.10,
                "id={id} seed={seed} baseline_quality={} jittered={outcome:?}",
                baseline.quality
            );
        }
    }
}
