use super::geometry::{StrokeBounds, StrokeCluster};
use super::{InterpretedRune, MIN_DIAGRAM_RUNE_CONFIDENCE};
use crate::data::{RuneCategory, RuneDef};
use crate::rune_drawing::{
    recognize_rune, template_strokes_for_rune, DrawnStroke, RecognitionOutcome, StrokePoint,
};

const MIN_RECOVERED_RUNE_CONFIDENCE: f32 = 0.52;
const MIN_RUNE_SCALE_IN_CIRCLE: f32 = 0.12;

pub(super) fn extract_overlapped_spheres(
    cluster: &StrokeCluster,
    available_runes: &[&RuneDef],
    circle_bounds: StrokeBounds,
    interpreted: &mut Vec<InterpretedRune>,
    rejected_marks: &mut usize,
) -> bool {
    if cluster.strokes.len() <= 1 || !available_runes.iter().any(|rune| rune.id == "sphere") {
        return false;
    }

    if let Some(recognized) = recognize_rune(&cluster.strokes, available_runes.iter().copied()) {
        let category = available_runes
            .iter()
            .find(|rune| rune.id == recognized.rune_id)
            .map(|rune| rune.category);
        if recognized.accepted
            && (recognized.rune_id == "healing" || category != Some(RuneCategory::Effect))
        {
            return false;
        }
    }

    let mut sphere_indices = Vec::new();
    for (index, stroke) in cluster.strokes.iter().enumerate() {
        let Some(bounds) = StrokeBounds::from_stroke(stroke) else {
            continue;
        };
        let Some(recognized) = recognize_rune(
            std::slice::from_ref(stroke),
            available_runes.iter().copied(),
        ) else {
            continue;
        };
        if recognized.rune_id == "sphere" && recognized.confidence >= MIN_DIAGRAM_RUNE_CONFIDENCE {
            push_recognized_rune(
                recognized,
                bounds,
                circle_bounds,
                available_runes,
                interpreted,
                rejected_marks,
            );
            sphere_indices.push(index);
        }
    }

    if sphere_indices.is_empty() {
        return false;
    }

    let remaining_groups = remaining_stroke_groups(cluster, &sphere_indices);
    if remaining_groups.is_empty() {
        return true;
    }

    for remaining in remaining_groups {
        let Some(bounds) = StrokeBounds::from_strokes(&remaining) else {
            *rejected_marks += 1;
            continue;
        };
        if let Some(recognized) = recognize_rune(&remaining, available_runes.iter().copied()) {
            push_recognized_rune(
                recognized,
                bounds,
                circle_bounds,
                available_runes,
                interpreted,
                rejected_marks,
            );
        } else {
            *rejected_marks += 1;
        }
    }

    true
}

pub(super) fn recover_contaminated_multi_stroke_rune(
    cluster: &StrokeCluster,
    available_runes: &[&RuneDef],
    circle_bounds: StrokeBounds,
    interpreted: &mut Vec<InterpretedRune>,
    rejected_marks: &mut usize,
) -> bool {
    if cluster.strokes.len() < 3 {
        return false;
    }
    if recognize_rune(&cluster.strokes, available_runes.iter().copied())
        .is_some_and(|recognized| recognized.accepted)
    {
        return false;
    }
    let Some(recovery) = best_recovery_window(cluster, available_runes, circle_bounds) else {
        return false;
    };
    push_recognized_rune(
        recovery.recognized,
        recovery.bounds,
        circle_bounds,
        available_runes,
        interpreted,
        rejected_marks,
    );
    *rejected_marks += cluster.strokes.len().saturating_sub(recovery.stroke_count);
    true
}

struct RecoveredWindow {
    recognized: RecognitionOutcome,
    bounds: StrokeBounds,
    stroke_count: usize,
}

fn best_recovery_window(
    cluster: &StrokeCluster,
    available_runes: &[&RuneDef],
    circle_bounds: StrokeBounds,
) -> Option<RecoveredWindow> {
    let mut best = None::<RecoveredWindow>;

    for start in 0..cluster.strokes.len() {
        for end in (start + 2)..=cluster.strokes.len() {
            if start == 0 && end == cluster.strokes.len() {
                continue;
            }
            let strokes = &cluster.strokes[start..end];
            let Some(bounds) = StrokeBounds::from_strokes(strokes) else {
                continue;
            };
            if bounds.scale_relative(circle_bounds) < MIN_RUNE_SCALE_IN_CIRCLE {
                continue;
            }
            let Some(recognized) = recognize_rune(strokes, available_runes.iter().copied()) else {
                continue;
            };
            if !is_recoverable_multi_stroke_rune(&recognized, strokes.len(), available_runes) {
                continue;
            }
            let candidate = RecoveredWindow {
                recognized,
                bounds,
                stroke_count: strokes.len(),
            };
            if best.as_ref().is_none_or(|current| {
                candidate.stroke_count > current.stroke_count
                    || (candidate.stroke_count == current.stroke_count
                        && candidate
                            .recognized
                            .confidence
                            .total_cmp(&current.recognized.confidence)
                            .is_gt())
            }) {
                best = Some(candidate);
            }
        }
    }

    best
}

fn is_recoverable_multi_stroke_rune(
    recognized: &RecognitionOutcome,
    stroke_count: usize,
    available_runes: &[&RuneDef],
) -> bool {
    if !recognized.accepted || recognized.confidence < MIN_RECOVERED_RUNE_CONFIDENCE {
        return false;
    }
    if available_runes
        .iter()
        .find(|rune| rune.id == recognized.rune_id)
        .map(|rune| rune.category)
        != Some(RuneCategory::Effect)
    {
        return false;
    }
    let Some(template) = template_strokes_for_rune(&recognized.rune_id) else {
        return false;
    };
    template.len() >= 3 && stroke_count == template.len()
}

fn remaining_stroke_groups(
    cluster: &StrokeCluster,
    removed_local_indices: &[usize],
) -> Vec<Vec<DrawnStroke>> {
    let mut groups = Vec::new();
    let mut current = Vec::new();
    let mut last_original_index = None;

    for (local_index, (original_index, stroke)) in
        cluster.indices.iter().zip(&cluster.strokes).enumerate()
    {
        if removed_local_indices.contains(&local_index) {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            last_original_index = None;
            continue;
        }
        if last_original_index.is_some_and(|last| *original_index != last + 1)
            && !current.is_empty()
        {
            groups.push(std::mem::take(&mut current));
        }
        current.push(stroke.clone());
        last_original_index = Some(*original_index);
    }

    if !current.is_empty() {
        groups.push(current);
    }
    groups
}

pub(super) fn push_recognized_rune(
    recognized: RecognitionOutcome,
    bounds: StrokeBounds,
    circle_bounds: StrokeBounds,
    available_runes: &[&RuneDef],
    interpreted: &mut Vec<InterpretedRune>,
    rejected_marks: &mut usize,
) {
    if recognized.confidence < MIN_DIAGRAM_RUNE_CONFIDENCE {
        *rejected_marks += 1;
        return;
    }
    let center = bounds.center();
    let scale = bounds.scale_relative(circle_bounds);
    let orbit = normalized_orbit(center, circle_bounds);
    let category = available_runes
        .iter()
        .find(|rune| rune.id == recognized.rune_id)
        .map(|rune| rune.category);
    if scale < MIN_RUNE_SCALE_IN_CIRCLE {
        *rejected_marks += 1;
        return;
    }
    // D1/D2: this rune's *shape* quality is no longer multiplied by circle
    // quality or board-position layout here — that stacked three
    // independently-meaningful scores into one opaque number (plan issue
    // D2). Circle quality now contributes once, additively, in
    // `evaluate()` (prd.md §5.4); layout position bias is deleted for
    // freehand scoring (plan issue D1) rather than silently kept.
    let potency = potency_for_rune(category, scale, recognized.ink_ratio);
    interpreted.push(InterpretedRune {
        rune_id: recognized.rune_id,
        confidence: recognized.confidence,
        quality: recognized.quality.clamp(0.0, 1.0),
        center,
        scale,
        orbit,
        potency,
    });
}

/// Magnitude channel (plan Phase 2 item 1): how strongly this rune's size
/// and stroke completeness scale its effect, independent of shape quality
/// (which keeps its existing role elsewhere). 1.0 at a category's
/// reference size (`RuneCategory::ideal_scale_in_circle`) drawn with a
/// fully-traced stroke.
fn potency_for_rune(category: Option<RuneCategory>, scale: f32, ink_ratio: f32) -> f32 {
    let ideal = category.map_or(0.15, RuneCategory::ideal_scale_in_circle);
    let scale_ratio = scale / ideal.max(0.001);
    let from_scale = potency_from_scale_ratio(scale_ratio);
    // Under-drawn ink pulls potency down before it breaks identity; extra
    // ink beyond the template's length earns no bonus.
    let ink_factor = ink_ratio.clamp(0.5, 1.0);
    (from_scale * ink_factor).clamp(0.35, 2.2)
}

/// Piecewise-linear through (0.5x reference size -> 0.6 potency), (1.0x ->
/// 1.0), (2.0x -> 1.6) — the documented curve from the plan. Each
/// segment's slope continues past those anchor points rather than
/// flattening immediately; `potency_for_rune` applies the final hard
/// clamp.
fn potency_from_scale_ratio(ratio: f32) -> f32 {
    if ratio < 1.0 {
        1.0 + (ratio - 1.0) * 0.8
    } else {
        1.0 + (ratio - 1.0) * 0.6
    }
}

fn normalized_orbit(center: StrokePoint, circle_bounds: StrokeBounds) -> f32 {
    let circle_center = circle_bounds.center();
    let rx = (circle_bounds.width() * 0.5).max(0.001);
    let ry = (circle_bounds.height() * 0.5).max(0.001);
    let nx = (center.x - circle_center.x) / rx;
    let ny = (center.y - circle_center.y) / ry;
    (nx * nx + ny * ny).sqrt()
}
