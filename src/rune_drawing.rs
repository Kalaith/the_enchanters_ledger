//! Freehand rune stroke capture and local template recognition.

use crate::data::RuneDef;
use serde::{Deserialize, Serialize};

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
                .map(|template| adjusted_score_for_rune(&rune.id, &candidate, &template))
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

fn adjusted_score_for_rune(
    rune_id: &str,
    candidate: &NormalizedDrawing,
    template: &NormalizedDrawing,
) -> f32 {
    let score = score_against_template(candidate, template);
    let circle = circle_likeness(candidate);
    match rune_id {
        "sphere" => score.max(circle * 0.92),
        "safer" if circle > 0.62 => score * (1.0 - (circle - 0.62) * 1.25).clamp(0.42, 1.0),
        _ => score,
    }
}

fn template_variants_for_rune(rune_id: &str) -> Vec<Vec<DrawnStroke>> {
    let mut variants = template_strokes_for_rune(rune_id)
        .into_iter()
        .collect::<Vec<_>>();
    if rune_id == "touch" {
        variants.push(raw(&[&[
            (0.50, 0.16),
            (0.50, 0.84),
            (0.34, 0.64),
            (0.50, 0.84),
            (0.66, 0.64),
        ]]));
        variants.push(raw(&[
            &[(0.50, 0.16), (0.50, 0.84)],
            &[(0.34, 0.64), (0.50, 0.84)],
            &[(0.66, 0.64), (0.50, 0.84)],
        ]));
    } else if rune_id == "continuous" {
        variants.push(raw(&[
            &[
                (0.16, 0.50),
                (0.32, 0.22),
                (0.50, 0.50),
                (0.32, 0.78),
                (0.16, 0.50),
            ],
            &[
                (0.50, 0.50),
                (0.68, 0.22),
                (0.84, 0.50),
                (0.68, 0.78),
                (0.50, 0.50),
            ],
        ]));
    }
    variants
}

pub fn template_strokes_for_rune(rune_id: &str) -> Option<Vec<DrawnStroke>> {
    match rune_id {
        "light" => Some(raw(&[
            &[(0.50, 0.16), (0.50, 0.84)],
            &[(0.25, 0.50), (0.75, 0.50)],
            &[(0.34, 0.28), (0.66, 0.72)],
            &[(0.66, 0.28), (0.34, 0.72)],
        ])),
        "warmth" => Some(raw(&[&[
            (0.12, 0.62),
            (0.24, 0.38),
            (0.38, 0.62),
            (0.52, 0.38),
            (0.66, 0.62),
            (0.82, 0.38),
        ]])),
        "spark" => Some(raw(&[&[
            (0.60, 0.12),
            (0.36, 0.46),
            (0.56, 0.46),
            (0.38, 0.88),
        ]])),
        "fire" => Some(raw(&[
            &[
                (0.52, 0.10),
                (0.30, 0.42),
                (0.38, 0.76),
                (0.52, 0.92),
                (0.70, 0.68),
                (0.60, 0.38),
                (0.52, 0.10),
            ],
            &[(0.50, 0.72), (0.46, 0.48), (0.56, 0.30)],
        ])),
        "wind" => Some(raw(&[
            &[(0.12, 0.32), (0.46, 0.26), (0.82, 0.34)],
            &[(0.18, 0.52), (0.56, 0.44), (0.86, 0.54)],
            &[(0.10, 0.70), (0.38, 0.64), (0.64, 0.72)],
        ])),
        "frost" => Some(raw(&[
            &[(0.50, 0.12), (0.50, 0.88)],
            &[(0.18, 0.30), (0.82, 0.70)],
            &[(0.82, 0.30), (0.18, 0.70)],
        ])),
        "force" => Some(raw(&[&[
            (0.50, 0.12),
            (0.86, 0.50),
            (0.50, 0.88),
            (0.14, 0.50),
            (0.50, 0.12),
        ]])),
        "growth" => Some(raw(&[
            &[(0.50, 0.88), (0.50, 0.22)],
            &[(0.50, 0.55), (0.25, 0.38), (0.20, 0.28)],
            &[(0.50, 0.45), (0.75, 0.30), (0.82, 0.20)],
        ])),
        "sound" => Some(raw(&[
            &[(0.18, 0.48), (0.30, 0.40), (0.30, 0.60), (0.18, 0.52)],
            &[(0.42, 0.32), (0.58, 0.50), (0.42, 0.68)],
            &[(0.60, 0.20), (0.82, 0.50), (0.60, 0.80)],
        ])),
        "healing" => {
            let mut strokes = circle(0.50, 0.50, 0.34, 18);
            strokes.extend(raw(&[
                &[(0.50, 0.28), (0.50, 0.72)],
                &[(0.28, 0.50), (0.72, 0.50)],
            ]));
            Some(strokes)
        }
        "water" => Some(raw(&[
            &[
                (0.50, 0.12),
                (0.30, 0.50),
                (0.50, 0.88),
                (0.70, 0.50),
                (0.50, 0.12),
            ],
            &[
                (0.18, 0.68),
                (0.34, 0.58),
                (0.50, 0.68),
                (0.66, 0.58),
                (0.82, 0.68),
            ],
        ])),
        "teleportation" => Some(raw(&[
            &[
                (0.25, 0.32),
                (0.42, 0.32),
                (0.42, 0.68),
                (0.25, 0.68),
                (0.25, 0.32),
            ],
            &[
                (0.58, 0.32),
                (0.75, 0.32),
                (0.75, 0.68),
                (0.58, 0.68),
                (0.58, 0.32),
            ],
            &[(0.42, 0.50), (0.58, 0.50)],
        ])),
        "gravity" => Some(raw(&[
            &[(0.50, 0.12), (0.50, 0.82)],
            &[(0.32, 0.64), (0.50, 0.84), (0.68, 0.64)],
            &[(0.28, 0.24), (0.72, 0.24)],
        ])),
        "summoning" => Some(raw(&[&[
            (0.50, 0.12),
            (0.62, 0.44),
            (0.88, 0.44),
            (0.67, 0.60),
            (0.76, 0.88),
            (0.50, 0.70),
            (0.24, 0.88),
            (0.33, 0.60),
            (0.12, 0.44),
            (0.38, 0.44),
            (0.50, 0.12),
        ]])),
        "time" => Some(raw(&[&[
            (0.24, 0.14),
            (0.76, 0.14),
            (0.52, 0.50),
            (0.76, 0.86),
            (0.24, 0.86),
            (0.48, 0.50),
            (0.24, 0.14),
        ]])),
        "sphere" => Some(circle(0.50, 0.50, 0.34, 24)),
        "touch" => Some(raw(&[
            &[(0.50, 0.16), (0.50, 0.68)],
            &[(0.34, 0.66), (0.50, 0.84), (0.66, 0.66)],
        ])),
        "beam" => Some(raw(&[
            &[(0.14, 0.50), (0.84, 0.50)],
            &[(0.66, 0.34), (0.84, 0.50), (0.66, 0.66)],
        ])),
        "aura" => Some(raw(&[
            &[
                (0.50, 0.18),
                (0.82, 0.38),
                (0.82, 0.62),
                (0.50, 0.82),
                (0.18, 0.62),
                (0.18, 0.38),
                (0.50, 0.18),
            ],
            &[(0.32, 0.50), (0.68, 0.50)],
        ])),
        "burst" => Some(raw(&[
            &[(0.50, 0.16), (0.50, 0.84)],
            &[(0.16, 0.50), (0.84, 0.50)],
            &[(0.26, 0.26), (0.74, 0.74)],
            &[(0.74, 0.26), (0.26, 0.74)],
        ])),
        "cone" => Some(raw(&[&[
            (0.18, 0.76),
            (0.50, 0.22),
            (0.82, 0.76),
            (0.18, 0.76),
        ]])),
        "continuous" => Some(raw(&[&[
            (0.18, 0.50),
            (0.34, 0.24),
            (0.50, 0.50),
            (0.66, 0.76),
            (0.82, 0.50),
            (0.66, 0.24),
            (0.50, 0.50),
            (0.34, 0.76),
            (0.18, 0.50),
        ]])),
        "on_touch" => Some(raw(&[
            &[(0.50, 0.16), (0.50, 0.62)],
            &[(0.50, 0.78), (0.50, 0.80)],
            &[(0.34, 0.62), (0.50, 0.78), (0.66, 0.62)],
        ])),
        "on_impact" => Some(raw(&[
            &[(0.20, 0.20), (0.78, 0.78)],
            &[(0.58, 0.34), (0.82, 0.34), (0.82, 0.58)],
            &[(0.26, 0.76), (0.42, 0.60)],
        ])),
        "on_command" => Some(raw(&[&[
            (0.18, 0.28),
            (0.76, 0.28),
            (0.76, 0.60),
            (0.54, 0.60),
            (0.40, 0.78),
            (0.42, 0.60),
            (0.18, 0.60),
            (0.18, 0.28),
        ]])),
        "at_dawn" => Some(raw(&[
            &[(0.18, 0.64), (0.82, 0.64)],
            &[
                (0.34, 0.64),
                (0.42, 0.42),
                (0.50, 0.36),
                (0.58, 0.42),
                (0.66, 0.64),
            ],
            &[(0.50, 0.18), (0.50, 0.30)],
        ])),
        "at_night" => Some(raw(&[&[
            (0.64, 0.16),
            (0.42, 0.24),
            (0.30, 0.48),
            (0.42, 0.74),
            (0.66, 0.86),
            (0.54, 0.60),
            (0.54, 0.38),
            (0.64, 0.16),
        ]])),
        "safer" => Some(raw(&[&[
            (0.50, 0.12),
            (0.80, 0.26),
            (0.74, 0.66),
            (0.50, 0.88),
            (0.26, 0.66),
            (0.20, 0.26),
            (0.50, 0.12),
        ]])),
        "stronger" => Some(raw(&[
            &[(0.50, 0.84), (0.50, 0.18)],
            &[(0.30, 0.38), (0.50, 0.18), (0.70, 0.38)],
            &[(0.30, 0.66), (0.70, 0.66)],
        ])),
        "larger" => Some(raw(&[
            &[
                (0.38, 0.38),
                (0.62, 0.38),
                (0.62, 0.62),
                (0.38, 0.62),
                (0.38, 0.38),
            ],
            &[(0.16, 0.50), (0.30, 0.50)],
            &[(0.70, 0.50), (0.84, 0.50)],
            &[(0.50, 0.16), (0.50, 0.30)],
            &[(0.50, 0.70), (0.50, 0.84)],
        ])),
        "longer_duration" => Some(raw(&[
            &[(0.16, 0.50), (0.84, 0.50)],
            &[(0.28, 0.36), (0.28, 0.64)],
            &[(0.50, 0.36), (0.50, 0.64)],
            &[(0.72, 0.36), (0.72, 0.64)],
        ])),
        "faster" => Some(raw(&[
            &[(0.18, 0.38), (0.70, 0.38)],
            &[(0.54, 0.24), (0.70, 0.38), (0.54, 0.52)],
            &[(0.30, 0.62), (0.82, 0.62)],
            &[(0.66, 0.48), (0.82, 0.62), (0.66, 0.76)],
        ])),
        "hidden" => Some(raw(&[
            &[
                (0.14, 0.50),
                (0.34, 0.28),
                (0.50, 0.22),
                (0.66, 0.28),
                (0.86, 0.50),
                (0.66, 0.72),
                (0.50, 0.78),
                (0.34, 0.72),
                (0.14, 0.50),
            ],
            &[(0.24, 0.82), (0.78, 0.18)],
        ])),
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct NormalizedDrawing {
    strokes: Vec<Vec<StrokePoint>>,
    aspect_ratio: f32,
    total_length: f32,
}

impl NormalizedDrawing {
    fn from_strokes(strokes: &[DrawnStroke]) -> Option<Self> {
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

fn raw(strokes: &[&[(f32, f32)]]) -> Vec<DrawnStroke> {
    strokes
        .iter()
        .map(|stroke| DrawnStroke {
            points: stroke
                .iter()
                .map(|(x, y)| StrokePoint::new(*x, *y))
                .collect(),
        })
        .collect()
}

fn circle(cx: f32, cy: f32, radius: f32, steps: usize) -> Vec<DrawnStroke> {
    let mut points = Vec::with_capacity(steps + 1);
    for index in 0..=steps {
        let angle = -std::f32::consts::FRAC_PI_2
            + std::f32::consts::TAU * index as f32 / steps.max(1) as f32;
        points.push(StrokePoint::new(
            cx + radius * angle.cos(),
            cy + radius * angle.sin(),
        ));
    }
    vec![DrawnStroke { points }]
}

#[cfg(test)]
mod tests;
