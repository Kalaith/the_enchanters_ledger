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
    let corner_penalty = if corners >= 6 {
        (corners as f32 - 5.0) * 0.06
    } else {
        0.0
    };
    let structural_score = (closed * 0.30 + circular * 0.70 - corner_penalty).clamp(0.0, 1.0);
    let issue = if closed < 0.72 {
        Some(ShapeIssue::NotClosed)
    } else if circular < 0.66 {
        Some(ShapeIssue::NotRoundEnough)
    } else if corners >= 7 && circular < 0.82 {
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
        .max_by(|(_, a), (_, b)| closure_score(a).total_cmp(&closure_score(b)));
    let Some((outline_index, outline)) = closed_stroke else {
        return RuneShapeReport {
            structural_score: 0.20,
            issue: Some(ShapeIssue::MissingAuraStructure),
        };
    };
    let closed = closure_score(outline);
    let corners = corner_count(outline);
    let side_score = (1.0 - (corners as f32 - 6.0).abs() / 4.0).clamp(0.0, 1.0);
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
    let side_score = (1.0 - (corners as f32 - 3.0).abs() / 3.0).clamp(0.0, 1.0);
    let straight = straight_section_score(stroke);
    let structural_score = (closed * 0.28 + side_score * 0.42 + straight * 0.30).clamp(0.0, 1.0);
    let issue = if closed < 0.70 {
        Some(ShapeIssue::NotClosed)
    } else if corners < 3 || side_score < 0.50 {
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
    let side_score = (1.0 - (corners as f32 - 4.0).abs() / 3.0).clamp(0.0, 1.0);
    let straight = straight_section_score(stroke);
    let structural_score = (closed * 0.26 + side_score * 0.44 + straight * 0.30).clamp(0.0, 1.0);
    let issue = if closed < 0.70 {
        Some(ShapeIssue::NotClosed)
    } else if corners < 4 || side_score < 0.55 {
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
    let side_score = (1.0 - (corners as f32 - 6.0).abs() / 4.0).clamp(0.0, 1.0);
    let straight = straight_section_score(stroke);
    let structural_score = (closed * 0.24 + side_score * 0.46 + straight * 0.30).clamp(0.0, 1.0);
    let issue = if closed < 0.70 {
        Some(ShapeIssue::NotClosed)
    } else if corners < 5 {
        Some(ShapeIssue::NotEnoughSides)
    } else if corners > 8 {
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

fn corner_count(points: &[StrokePoint]) -> usize {
    let sampled = resample(points, 36);
    if sampled.len() < 3 {
        return 0;
    }
    let mut count = 0;
    let mut previous_was_corner = false;
    for index in 0..sampled.len() {
        let previous = sampled[(index + sampled.len() - 2) % sampled.len()];
        let current = sampled[index];
        let next = sampled[(index + 2) % sampled.len()];
        let turn = turn_angle(previous, current, next);
        let is_corner = turn > 0.68;
        if is_corner && !previous_was_corner {
            count += 1;
        }
        previous_was_corner = is_corner;
    }
    count
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
        .max_by(|a, b| a.x.total_cmp(&b.x))
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
