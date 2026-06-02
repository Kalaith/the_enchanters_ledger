use super::{GamePhase, GameSession};
use crate::data::{CommissionDef, GameData};
use serde::{Deserialize, Serialize};

pub const DISCOVERY_INSIGHT: i64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkOrderKind {
    #[default]
    Story,
    Talisman,
}

impl WorkOrderKind {
    pub fn panel_title(self) -> &'static str {
        match self {
            WorkOrderKind::Story => "Story Quest",
            WorkOrderKind::Talisman => "Day Talisman",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub day: u32,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct DiscoveryReward {
    pub name: String,
    pub insight: i64,
}

impl GameSession {
    pub fn current_commission<'a>(&self, data: &'a GameData) -> &'a CommissionDef {
        match self.player.active_work {
            WorkOrderKind::Story => data.commission(self.player.current_commission),
            WorkOrderKind::Talisman => data.talisman_job(self.player.current_talisman),
        }
    }

    pub fn story_commission<'a>(&self, data: &'a GameData) -> &'a CommissionDef {
        data.commission(self.player.current_commission)
    }

    pub fn active_work_kind(&self) -> WorkOrderKind {
        self.player.active_work
    }

    pub fn available_talisman_jobs<'a>(
        &self,
        data: &'a GameData,
    ) -> Vec<(usize, &'a CommissionDef)> {
        data.talisman_jobs
            .iter()
            .enumerate()
            .filter(|(_, job)| self.commission_is_unlocked(job, data))
            .collect()
    }

    pub fn select_story_work(&mut self, data: &GameData) -> Result<String, String> {
        let commission = self.story_commission(data);
        if !self.commission_is_unlocked(commission, data) {
            return Err(format!(
                "{} is filed in the journal until research unlocks: {}.",
                commission.customer,
                self.locked_requirement_names(commission, data).join(" + ")
            ));
        }

        let customer = commission.customer.clone();
        self.player.active_work = WorkOrderKind::Story;
        self.clear_board();
        self.add_journal_entry(
            "Story quest pinned",
            format!("Pinned {} as the active workshop order.", customer),
        );
        Ok(format!("Pinned story quest: {}", customer))
    }

    pub fn select_talisman_work(
        &mut self,
        index: usize,
        data: &GameData,
    ) -> Result<String, String> {
        let Some(job) = data.talisman_jobs.get(index) else {
            return Err("That talisman packet is not in the journal.".to_owned());
        };
        if !self.commission_is_unlocked(job, data) {
            return Err(format!(
                "{} still needs research: {}.",
                job.item,
                self.locked_requirement_names(job, data).join(" + ")
            ));
        }

        let item = job.item.clone();
        self.player.active_work = WorkOrderKind::Talisman;
        self.player.current_talisman = index;
        self.clear_board();
        self.add_journal_entry(
            "Day talisman pinned",
            format!("Pinned {} for routine coins and client notes.", item),
        );
        Ok(format!("Pinned day talisman: {}", item))
    }

    pub fn ensure_playable_work(&mut self, data: &GameData) -> Option<String> {
        if self.phase != GamePhase::Playing || self.player.active_work != WorkOrderKind::Story {
            return None;
        }
        let story = self.story_commission(data);
        if self.commission_is_unlocked(story, data) {
            return None;
        }

        let customer = story.customer.clone();
        let locked = self.locked_requirement_names(story, data).join(" + ");
        let Some(job_index) = self.first_available_talisman_index(data) else {
            return None;
        };
        let item = data.talisman_job(job_index).item.clone();
        self.player.active_work = WorkOrderKind::Talisman;
        self.player.current_talisman = job_index;
        self.clear_board();
        self.add_journal_entry(
            "Story quest filed",
            format!(
                "{} needs {}. Pinned {} for day work until research catches up.",
                customer, locked, item
            ),
        );
        Some(format!(
            "{} needs more research; {} is pinned for day work.",
            customer, item
        ))
    }

    pub fn skip_commission(&mut self, data: &GameData) {
        self.player.day += 1;
        if self.active_work_kind() == WorkOrderKind::Story {
            self.player.reputation -= 1;
            self.player.current_commission =
                (self.player.current_commission + 1) % data.commissions.len().max(1);
            self.add_journal_entry(
                "Story quest declined",
                "Moved the story ledger to the next commission. Reputation -1.",
            );
        } else {
            self.rotate_talisman_work(data);
            self.add_journal_entry(
                "Day work refreshed",
                "Pulled a different routine talisman packet from the journal.",
            );
        }
        self.clear_board();
    }

    pub fn research(&mut self, data: &GameData) -> Result<String, String> {
        self.complete_research_tutorial_hint();
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
        let message = match next_rank {
            2 => "Unlocked volatile adventurer runes: Fire, Wind, Frost, Force, Burst, Impact."
                .to_owned(),
            3 => "Unlocked refined trade runes: Growth, Sound, Healing, Water, Dawn, Hidden."
                .to_owned(),
            _ => "Unlocked forbidden theory: Gravity, Teleportation, Summoning, Time.".to_owned(),
        };
        self.add_journal_entry(
            "Research completed",
            format!(
                "Spent {} coins and {} insight. {}",
                coin_cost, insight_cost, message
            ),
        );
        if self.commission_is_unlocked(self.story_commission(data), data) {
            self.player.active_work = WorkOrderKind::Story;
            self.clear_board();
            self.add_journal_entry(
                "Story quest reopened",
                "Research now covers the active story notation; the story quest is pinned again.",
            );
            Ok(format!("{} Story quest pinned.", message))
        } else {
            Ok(message)
        }
    }

    pub(super) fn add_journal_entry(&mut self, title: impl Into<String>, body: impl Into<String>) {
        self.journal.push(JournalEntry {
            day: self.player.day,
            title: title.into(),
            body: body.into(),
        });
        if self.journal.len() > 12 {
            let excess = self.journal.len() - 12;
            self.journal.drain(0..excess);
        }
    }

    pub(super) fn commission_is_unlocked(
        &self,
        commission: &CommissionDef,
        data: &GameData,
    ) -> bool {
        [
            commission.required_effect.as_str(),
            commission.required_shape.as_str(),
            commission.required_trigger.as_str(),
        ]
        .into_iter()
        .all(|id| data.rune(id).is_some_and(|rune| self.can_use_rune(rune)))
    }

    pub(super) fn rotate_talisman_work(&mut self, data: &GameData) {
        if let Some(next) = self.next_available_talisman_index(data) {
            self.player.current_talisman = next;
            self.player.active_work = WorkOrderKind::Talisman;
        }
    }

    fn first_available_talisman_index(&self, data: &GameData) -> Option<usize> {
        data.talisman_jobs
            .iter()
            .position(|job| self.commission_is_unlocked(job, data))
    }

    fn next_available_talisman_index(&self, data: &GameData) -> Option<usize> {
        let available = self
            .available_talisman_jobs(data)
            .into_iter()
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if available.is_empty() {
            return None;
        }
        available
            .iter()
            .copied()
            .find(|index| *index > self.player.current_talisman)
            .or_else(|| available.first().copied())
    }

    fn locked_requirement_names(&self, commission: &CommissionDef, data: &GameData) -> Vec<String> {
        [
            commission.required_effect.as_str(),
            commission.required_shape.as_str(),
            commission.required_trigger.as_str(),
        ]
        .into_iter()
        .filter(|id| data.rune(id).is_some_and(|rune| !self.can_use_rune(rune)))
        .map(|id| data.rune_name(id).to_owned())
        .collect()
    }
}
