//! Whole-diagram interpretation built on top of rune stroke recognition.

use crate::data::RuneDef;
use crate::magical_circle::{
    analyze_magical_circle, CircleMark, CircleStrokeKind, MagicalCircleSpell,
};
use crate::rune_drawing::{DrawnStroke, StrokePoint};
use serde::{Deserialize, Serialize};

mod circle;
mod geometry;
mod recognition;
mod scope;
#[cfg(test)]
mod tests;

pub(crate) use circle::{gather_circle_candidates, is_inside_working_circle, select_working_circle_for_strokes};
pub(crate) use geometry::{cluster_strokes, StrokeBounds};
use scope::interpret_scope;

pub const MIN_CIRCLE_QUALITY: f32 = 0.32;
pub const MIN_DIAGRAM_RUNE_CONFIDENCE: f32 = 0.32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DiagramInterpretation {
    pub circle_quality: f32,
    pub circle_found: bool,
    pub runes: Vec<InterpretedRune>,
    pub rejected_marks: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spell: Option<MagicalCircleSpell>,
}

impl DiagramInterpretation {
    pub fn accepted(&self) -> bool {
        self.circle_found && self.circle_quality >= MIN_CIRCLE_QUALITY && !self.runes.is_empty()
    }

    pub fn average_rune_quality(&self) -> f32 {
        if self.runes.is_empty() {
            0.0
        } else {
            self.runes.iter().map(|rune| rune.quality).sum::<f32>() / self.runes.len() as f32
        }
    }

    /// Average magnitude channel across every rune found — 1.0 is
    /// reference size with a fully-traced stroke; see `InterpretedRune::potency`.
    pub fn average_rune_potency(&self) -> f32 {
        if self.runes.is_empty() {
            0.0
        } else {
            self.runes.iter().map(|rune| rune.potency).sum::<f32>() / self.runes.len() as f32
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterpretedRune {
    pub rune_id: String,
    pub confidence: f32,
    pub quality: f32,
    pub center: StrokePoint,
    #[serde(default)]
    pub scale: f32,
    #[serde(default)]
    pub orbit: f32,
    /// Magnitude channel (plan Phase 2): how strongly this rune's *size and
    /// completeness* — as opposed to its shape quality — should scale its
    /// effect. 1.0 at a category's reference size with a fully-traced
    /// stroke; see `recognition::potency_for_rune` and prd.md §4.
    #[serde(default = "default_potency")]
    pub potency: f32,
}

fn default_potency() -> f32 {
    1.0
}

pub fn interpret_diagram<'a>(
    strokes: &[DrawnStroke],
    runes: impl IntoIterator<Item = &'a RuneDef>,
) -> DiagramInterpretation {
    let useful = strokes
        .iter()
        .enumerate()
        .filter(|(_, stroke)| stroke.has_ink())
        .collect::<Vec<_>>();

    let circle_candidates = gather_circle_candidates(&useful);
    let Some((circle_member_indices, circle_quality, circle_bounds)) =
        select_working_circle_for_strokes(&circle_candidates, &useful)
    else {
        return DiagramInterpretation {
            circle_found: false,
            ..Default::default()
        };
    };

    let circle_found = circle_quality >= MIN_CIRCLE_QUALITY;
    let available_runes = runes.into_iter().collect::<Vec<_>>();
    let scope_ink = useful
        .into_iter()
        .filter(|(index, _)| !circle_member_indices.contains(index))
        .map(|(index, stroke)| (index, stroke.clone()))
        .collect::<Vec<_>>();

    let outcome = interpret_scope(&scope_ink, circle_bounds, &available_runes, 0);
    let mut interpreted = outcome.runes;
    let rejected_marks = outcome.rejected_marks;

    interpreted.sort_by(|a, b| {
        a.center
            .y
            .total_cmp(&b.center.y)
            .then(a.center.x.total_cmp(&b.center.x))
    });
    let circle_marks = outcome.circle_marks;
    let spell = if circle_found {
        analyze_magical_circle(
            circle_quality,
            &circle_marks,
            &interpreted,
            &available_runes,
        )
    } else {
        None
    };

    DiagramInterpretation {
        circle_quality,
        circle_found,
        runes: interpreted,
        rejected_marks,
        spell,
    }
}

pub(crate) fn is_circle_structure(index: usize, marks: &[(usize, CircleMark)]) -> bool {
    let reinforcement_count = marks
        .iter()
        .filter(|(_, mark)| mark.kind == CircleStrokeKind::ReinforcementRing && mark.quality > 0.48)
        .count();
    let satellite_count = marks
        .iter()
        .filter(|(_, mark)| mark.kind == CircleStrokeKind::SatelliteSeal && mark.quality > 0.68)
        .count();
    let script_count = marks
        .iter()
        .filter(|(_, mark)| mark.kind == CircleStrokeKind::ScriptMark && mark.quality > 0.42)
        .count();
    let radial_count = marks
        .iter()
        .filter(|(_, mark)| mark.kind == CircleStrokeKind::RadialSpoke && mark.quality > 0.68)
        .count();
    marks.iter().any(|(mark_index, mark)| {
        *mark_index == index
            && if mark.kind == CircleStrokeKind::SatelliteSeal {
                satellite_count >= 3 && mark.quality > 0.68
            } else if mark.kind == CircleStrokeKind::ReinforcementRing {
                reinforcement_count >= 2 && mark.quality > 0.48
            } else if mark.kind == CircleStrokeKind::ScriptMark {
                script_count >= 8 && mark.quality > 0.42
            } else if mark.kind == CircleStrokeKind::RadialSpoke {
                radial_count >= 6 && mark.quality > 0.68
            } else {
                mark.kind.is_circle_structure()
            }
    })
}
