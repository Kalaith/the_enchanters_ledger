//! Runtime state, save data, and enchantment evaluation.

use crate::data::{GameConfig, GameData, RuneDef};
use crate::rune_diagram::interpret_diagram;
use crate::rune_drawing::{erase_strokes_at, DrawnStroke, StrokePoint};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

mod board;
mod evaluate;
mod save;
#[cfg(test)]
mod tests;
mod text;
mod tutorial;
mod work;

pub use board::{
    node_distance, node_grid, DesignBoard, GuideTemplate, Link, BOARD_COLUMNS, BOARD_NODE_COUNT,
    BOARD_ROWS,
};
pub use save::migrate_save_value;
use text::percent;
pub use tutorial::TutorialStage;
pub use work::{DiscoveryReward, JournalEntry, WorkOrderKind, DISCOVERY_INSIGHT};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamePhase {
    Title,
    Naming,
    Playing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnchantGrade {
    Brilliant,
    Reliable,
    Unstable,
    Failed,
}

impl EnchantGrade {
    pub fn label(self) -> &'static str {
        match self {
            EnchantGrade::Brilliant => "Brilliant",
            EnchantGrade::Reliable => "Reliable",
            EnchantGrade::Unstable => "Unstable",
            EnchantGrade::Failed => "Failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub name: String,
    pub coins: i64,
    pub reputation: i64,
    pub insight: i64,
    pub focus: f32,
    pub day: u32,
    pub workshop_rank: u32,
    pub completed_orders: u32,
    pub accidents: u32,
    pub current_commission: usize,
    #[serde(default)]
    pub current_talisman: usize,
    #[serde(default)]
    pub active_work: WorkOrderKind,
    #[serde(default)]
    pub tutorial_stage: TutorialStage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredRecipe {
    pub signature: String,
    pub name: String,
    pub uses: u32,
    pub best_score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnchantResult {
    pub title: String,
    pub grade: EnchantGrade,
    pub score: i32,
    pub power: i32,
    pub stability: i32,
    pub mana_cost: i32,
    pub safety: i32,
    pub matched_request: bool,
    pub side_effect: String,
    pub accident: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: String,
    pub phase: GamePhase,
    pub player: PlayerState,
    pub board: DesignBoard,
    pub discoveries: Vec<DiscoveredRecipe>,
    #[serde(default)]
    pub journal: Vec<JournalEntry>,
}

#[derive(Debug, Clone)]
pub struct CraftReport {
    pub result: EnchantResult,
    pub discovery: Option<DiscoveryReward>,
    pub reward: i64,
    pub reputation: i64,
    pub insight: i64,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GameSession {
    pub phase: GamePhase,
    pub player: PlayerState,
    pub board: DesignBoard,
    pub discoveries: Vec<DiscoveredRecipe>,
    pub journal: Vec<JournalEntry>,
}

impl GameSession {
    pub fn new(config: &GameConfig) -> Self {
        Self {
            phase: GamePhase::Title,
            player: PlayerState {
                name: "Steve".to_owned(),
                coins: config.starting_coins,
                reputation: config.starting_reputation,
                insight: config.starting_insight,
                focus: config.starting_focus,
                day: 1,
                workshop_rank: 1,
                completed_orders: 0,
                accidents: 0,
                current_commission: 0,
                current_talisman: 0,
                active_work: WorkOrderKind::Story,
                tutorial_stage: TutorialStage::new_game(),
            },
            board: DesignBoard::new(),
            discoveries: Vec::new(),
            journal: Vec::new(),
        }
    }

    pub fn from_save(save: SaveData) -> Self {
        Self {
            phase: save.phase,
            player: save.player,
            board: save.board,
            discoveries: save.discoveries,
            journal: save.journal,
        }
    }

    pub fn to_save(&self, version: &str) -> SaveData {
        SaveData {
            version: version.to_owned(),
            phase: self.phase,
            player: self.player.clone(),
            board: self.board.clone(),
            discoveries: self.discoveries.clone(),
            journal: self.journal.clone(),
        }
    }

    pub fn start_playing(&mut self) {
        if self.player.name.trim().is_empty() {
            self.player.name = "Steve".to_owned();
        }
        self.phase = GamePhase::Playing;
    }

    pub fn append_name_char(&mut self, c: char) {
        if self.phase != GamePhase::Naming || self.player.name.chars().count() >= 18 {
            return;
        }
        if c.is_ascii_alphanumeric() || c == ' ' || c == '\'' || c == '-' {
            if self.player.name == "Steve" {
                self.player.name.clear();
            }
            self.player.name.push(c);
        }
    }

    pub fn pop_name_char(&mut self) {
        if self.phase == GamePhase::Naming {
            self.player.name.pop();
        }
    }

    pub fn update_focus(&mut self, config: &GameConfig, dt: f32) {
        self.player.focus =
            (self.player.focus + config.focus_per_second * dt).min(config.max_focus);
    }

    pub fn can_use_rune(&self, rune: &RuneDef) -> bool {
        if let Some(allowed) = self.player.tutorial_stage.allowed_runes() {
            return allowed.iter().any(|id| *id == rune.id);
        }
        rune.tier <= self.player.workshop_rank
    }

    pub fn select_rune(&mut self, rune_id: &str, data: &GameData) -> Result<(), String> {
        let rune = data
            .rune(rune_id)
            .ok_or_else(|| format!("Unknown rune: {rune_id}"))?;
        if !self.can_use_rune(rune) {
            return Err(format!("{} is still locked in your archive.", rune.name));
        }
        self.board.selected_rune = Some(rune_id.to_owned());
        self.board.template_armed = true;
        self.board.link_anchor = None;
        Ok(())
    }

    pub fn deselect_rune(&mut self) {
        self.board.selected_rune = None;
        self.board.template_armed = false;
        self.board.link_anchor = None;
    }

    pub fn place_guide_template(
        &mut self,
        center: StrokePoint,
        data: &GameData,
    ) -> Result<String, String> {
        let rune_id = self
            .board
            .selected_rune
            .clone()
            .ok_or_else(|| "Select a rune in the guide first.".to_owned())?;
        let rune = data
            .rune(&rune_id)
            .ok_or_else(|| format!("Unknown rune: {rune_id}"))?;
        if !self.can_use_rune(rune) {
            return Err(format!("{} is still locked in your guide.", rune.name));
        }
        if self.board.guide_templates.len() >= 12 {
            self.board.guide_templates.remove(0);
        }
        self.board.guide_templates.push(GuideTemplate {
            rune_id: rune_id.clone(),
            center,
            scale: 0.22,
        });
        self.board.template_armed = false;
        self.board.last_diagram = None;
        self.board.last_interpretation_note = None;
        self.board.last_diagnostic_log = None;
        Ok(format!(
            "Placed {} as a tracing guide; only your ink will be scored.",
            rune.name
        ))
    }

    pub fn remove_guide_template(
        &mut self,
        index: usize,
        data: &GameData,
    ) -> Result<String, String> {
        if index >= self.board.guide_templates.len() {
            return Err("That tracing guide is already gone.".to_owned());
        }
        let template = self.board.guide_templates.remove(index);
        self.board.last_diagram = None;
        self.board.last_interpretation_note = None;
        self.board.last_diagnostic_log = None;
        Ok(format!(
            "Removed {} guide.",
            data.rune_name(&template.rune_id)
        ))
    }

    pub fn move_guide_template(&mut self, index: usize, center: StrokePoint) -> Result<(), String> {
        let Some(template) = self.board.guide_templates.get_mut(index) else {
            return Err("That tracing guide is already gone.".to_owned());
        };
        template.center = center;
        self.board.last_diagram = None;
        self.board.last_interpretation_note = None;
        self.board.last_diagnostic_log = None;
        Ok(())
    }

    pub fn start_drawing_stroke(&mut self, point: StrokePoint) {
        self.board.last_diagnostic_log = None;
        self.board.last_diagram = None;
        self.board.last_interpretation_note = None;
        self.board.active_stroke = Some(DrawnStroke::new(point));
    }

    pub fn extend_drawing_stroke(&mut self, point: StrokePoint) {
        if let Some(stroke) = &mut self.board.active_stroke {
            stroke.push(point);
        }
    }

    pub fn finish_drawing_stroke(&mut self) {
        if let Some(stroke) = self.board.active_stroke.take() {
            if stroke.has_ink() {
                self.board.drawing_strokes.push(stroke);
                self.board.last_recognition = None;
                self.board.last_diagnostic_log = None;
            }
        }
    }

    pub fn clear_drawing(&mut self) {
        self.board.clear_drawing();
    }

    pub fn erase_drawing_at(&mut self, point: StrokePoint, radius: f32) -> bool {
        let erased = erase_strokes_at(&mut self.board.drawing_strokes, point, radius);
        if erased {
            self.board.last_diagram = None;
            self.board.last_interpretation_note = None;
            self.board.last_diagnostic_log = None;
        }
        erased
    }

    pub fn interpret_drawing(&mut self, data: &GameData) -> Result<String, String> {
        if self.board.drawing_strokes.is_empty() {
            return Err("The diagram slate is blank.".to_owned());
        }

        let unlocked = data.runes.iter().filter(|rune| self.can_use_rune(rune));
        let interpretation = interpret_diagram(&self.board.drawing_strokes, unlocked);
        self.board.last_diagram = Some(interpretation.clone());

        if !interpretation.circle_found {
            self.board.last_interpretation_note =
                Some("No enclosing circle was readable.".to_owned());
            self.spend_focus(0.8)?;
            return Err(
                "The diagram needs an enclosing circle before it can hold meaning.".to_owned(),
            );
        }
        if !interpretation.accepted() {
            self.board.last_interpretation_note = Some(format!(
                "Circle {}%, but no inner rune was clear enough.",
                percent(interpretation.circle_quality)
            ));
            self.spend_focus(0.8)?;
            return Err(format!(
                "The circle reads at {}%, but no inner rune is clear enough.",
                percent(interpretation.circle_quality)
            ));
        }

        self.spend_focus(1.5 + interpretation.runes.len() as f32 * 0.7)?;
        let mut occupied = HashSet::new();
        let mut placed_nodes = Vec::new();
        self.board.clear_marks();
        for rune in &interpretation.runes {
            let node = node_for_diagram_center(rune.center, &occupied);
            occupied.insert(node);
            placed_nodes.push(node);
            self.board.place(node, &rune.rune_id, rune.quality);
        }
        for nodes in placed_nodes.windows(2) {
            let link = Link::new(nodes[0], nodes[1]);
            if !self.board.links.contains(&link) {
                self.board.links.push(link);
            }
        }
        self.board.active_stroke = None;
        self.board.last_recognition = None;
        self.board.last_diagram = Some(interpretation.clone());

        let names = interpretation
            .runes
            .iter()
            .map(|rune| data.rune_name(&rune.rune_id).to_owned())
            .collect::<Vec<_>>()
            .join(" + ");
        let recognized_ids = interpretation
            .runes
            .iter()
            .map(|rune| rune.rune_id.as_str())
            .collect::<Vec<_>>();
        let commission = self.current_commission(data);
        let missing_required = [
            commission.required_effect.as_str(),
            commission.required_shape.as_str(),
            commission.required_trigger.as_str(),
        ]
        .into_iter()
        .filter(|id| !recognized_ids.contains(id))
        .map(|id| data.rune_name(id).to_owned())
        .collect::<Vec<_>>();
        let commission_fit = if missing_required.is_empty() {
            " Ready to test this commission.".to_owned()
        } else {
            format!(
                " Missing for this commission: {}.",
                missing_required.join(" + ")
            )
        };
        let rejected = if interpretation.rejected_marks > 0 {
            format!(
                "; {} unclear mark(s) ignored",
                interpretation.rejected_marks
            )
        } else {
            String::new()
        };
        let circle_spell = interpretation
            .spell
            .as_ref()
            .map(|spell| format!(" {}", spell.note()))
            .unwrap_or_default();
        let note = format!(
            "Recognized runes: {} (circle {}%, rune quality {}%{}).{}{}",
            names,
            percent(interpretation.circle_quality),
            percent(interpretation.average_rune_quality()),
            rejected,
            commission_fit,
            circle_spell
        );
        self.board.last_interpretation_note = Some(note.clone());
        Ok(note)
    }

    pub fn clear_board(&mut self) {
        self.board.clear_marks();
        self.board.clear_drawing();
        self.board.guide_templates.clear();
        self.board.last_diagram = None;
        self.board.last_interpretation_note = None;
        self.board.last_diagnostic_log = None;
        self.board.last_evaluation = None;
    }

    fn spend_focus(&mut self, amount: f32) -> Result<(), String> {
        if self.player.focus + f32::EPSILON < amount {
            return Err("Not enough focus to ink more lines.".to_owned());
        }
        self.player.focus -= amount;
        Ok(())
    }
}

fn node_for_diagram_center(center: StrokePoint, occupied: &HashSet<usize>) -> usize {
    (0..BOARD_NODE_COUNT)
        .filter(|node| !occupied.contains(node))
        .min_by(|a, b| {
            let (ax, ay) = node_grid(*a);
            let (bx, by) = node_grid(*b);
            let a_pos = normalized_node_position(ax, ay);
            let b_pos = normalized_node_position(bx, by);
            let a_distance = normalized_distance(center, a_pos);
            let b_distance = normalized_distance(center, b_pos);
            a_distance.total_cmp(&b_distance)
        })
        .unwrap_or(0)
}

fn normalized_node_position(col: i32, row: i32) -> StrokePoint {
    StrokePoint::new(
        col as f32 / (BOARD_COLUMNS as f32 - 1.0),
        row as f32 / (BOARD_ROWS as f32 - 1.0),
    )
}

fn normalized_distance(a: StrokePoint, b: StrokePoint) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}
