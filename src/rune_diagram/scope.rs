//! Recursive containment hierarchy (plan Phase 3 item 2): a working circle can enclose its own
//! ring-like sub-circles, each interpreted with its own local coordinate frame ("scope") exactly
//! like the top-level circle is — a ring that turns out to enclose ink of its own is treated as a
//! composite glyph rather than plain `ReinforcementRing` decoration. This is a first cut: scopes
//! fold their runes into one flat list (which `analyze_magical_circle` already consumes via
//! `InterpretedRune::scale`/`orbit`, both already relative to whatever bounds they were scored
//! against), not a full compositional grammar — see prd.md §5.5 for what's still Phase 4 scope.

use super::circle::{is_inside_working_circle, ring_shape_score};
use super::geometry::{cluster_strokes, StrokeBounds};
use super::recognition::{
    extract_overlapped_spheres, push_recognized_rune, recover_contaminated_multi_stroke_rune,
};
use super::{is_circle_structure, InterpretedRune};
use crate::data::RuneDef;
use crate::magical_circle::{classify_circle_stroke, CircleBounds, CircleMark};
use crate::rune_drawing::{recognize_rune, DrawnStroke};

/// Recursion cap — bounds worst-case cost on adversarial/degenerate geometry and matches the
/// plan's expectation of a handful of nesting levels (working circle -> vent -> sub-seal), not
/// unbounded depth.
const MAX_SCOPE_DEPTH: u32 = 3;
/// A nested ring candidate must be a substantial fraction of its parent scope — this overlaps
/// `ReinforcementRing`'s own scale band (0.28..=0.92) on purpose: a nested scope *is* a
/// reinforcement ring that also happens to enclose ink of its own.
const NESTED_RING_MIN_SCALE: f32 = 0.28;
const NESTED_RING_MAX_SCALE: f32 = 0.90;
const MIN_NESTED_RING_QUALITY: f32 = 0.40;
/// A nested ring candidate must sit clearly off the parent's center — concentric
/// "reinforcement ring stack" diagrams (the existing grand-circle idiom: several closed rings
/// sharing the working circle's own center, each independently classified as a plain
/// `ReinforcementRing`) must *not* be reinterpreted as sub-scopes, only a genuinely separate
/// sub-circle (e.g. a volcano's vent, drawn off to one side) should be. This threshold sits just
/// above `ReinforcementRing`'s own `orbit <= 0.15` band on purpose — anything that close to
/// center reads as reinforcement decoration, not a distinct scope.
const NESTED_RING_MIN_ORBIT: f32 = 0.20;

pub(crate) struct ScopeOutcome {
    pub(crate) runes: Vec<InterpretedRune>,
    pub(crate) rejected_marks: usize,
    pub(crate) circle_marks: Vec<CircleMark>,
}

/// Interprets one scope's worth of ink (already filtered to "inside `scope_bounds`, not part of
/// the stroke(s) that define this scope's own ring"): classifies structure marks, recurses into
/// any nested ring it finds, then clusters and recognizes whatever ink is left.
pub(crate) fn interpret_scope(
    ink: &[(usize, DrawnStroke)],
    scope_bounds: StrokeBounds,
    available_runes: &[&RuneDef],
    depth: u32,
) -> ScopeOutcome {
    let spell_bounds = CircleBounds::new(
        scope_bounds.min_x,
        scope_bounds.min_y,
        scope_bounds.max_x,
        scope_bounds.max_y,
    );
    let classified_marks = ink
        .iter()
        .filter_map(|(index, stroke)| {
            classify_circle_stroke(stroke, spell_bounds).map(|mark| (*index, mark))
        })
        .collect::<Vec<_>>();

    let mut runes = Vec::new();
    let mut rejected_marks = 0usize;
    let mut consumed = Vec::<usize>::new();

    if depth < MAX_SCOPE_DEPTH {
        for (ring_index, ring_bounds) in nested_ring_candidates(ink, scope_bounds) {
            if consumed.contains(&ring_index) {
                continue;
            }
            let nested_ink = ink
                .iter()
                .filter(|(index, stroke)| {
                    *index != ring_index
                        && !consumed.contains(index)
                        && is_inside_working_circle(stroke, ring_bounds)
                })
                .cloned()
                .collect::<Vec<_>>();
            if nested_ink.is_empty() {
                // A ring with nothing inside it is just a ring, not a sub-scope — leave it to
                // the ordinary structure-mark classification below (it already stayed in
                // `classified_marks` above).
                continue;
            }
            let sub = interpret_scope(&nested_ink, ring_bounds, available_runes, depth + 1);
            runes.extend(sub.runes);
            rejected_marks += sub.rejected_marks;
            consumed.push(ring_index);
            consumed.extend(nested_ink.into_iter().map(|(index, _)| index));
        }
    }

    let inner_strokes = ink
        .iter()
        .filter(|(index, stroke)| {
            !consumed.contains(index)
                && is_inside_working_circle(stroke, scope_bounds)
                && !is_circle_structure(*index, &classified_marks)
        })
        .map(|(index, stroke)| (*index, stroke.clone()))
        .collect::<Vec<_>>();

    let clusters = cluster_strokes(&inner_strokes);
    for cluster in clusters {
        if extract_overlapped_spheres(
            &cluster,
            available_runes,
            scope_bounds,
            &mut runes,
            &mut rejected_marks,
        ) {
            continue;
        }
        if recover_contaminated_multi_stroke_rune(
            &cluster,
            available_runes,
            scope_bounds,
            &mut runes,
            &mut rejected_marks,
        ) {
            continue;
        }
        let Some(recognized) = recognize_rune(&cluster.strokes, available_runes.iter().copied())
        else {
            rejected_marks += 1;
            continue;
        };
        let total_points = cluster
            .strokes
            .iter()
            .map(|stroke| stroke.points.len())
            .sum();
        push_recognized_rune(
            recognized,
            cluster.bounds,
            scope_bounds,
            total_points,
            available_runes,
            &mut runes,
            &mut rejected_marks,
        );
    }

    ScopeOutcome {
        runes,
        rejected_marks,
        circle_marks: classified_marks.into_iter().map(|(_, mark)| mark).collect(),
    }
}

/// Closed strokes inside `ink` that are plausibly a nested scope's own ring: a large-relative-to-
/// `scope_bounds` closed shape with decent ring-shape fidelity (`ring_shape_score`, which — unlike
/// `circle_quality` — has no absolute slate-space size floor, since a nested ring's absolute size
/// is only ever a fraction of its parent). Sorted best-first so the strongest rings claim their
/// contents before weaker overlapping candidates are considered.
fn nested_ring_candidates(
    ink: &[(usize, DrawnStroke)],
    scope_bounds: StrokeBounds,
) -> Vec<(usize, StrokeBounds)> {
    let mut candidates = ink
        .iter()
        .filter_map(|(index, stroke)| {
            let bounds = StrokeBounds::from_stroke(stroke)?;
            let scale = bounds.scale_relative(scope_bounds);
            if !(NESTED_RING_MIN_SCALE..=NESTED_RING_MAX_SCALE).contains(&scale) {
                return None;
            }
            if ring_orbit(bounds, scope_bounds) < NESTED_RING_MIN_ORBIT {
                return None;
            }
            let quality = ring_shape_score(stroke, bounds)?;
            (quality >= MIN_NESTED_RING_QUALITY).then_some((*index, bounds, quality))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| b.2.total_cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    candidates
        .into_iter()
        .map(|(index, bounds, _)| (index, bounds))
        .collect()
}

/// Ellipse-normalized distance of `bounds`' center from `scope_bounds`' center — 0 at dead
/// center, 1 at the scope's own edge. Mirrors `magical_circle`'s `normalized_orbit`, computed
/// here directly since that helper is private to `magical_circle`.
fn ring_orbit(bounds: StrokeBounds, scope_bounds: StrokeBounds) -> f32 {
    let center = bounds.center();
    let scope_center = scope_bounds.center();
    let rx = (scope_bounds.width() * 0.5).max(0.001);
    let ry = (scope_bounds.height() * 0.5).max(0.001);
    let nx = (center.x - scope_center.x) / rx;
    let ny = (center.y - scope_center.y) / ry;
    (nx * nx + ny * ny).sqrt()
}
