use super::geometry::{cluster_strokes, distance, StrokeBounds, StrokeCluster};
use super::{diagram_acceptance_band, InterpretedRune};
use crate::data::{RuneCategory, RuneDef};
use crate::rune_drawing::{
    recognize_rune_in_context, template_strokes_for_rune, DrawnStroke, RecognitionContext,
    RecognitionOutcome, StrokePoint,
};

const MIN_RECOVERED_RUNE_CONFIDENCE: f32 = 0.52;
/// Minimum total point count across a cluster's strokes for it to be scored as a rune at all —
/// replaces the old absolute `MIN_RUNE_SCALE_IN_CIRCLE` size floor (plan Phase 3 item 4 / C1,
/// C2): a legible rune is one with enough sampled shape detail, not one drawn above some
/// fraction of the working circle. A 100+-symbol diagram needs runes at 3-6% circle scale, which
/// the old 12% floor rejected outright regardless of how clean the shape was; a point-count
/// floor still rejects genuinely degenerate flicks (2-3 points) without penalizing a small but
/// carefully-traced rune. 4 is the lowest total point count among any real rune template
/// (`spark`, `cone` — see `assets/data/rune_templates.json`), so this floor never rejects a rune
/// drawn cleanly at its own canonical shape.
const MIN_RUNE_CLUSTER_POINTS: usize = 4;

/// A scope's bounds plus the acceptance context runes inside it are read with (plan Phase 5 item
/// 1), bundled since every recognition-path function needs both together, and separately they'd
/// push several of these functions over clippy's argument-count lint.
#[derive(Debug, Clone, Copy)]
pub(super) struct ScopeContext {
    pub(super) bounds: StrokeBounds,
    pub(super) recognition: RecognitionContext,
}

pub(super) fn extract_overlapped_spheres(
    cluster: &StrokeCluster,
    available_runes: &[&RuneDef],
    scope: ScopeContext,
    interpreted: &mut Vec<InterpretedRune>,
    rejected_marks: &mut usize,
) -> bool {
    if cluster.strokes.len() <= 1 || !available_runes.iter().any(|rune| rune.id == "sphere") {
        return false;
    }

    if let Some(recognized) = recognize_rune_in_context(
        &cluster.strokes,
        available_runes.iter().copied(),
        scope.recognition,
    ) {
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

    let diagram_rune_confidence = diagram_acceptance_band(scope.recognition).diagram_rune;
    let mut sphere_indices = Vec::new();
    for (index, stroke) in cluster.strokes.iter().enumerate() {
        let Some(bounds) = StrokeBounds::from_stroke(stroke) else {
            continue;
        };
        let Some(recognized) = recognize_rune_in_context(
            std::slice::from_ref(stroke),
            available_runes.iter().copied(),
            scope.recognition,
        ) else {
            continue;
        };
        if recognized.rune_id == "sphere" && recognized.confidence >= diagram_rune_confidence {
            push_recognized_rune(
                recognized,
                bounds,
                scope,
                stroke.points.len(),
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
        if let Some(recognized) = recognize_rune_in_context(
            &remaining,
            available_runes.iter().copied(),
            scope.recognition,
        ) {
            push_recognized_rune(
                recognized,
                bounds,
                scope,
                total_points(&remaining),
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
    scope: ScopeContext,
    interpreted: &mut Vec<InterpretedRune>,
    rejected_marks: &mut usize,
) -> bool {
    if cluster.strokes.len() < 3 {
        return false;
    }
    if recognize_rune_in_context(
        &cluster.strokes,
        available_runes.iter().copied(),
        scope.recognition,
    )
    .is_some_and(|recognized| recognized.accepted)
    {
        return false;
    }
    let Some(recovery) = best_recovery_window(cluster, available_runes, scope.recognition) else {
        return false;
    };
    push_recognized_rune(
        recovery.recognized,
        recovery.bounds,
        scope,
        recovery.total_points,
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
    total_points: usize,
}

/// Best contiguous *spatially-ordered* window of `cluster`'s strokes that recognizes as an
/// accepted multi-stroke effect rune — a bounded fallback for clusters contaminated by extra,
/// unrelated ink (plan issue A5). Windows over `spatial_order`, not `cluster.strokes`' original
/// draw order (plan Phase 3 item 5 / A2): drawing the same rune's strokes in a different order,
/// or interleaved with the contaminating ink at different points, no longer changes which window
/// is found.
fn best_recovery_window(
    cluster: &StrokeCluster,
    available_runes: &[&RuneDef],
    context: RecognitionContext,
) -> Option<RecoveredWindow> {
    let ordered = spatial_order(cluster);
    let mut best = None::<RecoveredWindow>;

    for start in 0..ordered.len() {
        for end in (start + 2)..=ordered.len() {
            if start == 0 && end == ordered.len() {
                continue;
            }
            let strokes = &ordered[start..end];
            let Some(bounds) = StrokeBounds::from_strokes(strokes) else {
                continue;
            };
            let points = total_points(strokes);
            if points < MIN_RUNE_CLUSTER_POINTS {
                continue;
            }
            let Some(recognized) =
                recognize_rune_in_context(strokes, available_runes.iter().copied(), context)
            else {
                continue;
            };
            if !is_recoverable_multi_stroke_rune(&recognized, strokes.len(), available_runes) {
                continue;
            }
            let candidate = RecoveredWindow {
                recognized,
                bounds,
                stroke_count: strokes.len(),
                total_points: points,
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

/// A greedy nearest-neighbor visiting order over `cluster`'s strokes (by bounding-box center),
/// starting from the lowest-indexed stroke for determinism and always stepping to whichever
/// unvisited stroke is spatially closest (ties broken by original index). Used by
/// `best_recovery_window` in place of original draw order.
fn spatial_order(cluster: &StrokeCluster) -> Vec<DrawnStroke> {
    let bounds = cluster
        .strokes
        .iter()
        .map(StrokeBounds::from_stroke)
        .collect::<Vec<_>>();
    let Some(mut current) = (0..cluster.strokes.len()).find(|index| bounds[*index].is_some())
    else {
        return cluster.strokes.clone();
    };
    let mut visited = vec![false; cluster.strokes.len()];
    let mut order = vec![current];
    visited[current] = true;

    for _ in 1..cluster.strokes.len() {
        let Some(current_center) = bounds[current].map(StrokeBounds::center) else {
            break;
        };
        let Some(next) = (0..cluster.strokes.len())
            .filter(|index| !visited[*index])
            .min_by(|a, b| {
                let a_distance =
                    bounds[*a].map_or(f32::INFINITY, |ab| distance(current_center, ab.center()));
                let b_distance =
                    bounds[*b].map_or(f32::INFINITY, |bb| distance(current_center, bb.center()));
                a_distance.total_cmp(&b_distance).then_with(|| a.cmp(b))
            })
        else {
            break;
        };
        order.push(next);
        visited[next] = true;
        current = next;
    }

    order
        .into_iter()
        .map(|index| cluster.strokes[index].clone())
        .collect()
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

/// The strokes left in `cluster` once `removed_local_indices` (the sphere strokes
/// `extract_overlapped_spheres` already pulled out) are set aside, regrouped by spatial
/// proximity — never by original draw-stroke-index adjacency (plan Phase 3 item 5 / A2): drawing
/// stroke 1 of one rune, then a second rune, then finishing the first rune no longer changes
/// which strokes end up grouped together. Reuses `cluster_strokes`, the same spatial grouping the
/// rest of diagram interpretation is built on, rather than a bespoke index-window heuristic.
fn remaining_stroke_groups(
    cluster: &StrokeCluster,
    removed_local_indices: &[usize],
) -> Vec<Vec<DrawnStroke>> {
    let remaining = cluster
        .indices
        .iter()
        .zip(&cluster.strokes)
        .enumerate()
        .filter(|(local_index, _)| !removed_local_indices.contains(local_index))
        .map(|(_, (original_index, stroke))| (*original_index, stroke.clone()))
        .collect::<Vec<_>>();
    cluster_strokes(&remaining)
        .into_iter()
        .map(|group| group.strokes)
        .collect()
}

pub(super) fn push_recognized_rune(
    recognized: RecognitionOutcome,
    bounds: StrokeBounds,
    scope: ScopeContext,
    total_points: usize,
    available_runes: &[&RuneDef],
    interpreted: &mut Vec<InterpretedRune>,
    rejected_marks: &mut usize,
) {
    if recognized.confidence < diagram_acceptance_band(scope.recognition).diagram_rune {
        *rejected_marks += 1;
        return;
    }
    if total_points < MIN_RUNE_CLUSTER_POINTS {
        *rejected_marks += 1;
        return;
    }
    let center = bounds.center();
    let scale = bounds.scale_relative(scope.bounds);
    let orbit = normalized_orbit(center, scope.bounds);
    let category = available_runes
        .iter()
        .find(|rune| rune.id == recognized.rune_id)
        .map(|rune| rune.category);
    // D1/D2: this rune's *shape* quality is no longer multiplied by circle
    // quality or board-position layout here — that stacked three
    // independently-meaningful scores into one opaque number (plan issue
    // D2). Circle quality now contributes once, additively, in
    // `evaluate()` (prd.md §5.4); layout position bias is deleted for
    // freehand scoring (plan issue D1) rather than silently kept.
    let potency = potency_for_rune(category, scale, recognized.ink_ratio);
    interpreted.push(InterpretedRune {
        rune_id: recognized.rune_id,
        // Set by `flatten_runes`, which is where scope nesting is known.
        scope_depth: 0,
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

fn total_points(strokes: &[DrawnStroke]) -> usize {
    strokes.iter().map(|stroke| stroke.points.len()).sum()
}

fn normalized_orbit(center: StrokePoint, circle_bounds: StrokeBounds) -> f32 {
    let circle_center = circle_bounds.center();
    let rx = (circle_bounds.width() * 0.5).max(0.001);
    let ry = (circle_bounds.height() * 0.5).max(0.001);
    let nx = (center.x - circle_center.x) / rx;
    let ny = (center.y - circle_center.y) / ry;
    (nx * nx + ny * ny).sqrt()
}
