use super::*;
use crate::data::GameData;

fn unlocked_rank_one(data: &GameData) -> Vec<&RuneDef> {
    data.runes.iter().filter(|rune| rune.tier == 1).collect()
}

fn all_runes(data: &GameData) -> Vec<&RuneDef> {
    data.runes.iter().collect()
}

#[test]
fn structural_rune_sample_set_recognizes_expected_runes() {
    let data = GameData::load().unwrap();

    for sample in samples::structural_rune_samples() {
        let result = recognize_rune(&sample.strokes, all_runes(&data)).unwrap();

        assert_eq!(
            result.rune_id, sample.rune_id,
            "sample={} result={result:?}",
            sample.name
        );
        assert!(result.accepted, "sample={} result={result:?}", sample.name);
        assert!(
            result.quality >= 0.44,
            "sample={} result={result:?}",
            sample.name
        );
    }
}

#[test]
fn ambiguous_shape_samples_report_alternatives() {
    let data = GameData::load().unwrap();

    for sample in samples::ambiguous_shape_samples() {
        let result = recognize_rune(&sample.strokes, all_runes(&data)).unwrap();

        assert!(
            !result.alternatives.is_empty(),
            "sample={} result={result:?}",
            sample.name
        );
        assert!(
            result.score_gap >= 0.0,
            "sample={} result={result:?}",
            sample.name
        );
        assert_eq!(
            result.ambiguous,
            result.score_gap < MIN_RECOGNITION_MARGIN,
            "sample={} result={result:?}",
            sample.name
        );
    }
}

#[test]
fn clean_template_recognizes_the_expected_rune() {
    let data = GameData::load().unwrap();
    let strokes = template_strokes_for_rune("light").unwrap();

    let result = recognize_rune(&strokes, unlocked_rank_one(&data)).unwrap();

    assert_eq!(result.rune_id, "light");
    assert!(result.confidence > 0.90, "{result:?}");
    assert!(result.accepted);
}

#[test]
fn unrelated_scribble_falls_below_acceptance_threshold() {
    let data = GameData::load().unwrap();
    let scribble = raw(&[
        &[(0.12, 0.18), (0.82, 0.34), (0.22, 0.50), (0.78, 0.72)],
        &[(0.16, 0.78), (0.42, 0.22), (0.72, 0.82), (0.88, 0.28)],
    ]);

    let result = recognize_rune(&scribble, unlocked_rank_one(&data)).unwrap();

    assert!(result.confidence < MIN_RECOGNITION_CONFIDENCE, "{result:?}");
    assert!(!result.accepted);
}

#[test]
fn simple_cross_is_not_a_light_rune() {
    let data = GameData::load().unwrap();
    let cross = raw(&[&[(0.50, 0.18), (0.50, 0.82)], &[(0.26, 0.50), (0.74, 0.50)]]);

    let result = recognize_rune(&cross, unlocked_rank_one(&data)).unwrap();

    assert!(result.confidence < MIN_RECOGNITION_CONFIDENCE, "{result:?}");
    assert!(!result.accepted);
}

#[test]
fn down_arrow_variant_recognizes_touch() {
    let data = GameData::load().unwrap();
    let down_arrow = raw(&[&[
        (0.52, 0.14),
        (0.52, 0.82),
        (0.32, 0.62),
        (0.52, 0.82),
        (0.72, 0.62),
    ]]);

    let result = recognize_rune(&down_arrow, unlocked_rank_one(&data)).unwrap();

    assert_eq!(result.rune_id, "touch", "{result:?}");
    assert!(result.accepted, "{result:?}");
}

#[test]
fn rough_inner_circle_prefers_sphere_over_safer() {
    let data = GameData::load().unwrap();
    let circle = raw(&[&[
        (0.42, 0.18),
        (0.66, 0.20),
        (0.80, 0.34),
        (0.82, 0.58),
        (0.68, 0.76),
        (0.44, 0.82),
        (0.24, 0.68),
        (0.18, 0.44),
        (0.28, 0.26),
        (0.42, 0.18),
    ]]);

    let result = recognize_rune(&circle, unlocked_rank_one(&data)).unwrap();

    assert_eq!(result.rune_id, "sphere", "{result:?}");
    assert!(result.accepted, "{result:?}");
}

#[test]
fn safer_template_still_recognizes_safer() {
    let data = GameData::load().unwrap();
    let safer = template_strokes_for_rune("safer").unwrap();

    let result = recognize_rune(&safer, unlocked_rank_one(&data)).unwrap();

    assert_eq!(result.rune_id, "safer", "{result:?}");
    assert!(result.accepted, "{result:?}");
}

#[test]
fn closed_star_does_not_read_as_touch() {
    let data = GameData::load().unwrap();
    let star = raw(&[&[
        (0.50, 0.10),
        (0.61, 0.38),
        (0.90, 0.38),
        (0.66, 0.56),
        (0.76, 0.86),
        (0.50, 0.68),
        (0.24, 0.86),
        (0.34, 0.56),
        (0.10, 0.38),
        (0.39, 0.38),
        (0.50, 0.10),
    ]]);

    let result = recognize_rune(&star, unlocked_rank_one(&data)).unwrap();

    assert!(result.rune_id != "touch" || !result.accepted, "{result:?}");
}

#[test]
fn circle_repaired_in_two_arcs_reads_as_sphere() {
    let data = GameData::load().unwrap();
    let mut right_arc = Vec::new();
    let mut left_arc = Vec::new();
    for index in 0..=12 {
        let angle = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * index as f32 / 12.0;
        right_arc.push(StrokePoint::new(
            0.50 + 0.34 * angle.cos(),
            0.50 + 0.34 * angle.sin(),
        ));
        left_arc.push(StrokePoint::new(
            0.50 - 0.34 * angle.cos(),
            0.50 - 0.34 * angle.sin(),
        ));
    }
    let strokes = vec![
        DrawnStroke { points: right_arc },
        DrawnStroke { points: left_arc },
    ];

    let merged = merge_continuation_strokes(&strokes);
    let result = recognize_rune(&strokes, unlocked_rank_one(&data)).unwrap();

    assert_eq!(merged.len(), 1, "{merged:?}");
    assert_eq!(result.rune_id, "sphere", "{result:?}");
    assert!(result.accepted, "{result:?}");
}

#[test]
fn connected_arrowhead_strokes_stay_separate() {
    let beam_with_attached_barbs = raw(&[
        &[(0.14, 0.50), (0.84, 0.50)],
        &[(0.84, 0.50), (0.66, 0.34)],
        &[(0.84, 0.50), (0.66, 0.66)],
    ]);

    let merged = merge_continuation_strokes(&beam_with_attached_barbs);

    assert_eq!(merged.len(), 3, "{merged:?}");
}

#[test]
fn recognition_is_deterministic_for_identical_ink() {
    let data = GameData::load().unwrap();
    let strokes = samples::structural_rune_samples()
        .into_iter()
        .find(|sample| sample.name == "sphere_rough")
        .unwrap()
        .strokes;

    let first = recognize_rune(&strokes, all_runes(&data)).unwrap();
    let second = recognize_rune(&strokes, all_runes(&data)).unwrap();

    assert_eq!(first, second);
}

#[test]
fn stroke_order_shuffle_keeps_identity() {
    let data = GameData::load().unwrap();
    let mut shuffled = template_strokes_for_rune("light").unwrap();
    shuffled.reverse();

    let result = recognize_rune(&shuffled, unlocked_rank_one(&data)).unwrap();

    assert_eq!(result.rune_id, "light", "{result:?}");
    assert!(result.accepted, "{result:?}");
}

#[test]
fn eraser_drops_sliver_fragments() {
    let mut strokes = raw(&[&[(0.20, 0.50), (0.203, 0.50), (0.206, 0.50), (0.80, 0.50)]]);

    let erased = erase_strokes_at(&mut strokes, StrokePoint::new(0.35, 0.50), 0.13);

    assert!(erased);
    assert_eq!(strokes.len(), 1, "{strokes:?}");
    assert!(
        strokes[0].points.iter().all(|point| point.x > 0.4),
        "{strokes:?}"
    );
}

#[test]
fn every_rune_has_a_data_driven_template() {
    // Template shapes live in assets/data/rune_templates.json (Phase 1 item
    // 3); this catches a JSON edit that drops/misspells a rune id before it
    // ever reaches recognition.
    let data = GameData::load().unwrap();
    for rune in &data.runes {
        let strokes = template_strokes_for_rune(&rune.id);
        assert!(strokes.is_some(), "no template for rune {}", rune.id);
        let strokes = strokes.unwrap();
        assert!(!strokes.is_empty(), "empty template for rune {}", rune.id);
        assert!(
            strokes.iter().all(|stroke| stroke.points.len() >= 2),
            "degenerate stroke in template for rune {}",
            rune.id
        );
    }
}

#[test]
fn touch_and_continuous_expose_their_extra_variants() {
    assert_eq!(
        template_variants_for_rune("touch").len(),
        3,
        "canonical + 2 variants"
    );
    assert_eq!(
        template_variants_for_rune("continuous").len(),
        2,
        "canonical + 1 variant"
    );
    assert_eq!(
        template_variants_for_rune("light").len(),
        1,
        "no variants defined for light"
    );
}

#[test]
fn canonicalize_stroke_converges_regardless_of_capture_density() {
    // Same physical drag, captured at a slow frame rate (few raw points)
    // and a fast one (many raw points, including near-duplicate jitter a
    // real mouse would produce). Canonicalizing both should land on
    // point-for-point comparable strokes, so recognition and quality never
    // see which capture rate produced them (A8).
    let sparse = DrawnStroke {
        points: vec![
            StrokePoint::new(0.10, 0.10),
            StrokePoint::new(0.50, 0.50),
            StrokePoint::new(0.90, 0.10),
        ],
    };
    let mut dense_points = Vec::new();
    for segment in sparse.points.windows(2) {
        for step in 0..=40 {
            let t = step as f32 / 40.0;
            dense_points.push(StrokePoint::new(
                segment[0].x + (segment[1].x - segment[0].x) * t,
                segment[0].y + (segment[1].y - segment[0].y) * t,
            ));
        }
    }
    let dense = DrawnStroke {
        points: dense_points,
    };

    let canonical_sparse = canonicalize_stroke(sparse);
    let canonical_dense = canonicalize_stroke(dense);

    assert_eq!(canonical_sparse.points.len(), canonical_dense.points.len());
    for (a, b) in canonical_sparse.points.iter().zip(&canonical_dense.points) {
        assert!(a.distance(*b) < 0.002, "sparse={a:?} dense={b:?}");
    }
}

#[test]
fn acceptance_bands_are_ordered_practice_strictest_sandbox_most_lenient() {
    // Plan Phase 5 item 1: "Practice strict, commissions moderate, sandbox lenient."
    let practice = acceptance_band(RecognitionContext::Practice);
    let commission = acceptance_band(RecognitionContext::Commission);
    let sandbox = acceptance_band(RecognitionContext::Sandbox);

    assert!(
        practice.confidence > commission.confidence,
        "{practice:?} vs {commission:?}"
    );
    assert!(
        commission.confidence > sandbox.confidence,
        "{commission:?} vs {sandbox:?}"
    );
    assert!(practice.margin > commission.margin);
    assert!(commission.margin > sandbox.margin);
    assert!(practice.ambiguous_confidence > commission.ambiguous_confidence);
    assert!(commission.ambiguous_confidence > sandbox.ambiguous_confidence);

    // Commission is bit-identical to the pre-Phase-5 hardcoded constants — no behavior change
    // for any existing caller of the context-free `recognize_rune`.
    assert_eq!(commission.confidence, MIN_RECOGNITION_CONFIDENCE);
    assert_eq!(commission.margin, MIN_RECOGNITION_MARGIN);
}

#[test]
fn context_changes_acceptance_cutoffs_never_the_underlying_score() {
    // "One recognizer... thresholds differ, behavior never does" — the same drawing must score
    // identically (same winning rune, same confidence/quality) in every context; only whether
    // that score clears the line differs.
    let data = crate::data::GameData::load().unwrap();
    let strokes = samples::circled_sample(&samples::ambiguous_shape_samples()[0], 0.5, 0.5, 0.20);
    let strokes = strokes
        .into_iter()
        .skip(1) // drop the outer circle, this test only cares about the inner rune's own score
        .collect::<Vec<_>>();

    let commission =
        recognize_rune_in_context(&strokes, data.runes.iter(), RecognitionContext::Commission)
            .unwrap();
    let practice =
        recognize_rune_in_context(&strokes, data.runes.iter(), RecognitionContext::Practice)
            .unwrap();
    let sandbox =
        recognize_rune_in_context(&strokes, data.runes.iter(), RecognitionContext::Sandbox)
            .unwrap();

    assert_eq!(commission.rune_id, practice.rune_id);
    assert_eq!(commission.rune_id, sandbox.rune_id);
    assert!(
        (commission.confidence - practice.confidence).abs() < 1e-5,
        "{commission:?} vs {practice:?}"
    );
    assert!(
        (commission.confidence - sandbox.confidence).abs() < 1e-5,
        "{commission:?} vs {sandbox:?}"
    );
    assert!(
        (commission.quality - practice.quality).abs() < 1e-5,
        "{commission:?} vs {practice:?}"
    );
}

#[test]
fn eraser_splits_a_stroke_without_clearing_the_whole_mark() {
    let mut strokes = raw(&[&[(0.12, 0.50), (0.88, 0.50)]]);

    let erased = erase_strokes_at(&mut strokes, StrokePoint::new(0.50, 0.50), 0.08);

    assert!(erased);
    assert_eq!(strokes.len(), 2, "{strokes:?}");
    assert!(strokes.iter().all(|stroke| stroke.has_ink()));
    assert!(strokes
        .iter()
        .flat_map(|stroke| &stroke.points)
        .all(|point| point.distance(StrokePoint::new(0.50, 0.50)) > 0.08));
}

#[test]
fn stable_recognition_without_previous_matches_plain_recognition() {
    let data = GameData::load().unwrap();
    let jittered = test_support::perturb(
        &template_strokes_for_rune("light").unwrap(),
        1.0,
        (0.0, 0.0),
        0.01,
        5,
    );

    let plain = recognize_rune(&jittered, all_runes(&data));
    let stable = recognize_rune_stable(
        &jittered,
        all_runes(&data),
        RecognitionContext::Commission,
        None,
    );

    assert_eq!(plain, stable);
}

#[test]
fn hysteresis_ignores_a_decisively_beaten_previous_reading() {
    let data = GameData::load().unwrap();
    let template = template_strokes_for_rune("sphere").unwrap();

    let outcome = recognize_rune_stable(
        &template,
        all_runes(&data),
        RecognitionContext::Commission,
        Some("light"),
    )
    .unwrap();

    assert_eq!(outcome.rune_id, "sphere", "{outcome:?}");
}

#[test]
fn hysteresis_keeps_previous_reading_on_a_near_tie() {
    let data = GameData::load().unwrap();
    let sample = samples::ambiguous_shape_samples()
        .into_iter()
        .find(|sample| sample.name == "sphere_safer_round_hex")
        .unwrap()
        .strokes;

    // Deterministically search seeded jitters of the known-ambiguous hex for
    // one that lands inside the hysteresis window, so the test keeps working
    // if tuning shifts the exact scores a little.
    let (near_tie, baseline) = (0..64u64)
        .find_map(|seed| {
            let jittered = test_support::perturb(&sample, 1.0, (0.0, 0.0), 0.012, seed);
            let outcome = recognize_rune(&jittered, all_runes(&data))?;
            (outcome.score_gap < RECOGNITION_HYSTERESIS_MARGIN && !outcome.alternatives.is_empty())
                .then_some((jittered, outcome))
        })
        .expect(
            "test precondition: no jitter of the ambiguous hex lands within \
             the hysteresis margin any more — pick another near-tie sample",
        );
    let runner_up = baseline.alternatives.first().unwrap().rune_id.clone();

    let sticky = recognize_rune_stable(
        &near_tie,
        all_runes(&data),
        RecognitionContext::Commission,
        Some(&runner_up),
    )
    .unwrap();

    assert_eq!(sticky.rune_id, runner_up, "{sticky:?}");
    assert!(sticky.ambiguous, "{sticky:?}");
    assert_eq!(
        sticky.alternatives.first().map(|alt| alt.rune_id.as_str()),
        Some(baseline.rune_id.as_str()),
        "the demoted true best should surface as the first alternative: {sticky:?}"
    );
}
