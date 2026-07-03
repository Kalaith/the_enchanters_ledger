//! Embedded game data for commissions, runes, and publishing metadata.

use macroquad_toolkit::assets::TextureConfig;
use macroquad_toolkit::data_loader::{load_embedded_json, load_embedded_json_labeled};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const GAME_CONFIG_JSON: &str = include_str!("../assets/data/game_config.json");
const RUNES_JSON: &str = include_str!("../assets/data/runes.json");
const COMMISSIONS_JSON: &str = include_str!("../assets/data/commissions.json");
const TALISMAN_JOBS_JSON: &str = include_str!("../assets/data/talisman_jobs.json");
const RECIPES_JSON: &str = include_str!("../assets/data/recipes.json");
const TEXTURE_MANIFEST_JSON: &str = include_str!("../assets/data/texture_manifest.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub game_name: String,
    pub display_name: String,
    pub save_slot: String,
    pub version: String,
    pub starting_coins: i64,
    pub starting_reputation: i64,
    pub starting_insight: i64,
    pub starting_focus: f32,
    pub max_focus: f32,
    pub focus_per_second: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuneCategory {
    Effect,
    Shape,
    Trigger,
    Modifier,
}

impl RuneCategory {
    pub const ALL: [RuneCategory; 4] = [
        RuneCategory::Effect,
        RuneCategory::Shape,
        RuneCategory::Trigger,
        RuneCategory::Modifier,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RuneCategory::Effect => "Effect",
            RuneCategory::Shape => "Shape",
            RuneCategory::Trigger => "Trigger",
            RuneCategory::Modifier => "Modifier",
        }
    }

    /// A rune's "normal" size, as scale relative to its working circle
    /// (`StrokeBounds::scale_relative`) — the reference point a magnitude
    /// channel (`rune_diagram`'s potency) and diagram harmony scoring
    /// (`magical_circle::size_harmony`) both measure size against.
    pub fn ideal_scale_in_circle(self) -> f32 {
        match self {
            RuneCategory::Effect => 0.18,
            RuneCategory::Shape => 0.15,
            RuneCategory::Trigger => 0.14,
            RuneCategory::Modifier => 0.12,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneDef {
    pub id: String,
    pub name: String,
    pub glyph: String,
    pub category: RuneCategory,
    pub description: String,
    pub tier: u32,
    pub power: i32,
    pub stability: i32,
    pub mana_cost: i32,
    pub safety: i32,
}

/// A named spell defined as a data predicate over a diagram's `rune_diagram::ScopeSpell` tree
/// (plan Phase 4 item 3) — the replacement for hand-written `match` arms in
/// `magical_circle::spell_name`. See `crate::recipes::match_recipe` for how `requires` is
/// evaluated and `assets/data/recipes.json` for the actual roster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeDef {
    pub id: String,
    pub name: String,
    pub tier: u32,
    pub requires: RecipeRequirements,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecipeRequirements {
    /// rune_id -> minimum total potency of that effect summed across the whole scope tree
    /// (this scope plus every descendant sub-scope) — see `ScopeSpell::total_potency`.
    #[serde(default)]
    pub effect: HashMap<String, EffectRequirement>,
    /// Checked against the root scope's own shape/trigger/modifier only — a diagram's overall
    /// "shape" and "trigger" character is set by its outermost circle, not by what a sub-scope
    /// vent happens to carry.
    #[serde(default)]
    pub shape: Option<String>,
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub modifier: Option<String>,
    /// Checked against the root scope's own structure-mark counts.
    #[serde(default)]
    pub structure: StructureRequirement,
    /// Each entry requires at least `count` *direct* child scopes whose own effects include
    /// `effect` — the mechanism for recipes like `volcano` that need several distinct vents.
    #[serde(default)]
    pub sub_scopes: Vec<SubScopeRequirement>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EffectRequirement {
    #[serde(default)]
    pub min_potency: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructureRequirement {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubScopeRequirement {
    pub effect: String,
    #[serde(default = "one")]
    pub count: usize,
}

fn one() -> usize {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionDef {
    pub id: String,
    pub customer: String,
    pub request: String,
    pub item: String,
    pub required_effect: String,
    pub required_shape: String,
    pub required_trigger: String,
    pub optional_modifier: Option<String>,
    pub difficulty: u32,
    pub reward: i64,
    pub reputation: i64,
    pub insight: i64,
    pub risk: String,
}

#[derive(Debug, Clone)]
pub struct GameData {
    pub config: GameConfig,
    pub runes: Vec<RuneDef>,
    pub commissions: Vec<CommissionDef>,
    pub talisman_jobs: Vec<CommissionDef>,
    pub recipes: Vec<RecipeDef>,
    pub texture_manifest: Vec<TextureConfig>,
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let config = load_embedded_json_labeled("game_config", GAME_CONFIG_JSON)?;
        let runes = load_embedded_json_labeled("runes", RUNES_JSON)?;
        let commissions = load_embedded_json_labeled("commissions", COMMISSIONS_JSON)?;
        let talisman_jobs = load_embedded_json_labeled("talisman_jobs", TALISMAN_JOBS_JSON)?;
        let recipes = load_embedded_json_labeled("recipes", RECIPES_JSON)?;
        let texture_manifest = load_embedded_json(TEXTURE_MANIFEST_JSON)?;

        Ok(Self {
            config,
            runes,
            commissions,
            talisman_jobs,
            recipes,
            texture_manifest,
        })
    }

    pub fn rune(&self, id: &str) -> Option<&RuneDef> {
        self.runes.iter().find(|rune| rune.id == id)
    }

    pub fn rune_name<'a>(&'a self, id: &'a str) -> &'a str {
        self.rune(id).map(|rune| rune.name.as_str()).unwrap_or(id)
    }

    pub fn runes_in_category(&self, category: RuneCategory) -> impl Iterator<Item = &RuneDef> {
        self.runes
            .iter()
            .filter(move |rune| rune.category == category)
    }

    pub fn commission(&self, index: usize) -> &CommissionDef {
        let safe_index = index % self.commissions.len().max(1);
        &self.commissions[safe_index]
    }

    pub fn talisman_job(&self, index: usize) -> &CommissionDef {
        let safe_index = index % self.talisman_jobs.len().max(1);
        &self.talisman_jobs[safe_index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_data_loads() {
        let data = GameData::load().unwrap();

        assert_eq!(data.config.game_name, "the_enchanters_ledger");
        assert!(data.rune("light").is_some());
        assert!(data.rune("continuous").is_some());
        assert!(!data.commissions.is_empty());
        assert!(!data.talisman_jobs.is_empty());
    }
}
