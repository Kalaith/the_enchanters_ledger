//! Whole-diagram interpretation built on top of rune stroke recognition.

use crate::data::RuneDef;
use crate::magical_circle::{
    analyze_magical_circle, CircleMark, CircleStrokeKind, MagicalCircleSpell,
};
use crate::rune_drawing::{DrawnStroke, RecognitionContext, StrokePoint};
use serde::{Deserialize, Serialize};

mod circle;
mod geometry;
mod recognition;
mod scope;
#[cfg(test)]
mod tests;

pub(crate) use circle::{
    gather_circle_candidates, is_inside_working_circle, select_working_circle_for_strokes,
};
pub(crate) use geometry::{cluster_strokes, StrokeBounds};
use scope::{build_scope_spell, flatten_runes, interpret_scope};

pub const MIN_CIRCLE_QUALITY: f32 = 0.32;
pub const MIN_DIAGRAM_RUNE_CONFIDENCE: f32 = 0.32;

/// A diagram-level context's acceptance cutoffs (plan Phase 5 item 1) — the diagram-level
/// counterpart to `rune_drawing::AcceptanceBand`, which only covers single-rune recognition.
/// `RecognitionContext::Commission`'s values are bit-identical to the pre-Phase-5 hardcoded
/// `MIN_CIRCLE_QUALITY`/`MIN_DIAGRAM_RUNE_CONFIDENCE` constants.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DiagramAcceptanceBand {
    pub(crate) circle: f32,
    pub(crate) diagram_rune: f32,
}

pub(crate) fn diagram_acceptance_band(context: RecognitionContext) -> DiagramAcceptanceBand {
    match context {
        RecognitionContext::Practice => DiagramAcceptanceBand {
            circle: 0.40,
            diagram_rune: 0.40,
        },
        RecognitionContext::Commission => DiagramAcceptanceBand {
            circle: MIN_CIRCLE_QUALITY,
            diagram_rune: MIN_DIAGRAM_RUNE_CONFIDENCE,
        },
        RecognitionContext::Sandbox => DiagramAcceptanceBand {
            circle: 0.24,
            diagram_rune: 0.24,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramInterpretation {
    pub circle_quality: f32,
    pub circle_found: bool,
    /// Centre of the working circle, in slate coordinates — the origin every
    /// placement angle in `crate::reading` is measured from.
    #[serde(default = "default_circle_center")]
    pub circle_center: StrokePoint,
    pub runes: Vec<InterpretedRune>,
    pub rejected_marks: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spell: Option<MagicalCircleSpell>,
    /// The compositional spell tree (plan Phase 4 item 1): the same runes as `runes`, but
    /// grouped by scope (root working circle + any nested sub-scopes, unflattened) for
    /// `crate::recipes::match_recipe` to evaluate data-defined recipes against. `None` exactly
    /// when `spell` is `None` (no circle, or too little structure/tier to read as a spell at
    /// all).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_spell: Option<ScopeSpell>,
}

/// One scope's worth of a diagram's spell content — the working circle itself, or one of its
/// nested sub-scopes (`rune_diagram::scope`) — as a predicate-matchable tree instead of a flat
/// stat blob. See `crate::recipes` for how `assets/data/recipes.json` is matched against this.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ScopeSpell {
    /// (rune_id, potency) for every Effect-category rune recognized directly in this scope
    /// (not including sub-scopes — see `total_potency` to aggregate across the whole tree).
    pub effects: Vec<(String, f32)>,
    pub shape: Option<String>,
    pub trigger: Option<String>,
    pub modifiers: Vec<String>,
    pub ring_count: usize,
    pub satellite_count: usize,
    pub radial_count: usize,
    pub perimeter_mark_count: usize,
    pub script_mark_count: usize,
    /// This scope's own containment measure — structure (rings, perimeter marks) plus a bonus
    /// for a `safer` modifier rune, independent of any parent/sibling scope. See prd.md's Phase
    /// 4 section for the exact formula and `state/evaluate.rs`'s containment-budget walk for how
    /// it's spent.
    pub containment: f32,
    pub sub_scopes: Vec<ScopeSpell>,
}

impl ScopeSpell {
    /// Sum of `effect_id`'s potency across this scope and every descendant sub-scope — the
    /// mechanism behind the plan's "repeated effect runes / sub-scopes feeding a parent ...
    /// multiply potency": a recipe's `min_potency` requirement is checked against this, not
    /// against any single rune's own potency.
    pub fn total_potency(&self, effect_id: &str) -> f32 {
        let own = self
            .effects
            .iter()
            .filter(|(id, _)| id == effect_id)
            .map(|(_, potency)| potency)
            .sum::<f32>();
        own + self
            .sub_scopes
            .iter()
            .map(|sub| sub.total_potency(effect_id))
            .sum::<f32>()
    }
}

impl Default for DiagramInterpretation {
    fn default() -> Self {
        Self {
            circle_quality: 0.0,
            circle_found: false,
            circle_center: default_circle_center(),
            runes: Vec::new(),
            rejected_marks: 0,
            spell: None,
            scope_spell: None,
        }
    }
}

impl DiagramInterpretation {
    /// `circle_found` is already exactly `circle_quality >= <the acceptance band's circle
    /// threshold>` computed at construction time (plan Phase 5 item 1) — re-deriving it against
    /// the `Commission`-band `MIN_CIRCLE_QUALITY` constant here would silently ignore whatever
    /// context (Practice/Sandbox) this interpretation was actually produced under.
    pub fn accepted(&self) -> bool {
        self.circle_found && !self.runes.is_empty()
    }

    pub fn average_rune_quality(&self) -> f32 {
        if self.runes.is_empty() {
            0.0
        } else {
            self.runes.iter().map(|rune| rune.quality).sum::<f32>() / self.runes.len() as f32
        }
    }

    /// Average magnitude channel across every rune found — 1.0 is
    /// reference size with a fully-traced stroke; see `InterpretedRune::potency`.
    pub fn average_rune_potency(&self) -> f32 {
        if self.runes.is_empty() {
            0.0
        } else {
            self.runes.iter().map(|rune| rune.potency).sum::<f32>() / self.runes.len() as f32
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterpretedRune {
    pub rune_id: String,
    pub confidence: f32,
    pub quality: f32,
    pub center: StrokePoint,
    #[serde(default)]
    pub scale: f32,
    #[serde(default)]
    pub orbit: f32,
    /// Which scope this rune was read in: 0 for the working circle itself, 1
    /// for a mark inside one of its sub-circles, and so on. `crate::reading`
    /// only groups marks that share a scope — a scope is a sentence.
    #[serde(default)]
    pub scope_depth: u32,
    /// Magnitude channel (plan Phase 2): how strongly this rune's *size and
    /// completeness* — as opposed to its shape quality — should scale its
    /// effect. 1.0 at a category's reference size with a fully-traced
    /// stroke; see `recognition::potency_for_rune` and prd.md §4.
    #[serde(default = "default_potency")]
    pub potency: f32,
}

fn default_potency() -> f32 {
    1.0
}

fn default_circle_center() -> StrokePoint {
    StrokePoint::new(0.5, 0.5)
}

/// Interprets `strokes` using the `Commission` acceptance band — bit-identical to this
/// function's pre-Phase-5 behavior. See `interpret_diagram_in_context` for Practice/Sandbox.
pub fn interpret_diagram<'a>(
    strokes: &[DrawnStroke],
    runes: impl IntoIterator<Item = &'a RuneDef>,
) -> DiagramInterpretation {
    interpret_diagram_in_context(strokes, runes, RecognitionContext::Commission)
}

/// Interprets `strokes`, accepting/rejecting the working circle and each contained rune per
/// `context`'s `DiagramAcceptanceBand` (plan Phase 5 item 1). All scoring (circle quality,
/// rune confidence/quality, potency, the whole scope tree) is computed exactly as
/// `interpret_diagram` always has — `context` only changes where the acceptance cutoffs fall.
pub fn interpret_diagram_in_context<'a>(
    strokes: &[DrawnStroke],
    runes: impl IntoIterator<Item = &'a RuneDef>,
    context: RecognitionContext,
) -> DiagramInterpretation {
    let band = diagram_acceptance_band(context);
    let useful = strokes
        .iter()
        .enumerate()
        .filter(|(_, stroke)| stroke.has_ink())
        .collect::<Vec<_>>();

    let circle_candidates = gather_circle_candidates(&useful);
    let Some((circle_member_indices, circle_quality, circle_bounds)) =
        select_working_circle_for_strokes(&circle_candidates, &useful, band.circle)
    else {
        return DiagramInterpretation {
            circle_found: false,
            ..Default::default()
        };
    };

    let circle_found = circle_quality >= band.circle;
    let available_runes = runes.into_iter().collect::<Vec<_>>();
    let scope_ink = useful
        .into_iter()
        .filter(|(index, _)| !circle_member_indices.contains(index))
        .map(|(index, stroke)| (index, stroke.clone()))
        .collect::<Vec<_>>();

    let outcome = interpret_scope(&scope_ink, circle_bounds, &available_runes, 0, context);
    let rejected_marks = outcome.rejected_marks;

    let mut interpreted = flatten_runes(&outcome);
    interpreted.sort_by(|a, b| {
        a.center
            .y
            .total_cmp(&b.center.y)
            .then(a.center.x.total_cmp(&b.center.x))
    });
    // The reading (`crate::reading`) is what turns placement into meaning; the
    // spell summary needs it so balance is measured over marks drawn together
    // rather than over every mark separately.
    let reading = crate::reading::read(&interpreted, circle_bounds.center(), &available_runes);
    let spell = if circle_found {
        analyze_magical_circle(
            circle_quality,
            &outcome.circle_marks,
            &interpreted,
            &available_runes,
            &reading,
        )
    } else {
        None
    };
    // Deliberately *not* gated on `spell.is_some()`: `analyze_magical_circle`'s own
    // "elaborate enough" gate (structure ≥2 marks or a tier-≥4 rune) exists for the
    // ring/satellite bonus struct's own purposes, but a small, low-structure diagram — a
    // fireball is the plan's own example — must still be matchable against
    // `assets/data/recipes.json` (`crate::recipes::match_recipe`). Any circle at all gets a
    // scope tree.
    let scope_spell = circle_found.then(|| build_scope_spell(&outcome, &available_runes));

    DiagramInterpretation {
        circle_quality,
        circle_found,
        circle_center: circle_bounds.center(),
        runes: interpreted,
        rejected_marks,
        spell,
        scope_spell,
    }
}

pub(crate) fn is_circle_structure(index: usize, marks: &[(usize, CircleMark)]) -> bool {
    let reinforcement_count = marks
        .iter()
        .filter(|(_, mark)| mark.kind == CircleStrokeKind::ReinforcementRing && mark.quality > 0.48)
        .count();
    let satellite_count = marks
        .iter()
        .filter(|(_, mark)| mark.kind == CircleStrokeKind::SatelliteSeal && mark.quality > 0.68)
        .count();
    let script_count = marks
        .iter()
        .filter(|(_, mark)| mark.kind == CircleStrokeKind::ScriptMark && mark.quality > 0.42)
        .count();
    let radial_count = marks
        .iter()
        .filter(|(_, mark)| mark.kind == CircleStrokeKind::RadialSpoke && mark.quality > 0.68)
        .count();
    marks.iter().any(|(mark_index, mark)| {
        *mark_index == index
            && if mark.kind == CircleStrokeKind::SatelliteSeal {
                satellite_count >= 3 && mark.quality > 0.68
            } else if mark.kind == CircleStrokeKind::ReinforcementRing {
                reinforcement_count >= 2 && mark.quality > 0.48
            } else if mark.kind == CircleStrokeKind::ScriptMark {
                script_count >= 8 && mark.quality > 0.42
            } else if mark.kind == CircleStrokeKind::RadialSpoke {
                radial_count >= 6 && mark.quality > 0.68
            } else {
                mark.kind.is_circle_structure()
            }
    })
}
