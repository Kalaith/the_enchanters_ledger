//! The data-declared structural profile a rune is scored against. Every field
//! here is deserialized straight out of `assets/data/rune_templates.json`; the
//! evaluator in `shape.rs` reads them generically, so adding a rune with
//! structural character is a JSON edit, never a new Rust branch.

use serde::Deserialize;

/// A rune's structural profile, declared in
/// `assets/data/rune_templates.json` (plan Phase 1 item 3). Every field maps
/// to a *generic* feature check — the evaluator has no idea which rune it is
/// scoring.
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
