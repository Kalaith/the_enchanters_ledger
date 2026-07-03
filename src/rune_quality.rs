//! Strict rune draftsmanship scoring for rewards and practice.

use crate::data::RuneDef;
use crate::rune_drawing::{
    acceptance_band, merge_continuation_strokes, recognize_rune_in_context, shape_report_for_rune,
    stroke_assignment, template_strokes_for_rune, template_variants_for_rune, DrawnStroke,
    RecognitionContext, ShapeIssue, StrokePoint,
};

#[derive(Debug, Clone)]
pub struct RunePracticeReport {
    pub target_rune_id: String,
    pub read_rune_id: Option<String>,
    pub confidence: f32,
    pub quality: f32,
    pub shape_score: f32,
    pub start_score: f32,
    pub stroke_order_score: f32,
    pub accepted: bool,
    pub feedback: String,
    pub mismatch_segments: Vec<PracticeMismatchSegment>,
}

#[derive(Debug, Clone, Copy)]
pub struct PracticeMismatchSegment {
    pub from: StrokePoint,
    pub to: StrokePoint,
    pub severity: f32,
}

pub fn strict_quality_for_rune(rune_id: &str, strokes: &[DrawnStroke]) -> Option<f32> {
    best_variant_scores(rune_id, strokes).map(|scores| scores.quality)
}

#[derive(Debug, Clone, Copy)]
struct StrictScores {
    shape: f32,
    start: f32,
    order: f32,
    quality: f32,
}

/// Scores the drawing against every accepted template variant and keeps the
/// best fit, so strict quality agrees with recognition instead of punishing a
/// legal variant for not matching the canonical stroke plan.
fn best_variant_scores(rune_id: &str, strokes: &[DrawnStroke]) -> Option<StrictScores> {
    let strokes = merge_continuation_strokes(strokes);
    let candidate = NormalizedDrawing::from_strokes(&strokes)?;
    template_variants_for_rune(rune_id)
        .into_iter()
        .filter_map(|template| {
            let template = NormalizedDrawing::from_strokes(&template)?;
            let shape = ordered_shape_score(&candidate, &template);
            let start = start_point_score(&candidate, &template);
            let order = stroke_order_score(&candidate, &template);
            let quality = (shape * 0.46 + start * 0.22 + order * 0.32).clamp(0.0, 1.0);
            Some(StrictScores {
                shape,
                start,
                order,
                quality,
            })
        })
        .reduce(|best, next| if next.quality > best.quality { next } else { best })
}

/// Scores a Practice-slate drawing (plan Phase 5 item 1): recognition itself is identical to
/// everywhere else, but acceptance uses `RecognitionContext::Practice`'s stricter band — this is
/// where correct technique is taught, so a mark that would pass in commission work still gets
/// flagged here.
pub fn practice_report_for_rune<'a>(
    rune_id: &str,
    strokes: &[DrawnStroke],
    runes: impl IntoIterator<Item = &'a RuneDef>,
) -> Option<RunePracticeReport> {
    let scores = best_variant_scores(rune_id, strokes)?;
    let recognized = recognize_rune_in_context(strokes, runes, RecognitionContext::Practice);
    let shape_score = scores.shape;
    let start_score = scores.start;
    let stroke_order_score = scores.order;
    let quality = scores.quality;
    let read_rune_id = recognized.as_ref().map(|outcome| outcome.rune_id.clone());
    let confidence = recognized
        .as_ref()
        .map_or(0.0, |outcome| outcome.confidence);
    let read_accepted = recognized.as_ref().is_some_and(|outcome| outcome.accepted);
    let accepted = read_rune_id.as_deref() == Some(rune_id) && read_accepted;
    let issue = shape_report_for_rune(rune_id, strokes).and_then(|report| report.issue);

    Some(RunePracticeReport {
        target_rune_id: rune_id.to_owned(),
        read_rune_id,
        confidence,
        quality,
        shape_score,
        start_score,
        stroke_order_score,
        accepted,
        mismatch_segments: mismatch_segments_for_rune(rune_id, strokes).unwrap_or_default(),
        feedback: practice_feedback(
            accepted,
            confidence,
            shape_score,
            start_score,
            stroke_order_score,
            issue,
        ),
    })
}

pub fn quality_label(score: f32) -> &'static str {
    if score >= 0.92 {
        "Masterwork"
    } else if score >= 0.78 {
        "Clean"
    } else if score >= 0.58 {
        "Readable"
    } else {
        "Practice"
    }
}

fn practice_feedback(
    accepted: bool,
    confidence: f32,
    shape: f32,
    start: f32,
    order: f32,
    issue: Option<ShapeIssue>,
) -> String {
    if let Some(issue) = issue {
        return issue.message().to_owned();
    }
    if confidence < acceptance_band(RecognitionContext::Practice).confidence {
        return "The mark is not readable yet. Trace the ghost lines and keep each stroke separate."
            .to_owned();
    }
    if start < 0.68 {
        return "Start each stroke on the gold dot. Circles score best when they begin at the top."
            .to_owned();
    }
    if order < 0.68 {
        return "Use the numbered stroke order and keep each stroke moving in the shown direction."
            .to_owned();
    }
    if shape < 0.70 {
        return "The rune reads, but the line shape drifts away from the guide.".to_owned();
    }
    if accepted {
        "Clean practice mark. This stroke discipline will earn better workshop results.".to_owned()
    } else {
        "Readable, but not clean enough for high-rating work yet.".to_owned()
    }
}

/// Highlights where a drawing deviates from its rune's template. Pairs
/// drawn strokes to template strokes using the same order-insensitive
/// assignment identity scoring uses (`stroke_assignment`), rather than
/// zipping in input order — otherwise a rune drawn in a different (but
/// still legal, since identity is order-insensitive) stroke order would
/// get its mismatch highlights compared against the wrong template stroke.
fn mismatch_segments_for_rune(
    rune_id: &str,
    strokes: &[DrawnStroke],
) -> Option<Vec<PracticeMismatchSegment>> {
    let template = template_strokes_for_rune(rune_id)?;
    let fit = DrawingFit::from_strokes(strokes)?;
    // Templates are stored at their own natural slate position (e.g. "light"
    // only spans x 0.25..0.75), not pre-normalized to a unit box, so map
    // each template point into its own unit box first, then into the
    // drawing's frame — otherwise a perfect, unshuffled copy of the
    // template would still get flagged as a mismatch (D3 follow-up).
    let template_fit = DrawingFit::from_strokes(&template)?;
    let mut segments = Vec::new();

    let inked_strokes: Vec<&DrawnStroke> = strokes.iter().filter(|s| s.has_ink()).collect();
    let candidate_points: Vec<Vec<StrokePoint>> = inked_strokes
        .iter()
        .map(|stroke| stroke.points.clone())
        .collect();
    let template_points: Vec<Vec<StrokePoint>> =
        template.iter().map(|stroke| stroke.points.clone()).collect();
    let assignment = stroke_assignment(&candidate_points, &template_points);

    let mut matched = vec![false; inked_strokes.len()];
    for (template_stroke, (candidate_index, _)) in template.iter().zip(assignment.iter()) {
        let Some(candidate_index) = *candidate_index else {
            continue;
        };
        matched[candidate_index] = true;
        let candidate = inked_strokes[candidate_index];
        let candidate_samples = resample(&candidate.points, 36);
        let template_samples = resample(&template_stroke.points, 36);
        for index in 1..candidate_samples.len().min(template_samples.len()) {
            let expected = fit.unit_to_source(template_fit.source_to_unit(template_samples[index]));
            let error = distance(candidate_samples[index], expected) / fit.span.max(0.001);
            if error > 0.12 {
                segments.push(PracticeMismatchSegment {
                    from: candidate_samples[index - 1],
                    to: candidate_samples[index],
                    severity: ((error - 0.12) / 0.18).clamp(0.25, 1.0),
                });
            }
        }
    }

    for (index, candidate) in inked_strokes.iter().enumerate() {
        if matched[index] {
            continue;
        }
        for segment in candidate.points.windows(2) {
            segments.push(PracticeMismatchSegment {
                from: segment[0],
                to: segment[1],
                severity: 1.0,
            });
        }
    }

    Some(segments)
}

#[derive(Debug, Clone, Copy)]
struct DrawingFit {
    origin_x: f32,
    origin_y: f32,
    span: f32,
}

impl DrawingFit {
    fn from_strokes(strokes: &[DrawnStroke]) -> Option<Self> {
        let mut points = strokes
            .iter()
            .filter(|stroke| stroke.has_ink())
            .flat_map(|stroke| &stroke.points);
        let first = *points.next()?;
        let mut min_x = first.x;
        let mut min_y = first.y;
        let mut max_x = first.x;
        let mut max_y = first.y;
        for point in points {
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
        Some(Self {
            origin_x: min_x - pad_x,
            origin_y: min_y - pad_y,
            span,
        })
    }

    fn unit_to_source(self, point: StrokePoint) -> StrokePoint {
        StrokePoint::new(
            self.origin_x + point.x * self.span,
            self.origin_y + point.y * self.span,
        )
    }

    fn source_to_unit(self, point: StrokePoint) -> StrokePoint {
        StrokePoint::new(
            (point.x - self.origin_x) / self.span.max(0.0001),
            (point.y - self.origin_y) / self.span.max(0.0001),
        )
    }
}

#[derive(Debug, Clone)]
struct NormalizedDrawing {
    strokes: Vec<Vec<StrokePoint>>,
    aspect_ratio: f32,
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

        Some(Self {
            strokes: normalized,
            aspect_ratio: width / height,
        })
    }
}

fn ordered_shape_score(candidate: &NormalizedDrawing, template: &NormalizedDrawing) -> f32 {
    let pairs = candidate.strokes.len().min(template.strokes.len());
    if pairs == 0 {
        return 0.0;
    }
    let stroke_score = candidate
        .strokes
        .iter()
        .zip(&template.strokes)
        .map(|(candidate, template)| strict_stroke_score(candidate, template))
        .sum::<f32>()
        / template.strokes.len().max(1) as f32;
    let count_score = count_score(candidate.strokes.len(), template.strokes.len());
    let aspect_score = ratio_score(candidate.aspect_ratio, template.aspect_ratio);
    (stroke_score * 0.78 + count_score * 0.12 + aspect_score * 0.10).clamp(0.0, 1.0)
}

fn stroke_order_score(candidate: &NormalizedDrawing, template: &NormalizedDrawing) -> f32 {
    if template.strokes.is_empty() {
        return 0.0;
    }
    let ordered = candidate
        .strokes
        .iter()
        .zip(&template.strokes)
        .map(|(candidate, template)| strict_stroke_score(candidate, template))
        .sum::<f32>()
        / template.strokes.len() as f32;
    (ordered * count_score(candidate.strokes.len(), template.strokes.len())).clamp(0.0, 1.0)
}

fn start_point_score(candidate: &NormalizedDrawing, template: &NormalizedDrawing) -> f32 {
    if template.strokes.is_empty() {
        return 0.0;
    }
    let score = candidate
        .strokes
        .iter()
        .zip(&template.strokes)
        .map(|(candidate, template)| {
            let Some(candidate_start) = candidate.first() else {
                return 0.0;
            };
            let Some(template_start) = template.first() else {
                return 0.0;
            };
            (1.0 - distance(*candidate_start, *template_start) / 0.38).clamp(0.0, 1.0)
        })
        .sum::<f32>()
        / template.strokes.len() as f32;
    (score * count_score(candidate.strokes.len(), template.strokes.len())).clamp(0.0, 1.0)
}

fn strict_stroke_score(candidate: &[StrokePoint], template: &[StrokePoint]) -> f32 {
    let candidate = resample(candidate, 24);
    let template = resample(template, 24);
    let distance_score = (1.0 - average_distance(&candidate, &template) / 0.43).clamp(0.0, 1.0);
    let endpoint_score = endpoint_score(&candidate, &template);
    (distance_score * 0.80 + endpoint_score * 0.20).clamp(0.0, 1.0)
}

fn endpoint_score(candidate: &[StrokePoint], template: &[StrokePoint]) -> f32 {
    let Some(candidate_start) = candidate.first() else {
        return 0.0;
    };
    let Some(candidate_end) = candidate.last() else {
        return 0.0;
    };
    let Some(template_start) = template.first() else {
        return 0.0;
    };
    let Some(template_end) = template.last() else {
        return 0.0;
    };
    let error = (distance(*candidate_start, *template_start)
        + distance(*candidate_end, *template_end))
        * 0.5;
    (1.0 - error / 0.42).clamp(0.0, 1.0)
}

fn count_score(candidate_count: usize, template_count: usize) -> f32 {
    let missing =
        template_count.saturating_sub(candidate_count) as f32 / template_count.max(1) as f32;
    let extra =
        candidate_count.saturating_sub(template_count) as f32 / template_count.max(1) as f32;
    (1.0 - missing * 0.50 - extra * 0.32).clamp(0.10, 1.0)
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
        let length = distance(start, end);
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
        .map(|segment| distance(segment[0], segment[1]))
        .sum()
}

fn average_distance(a: &[StrokePoint], b: &[StrokePoint]) -> f32 {
    let count = a.len().min(b.len()).max(1);
    a.iter()
        .zip(b.iter())
        .take(count)
        .map(|(a, b)| distance(*a, *b))
        .sum::<f32>()
        / count as f32
}

fn distance(a: StrokePoint, b: StrokePoint) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::rune_drawing::recognize_rune;

    #[test]
    fn circle_reads_with_wrong_start_but_scores_lower() {
        let data = GameData::load().unwrap();
        let canonical = template_strokes_for_rune("sphere").unwrap();
        let wrong_start = circle_starting_right();

        let canonical_read = recognize_rune(&canonical, rank_one(&data)).unwrap();
        let wrong_read = recognize_rune(&wrong_start, rank_one(&data)).unwrap();

        assert_eq!(wrong_read.rune_id, "sphere");
        assert!(wrong_read.accepted, "{wrong_read:?}");
        assert!(
            canonical_read.quality > wrong_read.quality + 0.08,
            "canonical={canonical_read:?} wrong={wrong_read:?}"
        );
    }

    #[test]
    fn practice_report_rewards_canonical_start_and_order() {
        let data = GameData::load().unwrap();
        let strokes = template_strokes_for_rune("light").unwrap();

        let report = practice_report_for_rune("light", &strokes, rank_one(&data)).unwrap();

        assert!(report.accepted, "{report:?}");
        assert!(report.quality > 0.92, "{report:?}");
        assert!(report.start_score > 0.92, "{report:?}");
        assert!(report.stroke_order_score > 0.92, "{report:?}");
    }

    #[test]
    fn canonical_template_draws_no_mismatch_segments() {
        let data = GameData::load().unwrap();
        let strokes = template_strokes_for_rune("light").unwrap();

        let report = practice_report_for_rune("light", &strokes, rank_one(&data)).unwrap();

        assert!(
            report.mismatch_segments.is_empty(),
            "a perfect copy of the template should never be flagged: {report:?}"
        );
    }

    #[test]
    fn reordered_strokes_do_not_produce_spurious_mismatches() {
        let data = GameData::load().unwrap();
        let mut shuffled = template_strokes_for_rune("light").unwrap();
        shuffled.reverse();

        let report = practice_report_for_rune("light", &shuffled, rank_one(&data)).unwrap();

        assert!(report.accepted, "{report:?}");
        assert!(
            report.mismatch_segments.is_empty(),
            "reordered-but-perfect strokes should not flag mismatches: {report:?}"
        );
    }

    #[test]
    fn practice_acceptance_matches_normal_recognition() {
        let data = GameData::load().unwrap();
        let strokes = rough_sphere();

        let recognized = recognize_rune(&strokes, rank_one(&data)).unwrap();
        let report = practice_report_for_rune("sphere", &strokes, rank_one(&data)).unwrap();

        assert_eq!(recognized.rune_id, "sphere", "{recognized:?}");
        assert_eq!(report.accepted, recognized.accepted, "{report:?}");
    }

    #[test]
    fn down_arrow_variant_earns_full_strict_quality() {
        let data = GameData::load().unwrap();
        let down_arrow = vec![DrawnStroke {
            points: vec![
                StrokePoint::new(0.50, 0.16),
                StrokePoint::new(0.50, 0.84),
                StrokePoint::new(0.34, 0.64),
                StrokePoint::new(0.50, 0.84),
                StrokePoint::new(0.66, 0.64),
            ],
        }];

        let strict = strict_quality_for_rune("touch", &down_arrow).unwrap();
        let report = practice_report_for_rune("touch", &down_arrow, rank_one(&data)).unwrap();

        assert!(strict > 0.90, "strict={strict} report={report:?}");
        assert!(report.accepted, "{report:?}");
        assert!(report.stroke_order_score > 0.90, "{report:?}");
    }

    #[test]
    fn practice_explains_missing_safer_sides() {
        let data = GameData::load().unwrap();
        let triangle = vec![DrawnStroke {
            points: vec![
                StrokePoint::new(0.50, 0.16),
                StrokePoint::new(0.84, 0.78),
                StrokePoint::new(0.16, 0.78),
                StrokePoint::new(0.50, 0.16),
            ],
        }];

        let report = practice_report_for_rune("safer", &triangle, rank_one(&data)).unwrap();

        assert!(!report.accepted, "{report:?}");
        assert!(report.feedback.contains("six clear sides"), "{report:?}");
        assert!(!report.mismatch_segments.is_empty(), "{report:?}");
    }

    fn rank_one(data: &GameData) -> Vec<&RuneDef> {
        data.runes.iter().filter(|rune| rune.tier == 1).collect()
    }

    fn circle_starting_right() -> Vec<DrawnStroke> {
        let mut points = Vec::new();
        for index in 0..=24 {
            let angle = std::f32::consts::TAU * index as f32 / 24.0;
            points.push(StrokePoint::new(
                0.50 + 0.34 * angle.cos(),
                0.50 + 0.34 * angle.sin(),
            ));
        }
        vec![DrawnStroke { points }]
    }

    fn rough_sphere() -> Vec<DrawnStroke> {
        vec![DrawnStroke {
            points: vec![
                StrokePoint::new(0.50, 0.14),
                StrokePoint::new(0.76, 0.24),
                StrokePoint::new(0.86, 0.52),
                StrokePoint::new(0.68, 0.80),
                StrokePoint::new(0.38, 0.84),
                StrokePoint::new(0.16, 0.62),
                StrokePoint::new(0.22, 0.30),
                StrokePoint::new(0.50, 0.14),
            ],
        }]
    }
}
