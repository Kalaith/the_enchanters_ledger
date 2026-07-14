//! Structural evaluation of drawn ink against a rune's declared profile.
//!
//! The rune id is only used to *look up* a `StructureSpec` and to name the
//! rune in feedback — no recognition logic branches on it (plan Phase 1
//! item 3). `spec` holds the JSON-declared shape of a profile, `geometry` the
//! generic measurements, and `issue` the player-facing wording; this module
//! is just the wiring between them.

mod geometry;
mod issue;
mod spec;

use super::templates::structure_spec_for_rune;
use super::DrawnStroke;
use crate::rune_drawing::StrokePoint;
use geometry::NormalizedInk;
use issue::display_name;
pub(crate) use issue::ShapeIssue;
pub(crate) use spec::StructureSpec;
use spec::{Feature, FeatureCheck};

#[derive(Debug, Clone)]
pub(crate) struct RuneShapeReport {
    pub(crate) structural_score: f32,
    pub(crate) issue: Option<ShapeIssue>,
}

/// Evaluates the rune's declared structural profile (if any) against the ink.
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
            .any(|stroke| geometry::closure_score(stroke) > 0.72);
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
            geometry::closure_score(a)
                .total_cmp(&geometry::closure_score(b))
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
        Feature::Closure => geometry::closure_score(primary),
        Feature::Roundness => geometry::circularity(primary, ink.aspect_ratio),
        Feature::Corners => {
            let corners = geometry::corner_count(primary);
            let target = check.target.unwrap_or(4.0);
            let tolerance = check.tolerance.unwrap_or(3.0).max(0.001);
            (1.0 - (corners - target).abs() / tolerance).clamp(0.0, 1.0)
        }
        Feature::CornerPenalty => {
            let corners = geometry::corner_count(primary);
            let above = check.above.unwrap_or(f32::INFINITY);
            let per_corner = check.per_corner.unwrap_or(0.0);
            if corners >= above + 1.0 {
                (corners - above) * per_corner
            } else {
                0.0
            }
        }
        Feature::Straightness => geometry::straight_section_score(primary),
        Feature::Directness => geometry::average_directness(ink),
        Feature::ArrowRight => geometry::rightward_arrow_score(ink),
        Feature::ArrowDown => geometry::downward_arrow_score(ink),
        Feature::CenterBar => center_bar_score(ink, primary_index),
        Feature::RayAngles => geometry::ray_angle_spread(ink),
        Feature::RayCenter => geometry::ray_center_score(ink),
        Feature::StrokeCount => geometry::count_score(
            ink.strokes.len(),
            check.target.unwrap_or(1.0).round() as usize,
        ),
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
        let corners = geometry::corner_count(primary);
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
        let corners = geometry::corner_count(primary);
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
        Feature::RayAngles | Feature::RayCenter => ShapeIssue::MissingRayStructure(name.to_owned()),
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
        .map(|(_, stroke)| geometry::horizontal_center_bar_score(stroke))
        .fold(0.0, f32::max)
}
