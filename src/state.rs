//! Runtime state, save data, and enchantment evaluation.

use crate::data::{GameConfig, GameData, RuneDef};
use crate::rune_diagram::{interpret_diagram_in_context, DiagramInterpretation};
use crate::rune_drawing::{canonicalize_stroke, erase_strokes_at, DrawnStroke, StrokePoint};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
pub use work::{
    DiscoveryReward, JournalEntry, WorkOrderKind, DISCOVERY_INSIGHT, GUIDE_FREE_INSIGHT,
};

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
    /// Per-rune skill history (plan Phase 5 item 2) — how many times a rune has been read
    /// (accepted) and how well, on average. Fades the guide-template opacity for that rune in
    /// the drawing slate (`ui/drawing.rs`) and never resets, so it's a durable measure of
    /// practice rather than a per-session streak.
    #[serde(default)]
    pub rune_mastery: HashMap<String, RuneMastery>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct RuneMastery {
    pub accepted_count: u32,
    pub quality_sum: f32,
}

impl RuneMastery {
    fn record(&mut self, quality: f32) {
        self.accepted_count += 1;
        self.quality_sum += quality.clamp(0.0, 1.0);
    }

    /// accepted count × mean quality (the plan's own formula) — a rune read once at perfect
    /// quality and a rune read many times at so-so quality can land at a similar score; both
    /// represent real, earned familiarity, not just raw repetition.
    pub fn score(&self) -> f32 {
        if self.accepted_count == 0 {
            0.0
        } else {
            self.accepted_count as f32 * (self.quality_sum / self.accepted_count as f32)
        }
    }
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
    /// Free-experimentation mode (plan Phase 5 item 1) — reuses the ordinary commission slate
    /// (`board`) rather than a separate screen: while active, `interpret_drawing` reads with
    /// `RecognitionContext::Sandbox` (the most forgiving acceptance band) and `evaluate()`
    /// treats the active commission's request as trivially matched, since there is nothing to
    /// deliver. Deliberately not persisted — like `board.active_stroke`, it's transient UI
    /// state, not save-worthy game state.
    pub sandbox_mode: bool,
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
                rune_mastery: HashMap::new(),
            },
            board: DesignBoard::new(),
            discoveries: Vec::new(),
            journal: Vec::new(),
            sandbox_mode: false,
        }
    }

    /// Toggles `sandbox_mode` on or off, clearing any interpretation feedback tied to the old
    /// context — a Commission-context "accepted" note left over from before flipping into
    /// Sandbox (or vice versa) is no longer meaningful.
    pub fn set_sandbox_mode(&mut self, enabled: bool) {
        self.sandbox_mode = enabled;
        self.board.clear_interpretation_feedback();
    }

    fn recognition_context(&self) -> crate::rune_drawing::RecognitionContext {
        if self.sandbox_mode {
            crate::rune_drawing::RecognitionContext::Sandbox
        } else {
            crate::rune_drawing::RecognitionContext::Commission
        }
    }

    pub fn from_save(save: SaveData) -> Self {
        Self {
            phase: save.phase,
            player: save.player,
            board: save.board,
            discoveries: save.discoveries,
            journal: save.journal,
            sandbox_mode: false,
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
        self.board.clear_interpretation_feedback();
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
        self.board.clear_interpretation_feedback();
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
        self.board.clear_interpretation_feedback();
        Ok(())
    }

    pub fn start_drawing_stroke(&mut self, point: StrokePoint) {
        self.board.clear_interpretation_feedback();
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
                self.board.drawing_strokes.push(canonicalize_stroke(stroke));
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
            self.board.clear_interpretation_feedback();
        }
        erased
    }

    pub fn interpret_drawing(&mut self, data: &GameData) -> Result<String, String> {
        if self.board.drawing_strokes.is_empty() {
            return Err("The diagram slate is blank.".to_owned());
        }

        let unlocked = data.runes.iter().filter(|rune| self.can_use_rune(rune));
        let interpretation = interpret_diagram_in_context(
            &self.board.drawing_strokes,
            unlocked,
            self.recognition_context(),
        );
        self.board.last_diagram = Some(interpretation.clone());

        self.ensure_interpretable(data, &interpretation)?;
        self.spend_focus(1.5 + interpretation.runes.len() as f32 * 0.7)?;
        let guide_free = self.board.guide_templates.is_empty();
        self.apply_interpreted_runes(&interpretation);

        let mut note = self.interpretation_note(data, &interpretation);
        if let Some(hint) = self.player_hint(data) {
            note.push_str(&format!(" {hint}"));
        }
        if guide_free {
            self.player.insight += GUIDE_FREE_INSIGHT;
            note.push_str(&format!(
                " Drawn guide-free: +{GUIDE_FREE_INSIGHT} insight."
            ));
            self.add_journal_entry(
                "Guide-free",
                format!(
                    "Read a diagram with no guide templates armed. +{GUIDE_FREE_INSIGHT} insight."
                ),
            );
        }
        self.board.last_interpretation_note = Some(note.clone());
        Ok(note)
    }

    pub fn clear_board(&mut self) {
        self.board.clear_marks();
        self.board.clear_drawing();
        self.board.guide_templates.clear();
        self.board.clear_interpretation_feedback();
        self.board.last_evaluation = None;
    }

    fn ensure_interpretable(
        &mut self,
        data: &GameData,
        interpretation: &DiagramInterpretation,
    ) -> Result<(), String> {
        if !interpretation.circle_found {
            let message = self.player_hint(data).unwrap_or_else(|| {
                "The diagram needs an enclosing circle before it can hold meaning.".to_owned()
            });
            return self
                .reject_interpretation("No enclosing circle was readable.".to_owned(), message);
        }
        if !interpretation.accepted() {
            let circle_percent = percent(interpretation.circle_quality);
            let message = self.player_hint(data).unwrap_or_else(|| {
                format!("The circle reads at {circle_percent}%, but no inner rune is clear enough.")
            });
            return self.reject_interpretation(
                format!("Circle {circle_percent}%, but no inner rune was clear enough."),
                message,
            );
        }
        Ok(())
    }

    /// Plan Phase 5 item 3: picks the single most relevant reason a diagram read poorly (or
    /// could read better), reusing the same typed recognition calls the dev-only
    /// `diagnose_diagram` clipboard tool already makes. `None` means the diagram looks clean —
    /// callers fall back to their existing generic wording in that case.
    fn player_hint(&self, data: &GameData) -> Option<String> {
        let unlocked = data.runes.iter().filter(|rune| self.can_use_rune(rune));
        crate::rune_diagnostics::player_hint(&self.board.drawing_strokes, unlocked)
    }

    fn reject_interpretation(&mut self, note: String, error: String) -> Result<(), String> {
        self.board.last_interpretation_note = Some(note);
        self.spend_focus(0.8)?;
        Err(error)
    }

    /// Records one accepted read of `rune_id` at `quality` toward its mastery history (plan
    /// Phase 5 item 2). Shared by `apply_interpreted_runes` (commission slate, Sandbox) and the
    /// Practice slate's own scoring path (`game.rs::score_practice`) — both read with the same
    /// recognizer (item 1), so both should build the same skill history.
    pub fn record_rune_mastery(&mut self, rune_id: &str, quality: f32) {
        self.player
            .rune_mastery
            .entry(rune_id.to_owned())
            .or_default()
            .record(quality);
    }

    fn apply_interpreted_runes(&mut self, interpretation: &DiagramInterpretation) {
        let mut occupied = HashSet::new();
        let mut placed_nodes = Vec::new();
        self.board.clear_marks();
        for rune in &interpretation.runes {
            let node = node_for_diagram_center(rune.center, &occupied);
            occupied.insert(node);
            placed_nodes.push(node);
            self.board
                .place(node, &rune.rune_id, rune.quality, rune.potency);
            // Mastery (plan Phase 5 item 2): every accepted read counts, not just delivered
            // commissions — Sandbox builds the same skill history as commissioned work, since
            // both read with the same recognizer, just different acceptance bands (item 1).
            self.record_rune_mastery(&rune.rune_id, rune.quality);
        }
        for nodes in placed_nodes.windows(2) {
            let link = Link::new(nodes[0], nodes[1]);
            if !self.board.links.contains(&link) {
                self.board.links.push(link);
            }
        }
        self.board.active_stroke = None;
        self.board.last_recognition = None;
    }

    fn interpretation_note(
        &self,
        data: &GameData,
        interpretation: &DiagramInterpretation,
    ) -> String {
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
            .collect::<HashSet<_>>();
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
        format!(
            "Recognized runes: {} (circle {}%, rune quality {}%{}).{}{}",
            names,
            percent(interpretation.circle_quality),
            percent(interpretation.average_rune_quality()),
            rejected,
            commission_fit,
            circle_spell
        )
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
