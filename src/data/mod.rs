//! Embedded game data for commissions, runes, and publishing metadata.

use macroquad_toolkit::assets::TextureConfig;
use macroquad_toolkit::data_loader::{load_embedded_json, load_embedded_json_labeled};
use serde::{Deserialize, Serialize};

const GAME_CONFIG_JSON: &str = include_str!("../../assets/data/game_config.json");
const RUNES_JSON: &str = include_str!("../../assets/data/runes.json");
const COMMISSIONS_JSON: &str = include_str!("../../assets/data/commissions.json");
const TEXTURE_MANIFEST_JSON: &str = include_str!("../../assets/data/texture_manifest.json");

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

    pub fn short(self) -> &'static str {
        match self {
            RuneCategory::Effect => "E",
            RuneCategory::Shape => "S",
            RuneCategory::Trigger => "T",
            RuneCategory::Modifier => "M",
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
    pub texture_manifest: Vec<TextureConfig>,
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let config = load_embedded_json_labeled("game_config", GAME_CONFIG_JSON)?;
        let runes = load_embedded_json_labeled("runes", RUNES_JSON)?;
        let commissions = load_embedded_json_labeled("commissions", COMMISSIONS_JSON)?;
        let texture_manifest = load_embedded_json(TEXTURE_MANIFEST_JSON)?;

        Ok(Self {
            config,
            runes,
            commissions,
            texture_manifest,
        })
    }

    pub fn rune(&self, id: &str) -> Option<&RuneDef> {
        self.runes.iter().find(|rune| rune.id == id)
    }

    pub fn rune_name(&self, id: &str) -> &str {
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

    pub fn required_runes<'a>(&'a self, commission: &'a CommissionDef) -> Vec<&'a str> {
        let mut required = vec![
            commission.required_effect.as_str(),
            commission.required_shape.as_str(),
            commission.required_trigger.as_str(),
        ];
        if let Some(modifier) = &commission.optional_modifier {
            required.push(modifier);
        }
        required
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
    }
}
