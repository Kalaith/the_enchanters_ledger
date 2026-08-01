use crate::rune_diagram::DiagramInterpretation;
use crate::rune_drawing::{DrawnStroke, RecognitionOutcome, StrokePoint};
use serde::{Deserialize, Serialize};

fn default_rune_quality() -> f32 {
    1.0
}

fn default_rune_potency() -> f32 {
    1.0
}

/// One mark the last interpretation read off the slate. This is the list
/// `evaluate` scores; where each mark sat is carried by the interpretation
/// itself (`DesignBoard::last_diagram`), which is what `crate::reading` reads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedRune {
    pub rune_id: String,
    #[serde(default = "default_rune_quality")]
    pub quality: f32,
    /// Magnitude channel (plan Phase 2) — see `InterpretedRune::potency`.
    #[serde(default = "default_rune_potency")]
    pub potency: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuideTemplate {
    pub rune_id: String,
    pub center: StrokePoint,
    pub scale: f32,
}

/// A tracing guide for the diagram's enclosing circle — the counterpart to
/// `GuideTemplate` for the one part of a diagram that is not a rune. Laid out by
/// `GameSession::place_reference_diagram` from `crate::perfect_diagram`, and
/// rendered from the same `perfect_diagram::circle_points` the reference ink
/// would use, so tracing it exactly is worth full circle quality.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircleGuide {
    pub center: StrokePoint,
    pub radius: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignBoard {
    pub placed: Vec<PlacedRune>,
    pub selected_rune: Option<String>,
    #[serde(default)]
    pub template_armed: bool,
    #[serde(default)]
    pub guide_templates: Vec<GuideTemplate>,
    #[serde(default)]
    pub circle_guide: Option<CircleGuide>,
    /// Tracing guides for structural work — reinforcement rings, satellite
    /// seals, sub-scope circles and decorative marks. Plain ink to copy rather
    /// than `GuideTemplate`s, since none of it is a rune with an identity to
    /// score.
    #[serde(default)]
    pub guide_structure: Vec<DrawnStroke>,
    #[serde(default)]
    pub drawing_strokes: Vec<DrawnStroke>,
    #[serde(default)]
    pub active_stroke: Option<DrawnStroke>,
    #[serde(default)]
    pub last_recognition: Option<RecognitionOutcome>,
    #[serde(default)]
    pub last_diagram: Option<DiagramInterpretation>,
    #[serde(default)]
    pub last_interpretation_note: Option<String>,
    #[serde(skip)]
    pub last_diagnostic_log: Option<String>,
    pub last_evaluation: Option<super::EnchantResult>,
}

impl DesignBoard {
    pub(super) fn new() -> Self {
        Self {
            placed: Vec::new(),
            selected_rune: Some("light".to_owned()),
            template_armed: false,
            guide_templates: Vec::new(),
            circle_guide: None,
            guide_structure: Vec::new(),
            drawing_strokes: Vec::new(),
            active_stroke: None,
            last_recognition: None,
            last_diagram: None,
            last_interpretation_note: None,
            last_diagnostic_log: None,
            last_evaluation: None,
        }
    }

    pub(super) fn place(&mut self, rune_id: &str, quality: f32, potency: f32) {
        self.placed.push(PlacedRune {
            rune_id: rune_id.to_owned(),
            quality,
            potency,
        });
    }

    pub(super) fn clear_drawing(&mut self) {
        self.drawing_strokes.clear();
        self.active_stroke = None;
        self.last_recognition = None;
        self.clear_interpretation_feedback();
    }

    pub(super) fn clear_marks(&mut self) {
        self.placed.clear();
        self.last_diagnostic_log = None;
    }

    pub(super) fn clear_interpretation_feedback(&mut self) {
        self.last_diagram = None;
        self.last_interpretation_note = None;
        self.last_diagnostic_log = None;
    }
}
