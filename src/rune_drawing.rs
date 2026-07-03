//! Freehand rune stroke capture and local template recognition.

use crate::data::RuneDef;
use serde::{Deserialize, Serialize};

#[cfg(test)]
pub(crate) mod samples;
mod scoring;
mod shape;
mod templates;

use scoring::{adjusted_score_for_rune, NormalizedDrawing};
pub(crate) use scoring::stroke_assignment;
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
    /// Drawn arc length ÷ the winning template's arc length, both measured
    /// after normalization to their own bounding box — so this isolates
    /// "was the stroke drawn to completion" from overall size. 1.0 is a
    /// full-length trace; a corner-cut or trailed-off stroke reads below
    /// 1.0. Feeds `potency` (magnitude channel, Phase 2) — see prd.md §4.
    #[serde(default = "default_ink_ratio")]
    pub ink_ratio: f32,
}

fn default_ink_ratio() -> f32 {
    1.0
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
    // Fade the ambiguity penalty in smoothly as the margin narrows, instead
    // of snapping a fixed ×0.92/×0.96 on the instant score_gap dips below
    // MIN_RECOGNITION_MARGIN — a 1-pixel wiggle that nudges the gap from
    // just above to just below the line should not visibly jump the
    // readout. Full penalty at gap=0, none at gap>=MIN_RECOGNITION_MARGIN,
    // matching the old step's endpoints exactly.
    let margin_relief = (score_gap / MIN_RECOGNITION_MARGIN).clamp(0.0, 1.0);
    confidence *= 1.0 - (1.0 - margin_relief) * 0.08;
    quality *= 1.0 - (1.0 - margin_relief) * 0.04;
    quality = quality.clamp(0.0, 1.0);
    let accepted = confidence >= MIN_RECOGNITION_CONFIDENCE
        && (!ambiguous || confidence >= AMBIGUOUS_ACCEPTANCE_CONFIDENCE);
    let ink_ratio = template_variants_for_rune(&rune_id)
        .into_iter()
        .filter_map(|template| NormalizedDrawing::from_strokes(&template))
        .max_by(|a, b| {
            adjusted_score_for_rune(&rune_id, &strokes, &candidate, a)
                .total_cmp(&adjusted_score_for_rune(&rune_id, &strokes, &candidate, b))
        })
        .map(|template| candidate.total_length() / template.total_length().max(0.01))
        .unwrap_or(1.0);

    Some(RecognitionOutcome {
        rune_id,
        confidence,
        quality,
        score_gap,
        ambiguous,
        alternatives,
        accepted,
        ink_ratio,
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

/// Target spacing between points after canonicalization — finer than the
/// 0.004 minimum capture spacing so shape/corner detail survives, coarse
/// enough that a fast and a slow draw of the same mark converge on
/// (near-)identical stored points.
pub const CANONICAL_STROKE_SPACING: f32 = 0.01;

/// Resamples a just-finished stroke to a fixed arc-length spacing, so the
/// *stored* ink no longer carries the drawing device's frame rate / mouse
/// speed (A8) — everything downstream (identity, quality, corner detection,
/// corpus capture) consumes this canonical form. Call once, when a stroke
/// ends; resampling live while the pen is still down would fight the
/// in-progress visual feedback for no benefit.
pub fn canonicalize_stroke(stroke: DrawnStroke) -> DrawnStroke {
    let length = polyline_length(&stroke.points);
    if stroke.points.len() < 2 || length <= 0.0001 {
        return stroke;
    }
    let target_count = ((length / CANONICAL_STROKE_SPACING).round() as usize + 1).max(2);
    DrawnStroke {
        points: resample_arc_length(&stroke.points, target_count),
    }
}

fn resample_arc_length(points: &[StrokePoint], target_count: usize) -> Vec<StrokePoint> {
    if points.len() < 2 || target_count < 2 {
        return points.to_vec();
    }
    let total = polyline_length(points);
    if total <= 0.0001 {
        return vec![points[0]; target_count];
    }
    (0..target_count)
        .map(|index| {
            let target = total * index as f32 / (target_count - 1) as f32;
            point_at_distance(points, target)
        })
        .collect()
}

fn point_at_distance(points: &[StrokePoint], target: f32) -> StrokePoint {
    let mut walked = 0.0;
    for segment in points.windows(2) {
        let step = segment[0].distance(segment[1]);
        if step <= 0.0 {
            continue;
        }
        if walked + step >= target {
            let t = ((target - walked) / step).clamp(0.0, 1.0);
            return StrokePoint::new(
                segment[0].x + (segment[1].x - segment[0].x) * t,
                segment[0].y + (segment[1].y - segment[0].y) * t,
            );
        }
        walked += step;
    }
    *points.last().unwrap()
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
