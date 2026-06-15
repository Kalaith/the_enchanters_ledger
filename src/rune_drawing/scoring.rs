use super::{shape_report_for_rune, DrawnStroke, StrokePoint};

#[derive(Debug, Clone)]
pub(super) struct NormalizedDrawing {
    strokes: Vec<Vec<StrokePoint>>,
    aspect_ratio: f32,
    total_length: f32,
}

impl NormalizedDrawing {
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
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let total_length = normalized.iter().map(|stroke| stroke_length(stroke)).sum();
        Some(Self {
            strokes: normalized,
            aspect_ratio: width / height,
            total_length,
        })
    }
}

pub(super) fn adjusted_score_for_rune(
    rune_id: &str,
    source_strokes: &[DrawnStroke],
    candidate: &NormalizedDrawing,
    template: &NormalizedDrawing,
) -> f32 {
    let score = score_against_template(candidate, template);
    let circle = circle_likeness(candidate);
    match rune_id {
        "sphere" => {
            let structure = shape_report_for_rune(rune_id, source_strokes)
                .map_or(1.0, |report| report.structural_score);
            score.max(circle * 0.92) * (0.52 + structure * 0.48)
        }
        "safer" => {
            let structure = shape_report_for_rune(rune_id, source_strokes)
                .map_or(1.0, |report| report.structural_score);
            let sphere_structure = shape_report_for_rune("sphere", source_strokes)
                .map_or(0.0, |report| report.structural_score);
            let circle_penalty = if sphere_structure > 0.80 && structure < 0.62 {
                0.42
            } else {
                1.0
            };
            score * (0.28 + structure * 0.72) * circle_penalty
        }
        "touch" => {
            let structure = shape_report_for_rune(rune_id, source_strokes)
                .map_or(1.0, |report| report.structural_score);
            score * (0.36 + structure * 0.64)
        }
        "force" => {
            let structure = shape_report_for_rune(rune_id, source_strokes)
                .map_or(1.0, |report| report.structural_score);
            score * (0.30 + structure * 0.70)
        }
        "beam" | "aura" | "burst" | "cone" => {
            let structure = shape_report_for_rune(rune_id, source_strokes)
                .map_or(1.0, |report| report.structural_score);
            score * (0.42 + structure * 0.58)
        }
        _ => score,
    }
}

fn score_against_template(candidate: &NormalizedDrawing, template: &NormalizedDrawing) -> f32 {
    let template_count = template.strokes.len();
    let candidate_count = candidate.strokes.len();
    if template_count == 0 || candidate_count == 0 {
        return 0.0;
    }

    let stroke_scores = match_strokes_order_insensitive(&candidate.strokes, &template.strokes);
    let shape_score = stroke_scores.iter().sum::<f32>() / template_count as f32;
    let weakest_stroke_score = stroke_scores.iter().copied().fold(1.0, f32::min);
    let missing = template_count.saturating_sub(candidate_count) as f32 / template_count as f32;
    let extra = candidate_count.saturating_sub(template_count) as f32 / template_count as f32;
    let extra_penalty = if template_count == 1 { 0.70 } else { 0.10 };
    let count_score = (1.0 - missing * 0.52 - extra * extra_penalty).clamp(0.10, 1.0);
    let aspect_score = ratio_score(candidate.aspect_ratio, template.aspect_ratio);
    let length_score = ratio_score(
        candidate.total_length.max(0.01),
        template.total_length.max(0.01),
    );

    let base_score =
        (shape_score * 0.76 + count_score * 0.08 + aspect_score * 0.09 + length_score * 0.07)
            .clamp(0.0, 1.0);
    let completeness_score = if candidate_count < template_count {
        (weakest_stroke_score * 1.7).clamp(0.0, 1.0)
    } else {
        (weakest_stroke_score * 1.7).clamp(0.28, 1.0)
    };
    (base_score * count_score * completeness_score).clamp(0.0, 1.0)
}

fn circle_likeness(drawing: &NormalizedDrawing) -> f32 {
    if drawing.strokes.len() != 1 || !is_closed_stroke(&drawing.strokes[0]) {
        return 0.0;
    }
    let points = &drawing.strokes[0];
    let center = StrokePoint::new(0.5, 0.5);
    let radii = points
        .iter()
        .map(|point| point.distance(center))
        .collect::<Vec<_>>();
    let mean = radii.iter().sum::<f32>() / radii.len().max(1) as f32;
    let variance = radii
        .iter()
        .map(|radius| (radius - mean) * (radius - mean))
        .sum::<f32>()
        / radii.len().max(1) as f32;
    let radius_score = (1.0 - variance.sqrt() / (mean * 0.48).max(0.001)).clamp(0.0, 1.0);
    let aspect_score = ratio_score(drawing.aspect_ratio, 1.0);
    (radius_score * 0.76 + aspect_score * 0.24).clamp(0.0, 1.0)
}

fn match_strokes_order_insensitive(
    candidate: &[Vec<StrokePoint>],
    template: &[Vec<StrokePoint>],
) -> Vec<f32> {
    let mut pairs = Vec::new();
    for (candidate_index, candidate_stroke) in candidate.iter().enumerate() {
        for (template_index, template_stroke) in template.iter().enumerate() {
            pairs.push((
                stroke_similarity(candidate_stroke, template_stroke),
                candidate_index,
                template_index,
            ));
        }
    }
    pairs.sort_by(|a, b| b.0.total_cmp(&a.0));

    let mut used_candidates = vec![false; candidate.len()];
    let mut scores = vec![0.0; template.len()];
    for (score, candidate_index, template_index) in pairs {
        if used_candidates[candidate_index] || scores[template_index] > 0.0 {
            continue;
        }
        used_candidates[candidate_index] = true;
        scores[template_index] = score;
    }
    scores
}

fn stroke_similarity(candidate: &[StrokePoint], template: &[StrokePoint]) -> f32 {
    let candidate = resample(candidate, 24);
    let template = resample(template, 24);
    let distance = shape_distance(&candidate, &template);
    let point_score = (1.0 - distance / 0.43).clamp(0.0, 1.0);
    let endpoint_score = if is_closed_stroke(&candidate) && is_closed_stroke(&template) {
        1.0
    } else {
        endpoint_similarity(&candidate, &template)
    };
    let directness_score = directness_similarity(&candidate, &template);
    (point_score * 0.72 + endpoint_score * 0.18 + directness_score * 0.10).clamp(0.0, 1.0)
}

fn shape_distance(candidate: &[StrokePoint], template: &[StrokePoint]) -> f32 {
    if is_closed_stroke(candidate) && is_closed_stroke(template) {
        return cyclic_average_pair_distance(candidate, template);
    }
    let direct = average_pair_distance(candidate, template);
    let reversed_template = template.iter().copied().rev().collect::<Vec<_>>();
    let reversed = average_pair_distance(candidate, &reversed_template);
    direct.min(reversed)
}

fn is_closed_stroke(points: &[StrokePoint]) -> bool {
    let Some(first) = points.first() else {
        return false;
    };
    let Some(last) = points.last() else {
        return false;
    };
    first.distance(*last) <= 0.18
}

fn cyclic_average_pair_distance(candidate: &[StrokePoint], template: &[StrokePoint]) -> f32 {
    let forward = best_cyclic_distance(candidate, template);
    let reversed = template.iter().copied().rev().collect::<Vec<_>>();
    forward.min(best_cyclic_distance(candidate, &reversed))
}

fn best_cyclic_distance(candidate: &[StrokePoint], template: &[StrokePoint]) -> f32 {
    let count = candidate.len().min(template.len()).max(1);
    (0..count)
        .map(|offset| {
            candidate
                .iter()
                .take(count)
                .enumerate()
                .map(|(index, point)| point.distance(template[(index + offset) % count]))
                .sum::<f32>()
                / count as f32
        })
        .fold(f32::INFINITY, f32::min)
}

fn endpoint_similarity(candidate: &[StrokePoint], template: &[StrokePoint]) -> f32 {
    let direct = endpoint_distance(candidate, template);
    let reversed_template = template.iter().copied().rev().collect::<Vec<_>>();
    let reversed = endpoint_distance(candidate, &reversed_template);
    (1.0 - direct.min(reversed) / 0.42).clamp(0.0, 1.0)
}

fn endpoint_distance(a: &[StrokePoint], b: &[StrokePoint]) -> f32 {
    let Some(a_start) = a.first() else {
        return 1.0;
    };
    let Some(a_end) = a.last() else {
        return 1.0;
    };
    let Some(b_start) = b.first() else {
        return 1.0;
    };
    let Some(b_end) = b.last() else {
        return 1.0;
    };
    (a_start.distance(*b_start) + a_end.distance(*b_end)) * 0.5
}

fn directness_similarity(candidate: &[StrokePoint], template: &[StrokePoint]) -> f32 {
    let candidate = stroke_directness(candidate);
    let template = stroke_directness(template);
    (1.0 - (candidate - template).abs() / 0.75).clamp(0.0, 1.0)
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

fn average_pair_distance(a: &[StrokePoint], b: &[StrokePoint]) -> f32 {
    let count = a.len().min(b.len()).max(1);
    a.iter()
        .zip(b.iter())
        .take(count)
        .map(|(a, b)| a.distance(*b))
        .sum::<f32>()
        / count as f32
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

    let mut samples = Vec::with_capacity(target_count);
    for sample_index in 0..target_count {
        let target = total * sample_index as f32 / (target_count.saturating_sub(1)).max(1) as f32;
        samples.push(point_at_distance(points, target));
    }
    samples
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
