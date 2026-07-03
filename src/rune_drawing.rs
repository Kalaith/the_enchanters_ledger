//! Freehand rune stroke capture and local template recognition.

use crate::data::RuneDef;
use serde::{Deserialize, Serialize};

#[cfg(test)]
pub(crate) mod samples;
mod scoring;
mod shape;
mod templates;

use scoring::{adjusted_score_for_rune, NormalizedDrawing};
pub(crate) use shape::{shape_report_for_rune, ShapeIssue};
#[cfg(test)]
pub(crate) use templates::raw;
pub use templates::template_strokes_for_rune;
pub(crate) use templates::template_variants_for_rune;

pub const MIN_RECOGNITION_CONFIDENCE: f32 = 0.32;
pub const MIN_RECOGNITION_MARGIN: f32 = 0.04;
const AMBIGUOUS_ACCEPTANCE_CONFIDENCE: f32 = 0.58;

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

const MIN_ERASE_FRAGMENT_LENGTH: f32 = 0.008;

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
                push_erase_fragment(&mut chunks, std::mem::take(&mut current));
            } else {
                current.push(point);
            }
        }
        push_erase_fragment(&mut chunks, current);
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

fn push_erase_fragment(chunks: &mut Vec<DrawnStroke>, points: Vec<StrokePoint>) {
    // Slivers left behind by the eraser read as extra strokes and poison
    // template stroke counts, so drop them instead of keeping debris.
    if points.len() >= 2 && polyline_length(&points) >= MIN_ERASE_FRAGMENT_LENGTH {
        chunks.push(DrawnStroke { points });
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecognitionCandidate {
    pub rune_id: String,
    pub confidence: f32,
    pub quality: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecognitionOutcome {
    pub rune_id: String,
    pub confidence: f32,
    pub quality: f32,
    #[serde(default)]
    pub score_gap: f32,
    #[serde(default)]
    pub ambiguous: bool,
    #[serde(default)]
    pub alternatives: Vec<RecognitionCandidate>,
    pub accepted: bool,
}

pub fn recognize_rune<'a>(
    strokes: &[DrawnStroke],
    runes: impl IntoIterator<Item = &'a RuneDef>,
) -> Option<RecognitionOutcome> {
    let strokes = merge_continuation_strokes(strokes);
    let candidate = NormalizedDrawing::from_strokes(&strokes)?;
    let mut scores = runes
        .into_iter()
        .filter_map(|rune| {
            let best_score = template_variants_for_rune(&rune.id)
                .into_iter()
                .filter_map(|template| NormalizedDrawing::from_strokes(&template))
                .map(|template| adjusted_score_for_rune(&rune.id, &strokes, &candidate, &template))
                .max_by(|a, b| a.total_cmp(b))?;
            let strict = crate::rune_quality::strict_quality_for_rune(&rune.id, &strokes)
                .unwrap_or(best_score);
            let quality = (best_score * (0.70 + strict * 0.30)).clamp(0.0, 1.0);
            Some((rune.id.clone(), best_score, quality))
        })
        .collect::<Vec<_>>();
    scores.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let (rune_id, mut confidence, mut quality) = scores.first().cloned()?;
    let raw_confidence = confidence;
    let alternatives = scores
        .iter()
        .skip(1)
        .take(3)
        .map(|(rune_id, confidence, quality)| RecognitionCandidate {
            rune_id: rune_id.clone(),
            confidence: *confidence,
            quality: *quality,
        })
        .collect::<Vec<_>>();
    let score_gap = alternatives
        .first()
        .map_or(raw_confidence, |candidate| {
            raw_confidence - candidate.confidence
        })
        .max(0.0);
    let ambiguous = score_gap < MIN_RECOGNITION_MARGIN;
    if let Some((_, second, _)) = scores.get(1) {
        let gap = confidence - *second;
        if gap < MIN_RECOGNITION_MARGIN {
            confidence *= 0.92;
            quality *= 0.96;
        }
    }
    quality = quality.clamp(0.0, 1.0);
    let accepted = confidence >= MIN_RECOGNITION_CONFIDENCE
        && (!ambiguous || confidence >= AMBIGUOUS_ACCEPTANCE_CONFIDENCE);

    Some(RecognitionOutcome {
        rune_id,
        confidence,
        quality,
        score_gap,
        ambiguous,
        alternatives,
        accepted,
    })
}

const MERGE_MAX_GAP: f32 = 0.02;
const MERGE_MAX_TURN_COS: f32 = 0.82;
const MERGE_DIRECTION_SPAN: f32 = 0.02;

/// Rejoins strokes that continue each other smoothly across a tiny gap, so a
/// mark repaired after erasing (or a shape drawn in several passes) reads as
/// one stroke again. Sharp joints such as arrowheads are left untouched.
pub(crate) fn merge_continuation_strokes(strokes: &[DrawnStroke]) -> Vec<DrawnStroke> {
    let mut merged = strokes
        .iter()
        .filter(|stroke| stroke.has_ink())
        .cloned()
        .collect::<Vec<_>>();
    'joining: loop {
        for left in 0..merged.len() {
            for right in (left + 1)..merged.len() {
                if let Some(joined) = join_continuation(&merged[left], &merged[right]) {
                    merged[left] = joined;
                    merged.remove(right);
                    continue 'joining;
                }
            }
        }
        return merged;
    }
}

fn join_continuation(a: &DrawnStroke, b: &DrawnStroke) -> Option<DrawnStroke> {
    if is_effectively_closed(a) || is_effectively_closed(b) {
        return None;
    }
    let a_length = polyline_length(&a.points);
    let b_length = polyline_length(&b.points);
    let gap_limit = MERGE_MAX_GAP.min(a_length.min(b_length) * 0.15);
    for flip_a in [false, true] {
        for flip_b in [false, true] {
            let a_junction = oriented_end(a, flip_a)?;
            let b_junction = oriented_start(b, flip_b)?;
            if a_junction.distance(b_junction) > gap_limit {
                continue;
            }
            let mut head = oriented_points(a, flip_a);
            let tail = oriented_points(b, flip_b);
            let (Some(arrive), Some(depart)) = (end_direction(&head), start_direction(&tail))
            else {
                continue;
            };
            if arrive.0 * depart.0 + arrive.1 * depart.1 < MERGE_MAX_TURN_COS {
                continue;
            }
            head.extend(tail);
            return Some(DrawnStroke { points: head });
        }
    }
    None
}

fn is_effectively_closed(stroke: &DrawnStroke) -> bool {
    let (Some(first), Some(last)) = (stroke.points.first(), stroke.points.last()) else {
        return false;
    };
    let length = polyline_length(&stroke.points);
    length > 0.0 && first.distance(*last) <= (length * 0.10).min(0.05)
}

fn oriented_start(stroke: &DrawnStroke, flipped: bool) -> Option<StrokePoint> {
    if flipped {
        stroke.points.last().copied()
    } else {
        stroke.points.first().copied()
    }
}

fn oriented_end(stroke: &DrawnStroke, flipped: bool) -> Option<StrokePoint> {
    oriented_start(stroke, !flipped)
}

fn oriented_points(stroke: &DrawnStroke, flipped: bool) -> Vec<StrokePoint> {
    if flipped {
        stroke.points.iter().rev().copied().collect()
    } else {
        stroke.points.clone()
    }
}

fn end_direction(points: &[StrokePoint]) -> Option<(f32, f32)> {
    let end = *points.last()?;
    let anchor = point_before_span(points.iter().rev().copied(), end)?;
    normalized_direction(anchor, end)
}

fn start_direction(points: &[StrokePoint]) -> Option<(f32, f32)> {
    let start = *points.first()?;
    let anchor = point_before_span(points.iter().copied(), start)?;
    normalized_direction(start, anchor)
}

fn point_before_span(
    points: impl Iterator<Item = StrokePoint>,
    from: StrokePoint,
) -> Option<StrokePoint> {
    let mut walked = 0.0;
    let mut previous = from;
    let mut anchor = None;
    for point in points.skip(1) {
        walked += previous.distance(point);
        previous = point;
        anchor = Some(point);
        if walked >= MERGE_DIRECTION_SPAN {
            break;
        }
    }
    anchor
}

fn normalized_direction(from: StrokePoint, to: StrokePoint) -> Option<(f32, f32)> {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let length = (dx * dx + dy * dy).sqrt();
    (length > 0.0001).then_some((dx / length, dy / length))
}

fn polyline_length(points: &[StrokePoint]) -> f32 {
    points
        .windows(2)
        .map(|segment| segment[0].distance(segment[1]))
        .sum()
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
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod confusion_gate;
#[cfg(test)]
mod property_tests;
