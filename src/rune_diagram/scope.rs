//! Recursive containment hierarchy (plan Phase 3 item 2): a working circle can enclose its own
//! ring-like sub-circles, each interpreted with its own local coordinate frame ("scope") exactly
//! like the top-level circle is — a ring that turns out to enclose ink of its own is treated as a
//! composite glyph rather than plain `ReinforcementRing` decoration. As of Phase 4, sub-scopes are
//! *not* flattened away: `ScopeOutcome` keeps each nested scope as its own node
//! (`sub_scopes: Vec<ScopeOutcome>`), and `flatten_runes`/`build_scope_spell` (below) are the two
//! separate views callers need — a flat `Vec<InterpretedRune>` for everything that scores runes
//! individually (unchanged from Phase 3), and a `ScopeSpell` tree for the recipe grammar
//! (plan Phase 4 item 1, `crate::recipes`).

use super::circle::{is_inside_working_circle, ring_shape_score};
use super::geometry::{bounds_gap, cluster_strokes, StrokeBounds};
use super::recognition::{
    extract_overlapped_spheres, push_recognized_rune, recover_contaminated_multi_stroke_rune,
    ScopeContext,
};
use super::{is_circle_structure, InterpretedRune, ScopeSpell};
use crate::data::{RuneCategory, RuneDef};
use crate::magical_circle::{
    classify_circle_stroke, diminishing_count, CircleBounds, CircleMark, CircleStrokeKind,
};
use crate::rune_drawing::{recognize_rune_in_context, DrawnStroke, RecognitionContext};

/// Fixed containment bonus a scope earns for having a `safer` modifier rune among its own ink —
/// plan Phase 4 item 2's explicit "circle quality, rings, perimeter script, safer runes"
/// containment inputs.
const SAFER_CONTAINMENT_BONUS: f32 = 1.5;
/// Targets the per-scope `diminishing_count` (see `magical_circle::diminishing_count`) ring/
/// perimeter contributions are measured against — deliberately smaller than the whole-diagram
/// targets in `magical_circle.rs`, since a single nested scope is a much smaller composition.
const SCOPE_RING_TARGET: usize = 2;
const SCOPE_PERIMETER_TARGET: usize = 8;

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

/// How close (relative to the mark's own size) another stroke must sit for a
/// would-be perimeter/script mark to instead count as part of a rune group —
/// effectively "touching/overlapping", far tighter than band tick spacing.
const GROUPED_INK_GAP_FACTOR: f32 = 0.12;
/// Only strokes of comparable size anchor a rescue — a reinforcement ring's
/// bounding box covers everything inside it, and must not vacuum every tick
/// it encloses into rune ink.
const GROUPED_INK_MAX_SPAN_RATIO: f32 = 4.0;

/// Plan Phase 3 item 4, "structure vs. rune ink by geometry": what makes a
/// perimeter tick or script flick *decorative* is that it stands alone; the
/// same-sized strokes of a small multi-stroke rune (e.g. an asterisk rune's
/// four crossing 2-point bars) practically touch or overlap one another. Any
/// small structure mark that touches another comparably-sized stroke is
/// flipped back to rune ink. Band ticks keep their spacing — adjacent ticks
/// sit apart by a good fraction of their own length — so real script and
/// perimeter bands keep their classification even when densely drawn.
fn keep_grouped_ink_as_runes(
    ink: &[(usize, DrawnStroke)],
    marks: Vec<(usize, CircleMark)>,
) -> Vec<(usize, CircleMark)> {
    let bounds: Vec<(usize, StrokeBounds)> = ink
        .iter()
        .filter_map(|(index, stroke)| StrokeBounds::from_stroke(stroke).map(|b| (*index, b)))
        .collect();
    marks
        .into_iter()
        .map(|(index, mark)| {
            if !matches!(
                mark.kind,
                CircleStrokeKind::PerimeterMark | CircleStrokeKind::ScriptMark
            ) {
                return (index, mark);
            }
            let Some((_, own)) = bounds.iter().find(|(i, _)| *i == index) else {
                return (index, mark);
            };
            let own_span = own.diagonal().max(0.001);
            let grouped = bounds.iter().any(|(other_index, other)| {
                *other_index != index
                    && other.diagonal() <= own_span * GROUPED_INK_MAX_SPAN_RATIO
                    && bounds_gap(*own, *other) <= GROUPED_INK_GAP_FACTOR * own_span
            });
            if grouped {
                (
                    index,
                    CircleMark {
                        kind: CircleStrokeKind::RuneInk,
                        ..mark
                    },
                )
            } else {
                (index, mark)
            }
        })
        .collect()
}

pub(crate) struct ScopeOutcome {
    /// Runes recognized directly in this scope — does *not* include sub-scope runes; see
    /// `flatten_runes`/`build_scope_spell` for the two different ways callers reassemble this
    /// with `sub_scopes` below.
    pub(crate) own_runes: Vec<InterpretedRune>,
    pub(crate) sub_scopes: Vec<ScopeOutcome>,
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
    context: RecognitionContext,
) -> ScopeOutcome {
    let spell_bounds = CircleBounds::new(
        scope_bounds.min_x,
        scope_bounds.min_y,
        scope_bounds.max_x,
        scope_bounds.max_y,
    );
    let classified_marks = keep_grouped_ink_as_runes(
        ink,
        ink.iter()
            .filter_map(|(index, stroke)| {
                classify_circle_stroke(stroke, spell_bounds).map(|mark| (*index, mark))
            })
            .collect::<Vec<_>>(),
    );

    let mut own_runes = Vec::new();
    let mut sub_scopes = Vec::new();
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
            let sub = interpret_scope(
                &nested_ink,
                ring_bounds,
                available_runes,
                depth + 1,
                context,
            );
            rejected_marks += sub.rejected_marks;
            sub_scopes.push(sub);
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

    let scope = ScopeContext {
        bounds: scope_bounds,
        recognition: context,
    };
    let clusters = cluster_strokes(&inner_strokes);
    for cluster in clusters {
        if extract_overlapped_spheres(
            &cluster,
            available_runes,
            scope,
            &mut own_runes,
            &mut rejected_marks,
        ) {
            continue;
        }
        if recover_contaminated_multi_stroke_rune(
            &cluster,
            available_runes,
            scope,
            &mut own_runes,
            &mut rejected_marks,
        ) {
            continue;
        }
        let Some(recognized) =
            recognize_rune_in_context(&cluster.strokes, available_runes.iter().copied(), context)
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
            scope,
            total_points,
            available_runes,
            &mut own_runes,
            &mut rejected_marks,
        );
    }

    ScopeOutcome {
        own_runes,
        sub_scopes,
        rejected_marks,
        circle_marks: classified_marks.into_iter().map(|(_, mark)| mark).collect(),
    }
}

/// Recreates Phase 3's flat `Vec<InterpretedRune>` (every scope's own runes, depth-first) — every
/// existing consumer of `DiagramInterpretation.runes` (UI, `evaluate()`'s per-rune stat math,
/// tests) keeps working unchanged; only `build_scope_spell` (below) needs the tree shape.
pub(crate) fn flatten_runes(outcome: &ScopeOutcome) -> Vec<InterpretedRune> {
    let mut runes = outcome.own_runes.clone();
    for sub in &outcome.sub_scopes {
        runes.extend(flatten_runes(sub));
    }
    runes
}

/// Builds the recipe-grammar view of `outcome` (plan Phase 4 item 1, `crate::recipes`): unlike
/// `flatten_runes`, sub-scopes stay nested rather than being folded in, and this scope's own
/// runes are grouped by category instead of kept as a flat list.
pub(crate) fn build_scope_spell(outcome: &ScopeOutcome, rune_defs: &[&RuneDef]) -> ScopeSpell {
    let mut effects = Vec::new();
    let mut shape = None;
    let mut trigger = None;
    let mut modifiers = Vec::new();
    for rune in &outcome.own_runes {
        let Some(def) = rune_defs.iter().find(|def| def.id == rune.rune_id) else {
            continue;
        };
        match def.category {
            RuneCategory::Effect => effects.push((rune.rune_id.clone(), rune.potency)),
            RuneCategory::Shape if shape.is_none() => shape = Some(rune.rune_id.clone()),
            RuneCategory::Trigger if trigger.is_none() => trigger = Some(rune.rune_id.clone()),
            RuneCategory::Modifier => modifiers.push(rune.rune_id.clone()),
            _ => {}
        }
    }

    let ring_count = count_kind(&outcome.circle_marks, CircleStrokeKind::ReinforcementRing);
    let satellite_count = count_kind(&outcome.circle_marks, CircleStrokeKind::SatelliteSeal);
    let radial_count = count_kind(&outcome.circle_marks, CircleStrokeKind::RadialSpoke);
    let perimeter_mark_count = count_kind(&outcome.circle_marks, CircleStrokeKind::PerimeterMark);
    let script_mark_count = count_kind(&outcome.circle_marks, CircleStrokeKind::ScriptMark);
    let structural = (diminishing_count(ring_count, SCOPE_RING_TARGET) * 0.6
        + diminishing_count(perimeter_mark_count, SCOPE_PERIMETER_TARGET) * 0.4)
        .clamp(0.0, 1.0);
    let safer_bonus = if modifiers.iter().any(|id| id == "safer") {
        SAFER_CONTAINMENT_BONUS
    } else {
        0.0
    };

    ScopeSpell {
        effects,
        shape,
        trigger,
        modifiers,
        ring_count,
        satellite_count,
        radial_count,
        perimeter_mark_count,
        script_mark_count,
        containment: structural + safer_bonus,
        sub_scopes: outcome
            .sub_scopes
            .iter()
            .map(|sub| build_scope_spell(sub, rune_defs))
            .collect(),
    }
}

fn count_kind(marks: &[CircleMark], kind: CircleStrokeKind) -> usize {
    marks.iter().filter(|mark| mark.kind == kind).count()
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
