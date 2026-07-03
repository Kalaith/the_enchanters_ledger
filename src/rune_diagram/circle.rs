use super::geometry::{distance, StrokeBounds};
use super::MIN_CIRCLE_QUALITY;
use crate::rune_drawing::{DrawnStroke, StrokePoint};

/// A candidate working circle: the (one or more, see `assemble_multi_stroke_circles`) original
/// stroke indices that make it up, its `circle_quality`, and its bounds.
pub(crate) type CircleCandidate = (Vec<usize>, f32, StrokeBounds);

const MAX_CIRCLE_CHAIN_STROKES: usize = 4;
/// Only strokes at least this large (relative to the slate) are considered possible arcs of a
/// multi-stroke circle — small rune strokes never need to be probed for chaining, which keeps
/// `assemble_multi_stroke_circles` cheap even in a diagram with hundreds of small runes.
const MIN_ARC_SPAN: f32 = 0.08;

/// Every circle candidate in the drawing: single strokes that already read as a circle, plus
/// chained multi-stroke arcs (`assemble_multi_stroke_circles`, plan Phase 3 item 1 / C5) — a
/// circle repaired or drawn in two or three arcs is a candidate here exactly like a one-stroke
/// circle. Shared by `interpret_diagram` and `rune_diagnostics` so both agree on what counts as
/// a circle.
pub(crate) fn gather_circle_candidates(useful: &[(usize, &DrawnStroke)]) -> Vec<CircleCandidate> {
    let mut candidates = useful
        .iter()
        .filter_map(|(index, stroke)| {
            let bounds = StrokeBounds::from_stroke(stroke)?;
            circle_quality(stroke, bounds).map(|score| (vec![*index], score, bounds))
        })
        .collect::<Vec<_>>();
    candidates.extend(assemble_multi_stroke_circles(useful));
    candidates
}

/// Chains open strokes whose endpoints nearly touch into a single merged polyline and scores it
/// with the ordinary `circle_quality` — so a circle drawn (or repaired) as 2-4 separate arcs is
/// still found. No direction/turn-angle check like `rune_drawing::join_continuation` uses: arcs
/// of a circle are expected to turn continuously, so requiring a nearly-straight join would
/// reject exactly the shapes this is meant to catch.
fn assemble_multi_stroke_circles(useful: &[(usize, &DrawnStroke)]) -> Vec<CircleCandidate> {
    let arcs = useful
        .iter()
        .filter(|(_, stroke)| stroke.points.len() >= 4 && stroke_span(stroke) >= MIN_ARC_SPAN)
        .copied()
        .collect::<Vec<_>>();
    if arcs.len() < 2 {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    for start in 0..arcs.len() {
        let mut chain_indices = vec![arcs[start].0];
        let mut chain_points = arcs[start].1.points.clone();
        let mut used = vec![start];
        while chain_indices.len() < MAX_CIRCLE_CHAIN_STROKES {
            let Some((next, flip, at_end)) = find_chain_extension(&chain_points, &arcs, &used)
            else {
                break;
            };
            attach_points(&mut chain_points, &arcs[next].1.points, flip, at_end);
            chain_indices.push(arcs[next].0);
            used.push(next);
        }
        if chain_indices.len() < 2 {
            continue;
        }
        let merged = DrawnStroke {
            points: chain_points,
        };
        let Some(bounds) = StrokeBounds::from_stroke(&merged) else {
            continue;
        };
        if let Some(quality) = circle_quality(&merged, bounds) {
            chain_indices.sort_unstable();
            chain_indices.dedup();
            candidates.push((chain_indices, quality, bounds));
        }
    }
    candidates
}

fn find_chain_extension(
    chain: &[StrokePoint],
    arcs: &[(usize, &DrawnStroke)],
    used: &[usize],
) -> Option<(usize, bool, bool)> {
    let chain_start = *chain.first()?;
    let chain_end = *chain.last()?;
    let span = points_span(chain).max(0.05);
    let gap_limit = (span * 0.12).max(0.02);

    let mut best: Option<(usize, bool, bool, f32)> = None;
    for (arc_index, (_, stroke)) in arcs.iter().enumerate() {
        if used.contains(&arc_index) {
            continue;
        }
        let Some(first) = stroke.points.first().copied() else {
            continue;
        };
        let Some(last) = stroke.points.last().copied() else {
            continue;
        };
        let options = [
            (distance(chain_end, first), false, true),
            (distance(chain_end, last), true, true),
            (distance(chain_start, last), false, false),
            (distance(chain_start, first), true, false),
        ];
        for (gap, flip, at_end) in options {
            if gap <= gap_limit && best.is_none_or(|(.., best_gap)| gap < best_gap) {
                best = Some((arc_index, flip, at_end, gap));
            }
        }
    }
    best.map(|(index, flip, at_end, _)| (index, flip, at_end))
}

fn attach_points(chain: &mut Vec<StrokePoint>, add: &[StrokePoint], flip: bool, at_end: bool) {
    let mut points = add.to_vec();
    if flip {
        points.reverse();
    }
    if at_end {
        chain.extend(points);
    } else {
        points.append(chain);
        *chain = points;
    }
}

fn stroke_span(stroke: &DrawnStroke) -> f32 {
    points_span(&stroke.points)
}

fn points_span(points: &[StrokePoint]) -> f32 {
    let Some(first) = points.first() else {
        return 0.0;
    };
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (first.x, first.x, first.y, first.y);
    for point in points {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }
    (max_x - min_x).max(max_y - min_y)
}

pub(crate) fn select_working_circle(candidates: &[CircleCandidate]) -> Option<CircleCandidate> {
    candidates
        .iter()
        .filter(|(_, quality, _)| *quality >= MIN_CIRCLE_QUALITY)
        .max_by(|a, b| {
            let a_span = a.2.width().max(a.2.height());
            let b_span = b.2.width().max(b.2.height());
            a_span
                .total_cmp(&b_span)
                .then_with(|| a.1.total_cmp(&b.1))
                .then_with(|| tie_break_key(&a.0).cmp(&tie_break_key(&b.0)))
        })
        .cloned()
        .or_else(|| {
            candidates
                .iter()
                .max_by(|a, b| {
                    a.1.total_cmp(&b.1)
                        .then_with(|| tie_break_key(&a.0).cmp(&tie_break_key(&b.0)))
                })
                .cloned()
        })
}

pub(crate) fn select_working_circle_for_strokes(
    candidates: &[CircleCandidate],
    useful: &[(usize, &DrawnStroke)],
) -> Option<CircleCandidate> {
    let contextual = candidates
        .iter()
        .map(|candidate| (candidate.clone(), enclosed_ink_count(candidate, useful)))
        .filter(|(_, enclosed)| *enclosed > 0)
        .max_by(|(a, a_enclosed), (b, b_enclosed)| {
            let a_valid = (a.1 >= MIN_CIRCLE_QUALITY) as u8;
            let b_valid = (b.1 >= MIN_CIRCLE_QUALITY) as u8;
            let a_span = a.2.width().max(a.2.height());
            let b_span = b.2.width().max(b.2.height());
            a_valid
                .cmp(&b_valid)
                .then_with(|| a_enclosed.cmp(b_enclosed))
                .then_with(|| a_span.total_cmp(&b_span))
                .then_with(|| a.1.total_cmp(&b.1))
                .then_with(|| tie_break_key(&a.0).cmp(&tie_break_key(&b.0)))
        })
        .map(|(candidate, _)| candidate);

    contextual.or_else(|| select_working_circle(candidates))
}

/// Deterministic tie-break: highest member stroke index wins, matching the old single-stroke
/// behavior (`b.0.cmp(&a.0)`, i.e. prefer the more-recently-drawn stroke).
fn tie_break_key(indices: &[usize]) -> usize {
    indices.iter().copied().max().unwrap_or(0)
}

pub(crate) fn circle_quality(stroke: &DrawnStroke, bounds: StrokeBounds) -> Option<f32> {
    if stroke.points.len() < 8 {
        return None;
    }
    let width = bounds.width();
    let height = bounds.height();
    let span = width.max(height).max(0.001);
    if span < 0.22 || width < 0.15 || height < 0.15 {
        return None;
    }

    let closure = 1.0 - distance(stroke.points[0], *stroke.points.last()?) / (span * 0.50);
    let aspect = ratio_score(width / height.max(0.001), 1.0);
    let center = bounds.center();
    let center_score = 1.0 - distance(center, StrokePoint::new(0.5, 0.5)) / 0.62;
    let radius_score = radius_consistency(&stroke.points, center);
    let coverage = angle_coverage(&stroke.points, center);
    let top_start = circle_start_score(stroke.points[0], center, bounds);

    let score = closure.clamp(0.0, 1.0) * 0.32
        + aspect * 0.18
        + center_score.clamp(0.0, 1.0) * 0.04
        + radius_score * 0.22
        + coverage * 0.18
        + top_start * 0.08;
    Some(score.clamp(0.0, 1.0))
}

/// Same shape scoring as `circle_quality`, without the absolute slate-space size floor and
/// without the "is it near the middle of the whole slate" term (meaningless for a ring that can
/// sit anywhere inside its parent scope) — used for nested-scope ring detection (plan Phase 3
/// item 2), where a ring's *absolute* size is only ever a fraction of its parent scope and would
/// otherwise never clear `circle_quality`'s top-level floor even when it reads as a clean, large
/// ring *relative to its own scope*.
pub(crate) fn ring_shape_score(stroke: &DrawnStroke, bounds: StrokeBounds) -> Option<f32> {
    if stroke.points.len() < 8 {
        return None;
    }
    let width = bounds.width();
    let height = bounds.height();
    let span = width.max(height).max(0.001);

    let closure = 1.0 - distance(stroke.points[0], *stroke.points.last()?) / (span * 0.50);
    let aspect = ratio_score(width / height.max(0.001), 1.0);
    let center = bounds.center();
    let radius_score = radius_consistency(&stroke.points, center);
    let coverage = angle_coverage(&stroke.points, center);
    let top_start = circle_start_score(stroke.points[0], center, bounds);

    let score = closure.clamp(0.0, 1.0) * 0.34
        + aspect * 0.20
        + radius_score * 0.24
        + coverage * 0.14
        + top_start * 0.08;
    Some(score.clamp(0.0, 1.0))
}

pub(crate) fn is_inside_working_circle(stroke: &DrawnStroke, circle_bounds: StrokeBounds) -> bool {
    let Some(bounds) = StrokeBounds::from_stroke(stroke) else {
        return false;
    };
    let center = bounds.center();
    let circle_center = circle_bounds.center();
    let rx = (circle_bounds.width() * 0.5).max(0.05);
    let ry = (circle_bounds.height() * 0.5).max(0.05);
    let nx = (center.x - circle_center.x) / rx;
    let ny = (center.y - circle_center.y) / ry;
    nx * nx + ny * ny <= 1.25
        && bounds.width() < circle_bounds.width() * 0.92
        && bounds.height() < circle_bounds.height() * 0.92
}

fn enclosed_ink_count(candidate: &CircleCandidate, useful: &[(usize, &DrawnStroke)]) -> usize {
    useful
        .iter()
        .filter(|(index, stroke)| {
            !candidate.0.contains(index) && is_inside_working_circle(stroke, candidate.2)
        })
        .count()
}

fn circle_start_score(start: StrokePoint, center: StrokePoint, bounds: StrokeBounds) -> f32 {
    let top = StrokePoint::new(center.x, bounds.min_y);
    let span = bounds.width().max(bounds.height()).max(0.001);
    (1.0 - distance(start, top) / (span * 0.80)).clamp(0.0, 1.0)
}

fn radius_consistency(points: &[StrokePoint], center: StrokePoint) -> f32 {
    let radii = points
        .iter()
        .map(|point| distance(*point, center))
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
    (1.0 - variance.sqrt() / (mean * 0.42).max(0.001)).clamp(0.0, 1.0)
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

fn ratio_score(candidate: f32, template: f32) -> f32 {
    let candidate = candidate.max(0.001);
    let template = template.max(0.001);
    (1.0 - (candidate / template).ln().abs() / 1.15).clamp(0.0, 1.0)
}
