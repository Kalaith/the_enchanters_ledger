//! The practice ladder: ten diagrams that trade quantity for difficulty.
//!
//! Level 1 is ten easy marks around one circle. Every level after it drops one
//! mark and adds difficulty somewhere else — smaller notation, higher tiers,
//! structural work, nested scopes — until level 10 is a single mark carrying
//! everything the notation can express.
//!
//! Levels are data (`assets/data/ladder.json`), not code: adding or retuning
//! one is a JSON edit, and `crate::perfect_diagram` lays out whatever the entry
//! asks for. The tests interpret every level's diagram back through the real
//! recognizer, so a level that cannot actually be drawn cannot ship.

use crate::data::GameData;
use crate::perfect_diagram::{
    perfect_diagram_for, DiagramRequest, PerfectDiagram, RuneScale, StructurePlan,
};
use serde::Deserialize;
use std::sync::OnceLock;

#[cfg(test)]
mod tests;

const LADDER_JSON: &str = include_str!("../assets/data/ladder.json");

/// One rung: what to draw, and how hard it is meant to be.
#[derive(Debug, Clone, Deserialize)]
pub struct LadderLevel {
    pub level: u32,
    pub title: String,
    pub complexity: String,
    pub brief: String,
    pub runes: Vec<String>,
    /// See `perfect_diagram::DiagramRequest::rune_scale` — the lower levels
    /// pack many marks in by drawing them small.
    #[serde(default = "default_rune_scale")]
    pub rune_scale: f32,
    #[serde(default)]
    pub structure: LadderStructure,
    /// Runes drawn inside each sub-scope circle; defaults to the level's first
    /// rune when the level asks for sub-scopes without naming contents.
    #[serde(default)]
    pub sub_scope_runes: Vec<String>,
    /// Marks to draw together, as indices into `runes` — the rung's demand that
    /// the reading say something specific, rather than just that the marks be
    /// present. See `crate::reading`.
    #[serde(default)]
    pub groups: Vec<Vec<usize>>,
}

fn default_rune_scale() -> f32 {
    1.0
}

/// The structural work a rung demands. Mirrors `data::StructureRequirement`
/// plus sub-scopes, kept separate so the ladder's JSON does not have to look
/// like a commission's.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct LadderStructure {
    #[serde(default)]
    pub rings: usize,
    #[serde(default)]
    pub satellites: usize,
    #[serde(default)]
    pub radials: usize,
    #[serde(default)]
    pub perimeter: usize,
    #[serde(default)]
    pub scripts: usize,
    #[serde(default)]
    pub sub_scopes: usize,
}

impl From<LadderStructure> for StructurePlan {
    fn from(structure: LadderStructure) -> Self {
        Self {
            rings: structure.rings,
            satellites: structure.satellites,
            radials: structure.radials,
            perimeter: structure.perimeter,
            scripts: structure.scripts,
            sub_scopes: structure.sub_scopes,
        }
    }
}

pub fn ladder_levels() -> &'static [LadderLevel] {
    static LEVELS: OnceLock<Vec<LadderLevel>> = OnceLock::new();
    LEVELS.get_or_init(|| {
        serde_json::from_str(LADDER_JSON).expect("assets/data/ladder.json should be valid")
    })
}

/// Lays out the diagram for one rung. Runes with no template data, or ids that
/// are not runes at all, are skipped — the same rule the rest of the layout
/// follows.
pub fn diagram_for_level(level: &LadderLevel, data: &GameData) -> PerfectDiagram {
    let runes = level
        .runes
        .iter()
        .filter_map(|id| data.rune(id))
        .collect::<Vec<_>>();
    let sub_scope_runes = if level.sub_scope_runes.is_empty() {
        runes.first().copied().into_iter().collect()
    } else {
        level
            .sub_scope_runes
            .iter()
            .filter_map(|id| data.rune(id))
            .collect()
    };

    perfect_diagram_for(&DiagramRequest {
        runes,
        structure: level.structure.into(),
        sub_scope_runes,
        rune_scale: RuneScale(level.rune_scale),
        groups: level.groups.clone(),
    })
}
