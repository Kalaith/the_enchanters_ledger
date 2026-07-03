use super::templates::structure_spec_for_rune;
use super::{DrawnStroke, StrokePoint};
use serde::Deserialize;

/// A rune's structural profile, declared in
/// `assets/data/rune_templates.json` (plan Phase 1 item 3). Every field maps
/// to a *generic* feature check below — the evaluator has no idea which rune
/// it is scoring, so adding rune #34 with structural character is a JSON
/// edit, never a new Rust branch.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StructureSpec {
    /// How much the structural score blends into identity:
    /// `score × (1 − blend + blend × structure)`.
    pub(crate) blend: f32,
    /// `score.max(circle_likeness × circle_floor)` before the blend — lets a
    /// clean round stroke float a circle-shaped rune's identity even when
    /// point-matching is mediocre.
    #[serde(default)]
    pub(crate) circle_floor: Option<f32>,
    /// If any stroke is effectively closed, structure collapses to this
    /// score and the "should be open" issue is raised.
    #[serde(default)]
    pub(crate) must_be_open: Option<MustBeOpen>,
    #[serde(default)]
    pub(crate) min_strokes: Option<usize>,
    #[serde(default)]
    pub(crate) max_strokes: Option<usize>,
    /// Structural score when the stroke count is outside min/max.
    #[serde(default = "default_fallback_score")]
    pub(crate) fallback_score: f32,
    /// Which issue to report on that fallback: "closure", "corners", or
    /// "center_bar".
    #[serde(default)]
    pub(crate) fallback_issue: Option<String>,
    pub(crate) checks: Vec<FeatureCheck>,
    /// Data-declared cross-rune disambiguation: when the *other* rune's
    /// structure fits this ink decisively better, dampen this rune's score.
    #[serde(default)]
    pub(crate) suppressed_by: Option<SuppressedBy>,
}

fn default_fallback_score() -> f32 {
    0.30
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MustBeOpen {
    pub(crate) score: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SuppressedBy {
    pub(crate) rune: String,
    pub(crate) their_structure_min: f32,
    pub(crate) own_structure_below: f32,
    pub(crate) factor: f32,
}

/// One weighted, generic feature check. `feature` picks the geometry
/// function; the optional `issue_*` fields decide when this check also
/// yields player-facing feedback (first triggered check in declaration
/// order wins).
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FeatureCheck {
    pub(crate) feature: Feature,
    #[serde(default)]
    pub(crate) weight: f32,
    /// `corners`: the side count the rune wants, with `tolerance` shaping
    /// how fast the score falls off per missing/extra corner. `stroke_count`:
    /// the stroke count the template expects.
    #[serde(default)]
    pub(crate) target: Option<f32>,
    #[serde(default)]
    pub(crate) tolerance: Option<f32>,
    /// `corner_penalty`: corners past `above` each subtract `per_corner`.
    #[serde(default)]
    pub(crate) above: Option<f32>,
    #[serde(default)]
    pub(crate) per_corner: Option<f32>,
    /// `stroke_count_band`: full score inside [min, max], `out_of_band_score`
    /// outside.
    #[serde(default)]
    pub(crate) min: Option<usize>,
    #[serde(default)]
    pub(crate) max: Option<usize>,
    #[serde(default)]
    pub(crate) out_of_band_score: Option<f32>,
    #[serde(default)]
    pub(crate) issue_below: Option<f32>,
    #[serde(default)]
    pub(crate) issue_below_count: Option<f32>,
    #[serde(default)]
    pub(crate) issue_above_count: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Feature {
    Closure,
    Roundness,
    Corners,
    CornerPenalty,
    Straightness,
    Directness,
    ArrowRight,
    ArrowDown,
    CenterBar,
    RayAngles,
    RayCenter,
    StrokeCount,
    StrokeCountBand,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ShapeIssue {
    NotClosed,
    NotRoundEnough,
    TooManyStraightLines(String),
    NotEnoughSides(String, u32),
    TooManySides(String, u32),
    NotStraightEnough,
    ShouldBeOpen(String),
    MissingArrowRight(String),
    MissingArrowDown(String),
    MissingCenterBar(String),
    MissingRayStructure(String),
}

fn number_word(n: u32) -> String {
    match n {
        1 => "one".into(),
        2 => "two".into(),
        3 => "three".into(),
        4 => "four".into(),
        5 => "five".into(),
        6 => "six".into(),
        7 => "seven".into(),
        8 => "eight".into(),
        9 => "nine".into(),
        10 => "ten".into(),
        11 => "eleven".into(),
        12 => "twelve".into(),
        other => other.to_string(),
    }
}

fn display_name(rune_id: &str) -> String {
    let mut chars = rune_id.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

impl ShapeIssue {
    pub(crate) fn message(&self) -> String {
        match self {
            ShapeIssue::NotClosed => "The stroke needs to close cleanly.".into(),
            ShapeIssue::NotRoundEnough => {
                "The circle drifts too far from a steady radius.".into()
            }
            ShapeIssue::TooManyStraightLines(name) => {
                format!("Too many straight sides are showing for {name}.")
            }
            ShapeIssue::NotEnoughSides(name, count) => {
                format!("{name} needs {} clear sides.", number_word(*count))
            }
            ShapeIssue::TooManySides(name, count) => {
                format!(
                    "{name} has too many corners; keep it to {} sides.",
                    number_word(*count)
                )
            }
            ShapeIssue::NotStraightEnough => {
                "The straight rune lines need to be cleaner.".into()
            }
            ShapeIssue::ShouldBeOpen(name) => {
                format!("{name} should be an open arrow, not a closed shape.")
            }
            ShapeIssue::MissingArrowRight(name) => {
                format!("{name} needs a clear right-pointing shaft and arrow head.")
            }
            ShapeIssue::MissingArrowDown(name) => {
                format!("{name} needs a clear shaft and arrow head.")
            }
            ShapeIssue::MissingCenterBar(name) => {
                format!("{name} needs a closed outer shape with a clear center bar.")
            }
            ShapeIssue::MissingRayStructure(name) => {
                format!("{name} needs straight rays crossing through the center.")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuneShapeReport {
    pub(crate) structural_score: f32,
    pub(crate) issue: Option<ShapeIssue>,
}

/// Evaluates the rune's declared structural profile (if any) against the
/// ink. The rune id is only used to *look up* the spec and to name the rune
/// in feedback — no recognition logic branches on it (plan Phase 1 item 3).
pub(crate) fn shape_report_for_rune(
    rune_id: &str,
    strokes: &[DrawnStroke],
) -> Option<RuneShapeReport> {
    let spec = structure_spec_for_rune(rune_id)?;
    let ink = NormalizedInk::from_strokes(strokes)?;
    Some(structure_report(spec, &ink, rune_id))
}

fn structure_report(spec: &StructureSpec, ink: &NormalizedInk, rune_id: &str) -> RuneShapeReport {
    let name = display_name(rune_id);

    if let Some(open) = &spec.must_be_open {
        let any_closed = ink
            .strokes
            .iter()
            .any(|stroke| closure_score(stroke) > 0.72);
        if any_closed {
            return RuneShapeReport {
                structural_score: open.score,
                issue: Some(ShapeIssue::ShouldBeOpen(name)),
            };
        }
    }

    let stroke_count = ink.strokes.len();
    let below_min = spec.min_strokes.is_some_and(|min| stroke_count < min);
    let above_max = spec.max_strokes.is_some_and(|max| stroke_count > max);
    if below_min || above_max {
        return RuneShapeReport {
            structural_score: spec.fallback_score,
            issue: fallback_issue(spec, &name),
        };
    }

    let (primary_index, primary) = primary_stroke(ink);
    let mut structural_score = 0.0;
    let mut issue = None;
    for check in &spec.checks {
        let score = feature_score(check, ink, primary, primary_index);
        match check.feature {
            Feature::CornerPenalty => structural_score -= score,
            _ => structural_score += score * check.weight,
        }
        if issue.is_none() {
            issue = check_issue(check, primary, &name, score);
        }
    }

    RuneShapeReport {
        structural_score: structural_score.clamp(0.0, 1.0),
        issue,
    }
}

fn fallback_issue(spec: &StructureSpec, name: &str) -> Option<ShapeIssue> {
    let corner_target = spec
        .checks
        .iter()
        .find(|check| check.feature == Feature::Corners)
        .and_then(|check| check.target)
        .map(|target| target.round() as u32)
        .unwrap_or(0);
    match spec.fallback_issue.as_deref() {
        Some("closure") => Some(ShapeIssue::NotClosed),
        Some("corners") => Some(ShapeIssue::NotEnoughSides(name.to_owned(), corner_target)),
        Some("center_bar") => Some(ShapeIssue::MissingCenterBar(name.to_owned())),
        _ => None,
    }
}

/// The stroke structural checks focus on: the most-closed one (a rune's
/// outline), ties broken toward the earlier stroke so the report is
/// deterministic. For single-stroke runes this is simply the stroke.
fn primary_stroke(ink: &NormalizedInk) -> (usize, &[StrokePoint]) {
    ink.strokes
        .iter()
        .enumerate()
        .max_by(|(a_index, a), (b_index, b)| {
            closure_score(a)
                .total_cmp(&closure_score(b))
                .then_with(|| b_index.cmp(a_index))
        })
        .map(|(index, stroke)| (index, stroke.as_slice()))
        .unwrap_or((0, &[]))
}

fn feature_score(
    check: &FeatureCheck,
    ink: &NormalizedInk,
    primary: &[StrokePoint],
    primary_index: usize,
) -> f32 {
    match check.feature {
        Feature::Closure => closure_score(primary),
        Feature::Roundness => circularity(primary, ink.aspect_ratio),
        Feature::Corners => {
            let corners = corner_count(primary);
            let target = check.target.unwrap_or(4.0);
            let tolerance = check.tolerance.unwrap_or(3.0).max(0.001);
            (1.0 - (corners - target).abs() / tolerance).clamp(0.0, 1.0)
        }
        Feature::CornerPenalty => {
            let corners = corner_count(primary);
            let above = check.above.unwrap_or(f32::INFINITY);
            let per_corner = check.per_corner.unwrap_or(0.0);
            if corners >= above + 1.0 {
                (corners - above) * per_corner
            } else {
                0.0
            }
        }
        Feature::Straightness => straight_section_score(primary),
        Feature::Directness => average_directness(ink),
        Feature::ArrowRight => rightward_arrow_score(ink),
        Feature::ArrowDown => downward_arrow_score(ink),
        Feature::CenterBar => center_bar_score(ink, primary_index),
        Feature::RayAngles => ray_angle_spread(ink),
        Feature::RayCenter => ray_center_score(ink),
        Feature::StrokeCount => {
            count_score(ink.strokes.len(), check.target.unwrap_or(1.0).round() as usize)
        }
        Feature::StrokeCountBand => {
            let min = check.min.unwrap_or(0);
            let max = check.max.unwrap_or(usize::MAX);
            if (min..=max).contains(&ink.strokes.len()) {
                1.0
            } else {
                check.out_of_band_score.unwrap_or(0.45)
            }
        }
    }
}

fn check_issue(
    check: &FeatureCheck,
    primary: &[StrokePoint],
    name: &str,
    score: f32,
) -> Option<ShapeIssue> {
    let target = check.target.unwrap_or(0.0).round() as u32;
    if check.feature == Feature::Corners {
        let corners = corner_count(primary);
        if check
            .issue_below_count
            .is_some_and(|threshold| corners < threshold)
        {
            return Some(ShapeIssue::NotEnoughSides(name.to_owned(), target));
        }
        if check
            .issue_above_count
            .is_some_and(|threshold| corners > threshold)
        {
            return Some(ShapeIssue::TooManySides(name.to_owned(), target));
        }
        if check.issue_below.is_some_and(|threshold| score < threshold) {
            return Some(ShapeIssue::NotEnoughSides(name.to_owned(), target));
        }
        return None;
    }
    if check.feature == Feature::CornerPenalty {
        let corners = corner_count(primary);
        if check
            .issue_above_count
            .is_some_and(|threshold| corners >= threshold)
        {
            return Some(ShapeIssue::TooManyStraightLines(name.to_owned()));
        }
        return None;
    }
    let below = check.issue_below.is_some_and(|threshold| score < threshold);
    if !below {
        return None;
    }
    Some(match check.feature {
        Feature::Closure => ShapeIssue::NotClosed,
        Feature::Roundness => ShapeIssue::NotRoundEnough,
        Feature::Straightness | Feature::Directness => ShapeIssue::NotStraightEnough,
        Feature::ArrowRight => ShapeIssue::MissingArrowRight(name.to_owned()),
        Feature::ArrowDown => ShapeIssue::MissingArrowDown(name.to_owned()),
        Feature::CenterBar => ShapeIssue::MissingCenterBar(name.to_owned()),
        Feature::RayAngles | Feature::RayCenter => {
            ShapeIssue::MissingRayStructure(name.to_owned())
        }
        _ => return None,
    })
}

/// The best "horizontal bar through the middle" among the non-outline
/// strokes — the outline itself (the most-closed stroke) is excluded, so a
/// hexagon ring never scores as its own center bar.
fn center_bar_score(ink: &NormalizedInk, primary_index: usize) -> f32 {
    ink.strokes
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != primary_index)
        .map(|(_, stroke)| horizontal_center_bar_score(stroke))
        .fold(0.0, f32::max)
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

fn downward_arrow_score(ink: &NormalizedInk) -> f32 {
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

fn ray_angle_spread(ink: &NormalizedInk) -> f32 {
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

fn ray_center_score(ink: &NormalizedInk) -> f32 {
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
