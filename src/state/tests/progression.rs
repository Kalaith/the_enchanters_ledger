//! Tutorial gating, rune mastery, insight rewards, and save migration.

use super::fixtures::*;
use crate::rune_drawing::StrokePoint;
use crate::state::{
    migrate_save_value, GameSession, TutorialStage, DISCOVERY_INSIGHT, GUIDE_FREE_INSIGHT,
};

#[test]
fn tutorial_starts_with_light_then_unlocks_shape_trigger_and_rank_one() {
    let data = data();
    let mut session = GameSession::new(&data.config);
    session.start_playing();

    assert_eq!(session.player.tutorial_stage, TutorialStage::FirstEffect);
    assert!(session.can_use_rune(data.rune("light").unwrap()));
    assert!(!session.can_use_rune(data.rune("sphere").unwrap()));
    assert!(!session.can_use_rune(data.rune("continuous").unwrap()));
    assert!(!session.can_use_rune(data.rune("warmth").unwrap()));

    session.board.drawing_strokes = circled_diagram(&[("light", 0.50, 0.50)]);
    session.interpret_drawing(&data).unwrap();
    let message = session.advance_tutorial_after_interpret().unwrap();
    assert!(message.contains("Sphere unlocked"), "{message}");
    assert_eq!(session.player.tutorial_stage, TutorialStage::FirstShape);
    assert!(session.can_use_rune(data.rune("sphere").unwrap()));
    assert!(!session.can_use_rune(data.rune("continuous").unwrap()));

    session.board.drawing_strokes =
        circled_diagram(&[("light", 0.32, 0.50), ("sphere", 0.58, 0.50)]);
    session.interpret_drawing(&data).unwrap();
    let message = session.advance_tutorial_after_interpret().unwrap();
    assert!(message.contains("Continuous unlocked"), "{message}");
    assert_eq!(session.player.tutorial_stage, TutorialStage::FirstTrigger);
    assert!(session.can_use_rune(data.rune("continuous").unwrap()));
    assert!(!session.can_use_rune(data.rune("warmth").unwrap()));

    session.board.drawing_strokes = circled_diagram(&[
        ("light", 0.26, 0.50),
        ("sphere", 0.50, 0.50),
        ("continuous", 0.74, 0.50),
    ]);
    session.interpret_drawing(&data).unwrap();
    let message = session.advance_tutorial_after_interpret().unwrap();
    assert!(message.contains("rank-one runes are open"), "{message}");
    assert_eq!(session.player.tutorial_stage, TutorialStage::ResearchHint);
    assert!(session.can_use_rune(data.rune("warmth").unwrap()));
    assert!(!session.can_use_rune(data.rune("fire").unwrap()));

    let _ = session.research(&data);
    assert_eq!(session.player.tutorial_stage, TutorialStage::Complete);
}

#[test]
fn accepted_reads_accumulate_rune_mastery() {
    // Plan Phase 5 item 2: "aids that fade with mastery" needs a real per-rune history first —
    // every accepted read (commission slate here; Practice is covered separately in game.rs's
    // score_practice) accumulates, it doesn't just remember the most recent draw.
    let data = data();
    let mut session = unlocked_session(&data);
    session.board.drawing_strokes = circled_diagram(&[
        ("light", 0.26, 0.50),
        ("sphere", 0.50, 0.50),
        ("continuous", 0.74, 0.50),
    ]);
    session.interpret_drawing(&data).unwrap();

    let light = session.player.rune_mastery.get("light").copied().unwrap();
    assert_eq!(light.accepted_count, 1, "{light:?}");
    assert!(light.score() > 0.0, "{light:?}");

    session.interpret_drawing(&data).unwrap();
    let light_again = session.player.rune_mastery.get("light").copied().unwrap();
    assert_eq!(light_again.accepted_count, 2, "{light_again:?}");
    assert!(
        light_again.score() > light.score(),
        "first={light:?} second={light_again:?}"
    );
}

#[test]
fn testing_new_recipe_awards_discovery_insight_once() {
    let data = data();
    let mut session = unlocked_session(&data);
    session.board.drawing_strokes = circled_diagram(&[
        ("light", 0.26, 0.50),
        ("sphere", 0.50, 0.50),
        ("continuous", 0.74, 0.50),
    ]);
    session.interpret_drawing(&data).unwrap();
    let insight_before = session.player.insight;

    let first = session.test_design(&data);
    let second = session.test_design(&data);

    assert_eq!(first.insight, DISCOVERY_INSIGHT);
    assert_eq!(second.insight, 0);
    assert_eq!(session.player.insight, insight_before + DISCOVERY_INSIGHT);
    assert!(first
        .notes
        .iter()
        .any(|note| note.contains("Discovery notes")));
}

#[test]
fn guide_free_interpretation_earns_insight_bonus_but_guided_does_not() {
    // Plan Phase 5 item 2: "eventually rewards guide-free drawing with an insight bonus."
    let data = data();
    let mut guided = unlocked_session(&data);
    guided
        .place_guide_template(StrokePoint::new(0.26, 0.50), &data)
        .unwrap();
    let insight_before_guided = guided.player.insight;
    guided.board.drawing_strokes = circled_diagram(&[
        ("light", 0.26, 0.50),
        ("sphere", 0.50, 0.50),
        ("continuous", 0.74, 0.50),
    ]);
    guided.interpret_drawing(&data).unwrap();
    assert_eq!(guided.player.insight, insight_before_guided, "{guided:?}");

    let mut free = unlocked_session(&data);
    let insight_before_free = free.player.insight;
    free.board.drawing_strokes = circled_diagram(&[
        ("light", 0.26, 0.50),
        ("sphere", 0.50, 0.50),
        ("continuous", 0.74, 0.50),
    ]);
    let note = free.interpret_drawing(&data).unwrap();
    assert_eq!(
        free.player.insight,
        insight_before_free + GUIDE_FREE_INSIGHT,
        "{free:?}"
    );
    assert!(note.contains("guide-free"), "{note}");
}

#[test]
fn legacy_save_migrates_to_current_shape() {
    let data = data();
    let value = serde_json::json!({
        "points": 42,
        "energy": 99.0,
        "turn": 3
    });

    let migrated = migrate_save_value(Some("0.1.0".to_owned()), value, &data.config).unwrap();

    assert_eq!(migrated.version, "1.0.0");
    assert_eq!(migrated.player.coins, 42);
    assert_eq!(migrated.player.focus, 99.0);
    assert_eq!(migrated.player.day, 3);
}
