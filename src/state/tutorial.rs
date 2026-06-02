use super::GameSession;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TutorialStage {
    FirstEffect,
    FirstShape,
    FirstTrigger,
    ResearchHint,
    Complete,
}

impl Default for TutorialStage {
    fn default() -> Self {
        Self::Complete
    }
}

impl TutorialStage {
    pub fn new_game() -> Self {
        Self::FirstEffect
    }

    pub fn prompt(self) -> Option<&'static str> {
        match self {
            Self::FirstEffect => Some(
                "Tutorial: the Light effect is open. Draw a circle with Light inside it to unlock a Shape rune.",
            ),
            Self::FirstShape => Some(
                "Tutorial: Sphere is open. Read Light + Sphere together to unlock a Trigger rune.",
            ),
            Self::FirstTrigger => Some(
                "Tutorial: Continuous is open. Read Light + Sphere + Continuous to open the rank-one shelf.",
            ),
            Self::ResearchHint => Some(
                "Tutorial: rank-one runes are open. Deliver work for coins and insight; Research opens higher rune tiers.",
            ),
            Self::Complete => None,
        }
    }

    pub fn allowed_runes(self) -> Option<&'static [&'static str]> {
        match self {
            Self::FirstEffect => Some(&["light"]),
            Self::FirstShape => Some(&["light", "sphere"]),
            Self::FirstTrigger => Some(&["light", "sphere", "continuous"]),
            Self::ResearchHint => None,
            Self::Complete => None,
        }
    }
}

impl GameSession {
    pub fn tutorial_prompt(&self) -> Option<&'static str> {
        self.player.tutorial_stage.prompt()
    }

    pub fn advance_tutorial_after_interpret(&mut self) -> Option<String> {
        let diagram = self.board.last_diagram.as_ref()?;
        let has = |id: &str| diagram.runes.iter().any(|rune| rune.rune_id == id);
        match self.player.tutorial_stage {
            TutorialStage::FirstEffect if has("light") => {
                self.player.tutorial_stage = TutorialStage::FirstShape;
                self.add_journal_entry(
                    "Shape rune unlocked",
                    "Reading Light opened the Sphere shape rune in the guide.",
                );
                Some(
                    "Tutorial: Sphere unlocked. Shapes tell an effect what form to take."
                        .to_owned(),
                )
            }
            TutorialStage::FirstShape if has("light") && has("sphere") => {
                self.player.tutorial_stage = TutorialStage::FirstTrigger;
                self.add_journal_entry(
                    "Trigger rune unlocked",
                    "Reading Light with Sphere opened the Continuous trigger rune.",
                );
                Some(
                    "Tutorial: Continuous unlocked. Triggers tell a finished diagram when to act."
                        .to_owned(),
                )
            }
            TutorialStage::FirstTrigger if has("light") && has("sphere") && has("continuous") => {
                self.player.tutorial_stage = TutorialStage::ResearchHint;
                self.add_journal_entry(
                    "Rank-one shelf opened",
                    "The starter effect, shape, and trigger are complete. Deliver work for coins and insight; Research opens higher rune tiers.",
                );
                Some(
                    "Tutorial: rank-one runes are open. Deliver work, earn insight, then Research to unlock higher shelves."
                        .to_owned(),
                )
            }
            _ => None,
        }
    }

    pub fn complete_research_tutorial_hint(&mut self) {
        if self.player.tutorial_stage == TutorialStage::ResearchHint {
            self.player.tutorial_stage = TutorialStage::Complete;
            self.add_journal_entry(
                "Research noted",
                "Research spends coins and insight to unlock higher rune shelves.",
            );
        }
    }
}
