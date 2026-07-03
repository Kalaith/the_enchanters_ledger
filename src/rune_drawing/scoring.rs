use super::templates::{all_template_rune_ids, template_variants_for_rune};
use super::{shape_report_for_rune, DrawnStroke, StrokePoint};
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub(super) struct NormalizedDrawing {
    strokes: Vec<Vec<StrokePoint>>,
    aspect_ratio: f32,
    total_length: f32,
}

/// Every rune's template variants, already normalized — built once, so the
/// hot recognition path (every cluster × every rune × every variant) stops
/// re-normalizing the same static template points on each call (plan Phase 3
/// item 6, "precompute template feature vectors once").
pub(super) fn normalized_variants_for_rune(rune_id: &str) -> &'static [NormalizedDrawing] {
    static CACHE: OnceLock<HashMap<String, Vec<NormalizedDrawing>>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            all_template_rune_ids()
                .map(|id| {
                    let variants = template_variants_for_rune(id)
                        .iter()
                        .filter_map(|variant| NormalizedDrawing::from_strokes(variant))
                        .collect();
                    (id.to_string(), variants)
                })
                .collect()
        })
        .get(rune_id)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

impl NormalizedDrawing {
    pub(super) fn stroke_count(&self) -> usize {
        self.strokes.len()
    }

    /// Arc length after normalization to the drawing's own bounding box —
    /// scale-invariant, so comparing a candidate's to a template's isolates
    /// "was this stroke drawn to completion" (ink_ratio) from "how big was
    /// it drawn" (scale).
    pub(super) fn total_length(&self) -> f32 {
        self.total_length
    }

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

/// Blends template point-matching with the rune's *declared* structural
/// profile (plan Phase 1 item 3). Fully data-driven: the rune id is only a
/// key into `rune_templates.json` — there are no per-rune branches here, so
/// a new rune's structural behavior is a JSON edit, not Rust.
pub(super) fn adjusted_score_for_rune(
    rune_id: &str,
    source_strokes: &[DrawnStroke],
    candidate: &NormalizedDrawing,
    template: &NormalizedDrawing,
) -> f32 {
    let mut score = score_against_template(candidate, template);
    let Some(spec) = super::templates::structure_spec_for_rune(rune_id) else {
        return score;
    };
    let structure = shape_report_for_rune(rune_id, source_strokes)
        .map_or(1.0, |report| report.structural_score);
    if let Some(floor) = spec.circle_floor {
        score = score.max(circle_likeness(candidate) * floor);
    }
    let mut adjusted = score * (1.0 - spec.blend + spec.blend * structure);
    if let Some(suppression) = &spec.suppressed_by {
        let their_structure = shape_report_for_rune(&suppression.rune, source_strokes)
            .map_or(0.0, |report| report.structural_score);
        if their_structure > suppression.their_structure_min
            && structure < suppression.own_structure_below
        {
            adjusted *= suppression.factor;
        }
    }
    adjusted
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

const MAX_OPTIMAL_ASSIGNMENT_STROKES: usize = 12;

fn match_strokes_order_insensitive(
    candidate: &[Vec<StrokePoint>],
    template: &[Vec<StrokePoint>],
) -> Vec<f32> {
    stroke_assignment(candidate, template)
        .into_iter()
        .map(|(_, score)| score)
        .collect()
}

/// The best pairing of template strokes to distinct candidate strokes (by
/// index into `candidate`, `None` if no candidate stroke remains), along
/// with its similarity score — one entry per template stroke, in template
/// order. Shared by identity scoring and anything that needs to know *which*
/// drawn stroke matched which template stroke (e.g. mismatch highlighting),
/// so both use the same assignment instead of two subtly different ones.
pub(crate) fn stroke_assignment(
    candidate: &[Vec<StrokePoint>],
    template: &[Vec<StrokePoint>],
) -> Vec<(Option<usize>, f32)> {
    let similarity = template
        .iter()
        .map(|template_stroke| {
            candidate
                .iter()
                .map(|candidate_stroke| stroke_similarity(candidate_stroke, template_stroke))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if candidate.len() <= MAX_OPTIMAL_ASSIGNMENT_STROKES {
        optimal_assignment(&similarity, candidate.len())
    } else {
        greedy_assignment(&similarity, candidate.len())
    }
}

/// Exact best-total pairing of template strokes to distinct candidate strokes.
/// Greedy pairing flips on near-ties, so a tiny redraw could change the score.
fn optimal_assignment(
    similarity: &[Vec<f32>],
    candidate_count: usize,
) -> Vec<(Option<usize>, f32)> {
    let template_count = similarity.len();
    let mask_count = 1usize << candidate_count;
    let mut best = vec![vec![f32::NEG_INFINITY; mask_count]; template_count + 1];
    let mut choice = vec![vec![0u8; mask_count]; template_count + 1];
    best[0][0] = 0.0;
    for row in 0..template_count {
        for mask in 0..mask_count {
            let value = best[row][mask];
            if value == f32::NEG_INFINITY {
                continue;
            }
            if value > best[row + 1][mask] {
                best[row + 1][mask] = value;
                choice[row + 1][mask] = 0;
            }
            for (candidate_index, score) in similarity[row].iter().enumerate() {
                let bit = 1usize << candidate_index;
                if mask & bit != 0 {
                    continue;
                }
                let next = value + score;
                if next > best[row + 1][mask | bit] {
                    best[row + 1][mask | bit] = next;
                    choice[row + 1][mask | bit] = candidate_index as u8 + 1;
                }
            }
        }
    }

    let mut mask = (0..mask_count)
        .max_by(|a, b| {
            best[template_count][*a]
                .total_cmp(&best[template_count][*b])
                .then_with(|| b.cmp(a))
        })
        .unwrap_or(0);
    let mut assignment = vec![(None, 0.0); template_count];
    for row in (0..template_count).rev() {
        let picked = choice[row + 1][mask];
        if picked > 0 {
            let candidate_index = picked as usize - 1;
            assignment[row] = (Some(candidate_index), similarity[row][candidate_index]);
            mask &= !(1usize << candidate_index);
        }
    }
    assignment
}

fn greedy_assignment(similarity: &[Vec<f32>], candidate_count: usize) -> Vec<(Option<usize>, f32)> {
    let mut pairs = Vec::new();
    for (template_index, row) in similarity.iter().enumerate() {
        for (candidate_index, score) in row.iter().enumerate() {
            pairs.push((*score, candidate_index, template_index));
        }
    }
    pairs.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });

    let mut used_candidates = vec![false; candidate_count];
    let mut assignment = vec![(None, 0.0); similarity.len()];
    for (score, candidate_index, template_index) in pairs {
        if used_candidates[candidate_index] || assignment[template_index].0.is_some() {
            continue;
        }
        used_candidates[candidate_index] = true;
        assignment[template_index] = (Some(candidate_index), score);
    }
    assignment
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
