use super::{DrawnStroke, StrokePoint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShapeIssue {
    NotClosed,
    NotRoundEnough,
    TooManyStraightLines,
    NotEnoughSides,
    TooManySides,
    NotStraightEnough,
    ShouldBeOpen,
    MissingArrowStructure,
    MissingBeamStructure,
    MissingAuraStructure,
    MissingBurstStructure,
    MissingConeStructure,
    MissingDiamondStructure,
}

impl ShapeIssue {
    pub(crate) fn message(self) -> &'static str {
        match self {
            ShapeIssue::NotClosed => "The stroke needs to close cleanly.",
            ShapeIssue::NotRoundEnough => "The circle drifts too far from a steady radius.",
            ShapeIssue::TooManyStraightLines => "Too many straight sides are showing for Sphere.",
            ShapeIssue::NotEnoughSides => "Safer needs six clear sides.",
            ShapeIssue::TooManySides => "Safer has too many corners; keep it to six sides.",
            ShapeIssue::NotStraightEnough => "The straight rune lines need to be cleaner.",
            ShapeIssue::ShouldBeOpen => "Touch should be an open arrow, not a closed shape.",
            ShapeIssue::MissingArrowStructure => "Touch needs a clear shaft and arrow head.",
            ShapeIssue::MissingBeamStructure => {
                "Beam needs a clear right-pointing shaft and arrow head."
            }
            ShapeIssue::MissingAuraStructure => {
                "Aura needs a closed outer shape with a clear center bar."
            }
            ShapeIssue::MissingBurstStructure => {
                "Burst needs straight rays crossing through the center."
            }
            ShapeIssue::MissingConeStructure => "Cone needs a closed triangular outline.",
            ShapeIssue::MissingDiamondStructure => "Force needs a closed diamond outline.",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuneShapeReport {
    pub(crate) structural_score: f32,
    pub(crate) issue: Option<ShapeIssue>,
}

pub(crate) fn shape_report_for_rune(
    rune_id: &str,
    strokes: &[DrawnStroke],
) -> Option<RuneShapeReport> {
    let ink = NormalizedInk::from_strokes(strokes)?;
    match rune_id {
        "sphere" => Some(sphere_report(&ink)),
        "safer" => Some(safer_report(&ink)),
        "touch" => Some(touch_report(&ink)),
        "beam" => Some(beam_report(&ink)),
        "aura" => Some(aura_report(&ink)),
        "burst" => Some(burst_report(&ink)),
        "cone" => Some(cone_report(&ink)),
        "force" => Some(diamond_report(&ink)),
        _ => None,
    }
}

fn sphere_report(ink: &NormalizedInk) -> RuneShapeReport {
    let Some(stroke) = ink.single_stroke() else {
        return RuneShapeReport {
            structural_score: 0.35,
            issue: Some(ShapeIssue::NotClosed),
        };
    };
    let closed = closure_score(stroke);
    let circular = circularity(stroke, ink.aspect_ratio);
    let corners = corner_count(stroke);
    let corner_penalty = if corners >= 6.0 {
        (corners - 5.0) * 0.06
    } else {
        0.0
    };
    let structural_score = (closed * 0.30 + circular * 0.70 - corner_penalty).clamp(0.0, 1.0);
    let issue = if closed < 0.72 {
        Some(ShapeIssue::NotClosed)
    } else if circular < 0.66 {
        Some(ShapeIssue::NotRoundEnough)
    } else if corners >= 7.0 && circular < 0.82 {
        Some(ShapeIssue::TooManyStraightLines)
    } else {
        None
    };
    RuneShapeReport {
        structural_score,
        issue,
    }
}

fn beam_report(ink: &NormalizedInk) -> RuneShapeReport {
    let any_closed = ink
        .strokes
        .iter()
        .any(|stroke| closure_score(stroke) > 0.72);
    if any_closed {
        return RuneShapeReport {
            structural_score: 0.18,
            issue: Some(ShapeIssue::ShouldBeOpen),
        };
    }

    let directness = average_directness(ink);
    let rightward = rightward_arrow_score(ink);
    let stroke_count_score = if (1..=3).contains(&ink.strokes.len()) {
        1.0
    } else {
        0.45
    };
    let structural_score =
        (directness * 0.38 + rightward * 0.42 + stroke_count_score * 0.20).clamp(0.0, 1.0);
    let issue = if rightward < 0.50 {
        Some(ShapeIssue::MissingBeamStructure)
    } else if directness < 0.54 {
        Some(ShapeIssue::NotStraightEnough)
    } else {
        None
    };
    RuneShapeReport {
        structural_score,
        issue,
    }
}

fn aura_report(ink: &NormalizedInk) -> RuneShapeReport {
    if ink.strokes.len() < 2 {
        return RuneShapeReport {
            structural_score: 0.32,
            issue: Some(ShapeIssue::MissingAuraStructure),
        };
    }

    let closed_stroke = ink
        .strokes
        .iter()
        .enumerate()
        .max_by(|(a_index, a), (b_index, b)| {
            closure_score(a)
                .total_cmp(&closure_score(b))
                .then_with(|| b_index.cmp(a_index))
        });
    let Some((outline_index, outline)) = closed_stroke else {
        return RuneShapeReport {
            structural_score: 0.20,
            issue: Some(ShapeIssue::MissingAuraStructure),
        };
    };
    let closed = closure_score(outline);
    let corners = corner_count(outline);
    let side_score = (1.0 - (corners - 6.0).abs() / 4.0).clamp(0.0, 1.0);
    let bar = ink
        .strokes
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != outline_index)
        .map(|(_, stroke)| horizontal_center_bar_score(stroke))
        .fold(0.0, f32::max);
    let structural_score =
        (closed * 0.26 + side_score * 0.24 + bar * 0.36 + count_score(ink.strokes.len(), 2) * 0.14)
            .clamp(0.0, 1.0);
    let issue = if closed < 0.70 {
        Some(ShapeIssue::NotClosed)
    } else if bar < 0.48 {
        Some(ShapeIssue::MissingAuraStructure)
    } else {
        None
    };
    RuneShapeReport {
        structural_score,
        issue,
    }
}

fn burst_report(ink: &NormalizedInk) -> RuneShapeReport {
    let directness = average_directness(ink);
    let angle_score = burst_angle_score(ink);
    let center_score = burst_center_score(ink);
    let count = count_score(ink.strokes.len(), 4);
    let structural_score =
        (directness * 0.30 + angle_score * 0.30 + center_score * 0.25 + count * 0.15)
            .clamp(0.0, 1.0);
    let issue = if center_score < 0.48 || angle_score < 0.50 {
        Some(ShapeIssue::MissingBurstStructure)
    } else if directness < 0.54 {
        Some(ShapeIssue::NotStraightEnough)
    } else {
        None
    };
    RuneShapeReport {
        structural_score,
        issue,
    }
}

fn cone_report(ink: &NormalizedInk) -> RuneShapeReport {
    let Some(stroke) = ink.single_stroke() else {
        return RuneShapeReport {
            structural_score: 0.30,
            issue: Some(ShapeIssue::MissingConeStructure),
        };
    };
    let closed = closure_score(stroke);
    let corners = corner_count(stroke);
    let side_score = (1.0 - (corners - 3.0).abs() / 3.0).clamp(0.0, 1.0);
    let straight = straight_section_score(stroke);
    let structural_score = (closed * 0.28 + side_score * 0.42 + straight * 0.30).clamp(0.0, 1.0);
    let issue = if closed < 0.70 {
        Some(ShapeIssue::NotClosed)
    } else if corners < 3.0 || side_score < 0.50 {
        Some(ShapeIssue::MissingConeStructure)
    } else if straight < 0.54 {
        Some(ShapeIssue::NotStraightEnough)
    } else {
        None
    };
    RuneShapeReport {
        structural_score,
        issue,
    }
}

fn diamond_report(ink: &NormalizedInk) -> RuneShapeReport {
    let Some(stroke) = ink.single_stroke() else {
        return RuneShapeReport {
            structural_score: 0.30,
            issue: Some(ShapeIssue::MissingDiamondStructure),
        };
    };
    let closed = closure_score(stroke);
    let corners = corner_count(stroke);
    let side_score = (1.0 - (corners - 4.0).abs() / 3.0).clamp(0.0, 1.0);
    let straight = straight_section_score(stroke);
    let structural_score = (closed * 0.26 + side_score * 0.44 + straight * 0.30).clamp(0.0, 1.0);
    let issue = if closed < 0.70 {
        Some(ShapeIssue::NotClosed)
    } else if corners < 4.0 || side_score < 0.55 {
        Some(ShapeIssue::MissingDiamondStructure)
    } else if straight < 0.54 {
        Some(ShapeIssue::NotStraightEnough)
    } else {
        None
    };
    RuneShapeReport {
        structural_score,
        issue,
    }
}

fn safer_report(ink: &NormalizedInk) -> RuneShapeReport {
    let Some(stroke) = ink.single_stroke() else {
        return RuneShapeReport {
            structural_score: 0.30,
            issue: Some(ShapeIssue::NotClosed),
        };
    };
    let closed = closure_score(stroke);
    let corners = corner_count(stroke);
    let side_score = (1.0 - (corners - 6.0).abs() / 4.0).clamp(0.0, 1.0);
    let straight = straight_section_score(stroke);
    let structural_score = (closed * 0.24 + side_score * 0.46 + straight * 0.30).clamp(0.0, 1.0);
    let issue = if closed < 0.70 {
        Some(ShapeIssue::NotClosed)
    } else if corners < 5.0 {
        Some(ShapeIssue::NotEnoughSides)
    } else if corners > 8.0 {
        Some(ShapeIssue::TooManySides)
    } else if straight < 0.54 {
        Some(ShapeIssue::NotStraightEnough)
    } else {
        None
    };
    RuneShapeReport {
        structural_score,
        issue,
    }
}

fn touch_report(ink: &NormalizedInk) -> RuneShapeReport {
    let any_closed = ink
        .strokes
        .iter()
        .any(|stroke| closure_score(stroke) > 0.72);
    if any_closed {
        return RuneShapeReport {
            structural_score: 0.18,
            issue: Some(ShapeIssue::ShouldBeOpen),
        };
    }

    let directness = ink
        .strokes
        .iter()
        .map(|stroke| stroke_directness(stroke))
        .sum::<f32>()
        / ink.strokes.len().max(1) as f32;
    let downward = touch_downward_score(ink);
    let stroke_count_score = if (1..=3).contains(&ink.strokes.len()) {
        1.0
    } else {
        0.45
    };
    let structural_score =
        (directness * 0.42 + downward * 0.38 + stroke_count_score * 0.20).clamp(0.0, 1.0);
    let issue = if downward < 0.48 {
        Some(ShapeIssue::MissingArrowStructure)
    } else if directness < 0.54 {
        Some(ShapeIssue::NotStraightEnough)
    } else {
        None
    };
    RuneShapeReport {
        structural_score,
        issue,
    }
}

#[derive(Debug, Clone)]
struct NormalizedInk {
    strokes: Vec<Vec<StrokePoint>>,
    aspect_ratio: f32,
}

impl NormalizedInk {
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
                    .collect()
            })
            .collect();

        Some(Self {
            strokes: normalized,
            aspect_ratio: width / height,
        })
    }

    fn single_stroke(&self) -> Option<&[StrokePoint]> {
        (self.strokes.len() == 1).then_some(self.strokes[0].as_slice())
    }
}

fn circularity(points: &[StrokePoint], aspect_ratio: f32) -> f32 {
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
fn corner_count(points: &[StrokePoint]) -> f32 {
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

fn straight_section_score(points: &[StrokePoint]) -> f32 {
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

fn touch_downward_score(ink: &NormalizedInk) -> f32 {
    let points = ink
        .strokes
        .iter()
        .flat_map(|stroke| stroke.iter().copied())
        .collect::<Vec<_>>();
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

fn average_directness(ink: &NormalizedInk) -> f32 {
    ink.strokes
        .iter()
        .map(|stroke| stroke_directness(stroke))
        .sum::<f32>()
        / ink.strokes.len().max(1) as f32
}

fn rightward_arrow_score(ink: &NormalizedInk) -> f32 {
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

fn horizontal_center_bar_score(stroke: &[StrokePoint]) -> f32 {
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

fn burst_angle_score(ink: &NormalizedInk) -> f32 {
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

fn burst_center_score(ink: &NormalizedInk) -> f32 {
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

fn count_score(candidate_count: usize, template_count: usize) -> f32 {
    let missing =
        template_count.saturating_sub(candidate_count) as f32 / template_count.max(1) as f32;
    let extra =
        candidate_count.saturating_sub(template_count) as f32 / template_count.max(1) as f32;
    (1.0 - missing * 0.50 - extra * 0.24).clamp(0.10, 1.0)
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

fn closure_score(points: &[StrokePoint]) -> f32 {
    let Some(first) = points.first() else {
        return 0.0;
    };
    let Some(last) = points.last() else {
        return 0.0;
    };
    (1.0 - first.distance(*last) / 0.18).clamp(0.0, 1.0)
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
mod tests {
    use super::*;

    fn points(raw: &[(f32, f32)]) -> Vec<StrokePoint> {
        raw.iter().map(|(x, y)| StrokePoint::new(*x, *y)).collect()
    }

    fn rounded_corners(points: &[StrokePoint]) -> i32 {
        corner_count(points).round() as i32
    }

    #[test]
    fn open_straight_line_has_no_corners() {
        let line = points(&[(0.15, 0.20), (0.85, 0.80)]);

        assert_eq!(rounded_corners(&line), 0);
    }

    #[test]
    fn open_bend_counts_a_single_corner() {
        let bend = points(&[(0.20, 0.20), (0.50, 0.80), (0.80, 0.20)]);

        assert_eq!(rounded_corners(&bend), 1);
    }

    #[test]
    fn closed_square_still_counts_four_corners() {
        let square = points(&[
            (0.20, 0.20),
            (0.80, 0.20),
            (0.80, 0.80),
            (0.20, 0.80),
            (0.20, 0.20),
        ]);

        assert_eq!(rounded_corners(&square), 4);
    }

    #[test]
    fn closed_hexagon_reads_closer_to_six_corners_than_four() {
        // The "safer" rune template: a hand-authored hexagon whose corners
        // aren't all equal-angle. Two of them (turn ~0.638 rad) fall just
        // short of the closed-shape threshold (0.60 was chosen so these
        // clear it; the *open*-stroke threshold is 0.68, which would have
        // missed them). corner_count is now continuous (corner_confidence
        // is a sigmoid, not a hard cutoff), so those two soft corners
        // contribute partial weight rather than either flipping fully in or
        // vanishing outright — the total lands near, not exactly at, 6.0.
        // What matters for recognition is that it reads far closer to a
        // hexagon (6) than a diamond (4); see safer_template_still_recognizes_safer
        // and the confusion-matrix gate for the actual recognition outcome.
        let hexagon = points(&[
            (0.50, 0.12),
            (0.80, 0.26),
            (0.74, 0.66),
            (0.50, 0.88),
            (0.26, 0.66),
            (0.20, 0.26),
            (0.50, 0.12),
        ]);

        let corners = corner_count(&hexagon);
        assert!(
            (5.0..=6.2).contains(&corners),
            "corners={corners}, expected close to 6"
        );
    }

    #[test]
    fn closed_diamond_still_counts_four_corners() {
        let diamond = points(&[
            (0.50, 0.12),
            (0.86, 0.50),
            (0.50, 0.88),
            (0.14, 0.50),
            (0.50, 0.12),
        ]);

        assert_eq!(rounded_corners(&diamond), 4);
    }
}
