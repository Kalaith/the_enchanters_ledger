use super::*;
use crate::data::GameData;
use crate::rune_drawing::template_strokes_for_rune;

fn rank_one(data: &GameData) -> Vec<&RuneDef> {
    data.runes.iter().filter(|rune| rune.tier == 1).collect()
}

fn all_runes(data: &GameData) -> Vec<&RuneDef> {
    data.runes.iter().collect()
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
    strokes.push(stroke_at(
        &[(0.16, 0.62), (0.22, 0.30), (0.50, 0.14)],
        0.46,
        0.45,
        0.19,
    ));
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
    indexed_strokes.push((
        5,
        stroke_at(
            &[(0.16, 0.62), (0.22, 0.30), (0.50, 0.14)],
            0.46,
            0.45,
            0.19,
        ),
    ));

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
fn centered_shape_rune_scores_better_than_off_center_shape() {
    let data = GameData::load().unwrap();
    let centered = touch_quality_at(0.50, 0.50, &data);
    let off_center = touch_quality_at(0.26, 0.50, &data);

    assert!(
        centered > off_center + 0.03,
        "centered={centered} off_center={off_center}"
    );
}

#[test]
fn interprets_high_tier_structured_circle_spell() {
    let data = GameData::load().unwrap();
    let interpretation = interpret_diagram(&high_tier_city_circle(), all_runes(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();
    let spell = interpretation.spell.as_ref().unwrap();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"gravity"), "{ids:?}");
    assert!(ids.contains(&"sphere"), "{ids:?}");
    assert!(ids.contains(&"continuous"), "{ids:?}");
    assert_eq!(spell.dominant_effect.as_deref(), Some("gravity"));
    assert_eq!(spell.tier_rank, 4, "{spell:?}");
    assert!(spell.complexity >= 0.72, "{spell:?}");
    assert!(spell.ring_count >= 4, "{spell:?}");
    assert!(spell.satellite_count >= 6, "{spell:?}");
    assert!(spell.perimeter_mark_count >= 32, "{spell:?}");
    assert!(spell.script_mark_count >= 32, "{spell:?}");
}

fn touch_quality_at(x: f32, y: f32, data: &GameData) -> f32 {
    let mut strokes = outer_circle();
    strokes.extend(template_at("touch", x, y, 0.18));
    let interpretation = interpret_diagram(&strokes, rank_one(data));
    interpretation
        .runes
        .iter()
        .find(|rune| rune.rune_id == "touch")
        .map(|rune| rune.quality)
        .unwrap_or(0.0)
}

fn outer_circle() -> Vec<DrawnStroke> {
    rough_circle(0.50, 0.50, 0.42, 0.40, 36)
}

fn high_tier_city_circle() -> Vec<DrawnStroke> {
    let mut strokes = outer_circle();
    strokes.extend(rough_circle(0.50, 0.50, 0.36, 0.34, 48));
    strokes.extend(rough_circle(0.50, 0.50, 0.30, 0.29, 44));
    strokes.extend(rough_circle(0.50, 0.50, 0.23, 0.22, 38));
    strokes.extend(rough_circle(0.50, 0.50, 0.16, 0.15, 32));
    strokes.extend(template_at("gravity", 0.28, 0.48, 0.22));
    strokes.extend(template_at("sphere", 0.50, 0.50, 0.18));
    strokes.extend(template_at("continuous", 0.73, 0.48, 0.17));
    strokes.extend(template_at("safer", 0.50, 0.73, 0.15));
    strokes.extend(satellite_seals(8, 0.30, 0.038));
    strokes.extend(radial_spokes(8, 0.31));
    strokes.extend(perimeter_ticks(36, 0.39, 0.016));
    strokes.extend(perimeter_ticks(24, 0.34, 0.012));
    strokes.extend(script_marks(24, 0.20, 0.010));
    strokes.extend(script_marks(32, 0.27, 0.008));
    strokes
}

fn rough_circle(cx: f32, cy: f32, rx: f32, ry: f32, steps: usize) -> Vec<DrawnStroke> {
    let mut points = Vec::new();
    for index in 0..=steps {
        let angle = std::f32::consts::TAU * index as f32 / steps as f32;
        let wobble = if index % 5 == 0 { 0.015 } else { 0.0 };
        points.push(StrokePoint::new(
            cx + (rx + wobble) * angle.cos(),
            cy + (ry - wobble * 0.5) * angle.sin(),
        ));
    }
    vec![DrawnStroke { points }]
}

fn rough_sphere(cx: f32, cy: f32, scale: f32) -> DrawnStroke {
    stroke_at(
        &[
            (0.50, 0.14),
            (0.76, 0.24),
            (0.86, 0.52),
            (0.68, 0.80),
            (0.38, 0.84),
            (0.16, 0.62),
            (0.22, 0.30),
            (0.50, 0.14),
        ],
        cx,
        cy,
        scale,
    )
}

fn flat_rough_circle(cx: f32, cy: f32, scale: f32) -> DrawnStroke {
    stroke_at(
        &[
            (0.30, 0.24),
            (0.58, 0.20),
            (0.76, 0.32),
            (0.80, 0.58),
            (0.66, 0.76),
            (0.38, 0.78),
            (0.20, 0.60),
            (0.18, 0.36),
            (0.30, 0.24),
        ],
        cx,
        cy,
        scale,
    )
}

fn satellite_seals(count: usize, orbit: f32, radius: f32) -> Vec<DrawnStroke> {
    (0..count)
        .flat_map(|index| {
            let angle =
                std::f32::consts::TAU * index as f32 / count as f32 + std::f32::consts::FRAC_PI_4;
            rough_circle(
                0.50 + orbit * angle.cos(),
                0.50 + orbit * angle.sin(),
                radius,
                radius * 0.92,
                16,
            )
        })
        .collect()
}

fn radial_spokes(count: usize, radius: f32) -> Vec<DrawnStroke> {
    (0..count)
        .map(|index| {
            let angle = std::f32::consts::TAU * index as f32 / count as f32;
            DrawnStroke {
                points: vec![
                    StrokePoint::new(0.50, 0.50),
                    StrokePoint::new(0.50 + radius * angle.cos(), 0.50 + radius * angle.sin()),
                ],
            }
        })
        .collect()
}

fn perimeter_ticks(count: usize, orbit: f32, half_len: f32) -> Vec<DrawnStroke> {
    (0..count)
        .map(|index| {
            let angle = std::f32::consts::TAU * index as f32 / count as f32;
            let center = StrokePoint::new(0.50 + orbit * angle.cos(), 0.50 + orbit * angle.sin());
            let tangent = angle + std::f32::consts::FRAC_PI_2;
            DrawnStroke {
                points: vec![
                    StrokePoint::new(
                        center.x - half_len * tangent.cos(),
                        center.y - half_len * tangent.sin(),
                    ),
                    StrokePoint::new(
                        center.x + half_len * tangent.cos(),
                        center.y + half_len * tangent.sin(),
                    ),
                ],
            }
        })
        .collect()
}

fn script_marks(count: usize, orbit: f32, half_len: f32) -> Vec<DrawnStroke> {
    (0..count)
        .map(|index| {
            let angle = std::f32::consts::TAU * (index as f32 + 0.35) / count as f32;
            let center = StrokePoint::new(0.50 + orbit * angle.cos(), 0.50 + orbit * angle.sin());
            let tangent = angle + std::f32::consts::FRAC_PI_2;
            let skew = if index % 2 == 0 { 0.55 } else { -0.55 };
            DrawnStroke {
                points: vec![
                    StrokePoint::new(
                        center.x - half_len * tangent.cos(),
                        center.y - half_len * tangent.sin(),
                    ),
                    StrokePoint::new(
                        center.x + half_len * tangent.cos(),
                        center.y + half_len * tangent.sin(),
                    ),
                    StrokePoint::new(
                        center.x
                            + half_len * (tangent + skew).cos()
                            + half_len * 0.35 * angle.cos(),
                        center.y
                            + half_len * (tangent + skew).sin()
                            + half_len * 0.35 * angle.sin(),
                    ),
                ],
            }
        })
        .collect()
}

fn template_at(rune_id: &str, cx: f32, cy: f32, scale: f32) -> Vec<DrawnStroke> {
    template_strokes_for_rune(rune_id)
        .unwrap()
        .into_iter()
        .map(|stroke| DrawnStroke {
            points: stroke
                .points
                .into_iter()
                .map(|point| {
                    StrokePoint::new(cx + (point.x - 0.5) * scale, cy + (point.y - 0.5) * scale)
                })
                .collect(),
        })
        .collect()
}

fn stroke_at(points: &[(f32, f32)], cx: f32, cy: f32, scale: f32) -> DrawnStroke {
    DrawnStroke {
        points: points
            .iter()
            .map(|(x, y)| StrokePoint::new(cx + (x - 0.5) * scale, cy + (y - 0.5) * scale))
            .collect(),
    }
}
