//! Shared slate fixtures: sessions, and the stroke builders that draw
//! working circles, runes, and structural marks onto them.

use crate::data::{CommissionDef, GameData};
use crate::rune_drawing::test_support::perturb;
use crate::rune_drawing::{template_strokes_for_rune, DrawnStroke, StrokePoint};
use crate::state::{GameSession, TutorialStage};

pub(super) fn data() -> GameData {
    GameData::load().unwrap()
}

pub(super) fn unlocked_session(data: &GameData) -> GameSession {
    let mut session = GameSession::new(&data.config);
    session.start_playing();
    session.player.tutorial_stage = TutorialStage::Complete;
    session
}

pub(super) fn placed_ids(session: &GameSession) -> Vec<String> {
    session
        .board
        .placed
        .iter()
        .map(|rune| rune.rune_id.clone())
        .collect()
}

pub(super) fn circled_diagram(runes: &[(&str, f32, f32)]) -> Vec<DrawnStroke> {
    let mut strokes = outer_circle();
    for (rune_id, x, y) in runes {
        strokes.extend(template_at(rune_id, *x, *y, 0.18));
    }
    strokes
}

pub(super) fn circled_order(order: &CommissionDef) -> Vec<DrawnStroke> {
    circled_diagram(&[
        (order.required_effect.as_str(), 0.26, 0.50),
        (order.required_shape.as_str(), 0.50, 0.50),
        (order.required_trigger.as_str(), 0.74, 0.50),
    ])
}

/// A working circle with a fixed `gravity` + `sphere` + `continuous` + `safer` core plus
/// caller-chosen structural-mark counts — used to compare how score scales with structure
/// well past `magical_circle.rs`'s old fixed targets (rings 3, satellites 5, radials 4,
/// perimeter 14, scripts 28).
pub(super) fn structured_circle(
    rings: usize,
    satellites: usize,
    radials: usize,
    perimeter: usize,
    scripts: usize,
) -> Vec<DrawnStroke> {
    let mut strokes = outer_circle();
    for index in 0..rings {
        // Each concentric ring's diameter stays within `ReinforcementRing`'s valid scale band
        // (0.28..=0.92 relative to the working circle) for up to 6 rings.
        let rx = 0.13 + index as f32 * 0.014;
        strokes.extend(rough_circle(0.50, 0.50, rx, rx * 0.95, 28));
    }
    strokes.extend(template_at("gravity", 0.28, 0.48, 0.22));
    strokes.extend(template_at("sphere", 0.50, 0.50, 0.18));
    strokes.extend(template_at("continuous", 0.73, 0.48, 0.17));
    strokes.extend(template_at("safer", 0.50, 0.73, 0.15));
    strokes.extend(satellite_seals(satellites, 0.30, 0.038));
    strokes.extend(radial_spokes(radials, 0.31));
    strokes.extend(perimeter_ticks(perimeter, 0.39, 0.016));
    strokes.extend(script_marks(scripts, 0.27, 0.008));
    strokes
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

/// A circle whose quality (~0.30) sits between `Sandbox`'s 0.24 acceptance floor and
/// `Commission`'s 0.32 one — an off-center, elongated, incomplete arc.
pub(super) fn weak_partial_circle() -> Vec<DrawnStroke> {
    let mut points = Vec::new();
    for index in 0..=28 {
        let angle = std::f32::consts::TAU * 0.6 * index as f32 / 28.0;
        points.push(StrokePoint::new(
            0.60 + 0.32 * angle.cos(),
            0.60 + 0.16 * angle.sin(),
        ));
    }
    vec![DrawnStroke { points }]
}

pub(super) fn outer_circle() -> Vec<DrawnStroke> {
    let mut points = Vec::new();
    for index in 0..=36 {
        let angle = std::f32::consts::TAU * index as f32 / 36.0;
        points.push(StrokePoint::new(
            0.50 + 0.42 * angle.cos(),
            0.50 + 0.40 * angle.sin(),
        ));
    }
    vec![DrawnStroke { points }]
}

pub(super) fn rough_circle(cx: f32, cy: f32, rx: f32, ry: f32, steps: usize) -> Vec<DrawnStroke> {
    let mut points = Vec::new();
    for index in 0..=steps {
        let angle = std::f32::consts::TAU * index as f32 / steps as f32;
        points.push(StrokePoint::new(
            cx + rx * angle.cos(),
            cy + ry * angle.sin(),
        ));
    }
    vec![DrawnStroke { points }]
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

pub(super) fn stroke_at(points: &[(f32, f32)], cx: f32, cy: f32, scale: f32) -> DrawnStroke {
    DrawnStroke {
        points: points
            .iter()
            .map(|(x, y)| StrokePoint::new(cx + (x - 0.5) * scale, cy + (y - 0.5) * scale))
            .collect(),
    }
}

pub(super) fn template_at(rune_id: &str, cx: f32, cy: f32, scale: f32) -> Vec<DrawnStroke> {
    place(template_strokes_for_rune(rune_id).unwrap(), cx, cy, scale)
}

/// Plan Phase 5 item 4: a degraded (but not sloppy) hand — roughly a 15% scale
/// wobble plus per-point noise — applied to a clean template before it is
/// placed on the slate. Reuses `perturb`, the same seeded jitter the
/// confusion-matrix gate (`rune_drawing::confusion_gate`) already exercises.
pub(super) fn jittered_template_at(
    rune_id: &str,
    cx: f32,
    cy: f32,
    scale: f32,
    seed: u64,
) -> Vec<DrawnStroke> {
    let raw = template_strokes_for_rune(rune_id).unwrap();
    place(
        perturb(&raw, 0.85, (0.02, -0.02), 0.015, seed),
        cx,
        cy,
        scale,
    )
}

/// Moves template-space strokes (a unit box centered on 0.5, 0.5) onto the
/// slate at `scale`, centered on `cx`/`cy`.
fn place(strokes: Vec<DrawnStroke>, cx: f32, cy: f32, scale: f32) -> Vec<DrawnStroke> {
    strokes
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

pub(super) fn degraded_circled_order(order: &CommissionDef) -> Vec<DrawnStroke> {
    let mut strokes = outer_circle();
    strokes.extend(jittered_template_at(
        &order.required_effect,
        0.26,
        0.50,
        0.18,
        11,
    ));
    strokes.extend(jittered_template_at(
        &order.required_shape,
        0.50,
        0.50,
        0.18,
        22,
    ));
    strokes.extend(jittered_template_at(
        &order.required_trigger,
        0.74,
        0.50,
        0.18,
        33,
    ));
    strokes
}

/// Layout for structure-demanding commissions: the proven degraded rune row
/// (same spots/seeds as `degraded_circled_order`), reinforcement rings
/// concentric at center, satellite seals clustered along the top, sub-scope
/// vents in the bottom corners (volcano-fixture geometry). Runes are drawn
/// with the same degraded hand as `degraded_circled_order`.
pub(super) fn degraded_structured_order(order: &CommissionDef) -> Vec<DrawnStroke> {
    let mut strokes = degraded_circled_order(order);
    for index in 0..order.required_structure.rings {
        let radius = 0.13 + index as f32 * 0.014;
        strokes.extend(rough_circle(0.50, 0.50, radius, radius * 0.95, 28));
    }
    let satellite_spots = [(0.42, 0.28), (0.53, 0.26), (0.64, 0.30)];
    assert!(
        order.required_structure.satellites <= satellite_spots.len(),
        "add satellite spots for {}",
        order.id
    );
    for spot in satellite_spots
        .iter()
        .take(order.required_structure.satellites)
    {
        strokes.extend(rough_circle(spot.0, spot.1, 0.045, 0.0414, 16));
    }
    strokes.extend(radial_spokes(order.required_structure.radials, 0.31));
    strokes.extend(perimeter_ticks(
        order.required_structure.perimeter,
        0.39,
        0.016,
    ));
    strokes.extend(script_marks(order.required_structure.scripts, 0.27, 0.008));
    let vent_spots = [(0.24, 0.76), (0.76, 0.76)];
    assert!(
        order.required_sub_scopes <= vent_spots.len(),
        "add vent spots for {}",
        order.id
    );
    for (index, spot) in vent_spots
        .iter()
        .take(order.required_sub_scopes)
        .enumerate()
    {
        strokes.extend(rough_circle(spot.0, spot.1, 0.125, 0.119, 32));
        strokes.extend(jittered_template_at(
            "force",
            spot.0 - 0.07,
            spot.1,
            0.17,
            44 + index as u64,
        ));
        strokes.extend(jittered_template_at(
            "fire",
            spot.0 + 0.07,
            spot.1,
            0.17,
            55 + index as u64,
        ));
    }
    strokes
}

pub(super) fn rough_circled_diagram() -> Vec<DrawnStroke> {
    let mut strokes = vec![DrawnStroke {
        points: vec![
            StrokePoint::new(0.24, 0.46),
            StrokePoint::new(0.28, 0.20),
            StrokePoint::new(0.50, 0.08),
            StrokePoint::new(0.78, 0.18),
            StrokePoint::new(0.60, 0.38),
            StrokePoint::new(0.88, 0.44),
            StrokePoint::new(0.82, 0.70),
            StrokePoint::new(0.56, 0.60),
            StrokePoint::new(0.62, 0.86),
            StrokePoint::new(0.34, 0.80),
            StrokePoint::new(0.40, 0.58),
            StrokePoint::new(0.14, 0.62),
        ],
    }];
    strokes.extend(rough_light(0.26, 0.42, 0.15));
    strokes.push(rough_sphere(0.50, 0.40, 0.13));
    strokes.push(rough_continuous_pair(0.50, 0.66, 0.20));
    strokes
}

pub(super) fn rough_light(cx: f32, cy: f32, scale: f32) -> Vec<DrawnStroke> {
    vec![
        stroke_at(&[(0.27, 0.50), (0.75, 0.48)], cx, cy, scale),
        stroke_at(&[(0.50, 0.82), (0.50, 0.18)], cx, cy, scale),
        stroke_at(&[(0.67, 0.28), (0.34, 0.75)], cx, cy, scale),
        stroke_at(&[(0.30, 0.33), (0.70, 0.70)], cx, cy, scale),
        stroke_at(&[(0.42, 0.43), (0.58, 0.57)], cx, cy, scale),
    ]
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

/// Both `continuous` diamonds in one unbroken stroke — the shape a hand
/// draws when it never lifts the pen between them.
pub(super) fn rough_continuous_pair(cx: f32, cy: f32, scale: f32) -> DrawnStroke {
    stroke_at(
        &[
            (0.16, 0.55),
            (0.32, 0.30),
            (0.50, 0.52),
            (0.67, 0.78),
            (0.85, 0.52),
            (0.68, 0.27),
            (0.50, 0.48),
            (0.33, 0.75),
            (0.16, 0.55),
        ],
        cx,
        cy,
        scale,
    )
}

pub(super) fn rough_continuous_diamonds(cx: f32, cy: f32, scale: f32) -> Vec<DrawnStroke> {
    vec![
        stroke_at(
            &[
                (0.16, 0.50),
                (0.32, 0.22),
                (0.50, 0.50),
                (0.32, 0.78),
                (0.16, 0.50),
            ],
            cx,
            cy,
            scale,
        ),
        stroke_at(
            &[
                (0.50, 0.50),
                (0.68, 0.22),
                (0.84, 0.50),
                (0.68, 0.78),
                (0.50, 0.50),
            ],
            cx,
            cy,
            scale,
        ),
    ]
}

pub(super) fn flat_rough_circle(cx: f32, cy: f32, scale: f32) -> DrawnStroke {
    stroke_at(
        &[
            (0.40, 0.18),
            (0.64, 0.18),
            (0.80, 0.30),
            (0.84, 0.56),
            (0.72, 0.76),
            (0.48, 0.84),
            (0.26, 0.74),
            (0.16, 0.52),
            (0.20, 0.32),
            (0.40, 0.18),
        ],
        cx,
        cy,
        scale,
    )
}

pub(super) fn rough_touch_arrow(cx: f32, cy: f32, scale: f32) -> Vec<DrawnStroke> {
    vec![
        stroke_at(&[(0.52, 0.14), (0.52, 0.82)], cx, cy, scale),
        stroke_at(&[(0.30, 0.58), (0.52, 0.82)], cx, cy, scale),
        stroke_at(&[(0.72, 0.58), (0.52, 0.82)], cx, cy, scale),
    ]
}
