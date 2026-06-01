//! Runtime state, save data, and enchantment evaluation.

use crate::data::{CommissionDef, GameConfig, GameData, RuneCategory, RuneDef};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub const BOARD_COLUMNS: usize = 5;
pub const BOARD_ROWS: usize = 4;
pub const BOARD_NODE_COUNT: usize = BOARD_COLUMNS * BOARD_ROWS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamePhase {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedRune {
    pub rune_id: String,
    pub node: usize,
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

    pub fn contains(self, node: usize) -> bool {
        self.a == node || self.b == node
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignBoard {
    pub placed: Vec<PlacedRune>,
    pub links: Vec<Link>,
    pub selected_rune: Option<String>,
    pub link_anchor: Option<usize>,
    pub last_evaluation: Option<EnchantResult>,
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
}

#[derive(Debug, Clone)]
pub struct CraftReport {
    pub result: EnchantResult,
    pub discovery: Option<String>,
    pub reward: i64,
    pub reputation: i64,
    pub insight: i64,
}

#[derive(Debug, Clone)]
pub struct GameSession {
    pub phase: GamePhase,
    pub player: PlayerState,
    pub board: DesignBoard,
    pub discoveries: Vec<DiscoveredRecipe>,
}

impl GameSession {
    pub fn new(config: &GameConfig) -> Self {
        Self {
            phase: GamePhase::Naming,
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
            },
            board: DesignBoard::new(),
            discoveries: Vec::new(),
        }
    }

    pub fn from_save(save: SaveData) -> Self {
        Self {
            phase: save.phase,
            player: save.player,
            board: save.board,
            discoveries: save.discoveries,
        }
    }

    pub fn to_save(&self, version: &str) -> SaveData {
        SaveData {
            version: version.to_owned(),
            phase: self.phase,
            player: self.player.clone(),
            board: self.board.clone(),
            discoveries: self.discoveries.clone(),
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

    pub fn current_commission<'a>(&self, data: &'a GameData) -> &'a CommissionDef {
        data.commission(self.player.current_commission)
    }

    pub fn can_use_rune(&self, rune: &RuneDef) -> bool {
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
        self.board.link_anchor = None;
        Ok(())
    }

    pub fn select_link_tool(&mut self) {
        self.board.selected_rune = None;
        self.board.link_anchor = None;
    }

    pub fn use_board_node(&mut self, node: usize, data: &GameData) -> Result<String, String> {
        if node >= BOARD_NODE_COUNT {
            return Err("That mark is outside the ledger page.".to_owned());
        }

        if let Some(rune_id) = self.board.selected_rune.clone() {
            let rune = data
                .rune(&rune_id)
                .ok_or_else(|| format!("Unknown rune: {rune_id}"))?;
            if !self.can_use_rune(rune) {
                return Err(format!("{} is still locked.", rune.name));
            }
            self.spend_focus(2.0)?;
            self.board.place(node, &rune_id);
            Ok(format!("Inked {} at node {}", rune.name, node + 1))
        } else {
            self.link_from_node(node, data)
        }
    }

    pub fn erase_node(&mut self, node: usize) -> Result<String, String> {
        if self.board.remove(node) {
            self.spend_focus(1.0)?;
            Ok(format!("Scraped node {} clean", node + 1))
        } else {
            Err("There is no rune on that node.".to_owned())
        }
    }

    pub fn clear_board(&mut self) {
        self.board.placed.clear();
        self.board.links.clear();
        self.board.link_anchor = None;
        self.board.last_evaluation = None;
    }

    pub fn test_design(&mut self, data: &GameData) -> CraftReport {
        let result = self.evaluate(data);
        let discovery = self.record_discovery(data, &result);
        self.board.last_evaluation = Some(result.clone());
        CraftReport {
            result,
            discovery,
            reward: 0,
            reputation: 0,
            insight: 0,
        }
    }

    pub fn deliver_design(&mut self, data: &GameData) -> CraftReport {
        let commission = self.current_commission(data).clone();
        let result = self.evaluate(data);
        let discovery = self.record_discovery(data, &result);
        let mut reward = 0;
        let mut reputation = 0;
        let mut insight = 1;

        match result.grade {
            EnchantGrade::Brilliant => {
                reward = commission.reward + (commission.reward / 3);
                reputation = commission.reputation + 2;
                insight = commission.insight + 2;
            }
            EnchantGrade::Reliable => {
                reward = commission.reward;
                reputation = commission.reputation;
                insight = commission.insight;
            }
            EnchantGrade::Unstable => {
                reward = commission.reward / 2;
                reputation = commission.reputation.saturating_sub(2);
                insight = (commission.insight / 2).max(1);
            }
            EnchantGrade::Failed => {
                reputation = -2;
            }
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
        self.player.current_commission =
            (self.player.current_commission + 1) % data.commissions.len().max(1);
        self.board.last_evaluation = Some(result.clone());
        self.clear_board();

        CraftReport {
            result,
            discovery,
            reward,
            reputation,
            insight,
        }
    }

    pub fn skip_commission(&mut self, data: &GameData) {
        self.player.day += 1;
        self.player.reputation -= 1;
        self.player.current_commission =
            (self.player.current_commission + 1) % data.commissions.len().max(1);
        self.clear_board();
    }

    pub fn research(&mut self) -> Result<String, String> {
        if self.player.workshop_rank >= 4 {
            return Err("The forbidden shelf is already open.".to_owned());
        }
        let next_rank = self.player.workshop_rank + 1;
        let coin_cost = 24 + next_rank as i64 * 16;
        let insight_cost = 6 + next_rank as i64 * 5;
        if self.player.coins < coin_cost || self.player.insight < insight_cost {
            return Err(format!(
                "Research needs {} coins and {} insight.",
                coin_cost, insight_cost
            ));
        }

        self.player.coins -= coin_cost;
        self.player.insight -= insight_cost;
        self.player.workshop_rank = next_rank;
        self.player.day += 1;
        Ok(match next_rank {
            2 => "Unlocked volatile adventurer runes: Fire, Wind, Frost, Force, Burst, Impact."
                .to_owned(),
            3 => "Unlocked refined trade runes: Growth, Sound, Healing, Water, Dawn, Hidden."
                .to_owned(),
            _ => "Unlocked forbidden theory: Gravity, Teleportation, Summoning, Time.".to_owned(),
        })
    }

    fn spend_focus(&mut self, amount: f32) -> Result<(), String> {
        if self.player.focus + f32::EPSILON < amount {
            return Err("Not enough focus to ink more lines.".to_owned());
        }
        self.player.focus -= amount;
        Ok(())
    }

    fn link_from_node(&mut self, node: usize, data: &GameData) -> Result<String, String> {
        let Some(first_rune) = self.board.rune_at(node) else {
            return Err("Linking starts from an inked rune.".to_owned());
        };
        if let Some(anchor) = self.board.link_anchor {
            if anchor == node {
                self.board.link_anchor = None;
                return Ok("Lifted the quill.".to_owned());
            }
            if self.board.rune_at(anchor).is_none() {
                self.board.link_anchor = Some(node);
                return Ok("Started a new link.".to_owned());
            }
            self.spend_focus(1.5)?;
            let link = Link::new(anchor, node);
            if !self.board.links.contains(&link) {
                self.board.links.push(link);
            }
            self.board.link_anchor = Some(node);
            let from = data.rune_name(first_rune);
            let to = data.rune_name(self.board.rune_at(anchor).unwrap_or(first_rune));
            Ok(format!("Linked {} with {}", to, from))
        } else {
            self.board.link_anchor = Some(node);
            Ok(format!("Anchored link at {}", data.rune_name(first_rune)))
        }
    }

    fn evaluate(&self, data: &GameData) -> EnchantResult {
        let commission = self.current_commission(data);
        let placed = self.placed_runes(data);
        let mut by_category: HashMap<RuneCategory, Vec<&RuneDef>> = HashMap::new();
        for rune in &placed {
            by_category.entry(rune.category).or_default().push(*rune);
        }

        let required = [
            commission.required_effect.as_str(),
            commission.required_shape.as_str(),
            commission.required_trigger.as_str(),
        ];
        let required_hits = required
            .iter()
            .filter(|id| placed.iter().any(|rune| rune.id == **id))
            .count();
        let optional_hit = commission
            .optional_modifier
            .as_ref()
            .is_some_and(|id| placed.iter().any(|rune| rune.id == *id));
        let has_core = [RuneCategory::Effect, RuneCategory::Shape, RuneCategory::Trigger]
            .iter()
            .all(|category| by_category.contains_key(category));
        let matched_request = required_hits == required.len();

        let mut power = placed.iter().map(|rune| rune.power).sum::<i32>();
        let mut stability = 48 + placed.iter().map(|rune| rune.stability).sum::<i32>();
        let mut mana_cost = placed.iter().map(|rune| rune.mana_cost).sum::<i32>();
        let mut safety = 35 + placed.iter().map(|rune| rune.safety).sum::<i32>();
        let mut score = required_hits as i32 * 24;

        if optional_hit {
            score += 10;
        }
        if has_core {
            score += 12;
        }

        let link_bonus = self.link_quality(data);
        power += link_bonus.power;
        stability += link_bonus.stability;
        mana_cost += link_bonus.cost;
        safety += link_bonus.safety;
        score += link_bonus.score;

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
        score = score.clamp(0, 120);

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

        let title = self.result_title(data, commission, &by_category, grade);
        let side_effect = self.side_effect(data, commission, grade, matched_request, accident);

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

    fn placed_runes<'a>(&'a self, data: &'a GameData) -> Vec<&'a RuneDef> {
        self.board
            .placed
            .iter()
            .filter_map(|placed| data.rune(&placed.rune_id))
            .collect()
    }

    fn link_quality(&self, data: &GameData) -> LinkQuality {
        let mut quality = LinkQuality::default();
        let mut seen_pairs = HashSet::new();

        for link in &self.board.links {
            let (Some(a), Some(b)) = (self.board.rune_at(link.a), self.board.rune_at(link.b)) else {
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

    fn result_title(
        &self,
        data: &GameData,
        commission: &CommissionDef,
        by_category: &HashMap<RuneCategory, Vec<&RuneDef>>,
        grade: EnchantGrade,
    ) -> String {
        let effect = by_category
            .get(&RuneCategory::Effect)
            .and_then(|runes| runes.first())
            .map(|rune| rune.name.as_str())
            .unwrap_or("Uncertain");
        let modifier = by_category
            .get(&RuneCategory::Modifier)
            .and_then(|runes| runes.first())
            .map(|rune| rune.name.as_str());

        match grade {
            EnchantGrade::Brilliant => match modifier {
                Some(modifier) => format!(
                    "{} of {} {}",
                    title_case(&commission.item),
                    modifier,
                    effect
                ),
                None => format!("Perfect {} of {}", title_case(&commission.item), effect),
            },
            EnchantGrade::Reliable => format!("{} of {}", title_case(&commission.item), effect),
            EnchantGrade::Unstable => format!("Volatile {} of {}", commission.item, effect),
            EnchantGrade::Failed => {
                let required = data.rune_name(&commission.required_effect);
                format!("{} of Mild {}", title_case(&commission.item), required)
            }
        }
    }

    fn side_effect(
        &self,
        data: &GameData,
        commission: &CommissionDef,
        grade: EnchantGrade,
        matched_request: bool,
        accident: bool,
    ) -> String {
        if !matched_request {
            return format!(
                "The diagram misses part of the request for {} + {} + {}.",
                data.rune_name(&commission.required_effect),
                data.rune_name(&commission.required_shape),
                data.rune_name(&commission.required_trigger)
            );
        }
        if accident {
            return "The test bench coughs up smoke and the ink keeps glowing after you lift the quill."
                .to_owned();
        }
        match grade {
            EnchantGrade::Brilliant => "Clean output, tidy mana draw, and a margin note worth publishing.".to_owned(),
            EnchantGrade::Reliable => "The enchantment works as commissioned and should hold under normal use.".to_owned(),
            EnchantGrade::Unstable => "It works, but the margins hiss whenever someone says the activation phrase.".to_owned(),
            EnchantGrade::Failed => "The result is more conversation piece than enchantment.".to_owned(),
        }
    }

    fn record_discovery(&mut self, data: &GameData, result: &EnchantResult) -> Option<String> {
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
        Some(name)
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

impl DesignBoard {
    fn new() -> Self {
        Self {
            placed: Vec::new(),
            links: Vec::new(),
            selected_rune: Some("light".to_owned()),
            link_anchor: None,
            last_evaluation: None,
        }
    }

    pub fn rune_at(&self, node: usize) -> Option<&str> {
        self.placed
            .iter()
            .find(|placed| placed.node == node)
            .map(|placed| placed.rune_id.as_str())
    }

    fn place(&mut self, node: usize, rune_id: &str) {
        if let Some(existing) = self.placed.iter_mut().find(|placed| placed.node == node) {
            existing.rune_id = rune_id.to_owned();
        } else {
            self.placed.push(PlacedRune {
                rune_id: rune_id.to_owned(),
                node,
            });
        }
        self.links
            .retain(|link| self.placed.iter().any(|r| r.node == link.a) && self.placed.iter().any(|r| r.node == link.b));
    }

    fn remove(&mut self, node: usize) -> bool {
        let before = self.placed.len();
        self.placed.retain(|placed| placed.node != node);
        self.links.retain(|link| !link.contains(node));
        if self.link_anchor == Some(node) {
            self.link_anchor = None;
        }
        before != self.placed.len()
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

pub fn node_grid(node: usize) -> (i32, i32) {
    ((node % BOARD_COLUMNS) as i32, (node / BOARD_COLUMNS) as i32)
}

pub fn node_distance(a: usize, b: usize) -> i32 {
    let (ax, ay) = node_grid(a);
    let (bx, by) = node_grid(b);
    (ax - bx).abs() + (ay - by).abs()
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

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[derive(Debug, Deserialize)]
struct LegacySave {
    points: Option<i64>,
    energy: Option<f32>,
    turn: Option<u32>,
}

pub fn migrate_save_value(
    detected_version: Option<String>,
    value: Value,
    config: &GameConfig,
) -> Result<SaveData, String> {
    let payload = value.get("data").cloned().unwrap_or(value);

    if let Ok(mut current) = serde_json::from_value::<SaveData>(payload.clone()) {
        current.version = config.version.clone();
        return Ok(current);
    }

    let legacy: LegacySave = serde_json::from_value(payload)
        .map_err(|err| format!("Unsupported save format {:?}: {}", detected_version, err))?;

    let mut session = GameSession::new(config);
    session.phase = GamePhase::Playing;
    if let Some(points) = legacy.points {
        session.player.coins = points;
    }
    if let Some(energy) = legacy.energy {
        session.player.focus = energy.clamp(0.0, config.max_focus);
    }
    if let Some(turn) = legacy.turn {
        session.player.day = turn.max(1);
    }

    Ok(session.to_save(&config.version))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;

    fn data() -> GameData {
        GameData::load().unwrap()
    }

    #[test]
    fn matching_design_scores_as_working_enchantment() {
        let data = data();
        let mut session = GameSession::new(&data.config);
        session.start_playing();
        session.select_rune("light", &data).unwrap();
        session.use_board_node(6, &data).unwrap();
        session.select_rune("sphere", &data).unwrap();
        session.use_board_node(7, &data).unwrap();
        session.select_rune("continuous", &data).unwrap();
        session.use_board_node(8, &data).unwrap();
        session.select_link_tool();
        session.use_board_node(6, &data).unwrap();
        session.use_board_node(7, &data).unwrap();
        session.use_board_node(8, &data).unwrap();

        let report = session.test_design(&data);

        assert!(report.result.matched_request);
        assert_ne!(report.result.grade, EnchantGrade::Failed);
        assert_eq!(session.discoveries.len(), 1);
    }

    #[test]
    fn legacy_save_migrates_to_current_shape() {
        let data = data();
        let value = serde_json::json!({
            "points": 42,
            "energy": 99.0,
            "turn": 3
        });

        let migrated =
            migrate_save_value(Some("0.1.0".to_owned()), value, &data.config).unwrap();

        assert_eq!(migrated.version, "1.0.0");
        assert_eq!(migrated.player.coins, 42);
        assert_eq!(migrated.player.focus, 99.0);
        assert_eq!(migrated.player.day, 3);
    }
}
