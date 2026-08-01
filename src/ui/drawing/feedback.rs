//! Everything drawn *on top of* the slate once Interpret has run.
//!
//! Nothing here shows while the pen is down — the uncertainty is the mechanic.
//! Kept apart from `drawing.rs` (which owns the slate surface, the ink, the
//! guides and the control strip) because this is the read-back layer: it only
//! ever consumes a finished `DiagramInterpretation` and never produces a
//! `UiAction`.

use crate::rune_diagram::DiagramInterpretation;
use crate::ui::canvas::point_to_screen;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;

/// The reading, revealed over the slate after Interpret.
///
/// This is the whole teaching surface for the placement grammar
/// (`.project/placement-rules.md` §4): the player commits to a drawing, presses
/// Interpret, and the game reads their handwriting back to them.
pub(super) fn draw_reading(lines: &[String], slate: Rect) {
    if lines.is_empty() {
        return;
    }
    let line_height = 19.0;
    let card = Rect::new(
        slate.x + 14.0,
        slate.bottom() - 22.0 - lines.len() as f32 * line_height,
        slate.w - 28.0,
        14.0 + lines.len() as f32 * line_height,
    );
    draw_surface(
        card,
        &SurfaceStyle::new(Color::new(0.94, 0.88, 0.72, 0.93))
            .with_border(1.0, Color::new(0.42, 0.27, 0.12, 0.70)),
    );
    for (index, line) in lines.iter().enumerate() {
        draw_text_block(
            line,
            card.x + 12.0,
            card.y + 6.0 + index as f32 * line_height,
            card.w - 24.0,
            line_height,
            14.0,
            1.0,
            Color::new(0.10, 0.06, 0.03, 1.0),
        );
    }
}

/// Which side of reference size a rune's magnitude channel landed on.
///
/// The split is the potency curve's own — `recognition::potency_from_scale_ratio`
/// anchors 1.0 at a category's reference size drawn with a fully-traced stroke —
/// and the ±0.12 dead band around it keeps a mark that is merely hand-drawn from
/// reading as a mistake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PotencyBand {
    Weak,
    Reference,
    Strong,
}

impl PotencyBand {
    fn of(potency: f32) -> Self {
        if potency < 0.88 {
            Self::Weak
        } else if potency > 1.12 {
            Self::Strong
        } else {
            Self::Reference
        }
    }

    fn color(self) -> Color {
        match self {
            Self::Weak => Color::new(0.95, 0.56, 0.44, 1.0),
            Self::Reference => Color::new(0.60, 0.92, 0.64, 1.0),
            Self::Strong => Color::new(0.97, 0.80, 0.38, 1.0),
        }
    }
}

const POTENCY_TAG_FONT: f32 = 12.0;
const POTENCY_TAG_HEIGHT: f32 = 16.0;

/// Per-rune potency, tagged onto the mark that earned it.
///
/// Potency is a per-rune magnitude channel (prd.md §4), but the ledger status
/// row can only afford one figure for the whole diagram, and an average only
/// says *that* something was drawn off-size. The tag says *which* mark — the
/// only form of the number a player can act on, since the fix is to redraw that
/// one rune larger, smaller, or with its stroke finished.
pub(super) fn draw_potency_tags(diagram: &DiagramInterpretation, slate: Rect) {
    for rune in &diagram.runes {
        let center = point_to_screen(slate, rune.center);
        let label = format!("{}%", (rune.potency * 100.0).round() as i32);
        let width = measure_ui_text(&label, None, POTENCY_TAG_FONT as u16, 1.0).width + 10.0;
        // Under the mark, so the tag never covers the ink it is describing;
        // clamped so a rune drawn against an edge still gets a readable figure.
        let tag = Rect::new(
            (center.x - width * 0.5).clamp(slate.x + 3.0, slate.right() - width - 3.0),
            (center.y + slate.w.min(slate.h) * rune.scale * 0.5 + 3.0)
                .clamp(slate.y + 3.0, slate.bottom() - POTENCY_TAG_HEIGHT - 3.0),
            width,
            POTENCY_TAG_HEIGHT,
        );
        let color = PotencyBand::of(rune.potency).color();
        draw_surface(
            tag,
            &SurfaceStyle::new(Color::new(0.09, 0.06, 0.035, 0.80))
                .with_border(1.0, with_alpha(color, 0.55)),
        );
        draw_text_centered(
            &label,
            tag.center().x,
            tag.bottom() - 4.0,
            TextStyle::new(POTENCY_TAG_FONT, color),
        );
    }
}

pub(super) fn draw_spell_feedback(diagram: &DiagramInterpretation, slate: Rect) {
    let Some(spell) = &diagram.spell else {
        return;
    };
    let center = slate.center();
    let base_radius = slate.w.min(slate.h) * 0.42;
    let alpha = (0.14 + spell.complexity * 0.30).clamp(0.16, 0.48);
    let color = if spell.tier_rank >= 4 {
        Color::new(0.38, 0.82, 0.95, alpha)
    } else if spell.tier_rank >= 3 {
        Color::new(0.86, 0.64, 0.20, alpha)
    } else {
        Color::new(0.44, 0.70, 0.44, alpha)
    };
    for index in 0..spell.ring_count.clamp(1, 4) {
        let radius = base_radius * (0.88 - index as f32 * 0.12);
        draw_circle_lines(center.x, center.y, radius, 2.0, color);
    }
    for index in 0..spell.satellite_count.min(10) {
        let angle = std::f32::consts::TAU * index as f32 / spell.satellite_count.max(1) as f32;
        let point = vec2(
            center.x + angle.cos() * base_radius * 0.62,
            center.y + angle.sin() * base_radius * 0.62,
        );
        draw_circle_lines(point.x, point.y, 8.0 + spell.intensity * 5.0, 1.7, color);
    }
    for index in 0..spell.radial_count.min(8) {
        let angle = std::f32::consts::TAU * index as f32 / spell.radial_count.max(1) as f32;
        draw_line(
            center.x,
            center.y,
            center.x + angle.cos() * base_radius * 0.72,
            center.y + angle.sin() * base_radius * 0.72,
            1.2,
            with_alpha(color, color.a * 0.55),
        );
    }
    for index in 0..spell.script_mark_count.min(36) {
        let angle = std::f32::consts::TAU * index as f32 / spell.script_mark_count.max(1) as f32;
        let radius = base_radius * (0.40 + (index % 3) as f32 * 0.10);
        let point = vec2(
            center.x + angle.cos() * radius,
            center.y + angle.sin() * radius,
        );
        draw_circle(point.x, point.y, 1.6, with_alpha(color, color.a * 0.70));
    }
}

#[cfg(test)]
mod tests {
    use super::PotencyBand;

    /// The bands have to agree with `recognition::potency_for_rune`'s own
    /// anchors, or a rune drawn at exactly reference size would be flagged.
    #[test]
    fn reference_size_rune_reads_as_reference() {
        assert_eq!(PotencyBand::of(1.0), PotencyBand::Reference);
        assert_eq!(PotencyBand::of(0.88), PotencyBand::Reference);
        assert_eq!(PotencyBand::of(1.12), PotencyBand::Reference);
    }

    /// `potency_for_rune` clamps to [0.35, 2.2]; both ends have to band.
    #[test]
    fn clamped_extremes_band_apart() {
        assert_eq!(PotencyBand::of(0.35), PotencyBand::Weak);
        assert_eq!(PotencyBand::of(2.2), PotencyBand::Strong);
    }
}
