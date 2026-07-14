//! The numbers a read reports — quality and potency — as distinct from the
//! identity it settles on.

use super::fixtures::*;
use crate::data::GameData;
use crate::rune_diagram::interpret_diagram;
use crate::rune_drawing::{DrawnStroke, StrokePoint};

#[test]
fn rune_quality_does_not_depend_on_board_position() {
    // D1: layout_quality's undocumented home-position bias was deleted for
    // freehand scoring (plan Phase 2 item 4) — a rune's *shape* quality
    // should depend on how well it was drawn, not where in the circle it
    // landed. Position bonuses/penalties belong to the future spell
    // grammar (Phase 4), where placement can mean something taught to the
    // player, not a silent recognition-quality tax.
    let data = GameData::load().unwrap();
    let centered = touch_quality_at(0.50, 0.50, &data);
    let off_center = touch_quality_at(0.26, 0.50, &data);

    assert!(
        (centered - off_center).abs() < 0.01,
        "centered={centered} off_center={off_center}"
    );
}

#[test]
fn doubling_effect_rune_scale_raises_potency() {
    // Phase 2 exit criterion: drawing the same effect rune at 2x size
    // measurably raises potency (and, in evaluate(), power). "light" is an
    // Effect rune; its ideal_scale_in_circle is 0.18, so scale 0.18 is
    // ~1.0x reference size and 0.36 is ~2x.
    let data = GameData::load().unwrap();
    let normal = potency_of("light", 0.18, &data);
    let doubled = potency_of("light", 0.36, &data);

    assert!(doubled > normal * 1.3, "normal={normal} doubled={doubled}");
}

#[test]
fn under_drawn_stroke_reads_but_reports_lower_potency() {
    // Phase 2 exit criterion: a half-hearted, short-stroked rune still
    // reads as itself (identity survives) but reports reduced potency
    // (magnitude does not) — this is the ink_ratio channel.
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.5, 0.5, 0.18));

    let full = interpret_diagram(&strokes, rank_one(&data));
    let full_rune = full
        .runes
        .iter()
        .find(|rune| rune.rune_id == "light")
        .unwrap();

    let mut short_strokes = outer_circle();
    short_strokes.extend(shortened_template_at("light", 0.5, 0.5, 0.18, 0.8));
    let short = interpret_diagram(&short_strokes, rank_one(&data));
    let short_rune = short
        .runes
        .iter()
        .find(|rune| rune.rune_id == "light")
        .unwrap();

    assert!(short.accepted(), "{short:?}");
    assert!(
        short_rune.potency < full_rune.potency * 0.9,
        "full={full_rune:?} short={short_rune:?}"
    );
}

fn potency_of(rune_id: &str, scale: f32, data: &GameData) -> f32 {
    let mut strokes = outer_circle();
    strokes.extend(template_at(rune_id, 0.5, 0.5, scale));
    let interpretation = interpret_diagram(&strokes, rank_one(data));
    interpretation
        .runes
        .iter()
        .find(|rune| rune.rune_id == rune_id)
        .map(|rune| rune.potency)
        .unwrap_or(0.0)
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

/// Every stroke of the template, cut short to `fraction` of its own length
/// (same start point, pulled-in end point) — simulates a hand that lifted
/// the pen early.
fn shortened_template_at(
    rune_id: &str,
    cx: f32,
    cy: f32,
    scale: f32,
    fraction: f32,
) -> Vec<DrawnStroke> {
    template_at(rune_id, cx, cy, scale)
        .into_iter()
        .map(|stroke| {
            let start = stroke.points[0];
            let end = *stroke.points.last().unwrap();
            DrawnStroke {
                points: vec![
                    start,
                    StrokePoint::new(
                        start.x + (end.x - start.x) * fraction,
                        start.y + (end.y - start.y) * fraction,
                    ),
                ],
            }
        })
        .collect()
}
