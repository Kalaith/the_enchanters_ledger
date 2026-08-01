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

/// Ink that traces exactly what the reference guides show — the circle guide's
/// own outline plus each rune guide at the position and size it is drawn at.
fn trace_the_guides(session: &GameSession) -> Vec<crate::rune_drawing::DrawnStroke> {
    use crate::perfect_diagram::{circle_points, RunePlacement};
    let circle = session.board.circle_guide.as_ref().unwrap();
    let mut ink = vec![crate::rune_drawing::DrawnStroke {
        points: circle_points(circle.center, circle.radius),
    }];
    for guide in &session.board.guide_templates {
        ink.extend(
            RunePlacement {
                rune_id: guide.rune_id.clone(),
                center: guide.center,
                scale: guide.scale,
            }
            .strokes(),
        );
    }
    ink
}

#[test]
fn tracing_the_reference_guides_reads_the_pinned_commission() {
    // The reference is only worth anything if tracing what it shows produces the
    // runes the commission asked for, so this goes through the guides the player
    // actually sees rather than regenerating the diagram.
    let data = data();
    let mut session = unlocked_session(&data);

    session.place_reference_diagram(&data).unwrap();
    assert!(session.board.circle_guide.is_some());
    // The reference lays out the order's whole notation, bonus modifier
    // included — the same page the manual shows for it.
    let wanted = crate::manual::notation_for(session.current_commission(&data), &data)
        .into_iter()
        .map(|rune| rune.id)
        .collect::<Vec<_>>();
    assert_eq!(
        session
            .board
            .guide_templates
            .iter()
            .map(|guide| guide.rune_id.clone())
            .collect::<Vec<_>>(),
        wanted
    );

    session.board.drawing_strokes = trace_the_guides(&session);
    session.interpret_drawing(&data).unwrap();

    let mut placed = placed_ids(&session);
    let mut expected = wanted;
    placed.sort();
    expected.sort();
    assert_eq!(placed, expected);
}

#[test]
fn the_reference_is_repeatable() {
    // Summoning it again mid-trace must not move anything under the player.
    let data = data();
    let mut session = unlocked_session(&data);

    session.place_reference_diagram(&data).unwrap();
    let first = session.board.guide_templates.clone();
    let circle = session.board.circle_guide.clone();
    session.place_reference_diagram(&data).unwrap();

    assert_eq!(session.board.guide_templates, first);
    assert_eq!(session.board.circle_guide, circle);
}

#[test]
fn the_reference_only_shows_notation_the_player_may_use() {
    // A fresh save is on the tutorial's first step, where only Light is open —
    // the reference must not hand over runes the guide itself would refuse to
    // place.
    let data = data();
    let mut session = GameSession::new(&data.config);
    session.start_playing();

    let note = session.place_reference_diagram(&data).unwrap();

    assert_eq!(
        session
            .board
            .guide_templates
            .iter()
            .map(|guide| guide.rune_id.as_str())
            .collect::<Vec<_>>(),
        vec!["light"]
    );
    assert!(note.contains("Still locked"), "{note}");
}

#[test]
fn sandbox_inks_the_reference_and_reads_it_back() {
    // Sandbox pays no delivery reward, so drawing the ideal outright is a way to
    // see a perfect read rather than a way to skip the work.
    let data = data();
    let mut session = unlocked_session(&data);
    session.set_sandbox_mode(true);

    session.place_reference_diagram(&data).unwrap();
    assert!(!session.board.drawing_strokes.is_empty());

    session.interpret_drawing(&data).unwrap();
    let diagram = session.board.last_diagram.as_ref().unwrap();
    assert!(diagram.circle_found);
    assert_eq!(
        diagram.runes.len(),
        crate::manual::notation_for(session.current_commission(&data), &data).len(),
        "{diagram:?}"
    );
}

#[test]
fn laying_out_a_manual_page_brings_its_structural_work_too() {
    // A commission that demands rings, seals and sub-circles has to arrive on
    // the slate whole — and tracing exactly what arrives has to read as that
    // commission, structure counts included.
    let data = data();
    let job = data
        .commissions
        .iter()
        .find(|job| job.required_sub_scopes > 0)
        .expect("a commission with sub-scopes");
    let mut session = unlocked_session(&data);
    session.player.workshop_rank = 4;
    session.player.focus = 1_000.0;

    session.place_reference_for(job, &data).unwrap();
    assert!(
        !session.board.guide_structure.is_empty(),
        "structural work did not reach the slate"
    );

    let mut ink = trace_the_guides(&session);
    ink.extend(session.board.guide_structure.iter().cloned());
    session.board.drawing_strokes = ink;
    session.interpret_drawing(&data).unwrap();

    let placed = placed_ids(&session);
    for required in [
        &job.required_effect,
        &job.required_shape,
        &job.required_trigger,
    ] {
        assert!(
            placed.contains(required),
            "{required} missing from {placed:?}"
        );
    }
    let tree = session
        .board
        .last_diagram
        .as_ref()
        .and_then(|diagram| diagram.scope_spell.as_ref())
        .expect("scope tree");
    assert!(tree.ring_count >= job.required_structure.rings);
    assert!(tree.sub_scopes.len() >= job.required_sub_scopes);
}
