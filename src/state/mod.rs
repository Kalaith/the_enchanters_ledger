//! Runtime state, save data, and enchantment evaluation.

use crate::data::{GameConfig, GameData, RuneCategory, RuneDef};
use crate::rune_diagram::interpret_diagram;
use crate::rune_drawing::{erase_strokes_at, DrawnStroke, StrokePoint};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

mod board;
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
        Ok(format!(
            "Removed {} guide.",
            data.rune_name(&template.rune_id)
        ))
    }

    pub fn start_drawing_stroke(&mut self, point: StrokePoint) {
        self.board.last_diagnostic_log = None;
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

    pub fn test_design(&mut self, data: &GameData) -> CraftReport {
        let result = self.evaluate(data);
        let discovery = self.record_discovery(data, &result);
        let mut insight = 0;
        let mut notes = Vec::new();
        if let Some(discovery) = &discovery {
            insight += discovery.insight;
            self.player.insight += discovery.insight;
            notes.push(format!("Discovery notes +{} insight", discovery.insight));
            self.add_journal_entry(
                "Insight gained",
                format!(
                    "New recipe: {}. Discovery notes +{} insight.",
                    discovery.name, discovery.insight
                ),
            );
        }
        self.board.last_evaluation = Some(result.clone());
        self.board.last_interpretation_note = Some(format!(
            "Test {}: {} | Score {}. {}",
            result.grade.label(),
            result.title,
            result.score,
            result.side_effect
        ));
        CraftReport {
            result,
            discovery,
            reward: 0,
            reputation: 0,
            insight,
            notes,
        }
    }

    pub fn deliver_design(&mut self, data: &GameData) -> CraftReport {
        let commission = self.current_commission(data).clone();
        let work_kind = self.active_work_kind();
        let result = self.evaluate(data);
        let discovery = self.record_discovery(data, &result);
        let (reward, mut reputation, client_insight) = match (work_kind, result.grade) {
            (WorkOrderKind::Story, EnchantGrade::Brilliant) => (
                commission.reward + (commission.reward / 3),
                commission.reputation + 2,
                commission.insight + 2,
            ),
            (WorkOrderKind::Story, EnchantGrade::Reliable) => {
                (commission.reward, commission.reputation, commission.insight)
            }
            (WorkOrderKind::Story, EnchantGrade::Unstable) => (
                commission.reward / 2,
                commission.reputation.saturating_sub(2),
                (commission.insight / 2).max(1),
            ),
            (WorkOrderKind::Story, EnchantGrade::Failed) => (0, -2, 0),
            (WorkOrderKind::Talisman, EnchantGrade::Brilliant) => (
                commission.reward + (commission.reward / 4),
                commission.reputation,
                commission.insight + 1,
            ),
            (WorkOrderKind::Talisman, EnchantGrade::Reliable) => {
                (commission.reward, commission.reputation, commission.insight)
            }
            (WorkOrderKind::Talisman, EnchantGrade::Unstable) => {
                (commission.reward / 2, 0, commission.insight.max(1))
            }
            (WorkOrderKind::Talisman, EnchantGrade::Failed) => (0, 0, 0),
        };
        let mut insight = client_insight;
        let mut notes = Vec::new();

        if client_insight > 0 {
            notes.push(format!("Client notes +{} insight", client_insight));
        }
        if let Some(discovery) = &discovery {
            insight += discovery.insight;
            notes.push(format!("Discovery notes +{} insight", discovery.insight));
        }

        if result.accident {
            self.player.accidents += 1;
            reputation -= 1;
        }

        self.player.coins += reward;
        self.player.reputation += reputation;
        self.player.insight += insight;
        self.player.completed_orders += 1;
        self.player.day += 1;
        if work_kind == WorkOrderKind::Story && result.grade != EnchantGrade::Failed {
            self.player.current_commission =
                (self.player.current_commission + 1) % data.commissions.len().max(1);
        } else if work_kind == WorkOrderKind::Talisman {
            self.rotate_talisman_work(data);
        }
        let body = if notes.is_empty() {
            format!("{} produced no usable notes.", result.title)
        } else {
            format!(
                "{}. Rewards: {} coins, {:+} reputation. {}.",
                result.title,
                reward,
                reputation,
                notes.join("; ")
            )
        };
        self.add_journal_entry(format!("Delivered {}", commission.item), body);
        self.clear_board();
        self.board.last_evaluation = Some(result.clone());

        CraftReport {
            result,
            discovery,
            reward,
            reputation,
            insight,
            notes,
        }
    }

    fn spend_focus(&mut self, amount: f32) -> Result<(), String> {
        if self.player.focus + f32::EPSILON < amount {
            return Err("Not enough focus to ink more lines.".to_owned());
        }
        self.player.focus -= amount;
        Ok(())
    }

    fn evaluate(&self, data: &GameData) -> EnchantResult {
        let commission = self.current_commission(data);
        let placed = self.placed_runes(data);
        let mut by_category: HashMap<RuneCategory, Vec<&RuneDef>> = HashMap::new();
        for placed_rune in &placed {
            by_category
                .entry(placed_rune.rune.category)
                .or_default()
                .push(placed_rune.rune);
        }

        let required = [
            commission.required_effect.as_str(),
            commission.required_shape.as_str(),
            commission.required_trigger.as_str(),
        ];
        let required_hits = required
            .iter()
            .filter(|id| placed.iter().any(|placed| placed.rune.id == **id))
            .count();
        let optional_hit = commission
            .optional_modifier
            .as_ref()
            .is_some_and(|id| placed.iter().any(|placed| placed.rune.id == *id));
        let has_core = [
            RuneCategory::Effect,
            RuneCategory::Shape,
            RuneCategory::Trigger,
        ]
        .iter()
        .all(|category| by_category.contains_key(category));
        let matched_request = required_hits == required.len();
        let average_quality = if placed.is_empty() {
            0.0
        } else {
            placed.iter().map(|placed| placed.quality).sum::<f32>() / placed.len() as f32
        };
        let weak_marks = placed.iter().filter(|placed| placed.quality < 0.58).count() as i32;
        let circle_spell = self
            .board
            .last_diagram
            .as_ref()
            .and_then(|diagram| diagram.spell.as_ref());

        let mut power = placed
            .iter()
            .map(|placed| {
                (placed.rune.power as f32 * (0.35 + placed.quality * 0.65)).round() as i32
            })
            .sum::<i32>();
        let mut stability = 48
            + placed
                .iter()
                .map(|placed| {
                    placed.rune.stability - ((1.0 - placed.quality).max(0.0) * 22.0).round() as i32
                })
                .sum::<i32>();
        let mut mana_cost = placed
            .iter()
            .map(|placed| {
                placed.rune.mana_cost + ((1.0 - placed.quality).max(0.0) * 12.0).round() as i32
            })
            .sum::<i32>();
        let mut safety = 35
            + placed
                .iter()
                .map(|placed| {
                    placed.rune.safety - ((1.0 - placed.quality).max(0.0) * 18.0).round() as i32
                })
                .sum::<i32>();
        let mut score = required_hits as i32 * 24;

        if optional_hit {
            score += 10;
        }
        if has_core {
            score += 12;
        }
        score += ((average_quality - 0.72) * 58.0).round() as i32;
        score -= weak_marks * 14;

        let link_bonus = self.link_quality(data);
        power += link_bonus.power;
        stability += link_bonus.stability;
        mana_cost += link_bonus.cost;
        safety += link_bonus.safety;
        score += link_bonus.score;

        if let Some(spell) = circle_spell {
            power += spell.power_bonus;
            stability += spell.stability_bonus;
            mana_cost += spell.mana_cost_delta;
            safety += spell.safety_bonus;
            score += spell.score_bonus;
        }

        for runes in by_category.values() {
            if runes.len() > 1 {
                let extra = runes.len() as i32 - 1;
                stability -= extra * 8;
                mana_cost += extra * 4;
            }
        }

        let difficulty = commission.difficulty as i32;
        power -= difficulty * 3;
        score += power / 2 + stability / 3 + safety / 4 - mana_cost / 4 - difficulty * 6;

        if !matched_request {
            score -= 30;
        }
        if !has_core {
            score -= 25;
        }

        power = power.max(0);
        stability = stability.clamp(0, 120);
        mana_cost = mana_cost.max(0);
        safety = safety.clamp(0, 120);
        let q = average_quality.clamp(0.0, 1.0);
        score = score.clamp(0, (68.0 + q * 52.0).round() as i32);

        let accident = stability < 26 || safety < 18;
        let grade = if !matched_request || !has_core || score < 35 {
            EnchantGrade::Failed
        } else if stability < 42 || safety < 32 || score < 64 {
            EnchantGrade::Unstable
        } else if score >= 92 && stability >= 68 && safety >= 48 {
            EnchantGrade::Brilliant
        } else {
            EnchantGrade::Reliable
        };

        let base_title = text::result_title(data, commission, &by_category, grade);
        let title = if grade != EnchantGrade::Failed {
            circle_spell
                .filter(|spell| spell.tier_rank >= 3)
                .map(|spell| spell.name.clone())
                .unwrap_or(base_title)
        } else {
            base_title
        };
        let mut side_effect = text::side_effect(
            data,
            commission,
            grade,
            matched_request,
            accident,
            average_quality,
            weak_marks,
        );
        if matched_request && !accident && grade != EnchantGrade::Failed {
            if let Some(spell) = circle_spell {
                side_effect = format!(
                    "{} The {} flares through {} ring(s) and {} satellite seal(s).",
                    side_effect, spell.name, spell.ring_count, spell.satellite_count
                );
            }
        }

        EnchantResult {
            title,
            grade,
            score,
            power,
            stability,
            mana_cost,
            safety,
            matched_request,
            side_effect,
            accident,
        }
    }

    fn placed_runes<'a>(&'a self, data: &'a GameData) -> Vec<PlacedRuneRef<'a>> {
        self.board
            .placed
            .iter()
            .filter_map(|placed| {
                data.rune(&placed.rune_id).map(|rune| PlacedRuneRef {
                    rune,
                    quality: placed.quality.clamp(0.0, 1.0),
                })
            })
            .collect()
    }

    fn link_quality(&self, data: &GameData) -> LinkQuality {
        let mut quality = LinkQuality::default();
        let mut seen_pairs = HashSet::new();

        for link in &self.board.links {
            let (Some(a), Some(b)) = (self.board.rune_at(link.a), self.board.rune_at(link.b))
            else {
                continue;
            };
            let (Some(a_rune), Some(b_rune)) = (data.rune(a), data.rune(b)) else {
                continue;
            };
            let categories = ordered_pair(a_rune.category, b_rune.category);
            seen_pairs.insert(categories);
            let distance = node_distance(link.a, link.b);

            quality.score += 5;
            quality.stability += 4;
            quality.power += 2;
            quality.cost += (distance - 1).max(0) * 2;
            if distance > 2 {
                quality.stability -= 4;
                quality.safety -= 3;
            }
        }

        if seen_pairs.contains(&ordered_pair(RuneCategory::Effect, RuneCategory::Shape)) {
            quality.score += 10;
            quality.stability += 6;
        }
        if seen_pairs.contains(&ordered_pair(RuneCategory::Shape, RuneCategory::Trigger)) {
            quality.score += 10;
            quality.stability += 6;
        }
        if seen_pairs.contains(&ordered_pair(RuneCategory::Effect, RuneCategory::Modifier)) {
            quality.score += 5;
            quality.safety += 6;
        }
        if self.board.placed.len() >= 3 && self.board.links.len() >= self.board.placed.len() {
            quality.score += 8;
            quality.stability += 8;
            quality.cost += 4;
        }

        quality
    }

    fn record_discovery(
        &mut self,
        data: &GameData,
        result: &EnchantResult,
    ) -> Option<DiscoveryReward> {
        if result.grade == EnchantGrade::Failed {
            return None;
        }
        let signature = self.signature(data)?;
        if let Some(existing) = self
            .discoveries
            .iter_mut()
            .find(|recipe| recipe.signature == signature)
        {
            existing.uses += 1;
            existing.best_score = existing.best_score.max(result.score);
            return None;
        }

        let name = result.title.clone();
        self.discoveries.push(DiscoveredRecipe {
            signature,
            name: name.clone(),
            uses: 1,
            best_score: result.score,
        });
        Some(DiscoveryReward {
            name,
            insight: DISCOVERY_INSIGHT,
        })
    }

    fn signature(&self, data: &GameData) -> Option<String> {
        let mut pieces = Vec::new();
        for category in RuneCategory::ALL {
            let mut names: Vec<&str> = self
                .board
                .placed
                .iter()
                .filter_map(|placed| data.rune(&placed.rune_id))
                .filter(|rune| rune.category == category)
                .map(|rune| rune.name.as_str())
                .collect();
            names.sort_unstable();
            if names.is_empty() && category != RuneCategory::Modifier {
                return None;
            }
            pieces.extend(names);
        }
        Some(pieces.join(" + "))
    }
}

#[derive(Debug, Default)]
struct LinkQuality {
    score: i32,
    power: i32,
    stability: i32,
    cost: i32,
    safety: i32,
}

#[derive(Debug, Clone, Copy)]
struct PlacedRuneRef<'a> {
    rune: &'a RuneDef,
    quality: f32,
}

fn ordered_pair(a: RuneCategory, b: RuneCategory) -> (RuneCategory, RuneCategory) {
    if category_rank(a) <= category_rank(b) {
        (a, b)
    } else {
        (b, a)
    }
}

fn category_rank(category: RuneCategory) -> u8 {
    match category {
        RuneCategory::Effect => 0,
        RuneCategory::Shape => 1,
        RuneCategory::Trigger => 2,
        RuneCategory::Modifier => 3,
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
