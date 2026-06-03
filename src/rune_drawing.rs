//! Freehand rune stroke capture and local template recognition.

use crate::data::RuneDef;
use serde::{Deserialize, Serialize};

mod scoring;
mod shape;
mod templates;

use scoring::{adjusted_score_for_rune, NormalizedDrawing};
pub(crate) use shape::{shape_report_for_rune, ShapeIssue};
#[cfg(test)]
pub(crate) use templates::raw;
pub use templates::template_strokes_for_rune;
use templates::template_variants_for_rune;

pub const MIN_RECOGNITION_CONFIDENCE: f32 = 0.32;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StrokePoint {
    pub x: f32,
    pub y: f32,
}

impl StrokePoint {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x: x.clamp(0.0, 1.0),
            y: y.clamp(0.0, 1.0),
        }
    }

    fn distance(self, other: StrokePoint) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DrawnStroke {
    pub points: Vec<StrokePoint>,
}

impl DrawnStroke {
    pub fn new(point: StrokePoint) -> Self {
        Self {
            points: vec![point],
        }
    }

    pub fn push(&mut self, point: StrokePoint) {
        if self
            .points
            .last()
            .is_none_or(|last| last.distance(point) >= 0.004)
        {
            self.points.push(point);
        }
    }

    pub fn has_ink(&self) -> bool {
        self.points.len() >= 2
    }
}

pub fn erase_strokes_at(strokes: &mut Vec<DrawnStroke>, center: StrokePoint, radius: f32) -> bool {
    let radius = radius.max(0.001);
    let mut changed = false;
    let mut revised = Vec::new();
    for stroke in strokes.drain(..) {
        if !stroke.has_ink() {
            continue;
        }
        let dense_points = densify_points(&stroke.points, (radius * 0.45).max(0.003));
        let mut chunks = Vec::<DrawnStroke>::new();
        let mut current = Vec::new();
        let mut erased_from_stroke = false;
        for point in dense_points {
            if point.distance(center) <= radius {
                erased_from_stroke = true;
                if current.len() >= 2 {
                    chunks.push(DrawnStroke { points: current });
                }
                current = Vec::new();
            } else {
                current.push(point);
            }
        }
        if current.len() >= 2 {
            chunks.push(DrawnStroke { points: current });
        }
        if erased_from_stroke {
            changed = true;
            revised.extend(chunks);
        } else {
            revised.push(stroke);
        }
    }
    *strokes = revised;
    changed
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecognitionOutcome {
    pub rune_id: String,
    pub confidence: f32,
    pub quality: f32,
    pub accepted: bool,
}

pub fn recognize_rune<'a>(
    strokes: &[DrawnStroke],
    runes: impl IntoIterator<Item = &'a RuneDef>,
) -> Option<RecognitionOutcome> {
    let candidate = NormalizedDrawing::from_strokes(strokes)?;
    let mut scores = runes
        .into_iter()
        .filter_map(|rune| {
            let best_score = template_variants_for_rune(&rune.id)
                .into_iter()
                .filter_map(|template| NormalizedDrawing::from_strokes(&template))
                .map(|template| adjusted_score_for_rune(&rune.id, strokes, &candidate, &template))
                .max_by(|a, b| a.total_cmp(b))?;
            let strict = crate::rune_quality::strict_quality_for_rune(&rune.id, strokes)
                .unwrap_or(best_score);
            let quality = (best_score * (0.70 + strict * 0.30)).clamp(0.0, 1.0);
            Some((rune.id.clone(), best_score, quality))
        })
        .collect::<Vec<_>>();
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    let (rune_id, mut confidence, mut quality) = scores.first().cloned()?;
    if let Some((_, second, _)) = scores.get(1) {
        let gap = confidence - *second;
        if gap < 0.04 {
            confidence *= 0.92;
            quality *= 0.96;
        }
    }
    quality = quality.clamp(0.0, 1.0);

    Some(RecognitionOutcome {
        rune_id,
        confidence,
        quality,
        accepted: confidence >= MIN_RECOGNITION_CONFIDENCE,
    })
}

fn densify_points(points: &[StrokePoint], max_gap: f32) -> Vec<StrokePoint> {
    let Some(first) = points.first().copied() else {
        return Vec::new();
    };
    let mut dense = vec![first];
    for segment in points.windows(2) {
        let start = segment[0];
        let end = segment[1];
        let steps = (start.distance(end) / max_gap.max(0.001)).ceil() as usize;
        for step in 1..=steps.max(1) {
            let t = step as f32 / steps.max(1) as f32;
            dense.push(StrokePoint::new(
                start.x + (end.x - start.x) * t,
                start.y + (end.y - start.y) * t,
            ));
        }
    }
    dense
}

#[cfg(test)]
mod tests;
