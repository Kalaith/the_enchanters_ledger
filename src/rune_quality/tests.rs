use super::*;
use crate::data::GameData;
use crate::rune_drawing::recognize_rune;

#[test]
fn circle_reads_with_wrong_start_but_scores_lower() {
    let data = GameData::load().unwrap();
    let canonical = template_strokes_for_rune("sphere").unwrap();
    let wrong_start = circle_starting_right();

    let canonical_read = recognize_rune(&canonical, rank_one(&data)).unwrap();
    let wrong_read = recognize_rune(&wrong_start, rank_one(&data)).unwrap();

    assert_eq!(wrong_read.rune_id, "sphere");
    assert!(wrong_read.accepted, "{wrong_read:?}");
    assert!(
        canonical_read.quality > wrong_read.quality + 0.08,
        "canonical={canonical_read:?} wrong={wrong_read:?}"
    );
}

#[test]
fn practice_report_rewards_canonical_start_and_order() {
    let data = GameData::load().unwrap();
    let strokes = template_strokes_for_rune("light").unwrap();

    let report = practice_report_for_rune("light", &strokes, rank_one(&data)).unwrap();

    assert!(report.accepted, "{report:?}");
    assert!(report.quality > 0.92, "{report:?}");
    assert!(report.start_score > 0.92, "{report:?}");
    assert!(report.stroke_order_score > 0.92, "{report:?}");
}

#[test]
fn canonical_template_draws_no_mismatch_segments() {
    let data = GameData::load().unwrap();
    let strokes = template_strokes_for_rune("light").unwrap();

    let report = practice_report_for_rune("light", &strokes, rank_one(&data)).unwrap();

    assert!(
        report.mismatch_segments.is_empty(),
        "a perfect copy of the template should never be flagged: {report:?}"
    );
}

#[test]
fn reordered_strokes_do_not_produce_spurious_mismatches() {
    let data = GameData::load().unwrap();
    let mut shuffled = template_strokes_for_rune("light").unwrap();
    shuffled.reverse();

    let report = practice_report_for_rune("light", &shuffled, rank_one(&data)).unwrap();

    assert!(report.accepted, "{report:?}");
    assert!(
        report.mismatch_segments.is_empty(),
        "reordered-but-perfect strokes should not flag mismatches: {report:?}"
    );
}

#[test]
fn practice_acceptance_matches_normal_recognition() {
    let data = GameData::load().unwrap();
    let strokes = rough_sphere();

    let recognized = recognize_rune(&strokes, rank_one(&data)).unwrap();
    let report = practice_report_for_rune("sphere", &strokes, rank_one(&data)).unwrap();

    assert_eq!(recognized.rune_id, "sphere", "{recognized:?}");
    assert_eq!(report.accepted, recognized.accepted, "{report:?}");
}

#[test]
fn down_arrow_variant_earns_full_strict_quality() {
    let data = GameData::load().unwrap();
    let down_arrow = vec![DrawnStroke {
        points: vec![
            StrokePoint::new(0.50, 0.16),
            StrokePoint::new(0.50, 0.84),
            StrokePoint::new(0.34, 0.64),
            StrokePoint::new(0.50, 0.84),
            StrokePoint::new(0.66, 0.64),
        ],
    }];

    let strict = strict_quality_for_rune("touch", &down_arrow).unwrap();
    let report = practice_report_for_rune("touch", &down_arrow, rank_one(&data)).unwrap();

    assert!(strict > 0.90, "strict={strict} report={report:?}");
    assert!(report.accepted, "{report:?}");
    assert!(report.stroke_order_score > 0.90, "{report:?}");
}

#[test]
fn practice_explains_missing_safer_sides() {
    let data = GameData::load().unwrap();
    let triangle = vec![DrawnStroke {
        points: vec![
            StrokePoint::new(0.50, 0.16),
            StrokePoint::new(0.84, 0.78),
            StrokePoint::new(0.16, 0.78),
            StrokePoint::new(0.50, 0.16),
        ],
    }];

    let report = practice_report_for_rune("safer", &triangle, rank_one(&data)).unwrap();

    assert!(!report.accepted, "{report:?}");
    assert!(report.feedback.contains("six clear sides"), "{report:?}");
    assert!(!report.mismatch_segments.is_empty(), "{report:?}");
}

fn rank_one(data: &GameData) -> Vec<&RuneDef> {
    data.runes.iter().filter(|rune| rune.tier == 1).collect()
}

fn circle_starting_right() -> Vec<DrawnStroke> {
    let mut points = Vec::new();
    for index in 0..=24 {
        let angle = std::f32::consts::TAU * index as f32 / 24.0;
        points.push(StrokePoint::new(
            0.50 + 0.34 * angle.cos(),
            0.50 + 0.34 * angle.sin(),
        ));
    }
    vec![DrawnStroke { points }]
}

fn rough_sphere() -> Vec<DrawnStroke> {
    vec![DrawnStroke {
        points: vec![
            StrokePoint::new(0.50, 0.14),
            StrokePoint::new(0.76, 0.24),
            StrokePoint::new(0.86, 0.52),
            StrokePoint::new(0.68, 0.80),
            StrokePoint::new(0.38, 0.84),
            StrokePoint::new(0.16, 0.62),
            StrokePoint::new(0.22, 0.30),
            StrokePoint::new(0.50, 0.14),
        ],
    }]
}
