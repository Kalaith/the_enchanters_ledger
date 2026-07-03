use crate::rune_diagram::DiagramInterpretation;
use crate::rune_drawing::{DrawnStroke, RecognitionOutcome, StrokePoint};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const BOARD_COLUMNS: usize = 5;
pub const BOARD_ROWS: usize = 4;
pub const BOARD_NODE_COUNT: usize = BOARD_COLUMNS * BOARD_ROWS;

fn default_rune_quality() -> f32 {
    1.0
}

fn default_rune_potency() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedRune {
    pub rune_id: String,
    pub node: usize,
    #[serde(default = "default_rune_quality")]
    pub quality: f32,
    /// Magnitude channel (plan Phase 2) — see `InterpretedRune::potency`.
    #[serde(default = "default_rune_potency")]
    pub potency: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub a: usize,
    pub b: usize,
}

impl Link {
    pub fn new(a: usize, b: usize) -> Self {
        if a <= b {
            Self { a, b }
        } else {
            Self { a: b, b: a }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuideTemplate {
    pub rune_id: String,
    pub center: StrokePoint,
    pub scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignBoard {
    pub placed: Vec<PlacedRune>,
    pub links: Vec<Link>,
    pub selected_rune: Option<String>,
    #[serde(default)]
    pub template_armed: bool,
    #[serde(default)]
    pub guide_templates: Vec<GuideTemplate>,
    #[serde(default)]
    pub selected_node: Option<usize>,
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
    pub link_anchor: Option<usize>,
    pub last_evaluation: Option<super::EnchantResult>,
}

impl DesignBoard {
    pub(super) fn new() -> Self {
        Self {
            placed: Vec::new(),
            links: Vec::new(),
            selected_rune: Some("light".to_owned()),
            template_armed: false,
            guide_templates: Vec::new(),
            selected_node: None,
            drawing_strokes: Vec::new(),
            active_stroke: None,
            last_recognition: None,
            last_diagram: None,
            last_interpretation_note: None,
            last_diagnostic_log: None,
            link_anchor: None,
            last_evaluation: None,
        }
    }

    pub fn rune_at(&self, node: usize) -> Option<&str> {
        self.placed_at(node).map(|placed| placed.rune_id.as_str())
    }

    pub fn placed_at(&self, node: usize) -> Option<&PlacedRune> {
        self.placed.iter().find(|placed| placed.node == node)
    }

    pub(super) fn place(&mut self, node: usize, rune_id: &str, quality: f32, potency: f32) {
        if let Some(existing) = self.placed.iter_mut().find(|placed| placed.node == node) {
            existing.rune_id = rune_id.to_owned();
            existing.quality = quality;
            existing.potency = potency;
        } else {
            self.placed.push(PlacedRune {
                rune_id: rune_id.to_owned(),
                node,
                quality,
                potency,
            });
        }
        let occupied: HashSet<usize> = self.placed.iter().map(|placed| placed.node).collect();
        self.links
            .retain(|link| occupied.contains(&link.a) && occupied.contains(&link.b));
    }

    pub(super) fn clear_drawing(&mut self) {
        self.drawing_strokes.clear();
        self.active_stroke = None;
        self.last_recognition = None;
        self.clear_interpretation_feedback();
    }

    pub(super) fn clear_marks(&mut self) {
        self.placed.clear();
        self.links.clear();
        self.selected_node = None;
        self.link_anchor = None;
        self.last_diagnostic_log = None;
    }

    pub(super) fn clear_interpretation_feedback(&mut self) {
        self.last_diagram = None;
        self.last_interpretation_note = None;
        self.last_diagnostic_log = None;
    }
}

pub fn node_grid(node: usize) -> (i32, i32) {
    ((node % BOARD_COLUMNS) as i32, (node / BOARD_COLUMNS) as i32)
}

pub fn node_distance(a: usize, b: usize) -> i32 {
    let (ax, ay) = node_grid(a);
    let (bx, by) = node_grid(b);
    (ax - bx).abs() + (ay - by).abs()
}
