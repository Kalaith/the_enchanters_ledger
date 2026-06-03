//! Whole-diagram interpretation built on top of rune stroke recognition.

use crate::data::RuneDef;
use crate::magical_circle::{
    analyze_magical_circle, classify_circle_stroke, CircleBounds, CircleMark, CircleStrokeKind,
    MagicalCircleSpell,
};
use crate::rune_drawing::{recognize_rune, DrawnStroke, StrokePoint};
use serde::{Deserialize, Serialize};

mod circle;
mod geometry;
mod recognition;
#[cfg(test)]
mod tests;

pub(crate) use circle::{circle_quality, is_inside_working_circle, select_working_circle};
pub(crate) use geometry::{cluster_strokes, StrokeBounds};
use recognition::{extract_overlapped_spheres, push_recognized_rune};

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

    let circle_candidates = useful
        .iter()
        .filter_map(|(index, stroke)| {
            let bounds = StrokeBounds::from_stroke(stroke)?;
            circle_quality(stroke, bounds).map(|score| (*index, score, bounds))
        })
        .collect::<Vec<_>>();
    let Some((circle_index, circle_quality, circle_bounds)) =
        select_working_circle(&circle_candidates)
    else {
        return DiagramInterpretation {
            circle_found: false,
            ..Default::default()
        };
    };

    let circle_found = circle_quality >= MIN_CIRCLE_QUALITY;
    let available_runes = runes.into_iter().collect::<Vec<_>>();
    let spell_bounds = CircleBounds::new(
        circle_bounds.min_x,
        circle_bounds.min_y,
        circle_bounds.max_x,
        circle_bounds.max_y,
    );
    let classified_marks = useful
        .iter()
        .filter(|(index, _)| *index != circle_index)
        .filter_map(|(index, stroke)| {
            classify_circle_stroke(stroke, spell_bounds).map(|mark| (*index, mark))
        })
        .collect::<Vec<_>>();
    let inner_strokes = useful
        .into_iter()
        .filter(|(index, stroke)| {
            *index != circle_index
                && is_inside_working_circle(stroke, circle_bounds)
                && !is_circle_structure(*index, &classified_marks)
        })
        .map(|(index, stroke)| (index, stroke.clone()))
        .collect::<Vec<_>>();

    let clusters = cluster_strokes(&inner_strokes);
    let mut interpreted = Vec::new();
    let mut rejected_marks = 0;
    for cluster in clusters {
        if extract_overlapped_spheres(
            &cluster,
            &available_runes,
            circle_bounds,
            circle_quality,
            &mut interpreted,
            &mut rejected_marks,
        ) {
            continue;
        }

        let Some(recognized) = recognize_rune(&cluster.strokes, available_runes.iter().copied())
        else {
            rejected_marks += 1;
            continue;
        };
        push_recognized_rune(
            recognized,
            cluster.bounds,
            circle_bounds,
            circle_quality,
            &available_runes,
            &mut interpreted,
            &mut rejected_marks,
        );
    }

    interpreted.sort_by(|a, b| {
        a.center
            .y
            .total_cmp(&b.center.y)
            .then(a.center.x.total_cmp(&b.center.x))
    });
    let circle_marks = classified_marks
        .into_iter()
        .map(|(_, mark)| mark)
        .collect::<Vec<_>>();
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
