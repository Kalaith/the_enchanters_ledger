//! Shared diagram fixtures: rune sets to interpret against, and the stroke
//! builders that draw working circles, runes, and structural marks.

use crate::data::{GameData, RuneDef};
use crate::rune_drawing::{template_strokes_for_rune, DrawnStroke, StrokePoint};

pub(super) fn rank_one(data: &GameData) -> Vec<&RuneDef> {
    data.runes.iter().filter(|rune| rune.tier == 1).collect()
}

pub(super) fn all_runes(data: &GameData) -> Vec<&RuneDef> {
    data.runes.iter().collect()
}

pub(super) fn outer_circle() -> Vec<DrawnStroke> {
    rough_circle(0.50, 0.50, 0.42, 0.40, 36)
}

pub(super) fn high_tier_city_circle() -> Vec<DrawnStroke> {
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

pub(super) fn rough_circle(cx: f32, cy: f32, rx: f32, ry: f32, steps: usize) -> Vec<DrawnStroke> {
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

pub(super) fn rough_sphere(cx: f32, cy: f32, scale: f32) -> DrawnStroke {
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

pub(super) fn flat_rough_circle(cx: f32, cy: f32, scale: f32) -> DrawnStroke {
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

pub(super) fn satellite_seals(count: usize, orbit: f32, radius: f32) -> Vec<DrawnStroke> {
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

pub(super) fn radial_spokes(count: usize, radius: f32) -> Vec<DrawnStroke> {
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

pub(super) fn perimeter_ticks(count: usize, orbit: f32, half_len: f32) -> Vec<DrawnStroke> {
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

pub(super) fn script_marks(count: usize, orbit: f32, half_len: f32) -> Vec<DrawnStroke> {
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

pub(super) fn template_at(rune_id: &str, cx: f32, cy: f32, scale: f32) -> Vec<DrawnStroke> {
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

pub(super) fn stroke_at(points: &[(f32, f32)], cx: f32, cy: f32, scale: f32) -> DrawnStroke {
    DrawnStroke {
        points: points
            .iter()
            .map(|(x, y)| StrokePoint::new(cx + (x - 0.5) * scale, cy + (y - 0.5) * scale))
            .collect(),
    }
}
