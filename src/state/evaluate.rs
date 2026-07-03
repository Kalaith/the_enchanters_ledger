use super::{
    node_distance, text, CraftReport, DiscoveredRecipe, DiscoveryReward, EnchantGrade,
    EnchantResult, GameSession, WorkOrderKind, DISCOVERY_INSIGHT,
};
use crate::data::{GameData, RuneCategory, RuneDef};
use crate::recipes::match_recipe;
use crate::rune_diagram::ScopeSpell;
use std::collections::{HashMap, HashSet};

impl GameSession {
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
            ("effect", commission.required_effect.as_str()),
            ("shape", commission.required_shape.as_str()),
            ("trigger", commission.required_trigger.as_str()),
        ];
        let required_hits = required
            .iter()
            .filter(|(_, id)| placed.iter().any(|placed| placed.rune.id == *id))
            .count();
        // First missing piece, by category — feeds `text::backfire_cause` (plan Phase 4 item 4)
        // a specific "which rule was broken" instead of the generic "misses part of the
        // request" message every unmatched commission used to get.
        let missing_requirement = required
            .iter()
            .find(|(_, id)| !placed.iter().any(|placed| placed.rune.id == *id))
            .map(|(kind, id)| (*kind, *id));
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
        // Sandbox (plan Phase 5 item 1) is free experimentation, not commissioned work — there
        // is no specific request to match, so `evaluate()`'s request-matching gate is trivially
        // satisfied rather than forcing every sandbox diagram to `Failed`.
        let matched_request = self.sandbox_mode || required_hits == required.len();
        let average_quality = if placed.is_empty() {
            0.0
        } else {
            placed.iter().map(|placed| placed.quality).sum::<f32>() / placed.len() as f32
        };
        let weak_marks = placed.iter().filter(|placed| placed.quality < 0.58).count() as i32;
        let circle_quality = self
            .board
            .last_diagram
            .as_ref()
            .map_or(0.0, |diagram| diagram.circle_quality);
        let circle_spell = self
            .board
            .last_diagram
            .as_ref()
            .and_then(|diagram| diagram.spell.as_ref());
        let scope_spell = self
            .board
            .last_diagram
            .as_ref()
            .and_then(|diagram| diagram.scope_spell.as_ref());
        // Recipes as data (plan Phase 4 item 3): the best-matching named spell, evaluated as a
        // predicate over the scope tree — see `crate::recipes::match_recipe` and
        // `assets/data/recipes.json`. Takes priority over `circle_spell`'s generic tier-based
        // name (§5.7 of prd.md) when present.
        let matched_recipe = scope_spell.and_then(|tree| match_recipe(tree, &data.recipes));

        // Magnitude channel (plan Phase 2): potency scales power and mana
        // cost per rune, on top of (not folded into) quality's existing
        // effect — a bigger, fully-traced effect rune costs more and hits
        // harder than the same rune drawn small, independent of how
        // precisely its shape was drawn.
        let mut power = placed
            .iter()
            .map(|placed| {
                (placed.rune.power as f32 * (0.35 + placed.quality * 0.65) * placed.potency).round()
                    as i32
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
                (placed.rune.mana_cost as f32 * placed.potency).round() as i32
                    + ((1.0 - placed.quality).max(0.0) * 12.0).round() as i32
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

        // D2: circle quality used to be multiplied into *every* placed
        // rune's stored quality (so its effect scaled with rune count and
        // was invisible as its own term). It now contributes once,
        // additively, around a "decent circle" baseline of 0.55.
        stability += ((circle_quality - 0.55) * 20.0).round() as i32;
        safety += ((circle_quality - 0.55) * 14.0).round() as i32;
        score += ((circle_quality - 0.55) * 10.0).round() as i32;

        // Containment budget v2 (plan Phase 4 item 2): potency must be covered by *its own
        // scope's* containment, not just a single whole-diagram total — a vent's fire potency is
        // checked against that vent's own rings/perimeter/safer-rune containment, separately
        // from the root circle's. `total_potency_excess` walks the whole scope tree; still a
        // first cut (see prd.md), not a fully balanced version.
        let total_excess =
            scope_spell.map_or(0.0, |tree| total_potency_excess(tree, circle_quality));
        stability -= (total_excess * 10.0).round() as i32;

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
            // Recipes as data (item 3) take priority over `circle_spell`'s generic tier-based
            // name — a matched recipe is always more specific than "grand enough to get a name".
            matched_recipe
                .map(|recipe| recipe.name.clone())
                .or_else(|| {
                    circle_spell
                        .filter(|spell| spell.tier_rank >= 3)
                        .map(|spell| spell.name.clone())
                })
                .unwrap_or(base_title)
        } else {
            base_title
        };

        let duplicate_effect_count = by_category
            .get(&RuneCategory::Effect)
            .map_or(0, |runes| runes.len().saturating_sub(1));
        let side_effect = text::backfire_cause(
            data,
            accident,
            missing_requirement,
            total_excess,
            duplicate_effect_count,
        )
        .unwrap_or_else(|| {
            let mut generic = text::side_effect(
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
                    generic = format!(
                        "{} The {} flares through {} ring(s) and {} satellite seal(s).",
                        generic, spell.name, spell.ring_count, spell.satellite_count
                    );
                }
            }
            generic
        });

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
                    potency: placed.potency.clamp(0.0, 3.0),
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
        // A matched recipe (plan Phase 4 item 3) is a stable, semantic key — every "volcano" is
        // the same discovery regardless of minor potency/quality drift — so it takes priority
        // over the sorted-name-bag fallback below, which stays for arbitrary/unnamed combos.
        if let Some(recipe) = self
            .board
            .last_diagram
            .as_ref()
            .and_then(|diagram| diagram.scope_spell.as_ref())
            .and_then(|tree| match_recipe(tree, &data.recipes))
        {
            return Some(recipe.id.clone());
        }

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
    potency: f32,
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

/// Total stability-costing potency excess across the whole scope tree (plan Phase 4 item 2,
/// containment budget v2): the root scope's capacity mirrors Phase 2's original whole-circle
/// formula (baseline + drawn circle quality + structural containment); every sub-scope instead
/// gets its own, smaller budget from *its own* local structure/safer-rune containment only — a
/// vent's fire potency has to be covered by that vent's own rings and wards, not borrowed from
/// the crater's. Never negative: a scope with more containment than potency contributes nothing,
/// not a stability bonus.
fn total_potency_excess(tree: &ScopeSpell, circle_quality: f32) -> f32 {
    fn excess_at(tree: &ScopeSpell, circle_quality: f32, is_root: bool) -> f32 {
        let local_potency = tree.effects.iter().map(|(_, potency)| potency).sum::<f32>();
        let local_capacity = if is_root {
            2.0 + circle_quality * 6.0 + tree.containment * 4.0
        } else {
            1.0 + tree.containment * 3.0
        };
        let own_excess = (local_potency - local_capacity).max(0.0);
        own_excess
            + tree
                .sub_scopes
                .iter()
                .map(|sub| excess_at(sub, circle_quality, false))
                .sum::<f32>()
    }
    excess_at(tree, circle_quality, true)
}
