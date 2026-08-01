//! Scenes for the headless screenshot harness.
//!
//! Each arm seeds the game into a state worth a picture — see
//! `docs/screenshot_capture_harness_guide.md` and `scripts/capture_ui.ps1`.
//! Kept out of `game.rs` because it is documentation tooling rather than
//! gameplay, and it grows a line every time something becomes worth showing.

use super::Game;
use crate::state::{GamePhase, TutorialStage};

pub(super) fn begin(game: &mut Game, scene: &str) {
    match scene {
        "title" => game.session.phase = GamePhase::Title,
        "naming" => game.session.phase = GamePhase::Naming,
        "manual" => {
            game.session.start_playing();
            game.session.player.tutorial_stage = TutorialStage::Complete;
            game.manual_open = true;
            // The newest practice-ladder rung, so the captured screenshot
            // always shows the most recently added diagram.
            game.manual_page = game
                .manual
                .iter()
                .rposition(|entry| entry.kind == "Practice Ladder")
                .unwrap_or(0);
        }
        // Two readings of the same four marks, for `.project/placement-rules.md`:
        // spread, then with Safer pulled in beside Light.
        "reading" | "reading_grouped" => {
            game.session.start_playing();
            game.session.player.tutorial_stage = TutorialStage::Complete;
            let commission = game.session.current_commission(&game.data).clone();
            let runes = crate::manual::notation_for(&commission, &game.data)
                .into_iter()
                .filter_map(|rune| game.data.rune(&rune.id))
                .collect::<Vec<_>>();
            let groups = if scene == "reading_grouped" {
                vec![vec![0, 3]]
            } else {
                Vec::new()
            };
            let diagram = crate::perfect_diagram::perfect_diagram_for(
                &crate::perfect_diagram::DiagramRequest {
                    runes,
                    groups,
                    ..Default::default()
                },
            );
            game.session.board.drawing_strokes = diagram.strokes();
            let _ = game.session.interpret_drawing(&game.data);
            game.show_interpretation_feedback();
        }
        "reference" => {
            game.session.start_playing();
            game.session.player.tutorial_stage = TutorialStage::Complete;
            if let Ok(msg) = game.session.place_reference_diagram(&game.data) {
                game.notifications.info(msg);
            }
        }
        _ => {
            // Default: jump straight into the workshop gameplay screen.
            game.session.start_playing();
        }
    }
}
