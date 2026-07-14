//! The slate itself: guide templates, ink lifecycle, and which acceptance
//! band `interpret_drawing` reads with.

use super::fixtures::*;
use crate::rune_drawing::StrokePoint;
use crate::state::{EnchantGrade, GameSession};

#[test]
fn interpret_keeps_slate_ink_and_guides_for_review() {
    let data = data();
    let mut session = unlocked_session(&data);
    session.select_rune("light", &data).unwrap();
    session
        .place_guide_template(StrokePoint::new(0.26, 0.50), &data)
        .unwrap();
    session.board.drawing_strokes = circled_diagram(&[
        ("light", 0.26, 0.50),
        ("sphere", 0.50, 0.50),
        ("continuous", 0.74, 0.50),
    ]);
    let ink_count = session.board.drawing_strokes.len();

    session.interpret_drawing(&data).unwrap();

    assert_eq!(session.board.drawing_strokes.len(), ink_count);
    assert_eq!(session.board.guide_templates.len(), 1);
    assert_eq!(session.board.placed.len(), 3);
    assert!(session.board.last_interpretation_note.is_some());
}

#[test]
fn guide_templates_do_not_score_as_inked_runes() {
    let data = data();
    let mut session = GameSession::new(&data.config);

    session.select_rune("light", &data).unwrap();
    session
        .place_guide_template(StrokePoint::new(0.5, 0.5), &data)
        .unwrap();

    let report = session.test_design(&data);

    assert_eq!(session.board.guide_templates.len(), 1);
    assert!(session.board.placed.is_empty());
    assert!(!report.result.matched_request);
    assert_eq!(report.result.grade, EnchantGrade::Failed);
}

#[test]
fn guide_templates_can_be_removed_individually() {
    let data = data();
    let mut session = GameSession::new(&data.config);

    session.select_rune("light", &data).unwrap();
    session
        .place_guide_template(StrokePoint::new(0.25, 0.50), &data)
        .unwrap();
    session.select_rune("light", &data).unwrap();
    session
        .place_guide_template(StrokePoint::new(0.75, 0.50), &data)
        .unwrap();
    session.board.drawing_strokes = template_at("light", 0.50, 0.50, 0.18);

    let message = session.remove_guide_template(0, &data).unwrap();

    assert!(message.contains("Removed Light guide"), "{message}");
    assert_eq!(session.board.guide_templates.len(), 1);
    assert_eq!(session.board.guide_templates[0].center.x, 0.75);
    assert!(!session.board.drawing_strokes.is_empty());
}

#[test]
fn guide_templates_can_be_moved_without_touching_ink() {
    let data = data();
    let mut session = unlocked_session(&data);
    session.select_rune("light", &data).unwrap();
    session
        .place_guide_template(StrokePoint::new(0.25, 0.50), &data)
        .unwrap();
    session.board.drawing_strokes = circled_diagram(&[
        ("light", 0.26, 0.50),
        ("sphere", 0.50, 0.50),
        ("continuous", 0.74, 0.50),
    ]);
    let ink_count = session.board.drawing_strokes.len();
    session.interpret_drawing(&data).unwrap();
    assert!(session.board.last_diagram.is_some());

    session
        .move_guide_template(0, StrokePoint::new(0.75, 0.25))
        .unwrap();

    assert_eq!(session.board.guide_templates[0].center.x, 0.75);
    assert_eq!(session.board.guide_templates[0].center.y, 0.25);
    assert_eq!(session.board.drawing_strokes.len(), ink_count);
    assert!(session.board.last_diagram.is_none());
    assert!(session.board.last_interpretation_note.is_none());
}

#[test]
fn deselect_rune_cancels_armed_template_placement() {
    let data = data();
    let mut session = GameSession::new(&data.config);

    session.select_rune("light", &data).unwrap();
    session.deselect_rune();

    assert!(session.board.selected_rune.is_none());
    assert!(!session.board.template_armed);
}

#[test]
fn starting_new_ink_clears_previous_interpretation_state() {
    let data = data();
    let mut session = unlocked_session(&data);
    session.board.drawing_strokes = circled_diagram(&[
        ("light", 0.26, 0.50),
        ("sphere", 0.50, 0.50),
        ("continuous", 0.74, 0.50),
    ]);
    session.interpret_drawing(&data).unwrap();
    assert!(session.board.last_diagram.is_some());
    assert!(session.board.last_interpretation_note.is_some());

    session.start_drawing_stroke(StrokePoint::new(0.15, 0.15));

    assert!(session.board.active_stroke.is_some());
    assert!(session.board.last_diagram.is_none());
    assert!(session.board.last_interpretation_note.is_none());
}

#[test]
fn circle_free_diagram_rejection_surfaces_the_specific_player_hint() {
    let data = data();
    let mut session = unlocked_session(&data);
    session.board.drawing_strokes = template_at("light", 0.50, 0.50, 0.18);

    let error = session.interpret_drawing(&data).unwrap_err();
    assert_eq!(
        error,
        "No closed shape reads as a circle yet — draw one continuous loop around your runes."
    );
}

#[test]
fn sandbox_mode_accepts_a_weak_circle_commission_mode_rejects() {
    // Plan Phase 5 item 1 exit criterion: the same weak circle (quality ~0.30 — below
    // Commission's 0.32 floor but above Sandbox's 0.24 one) is rejected in ordinary commission
    // work and accepted in Sandbox, with no change to the recognizer itself — only which
    // acceptance band `interpret_drawing` reads with (`GameSession::recognition_context`).
    let data = data();
    let strokes = weak_partial_circle();

    let mut commission = unlocked_session(&data);
    commission.board.drawing_strokes = strokes.clone();
    let _ = commission.interpret_drawing(&data); // expected to Err: circle too weak to accept
    assert!(
        !commission.board.last_diagram.as_ref().unwrap().circle_found,
        "{:?}",
        commission.board.last_diagram
    );

    let mut sandbox = unlocked_session(&data);
    sandbox.set_sandbox_mode(true);
    sandbox.board.drawing_strokes = strokes;
    let _ = sandbox.interpret_drawing(&data); // still Err (no rune ink drawn), circle is what's under test
    assert!(
        sandbox.board.last_diagram.as_ref().unwrap().circle_found,
        "{:?}",
        sandbox.board.last_diagram
    );
}
