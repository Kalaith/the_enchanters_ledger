use super::geometry::{distance, StrokeBounds};
use super::MIN_CIRCLE_QUALITY;
use crate::rune_drawing::{DrawnStroke, StrokePoint};

pub(crate) fn select_working_circle(
    candidates: &[(usize, f32, StrokeBounds)],
) -> Option<(usize, f32, StrokeBounds)> {
    candidates
        .iter()
        .filter(|(_, quality, _)| *quality >= MIN_CIRCLE_QUALITY)
        .max_by(|a, b| {
            let a_span = a.2.width().max(a.2.height());
            let b_span = b.2.width().max(b.2.height());
            a_span.total_cmp(&b_span).then_with(|| a.1.total_cmp(&b.1))
        })
        .copied()
        .or_else(|| {
            candidates
                .iter()
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .copied()
        })
}

pub(crate) fn select_working_circle_for_strokes(
    candidates: &[(usize, f32, StrokeBounds)],
    useful: &[(usize, &DrawnStroke)],
) -> Option<(usize, f32, StrokeBounds)> {
    let contextual = candidates
        .iter()
        .map(|candidate| (*candidate, enclosed_ink_count(*candidate, useful)))
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
        })
        .map(|(candidate, _)| candidate);

    contextual.or_else(|| select_working_circle(candidates))
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

fn enclosed_ink_count(
    candidate: (usize, f32, StrokeBounds),
    useful: &[(usize, &DrawnStroke)],
) -> usize {
    useful
        .iter()
        .filter(|(index, stroke)| {
            *index != candidate.0 && is_inside_working_circle(stroke, candidate.2)
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
