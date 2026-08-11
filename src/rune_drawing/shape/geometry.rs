//! Generic ink measurements. Nothing here knows what a rune is — each
//! function answers one geometric question about a stroke (or the whole
//! normalized drawing), and `shape.rs` wires them to the data-declared
//! `Feature` a rune's spec asks for.

use crate::rune_drawing::{DrawnStroke, StrokePoint};

/// The drawing's strokes rescaled into a square [0, 1] box (uniform scale,
/// centered on the shorter axis) so every measurement below is size- and
/// position-independent. `aspect_ratio` preserves the original proportions,
/// which uniform scaling would otherwise discard.
#[derive(Debug, Clone)]
pub(super) struct NormalizedInk {
    pub(super) strokes: Vec<Vec<StrokePoint>>,
    pub(super) aspect_ratio: f32,
}

impl NormalizedInk {
    pub(super) fn from_strokes(strokes: &[DrawnStroke]) -> Option<Self> {
        let strokes = strokes
            .iter()
            .filter(|stroke| stroke.has_ink())
            .cloned()
            .collect::<Vec<_>>();
        if strokes.is_empty() {
            return None;
        }

        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for point in strokes.iter().flat_map(|stroke| &stroke.points) {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }

        let width = (max_x - min_x).max(0.001);
        let height = (max_y - min_y).max(0.001);
        let span = width.max(height).max(0.06);
        let pad_x = (span - width) * 0.5;
        let pad_y = (span - height) * 0.5;
        let origin_x = min_x - pad_x;
        let origin_y = min_y - pad_y;
        let normalized = strokes
            .iter()
            .map(|stroke| {
                stroke
                    .points
                    .iter()
                    .map(|point| {
                        StrokePoint::new((point.x - origin_x) / span, (point.y - origin_y) / span)
                    })
                    .collect()
            })
            .collect();

        Some(Self {
            strokes: normalized,
            aspect_ratio: width / height,
        })
    }
}

pub(super) fn circularity(points: &[StrokePoint], aspect_ratio: f32) -> f32 {
    let center = StrokePoint::new(0.5, 0.5);
    let radii = points
        .iter()
        .map(|point| point.distance(center))
        .filter(|radius| *radius > 0.001)
        .collect::<Vec<_>>();
    if radii.is_empty() {
        return 0.0;
    }
    let mean = radii.iter().sum::<f32>() / radii.len() as f32;
    let variance = radii
        .iter()
        .map(|radius| {
            let delta = radius - mean;
            delta * delta
        })
        .sum::<f32>()
        / radii.len() as f32;
    let radius_score = (1.0 - variance.sqrt() / (mean * 0.46).max(0.001)).clamp(0.0, 1.0);
    let aspect_score = ratio_score(aspect_ratio, 1.0);
    let coverage_score = angle_coverage(points, center);
    (radius_score * 0.58 + aspect_score * 0.24 + coverage_score * 0.18).clamp(0.0, 1.0)
}

const CLOSED_CORNER_THRESHOLD: f32 = 0.60;
const OPEN_CORNER_THRESHOLD: f32 = 0.68;
// How sharply corner-ness turns on around the threshold. At ±0.1 rad from
// the threshold this already reaches ~0.88/0.12, so it reads as a count
// near a whole number for anything but genuinely marginal turns.
const CORNER_SIGMOID_SLOPE: f32 = 20.0;

/// A turn angle's corner-ness in [0, 1], via a logistic ramp centered on
/// `threshold` instead of a hard cutoff — a turn a few degrees short of the
/// line still contributes partial corner weight instead of vanishing
/// outright. Softens the A3 cliff where a hand-authored hexagon's corner at
/// 0.638 rad used to read as *no corner at all* against a 0.68 threshold.
fn corner_confidence(angle: f32, threshold: f32) -> f32 {
    1.0 / (1.0 + (-(CORNER_SIGMOID_SLOPE * (angle - threshold))).exp())
}

/// Sums the confidence of each local peak in a corner-confidence sequence.
/// Using peaks (not a straight sum) keeps one wide corner from being
/// counted several times over the samples it spans; `wraps` lets a closed
/// shape's peak search cross the seam between last and first sample.
fn sum_of_corner_peaks(confidences: &[f32], wraps: bool) -> f32 {
    let count = confidences.len();
    let mut total = 0.0;
    for index in 0..count {
        let value = confidences[index];
        if value < 0.05 {
            continue;
        }
        let previous = if index == 0 {
            if wraps {
                confidences[count - 1]
            } else {
                f32::NEG_INFINITY
            }
        } else {
            confidences[index - 1]
        };
        let next = if index + 1 == count {
            if wraps {
                confidences[0]
            } else {
                f32::NEG_INFINITY
            }
        } else {
            confidences[index + 1]
        };
        // >= previous, > next: a flat-topped peak is credited once, to its
        // later sample, rather than once per sample on the plateau.
        if value >= previous && value > next {
            total += value;
        }
    }
    total
}

/// A continuous estimate of how many corners a stroke has. Whole numbers for
/// clear-cut shapes (a clean square reads ~4.0), fractional near a
/// threshold-straddling corner instead of snapping straight to the nearest
/// integer — see `corner_confidence`.
pub(super) fn corner_count(points: &[StrokePoint]) -> f32 {
    let mut sampled = resample(points, 36);
    if sampled.len() < 5 {
        return 0.0;
    }
    // Only closed strokes may wrap around the ends; wrapping an open stroke
    // manufactures phantom corners at its endpoints.
    let closed = sampled
        .first()
        .zip(sampled.last())
        .is_some_and(|(first, last)| first.distance(*last) <= 0.18);
    if closed {
        // Drop the duplicated seam sample so a corner at the stroke's start
        // point is not counted twice.
        if sampled
            .first()
            .zip(sampled.last())
            .is_some_and(|(first, last)| first.distance(*last) <= 0.01)
        {
            sampled.pop();
        }
        let count = sampled.len();
        // Closed shapes get a lower threshold than open strokes, whose
        // corners tend to be sharper by construction (arrowheads, crosses).
        let confidences = (0..count)
            .map(|index| {
                let previous = sampled[(index + count - 2) % count];
                let next = sampled[(index + 2) % count];
                let angle = turn_angle(previous, sampled[index], next);
                corner_confidence(angle, CLOSED_CORNER_THRESHOLD)
            })
            .collect::<Vec<_>>();
        sum_of_corner_peaks(&confidences, true)
    } else {
        let confidences = (2..sampled.len() - 2)
            .map(|index| {
                let angle = turn_angle(sampled[index - 2], sampled[index], sampled[index + 2]);
                corner_confidence(angle, OPEN_CORNER_THRESHOLD)
            })
            .collect::<Vec<_>>();
        sum_of_corner_peaks(&confidences, false)
    }
}

pub(super) fn straight_section_score(points: &[StrokePoint]) -> f32 {
    let sampled = resample(points, 36);
    if sampled.len() < 4 {
        return 0.0;
    }
    let straight = sampled
        .windows(4)
        .filter(|window| turn_angle(window[0], window[1], window[3]) < 0.34)
        .count();
    straight as f32 / (sampled.len() - 3).max(1) as f32
}

pub(super) fn downward_arrow_score(ink: &NormalizedInk) -> f32 {
    let points = all_points(ink);
    if points.is_empty() {
        return 0.0;
    }
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f32::INFINITY, f32::min);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f32::NEG_INFINITY, f32::max);
    let vertical_span = (max_y - min_y).max(0.001);
    let bottom_points = points
        .iter()
        .filter(|point| point.y > min_y + vertical_span * 0.62)
        .count();
    let top_points = points
        .iter()
        .filter(|point| point.y < min_y + vertical_span * 0.25)
        .count();
    ((bottom_points as f32 * 0.22) + (top_points as f32 * 0.10)).clamp(0.0, 1.0)
}

pub(super) fn average_directness(ink: &NormalizedInk) -> f32 {
    ink.strokes
        .iter()
        .map(|stroke| stroke_directness(stroke))
        .sum::<f32>()
        / ink.strokes.len().max(1) as f32
}

pub(super) fn rightward_arrow_score(ink: &NormalizedInk) -> f32 {
    let points = all_points(ink);
    if points.is_empty() {
        return 0.0;
    }
    let bounds = point_bounds(&points);
    let width = (bounds.2 - bounds.0).max(0.001);
    let height = (bounds.3 - bounds.1).max(0.001);
    let axis_score = (width / (width + height)).clamp(0.0, 1.0);
    let mid_y = bounds.1 + height * 0.5;
    let tip = points
        .iter()
        .max_by(|a, b| {
            a.x.total_cmp(&b.x)
                .then_with(|| (b.y - mid_y).abs().total_cmp(&(a.y - mid_y).abs()))
        })
        .copied()
        .unwrap_or(StrokePoint::new(0.5, 0.5));
    let tip_score = (1.0 - (tip.y - mid_y).abs() / (height * 0.72).max(0.001)).clamp(0.0, 1.0);
    let head_points = points
        .iter()
        .filter(|point| point.x > bounds.0 + width * 0.58)
        .copied()
        .collect::<Vec<_>>();
    let head_spread = if head_points.len() >= 2 {
        let head_bounds = point_bounds(&head_points);
        ((head_bounds.3 - head_bounds.1) / height.max(0.001)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let horizontal_line = ink
        .strokes
        .iter()
        .map(|stroke| horizontal_center_bar_score(stroke))
        .fold(0.0, f32::max);
    (axis_score * 0.30 + tip_score * 0.24 + head_spread * 0.24 + horizontal_line * 0.22)
        .clamp(0.0, 1.0)
}

pub(super) fn horizontal_center_bar_score(stroke: &[StrokePoint]) -> f32 {
    if stroke.len() < 2 {
        return 0.0;
    }
    let bounds = point_bounds(stroke);
    let width = (bounds.2 - bounds.0).max(0.001);
    let height = (bounds.3 - bounds.1).max(0.001);
    let horizontal = (width / (width + height)).clamp(0.0, 1.0);
    let mean_y = stroke.iter().map(|point| point.y).sum::<f32>() / stroke.len() as f32;
    let center = (1.0 - (mean_y - 0.5).abs() / 0.34).clamp(0.0, 1.0);
    let length = (width / 0.34).clamp(0.0, 1.0);
    (stroke_directness(stroke) * 0.32 + horizontal * 0.30 + center * 0.20 + length * 0.18)
        .clamp(0.0, 1.0)
}

pub(super) fn ray_angle_spread(ink: &NormalizedInk) -> f32 {
    let mut bins = [false; 4];
    for stroke in &ink.strokes {
        let Some(start) = stroke.first() else {
            continue;
        };
        let Some(end) = stroke.last() else {
            continue;
        };
        let mut angle = (end.y - start.y).atan2(end.x - start.x);
        while angle < 0.0 {
            angle += std::f32::consts::PI;
        }
        while angle >= std::f32::consts::PI {
            angle -= std::f32::consts::PI;
        }
        let bin = ((angle / std::f32::consts::PI) * bins.len() as f32).round() as usize;
        bins[bin % bins.len()] = true;
    }
    bins.iter().filter(|filled| **filled).count() as f32 / bins.len() as f32
}

pub(super) fn ray_center_score(ink: &NormalizedInk) -> f32 {
    let center = StrokePoint::new(0.5, 0.5);
    ink.strokes
        .iter()
        .filter_map(|stroke| {
            let start = stroke.first().copied()?;
            let end = stroke.last().copied()?;
            let error = point_segment_distance(center, start, end);
            Some((1.0 - error / 0.20).clamp(0.0, 1.0))
        })
        .sum::<f32>()
        / ink.strokes.len().max(1) as f32
}

pub(super) fn count_score(candidate_count: usize, template_count: usize) -> f32 {
    let missing =
        template_count.saturating_sub(candidate_count) as f32 / template_count.max(1) as f32;
    let extra =
        candidate_count.saturating_sub(template_count) as f32 / template_count.max(1) as f32;
    (1.0 - missing * 0.50 - extra * 0.24).clamp(0.10, 1.0)
}

pub(super) fn closure_score(points: &[StrokePoint]) -> f32 {
    let Some(first) = points.first() else {
        return 0.0;
    };
    let Some(last) = points.last() else {
        return 0.0;
    };
    (1.0 - first.distance(*last) / 0.18).clamp(0.0, 1.0)
}

fn all_points(ink: &NormalizedInk) -> Vec<StrokePoint> {
    ink.strokes
        .iter()
        .flat_map(|stroke| stroke.iter().copied())
        .collect()
}

fn point_bounds(points: &[StrokePoint]) -> (f32, f32, f32, f32) {
    points.iter().fold(
        (
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ),
        |bounds, point| {
            (
                bounds.0.min(point.x),
                bounds.1.min(point.y),
                bounds.2.max(point.x),
                bounds.3.max(point.y),
            )
        },
    )
}

fn point_segment_distance(point: StrokePoint, start: StrokePoint, end: StrokePoint) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= f32::EPSILON {
        return point.distance(start);
    }
    let t = (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_sq).clamp(0.0, 1.0);
    point.distance(StrokePoint::new(start.x + dx * t, start.y + dy * t))
}

fn stroke_directness(points: &[StrokePoint]) -> f32 {
    let Some(start) = points.first() else {
        return 0.0;
    };
    let Some(end) = points.last() else {
        return 0.0;
    };
    start.distance(*end) / stroke_length(points).max(0.001)
}

fn angle_coverage(points: &[StrokePoint], center: StrokePoint) -> f32 {
    let mut bins = [false; 8];
    for point in points {
        let angle = (point.y - center.y).atan2(point.x - center.x);
        let normalized = (angle + std::f32::consts::TAU) % std::f32::consts::TAU;
        let index = ((normalized / std::f32::consts::TAU) * bins.len() as f32) as usize;
        bins[index.min(bins.len() - 1)] = true;
    }
    bins.iter().filter(|filled| **filled).count() as f32 / bins.len() as f32
}

fn turn_angle(a: StrokePoint, b: StrokePoint, c: StrokePoint) -> f32 {
    let ab = (b.x - a.x, b.y - a.y);
    let bc = (c.x - b.x, c.y - b.y);
    let ab_len = (ab.0 * ab.0 + ab.1 * ab.1).sqrt().max(0.001);
    let bc_len = (bc.0 * bc.0 + bc.1 * bc.1).sqrt().max(0.001);
    let dot = ((ab.0 * bc.0 + ab.1 * bc.1) / (ab_len * bc_len)).clamp(-1.0, 1.0);
    dot.acos()
}

fn ratio_score(candidate: f32, template: f32) -> f32 {
    let candidate = candidate.max(0.001);
    let template = template.max(0.001);
    (1.0 - (candidate / template).ln().abs() / 1.60).clamp(0.0, 1.0)
}

fn resample(points: &[StrokePoint], target_count: usize) -> Vec<StrokePoint> {
    if points.len() <= 1 {
        return points.to_vec();
    }
    let total = stroke_length(points);
    if total <= 0.0001 {
        return vec![points[0]; target_count];
    }
    (0..target_count)
        .map(|index| {
            let target = total * index as f32 / (target_count.saturating_sub(1)).max(1) as f32;
            point_at_distance(points, target)
        })
        .collect()
}

fn point_at_distance(points: &[StrokePoint], target: f32) -> StrokePoint {
    let mut walked = 0.0;
    for segment in points.windows(2) {
        let start = segment[0];
        let end = segment[1];
        let length = start.distance(end);
        if walked + length >= target {
            let t = ((target - walked) / length.max(0.0001)).clamp(0.0, 1.0);
            return StrokePoint::new(
                start.x + (end.x - start.x) * t,
                start.y + (end.y - start.y) * t,
            );
        }
        walked += length;
    }
    *points.last().unwrap_or(&StrokePoint::new(0.5, 0.5))
}

fn stroke_length(points: &[StrokePoint]) -> f32 {
    points
        .windows(2)
        .map(|segment| segment[0].distance(segment[1]))
        .sum()
}

#[cfg(test)]
mod tests;
