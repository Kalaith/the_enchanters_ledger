//! Commission and talisman work: what a diagram scores, what it delivers,
//! and which orders a given hand can actually clear.

use super::fixtures::*;
use crate::state::{EnchantGrade, WorkOrderKind};

#[test]
fn matching_design_scores_as_working_enchantment() {
    let data = data();
    let mut session = unlocked_session(&data);
    session.board.drawing_strokes = circled_diagram(&[
        ("light", 0.26, 0.50),
        ("sphere", 0.50, 0.50),
        ("continuous", 0.74, 0.50),
    ]);
    session.interpret_drawing(&data).unwrap();

    let report = session.test_design(&data);

    assert!(report.result.matched_request);
    assert_ne!(report.result.grade, EnchantGrade::Failed);
    assert_eq!(session.discoveries.len(), 1);
    assert_eq!(session.board.placed.len(), 3);
    assert_eq!(session.board.links.len(), 2);
}

#[test]
fn rough_design_still_works_but_scores_lower_than_clean_ink() {
    let data = data();
    let mut clean = unlocked_session(&data);
    clean.board.drawing_strokes = circled_diagram(&[
        ("light", 0.26, 0.50),
        ("sphere", 0.50, 0.50),
        ("continuous", 0.74, 0.50),
    ]);
    clean.interpret_drawing(&data).unwrap();
    let clean_report = clean.test_design(&data);

    let mut rough = unlocked_session(&data);
    rough.board.drawing_strokes = rough_circled_diagram();
    rough.interpret_drawing(&data).unwrap();
    let rough_report = rough.test_design(&data);

    assert!(rough_report.result.matched_request, "{rough_report:?}");
    assert_ne!(rough_report.result.grade, EnchantGrade::Failed);
    assert!(
        rough_report.result.score < clean_report.result.score,
        "rough={:?} clean={:?} qualities={:?}",
        rough_report.result,
        clean_report.result,
        rough.board.placed
    );
    assert_ne!(rough_report.result.grade, EnchantGrade::Brilliant);
}

#[test]
fn doubling_effect_rune_size_raises_power_in_report() {
    // Phase 2 exit criterion: drawing the same commission at 2x effect-rune
    // size measurably raises power in the test report.
    let data = data();
    let mut normal = unlocked_session(&data);
    normal.board.drawing_strokes = circled_diagram(&[
        ("light", 0.26, 0.50),
        ("sphere", 0.50, 0.50),
        ("continuous", 0.74, 0.50),
    ]);
    normal.interpret_drawing(&data).unwrap();
    let normal_report = normal.test_design(&data);

    let mut doubled = unlocked_session(&data);
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.26, 0.50, 0.36));
    strokes.extend(template_at("sphere", 0.50, 0.50, 0.18));
    strokes.extend(template_at("continuous", 0.74, 0.50, 0.18));
    doubled.board.drawing_strokes = strokes;
    doubled.interpret_drawing(&data).unwrap();
    let doubled_report = doubled.test_design(&data);

    assert!(doubled_report.result.matched_request, "{doubled_report:?}");
    assert!(
        doubled_report.result.power > normal_report.result.power,
        "normal={:?} doubled={:?}",
        normal_report.result,
        doubled_report.result
    );
}

#[test]
fn rough_down_arrow_counts_as_touch_for_mug_commission() {
    let data = data();
    let mut session = unlocked_session(&data);
    session.player.current_commission = 1;
    let mut strokes = outer_circle();
    strokes.extend(template_at("warmth", 0.30, 0.70, 0.18));
    strokes.extend(template_at("continuous", 0.30, 0.34, 0.18));
    strokes.extend(rough_touch_arrow(0.70, 0.48, 0.20));
    session.board.drawing_strokes = strokes;

    session.interpret_drawing(&data).unwrap();
    let ids = placed_ids(&session);
    let report = session.test_design(&data);

    assert!(ids.iter().any(|id| id == "touch"), "{ids:?}");
    assert!(report.result.matched_request, "{ids:?} {:?}", report.result);
    assert_ne!(report.result.grade, EnchantGrade::Failed);
}

#[test]
fn locked_story_commission_files_to_journal_and_pins_day_talisman() {
    let data = data();
    let mut session = unlocked_session(&data);
    session.player.current_commission = 2;

    let message = session.ensure_playable_work(&data).unwrap();

    assert_eq!(session.active_work_kind(), WorkOrderKind::Talisman);
    assert_eq!(session.story_commission(&data).id, "glowing_rabbit");
    assert_ne!(session.current_commission(&data).id, "glowing_rabbit");
    assert!(message.contains("needs more research"), "{message}");
    assert!(session
        .journal
        .iter()
        .any(|entry| entry.title == "Story quest filed"));
}

#[test]
fn day_talisman_delivery_keeps_story_and_records_insight_sources() {
    let data = data();
    let mut session = unlocked_session(&data);
    session.player.current_commission = 2;
    session.select_talisman_work(0, &data).unwrap();
    let story_index = session.player.current_commission;
    let job = data.talisman_job(0).clone();
    session.board.drawing_strokes = circled_order(&job);
    session.interpret_drawing(&data).unwrap();
    let insight_before = session.player.insight;

    let report = session.deliver_design(&data);

    assert_eq!(session.player.current_commission, story_index);
    assert_eq!(session.active_work_kind(), WorkOrderKind::Talisman);
    assert!(report.result.matched_request, "{:?}", report.result);
    assert_eq!(session.player.insight, insight_before + report.insight);
    assert!(report
        .notes
        .iter()
        .any(|note| note.contains("Client notes")));
    assert!(report
        .notes
        .iter()
        .any(|note| note.contains("Discovery notes")));
    assert!(session
        .journal
        .iter()
        .any(|entry| entry.body.contains("Client notes")));
}

#[test]
fn high_tier_circle_strengthens_city_shield_commission() {
    let data = data();
    let mut session = unlocked_session(&data);
    session.player.workshop_rank = 4;
    session.player.current_commission = data
        .commissions
        .iter()
        .position(|commission| commission.id == "city_shield")
        .unwrap();
    session.board.drawing_strokes = high_tier_city_circle();

    let note = session.interpret_drawing(&data).unwrap();
    let report = session.test_design(&data);
    let spell = session
        .board
        .last_diagram
        .as_ref()
        .and_then(|diagram| diagram.spell.as_ref())
        .unwrap();

    assert!(note.contains("grand"), "{note}");
    assert_eq!(spell.tier_rank, 4, "{spell:?}");
    assert!(report.result.matched_request, "{:?}", report.result);
    assert_eq!(
        report.result.grade,
        EnchantGrade::Brilliant,
        "{:?}",
        report.result
    );
    assert!(
        report.result.title.contains("Floating City"),
        "{:?}",
        report.result
    );
    assert!(report.result.power >= 40, "{:?}", report.result);
}

#[test]
fn simple_named_recipe_still_recognized_through_evaluate() {
    // Regression guard for the other migrated named recipes (plan Phase 4 item 3): the
    // `floating_stage` commission requires `gravity` (tier 4, so `workshop_rank` must be raised
    // or the recognizer correctly falls back to the best *unlocked* alternative — this tripped
    // up this test's first draft, see the "gravity misread as touch" note below) as its own
    // effect, with shape `aura` and trigger `on_command` — neither `sphere` nor `continuous`, so
    // this can't also satisfy the more specific `floating_city` recipe. A plain matching diagram
    // produces "Gravity Well" end-to-end through `test_design`/`evaluate`, not just at the
    // `interpret_diagram` layer.
    let data = data();
    let mut session = unlocked_session(&data);
    session.player.workshop_rank = 4;
    let commission = data
        .commissions
        .iter()
        .find(|commission| commission.id == "floating_stage")
        .unwrap()
        .clone();
    session.player.current_commission = data
        .commissions
        .iter()
        .position(|c| c.id == commission.id)
        .unwrap();
    session.board.drawing_strokes = circled_order(&commission);

    session.interpret_drawing(&data).unwrap();
    let report = session.test_design(&data);

    assert!(report.result.matched_request, "{:?}", report.result);
    assert!(
        report.result.title.contains("Gravity Well"),
        "{:?}",
        report.result
    );
}

#[test]
fn diminishing_returns_let_more_structure_always_score_higher() {
    // Plan Phase 4 item 1 / C6 exit criterion: complexity/containment/intensity no longer
    // hard-cap at a fixed structural-mark target (`diminishing_count` replaces `ratio_count`,
    // see magical_circle.rs) — a diagram with structure well beyond the old fixed targets must
    // score higher than one that stays within them, given the same core rune content.
    let data = data();
    let mut small = unlocked_session(&data);
    small.board.drawing_strokes = structured_circle(1, 2, 2, 6, 8);
    small.interpret_drawing(&data).unwrap();
    let small_report = small.test_design(&data);

    let mut large = unlocked_session(&data);
    large.board.drawing_strokes = structured_circle(6, 12, 10, 40, 70);
    large.interpret_drawing(&data).unwrap();
    let large_report = large.test_design(&data);

    assert!(
        large_report.result.score > small_report.result.score,
        "small={:?} large={:?}",
        small_report.result,
        large_report.result
    );
}

#[test]
fn backfire_message_names_uncontained_potency() {
    // Plan Phase 4 item 4: a diagram with a lot of drawn effect potency and no containment
    // structure at all (no rings, no `safer` rune) gets a cause-specific backfire message
    // instead of the old generic "ragged strokes leak value" one.
    let data = data();
    let mut session = unlocked_session(&data);
    let mut strokes = outer_circle();
    for index in 0..8 {
        let angle = std::f32::consts::TAU * index as f32 / 8.0;
        let cx = 0.50 + 0.22 * angle.cos();
        let cy = 0.50 + 0.22 * angle.sin();
        strokes.extend(template_at("spark", cx, cy, 0.24));
    }
    session.board.drawing_strokes = strokes;

    session.interpret_drawing(&data).unwrap();
    let report = session.test_design(&data);

    assert!(
        report
            .result
            .side_effect
            .contains("can't hold everything drawn"),
        "{:?}",
        report.result
    );
}

#[test]
fn early_commissions_still_clear_acceptance_when_drawn_with_a_degraded_hand() {
    let data = data();
    let early = data
        .commissions
        .iter()
        .filter(|commission| commission.difficulty <= 2)
        .collect::<Vec<_>>();
    assert!(!early.is_empty());

    for commission in early {
        let mut session = unlocked_session(&data);
        session.player.workshop_rank = 4;
        session.player.current_commission = data
            .commissions
            .iter()
            .position(|candidate| candidate.id == commission.id)
            .unwrap();
        session.board.drawing_strokes = degraded_circled_order(commission);

        session.interpret_drawing(&data).unwrap_or_else(|error| {
            panic!(
                "{} failed to interpret when drawn with jitter: {error}",
                commission.id
            )
        });
        let report = session.test_design(&data);
        assert!(
            report.result.matched_request,
            "{} did not match its own requirements once jittered: {:?}",
            commission.id, report.result
        );
        assert_ne!(
            report.result.grade,
            EnchantGrade::Failed,
            "{} graded Failed once jittered: {:?}",
            commission.id,
            report.result
        );
    }
}

#[test]
fn mid_and_endgame_commissions_clear_with_degraded_hand_and_structure() {
    // Plan Phase 5 item 4, mid/endgame legs: every commission past the early
    // ones — including those demanding rings/satellites (mid-game structure
    // marks) and sub-scope vents (endgame multi-scope work) — must be
    // passable when drawn with the same degraded "average person" hand the
    // early-commission pacing test uses.
    let data = data();
    let later = data
        .commissions
        .iter()
        .filter(|commission| commission.difficulty >= 3)
        .collect::<Vec<_>>();
    assert!(!later.is_empty());

    for commission in later {
        let mut session = unlocked_session(&data);
        session.player.workshop_rank = 4;
        session.player.current_commission = data
            .commissions
            .iter()
            .position(|candidate| candidate.id == commission.id)
            .unwrap();
        session.board.drawing_strokes = degraded_structured_order(commission);

        session.interpret_drawing(&data).unwrap_or_else(|error| {
            panic!(
                "{} failed to interpret when drawn with jitter: {error}",
                commission.id
            )
        });
        let report = session.test_design(&data);
        assert!(
            report.result.matched_request,
            "{} did not match its own requirements once jittered: {:?}",
            commission.id, report.result
        );
        assert_ne!(
            report.result.grade,
            EnchantGrade::Failed,
            "{} graded Failed once jittered: {:?}",
            commission.id,
            report.result
        );
    }
}

#[test]
fn structure_requiring_commission_rejects_a_plain_rune_diagram() {
    // The other half of the pacing gate: a commission that demands
    // structural work must NOT accept the plain three-rune diagram that
    // clears early commissions.
    let data = data();
    let commission = data
        .commissions
        .iter()
        .find(|commission| commission.id == "city_shield")
        .unwrap();
    let mut session = unlocked_session(&data);
    session.player.workshop_rank = 4;
    session.player.current_commission = data
        .commissions
        .iter()
        .position(|candidate| candidate.id == commission.id)
        .unwrap();
    session.board.drawing_strokes = degraded_circled_order(commission);

    session.interpret_drawing(&data).unwrap();
    let report = session.test_design(&data);
    assert!(
        !report.result.matched_request,
        "city_shield accepted a diagram with none of its required structure: {:?}",
        report.result
    );
}
