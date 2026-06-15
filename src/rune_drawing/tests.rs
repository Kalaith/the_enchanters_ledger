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
