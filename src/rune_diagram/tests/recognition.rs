//! Which marks a diagram reads as: the working circle it needs, the runes
//! found inside it, and the marks correctly refused.

use super::fixtures::*;
use crate::data::GameData;
use crate::rune_diagram::{cluster_strokes, interpret_diagram};
use crate::rune_drawing::samples;

#[test]
fn structural_rune_sample_set_reads_inside_full_diagrams() {
    let data = GameData::load().unwrap();

    for sample in samples::structural_rune_samples() {
        let strokes = samples::circled_sample(&sample, 0.50, 0.50, 0.17);
        let interpretation = interpret_diagram(&strokes, all_runes(&data));
        let ids = interpretation
            .runes
            .iter()
            .map(|rune| rune.rune_id.as_str())
            .collect::<Vec<_>>();

        assert!(
            interpretation.accepted(),
            "sample={} interpretation={interpretation:?}",
            sample.name
        );
        assert!(
            ids.contains(&sample.rune_id),
            "sample={} ids={ids:?} interpretation={interpretation:?}",
            sample.name
        );
    }
}

#[test]
fn rejects_diagram_without_outer_circle() {
    let data = GameData::load().unwrap();
    let strokes = template_at("light", 0.5, 0.5, 0.22);

    let interpretation = interpret_diagram(&strokes, rank_one(&data));

    assert!(!interpretation.circle_found);
    assert!(!interpretation.accepted());
}

#[test]
fn interprets_multiple_runes_inside_enclosing_circle() {
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.26, 0.50, 0.18));
    strokes.extend(template_at("sphere", 0.50, 0.50, 0.18));
    strokes.extend(template_at("continuous", 0.74, 0.50, 0.18));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"light"), "{ids:?}");
    assert!(ids.contains(&"sphere"), "{ids:?}");
    assert!(ids.contains(&"continuous"), "{ids:?}");
}

#[test]
fn overlapped_sphere_still_reads_inside_working_circle() {
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.50, 0.50, 0.18));
    strokes.push(rough_sphere(0.50, 0.50, 0.20));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"light"), "{ids:?}");
    assert!(ids.contains(&"sphere"), "{ids:?}");
}

#[test]
fn lone_large_centered_inner_circle_reads_as_sphere() {
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.26, 0.50, 0.18));
    strokes.extend(rough_circle(0.50, 0.50, 0.17, 0.16, 20));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"sphere"), "{ids:?}");
}

#[test]
fn screenshot_clear_light_and_sphere_read_together() {
    let data = GameData::load().unwrap();
    let mut strokes = rough_circle(0.30, 0.38, 0.25, 0.27, 28);
    strokes.push(flat_rough_circle(0.20, 0.28, 0.12));
    strokes.extend(template_at("light", 0.31, 0.36, 0.18));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"light"), "{ids:?}");
    assert!(ids.contains(&"sphere"), "{ids:?}");
}

#[test]
fn damaged_sphere_fragment_does_not_make_neighboring_light_fail() {
    let data = GameData::load().unwrap();
    let mut strokes = rough_circle(0.445, 0.51, 0.205, 0.44, 36);
    strokes.extend(template_at("light", 0.36, 0.36, 0.16));
    strokes.push(sphere_fragment());
    strokes.extend(template_at("continuous", 0.36, 0.66, 0.16));
    strokes.push(stroke_at(
        &[(0.60, 0.12), (0.36, 0.46), (0.56, 0.46), (0.38, 0.88)],
        0.485,
        0.42,
        0.09,
    ));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(ids.contains(&"light"), "{ids:?}");
    assert!(ids.contains(&"continuous"), "{ids:?}");
    assert!(!ids.contains(&"sphere"), "{ids:?}");
    assert!(!ids.contains(&"spark"), "{ids:?}");
}

#[test]
fn damaged_sphere_fragment_stays_out_of_neighboring_light_cluster() {
    let mut indexed_strokes = template_at("light", 0.36, 0.36, 0.16)
        .into_iter()
        .enumerate()
        .map(|(index, stroke)| (index + 1, stroke))
        .collect::<Vec<_>>();
    indexed_strokes.push((5, sphere_fragment()));

    let clusters = cluster_strokes(&indexed_strokes);
    let light_cluster = clusters
        .iter()
        .find(|cluster| cluster.indices.contains(&1))
        .unwrap();

    assert!(light_cluster.indices.contains(&4), "{clusters:?}");
    assert!(!light_cluster.indices.contains(&5), "{clusters:?}");
    assert!(
        clusters.iter().any(|cluster| cluster.indices == vec![5]),
        "{clusters:?}"
    );
}

/// The top-left arc of a `sphere` — enough of one to tempt the recognizer,
/// not enough to be one.
fn sphere_fragment() -> crate::rune_drawing::DrawnStroke {
    stroke_at(
        &[(0.16, 0.62), (0.22, 0.30), (0.50, 0.14)],
        0.46,
        0.45,
        0.19,
    )
}

#[test]
fn accepts_smaller_off_center_outer_circle() {
    let data = GameData::load().unwrap();
    let mut strokes = rough_circle(0.34, 0.42, 0.23, 0.19, 24);
    strokes.extend(template_at("light", 0.34, 0.42, 0.12));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"light"), "{ids:?}");
}

#[test]
fn rejects_simple_cross_inside_circle() {
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.push(stroke_at(&[(0.50, 0.18), (0.50, 0.82)], 0.50, 0.50, 0.20));
    strokes.push(stroke_at(&[(0.26, 0.50), (0.74, 0.50)], 0.50, 0.50, 0.20));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));

    assert!(!interpretation.accepted(), "{interpretation:?}");
    assert!(interpretation.runes.is_empty(), "{interpretation:?}");
    assert_eq!(interpretation.rejected_marks, 1);
}

#[test]
fn rune_far_below_old_twelve_percent_scale_floor_still_reads() {
    // Phase 3 item 4 exit criterion (C1/C2): a rune drawn at ~4% of the working circle's scale
    // — well under the old, now-removed `MIN_RUNE_SCALE_IN_CIRCLE` (0.12) floor — must still be
    // found. This is the scale a 100+-symbol grand diagram needs most of its runes drawn at.
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("spark", 0.30, 0.30, 0.035));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"spark"), "{ids:?}");
}
