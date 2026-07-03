use super::*;
use crate::data::{CommissionDef, GameData};
use crate::rune_drawing::test_support::perturb;
use crate::rune_drawing::{template_strokes_for_rune, DrawnStroke, StrokePoint};

fn data() -> GameData {
    GameData::load().unwrap()
}

fn unlocked_session(data: &GameData) -> GameSession {
    let mut session = GameSession::new(&data.config);
    session.start_playing();
    session.player.tutorial_stage = TutorialStage::Complete;
    session
}

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
fn rough_inner_circle_prefers_sphere_over_safer() {
    let data = data();
    let mut session = unlocked_session(&data);
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.42, 0.58, 0.18));
    strokes.push(flat_rough_circle(0.50, 0.34, 0.18));
    strokes.push(stroke_at(
        &[
            (0.16, 0.55),
            (0.32, 0.30),
            (0.50, 0.52),
            (0.67, 0.78),
            (0.85, 0.52),
            (0.68, 0.27),
            (0.50, 0.48),
            (0.33, 0.75),
            (0.16, 0.55),
        ],
        0.64,
        0.58,
        0.20,
    ));
    session.board.drawing_strokes = strokes;

    session.interpret_drawing(&data).unwrap();
    let ids = session
        .board
        .placed
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(ids.contains(&"sphere"), "{ids:?}");
    assert!(!ids.contains(&"safer"), "{ids:?}");
}

#[test]
fn screenshot_starter_runes_read_as_close_enough() {
    let data = data();
    let mut session = unlocked_session(&data);
    let mut strokes = rough_circle(0.50, 0.48, 0.38, 0.35, 34);
    strokes.extend(template_at("light", 0.34, 0.30, 0.18));
    strokes.extend(rough_continuous_diamonds(0.62, 0.32, 0.22));
    strokes.push(rough_sphere(0.49, 0.58, 0.18));
    session.board.drawing_strokes = strokes;

    session.interpret_drawing(&data).unwrap();
    let ids = session
        .board
        .placed
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(ids.contains(&"light"), "{ids:?}");
    assert!(ids.contains(&"continuous"), "{ids:?}");
    assert!(ids.contains(&"sphere"), "{ids:?}");
}

#[test]
fn tall_diagram_keeps_light_after_extracting_inner_sphere() {
    let data = data();
    let mut session = unlocked_session(&data);
    let mut strokes = rough_circle(0.445, 0.51, 0.205, 0.44, 36);
    strokes.extend(template_at("light", 0.36, 0.36, 0.16));
    strokes.push(rough_sphere(0.46, 0.45, 0.19));
    strokes.extend(template_at("continuous", 0.38, 0.62, 0.16));
    session.board.drawing_strokes = strokes;

    session.interpret_drawing(&data).unwrap();
    let ids = placed_ids(&session);

    assert!(ids.iter().any(|id| id == "light"), "{ids:?}");
    assert!(ids.iter().any(|id| id == "sphere"), "{ids:?}");
    assert!(ids.iter().any(|id| id == "continuous"), "{ids:?}");

    session.interpret_drawing(&data).unwrap();
    let second_ids = placed_ids(&session);

    assert_eq!(ids, second_ids);
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
    let ids = session
        .board
        .placed
        .iter()
        .map(|rune| rune.rune_id.clone())
        .collect::<Vec<_>>();
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
fn deselect_rune_cancels_armed_template_placement() {
    let data = data();
    let mut session = GameSession::new(&data.config);

    session.select_rune("light", &data).unwrap();
    session.deselect_rune();

    assert!(session.board.selected_rune.is_none());
    assert!(!session.board.template_armed);
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

/// A working circle with a fixed `gravity` + `sphere` + `continuous` + `safer` core plus
/// caller-chosen structural-mark counts — used to compare how score scales with structure
/// well past `magical_circle.rs`'s old fixed targets (rings 3, satellites 5, radials 4,
/// perimeter 14, scripts 28).
fn structured_circle(
    rings: usize,
    satellites: usize,
    radials: usize,
    perimeter: usize,
    scripts: usize,
) -> Vec<DrawnStroke> {
    let mut strokes = outer_circle();
    for index in 0..rings {
        // Each concentric ring's diameter stays within `ReinforcementRing`'s valid scale band
        // (0.28..=0.92 relative to the working circle) for up to 6 rings.
        let rx = 0.13 + index as f32 * 0.014;
        strokes.extend(rough_circle(0.50, 0.50, rx, rx * 0.95, 28));
    }
    strokes.extend(template_at("gravity", 0.28, 0.48, 0.22));
    strokes.extend(template_at("sphere", 0.50, 0.50, 0.18));
    strokes.extend(template_at("continuous", 0.73, 0.48, 0.17));
    strokes.extend(template_at("safer", 0.50, 0.73, 0.15));
    strokes.extend(satellite_seals(satellites, 0.30, 0.038));
    strokes.extend(radial_spokes(radials, 0.31));
    strokes.extend(perimeter_ticks(perimeter, 0.39, 0.016));
    strokes.extend(script_marks(scripts, 0.27, 0.008));
    strokes
}

fn circled_diagram(runes: &[(&str, f32, f32)]) -> Vec<DrawnStroke> {
    let mut strokes = outer_circle();
    for (rune_id, x, y) in runes {
        strokes.extend(template_at(rune_id, *x, *y, 0.18));
    }
    strokes
}

fn circled_order(order: &CommissionDef) -> Vec<DrawnStroke> {
    circled_diagram(&[
        (order.required_effect.as_str(), 0.26, 0.50),
        (order.required_shape.as_str(), 0.50, 0.50),
        (order.required_trigger.as_str(), 0.74, 0.50),
    ])
}

fn rough_circled_diagram() -> Vec<DrawnStroke> {
    let mut strokes = vec![DrawnStroke {
        points: vec![
            StrokePoint::new(0.24, 0.46),
            StrokePoint::new(0.28, 0.20),
            StrokePoint::new(0.50, 0.08),
            StrokePoint::new(0.78, 0.18),
            StrokePoint::new(0.60, 0.38),
            StrokePoint::new(0.88, 0.44),
            StrokePoint::new(0.82, 0.70),
            StrokePoint::new(0.56, 0.60),
            StrokePoint::new(0.62, 0.86),
            StrokePoint::new(0.34, 0.80),
            StrokePoint::new(0.40, 0.58),
            StrokePoint::new(0.14, 0.62),
        ],
    }];
    strokes.extend(rough_light(0.26, 0.42, 0.15));
    strokes.push(rough_sphere(0.50, 0.40, 0.13));
    strokes.push(stroke_at(
        &[
            (0.16, 0.55),
            (0.32, 0.30),
            (0.50, 0.52),
            (0.67, 0.78),
            (0.85, 0.52),
            (0.68, 0.27),
            (0.50, 0.48),
            (0.33, 0.75),
            (0.16, 0.55),
        ],
        0.50,
        0.66,
        0.20,
    ));
    strokes
}

fn high_tier_city_circle() -> Vec<DrawnStroke> {
    let mut strokes = outer_circle();
    strokes.extend(rough_circle(0.50, 0.50, 0.36, 0.34, 48));
    strokes.extend(rough_circle(0.50, 0.50, 0.30, 0.29, 44));
    strokes.extend(rough_circle(0.50, 0.50, 0.23, 0.22, 38));
    strokes.extend(rough_circle(0.50, 0.50, 0.16, 0.15, 32));
    strokes.extend(template_at("gravity", 0.28, 0.48, 0.22));
    strokes.extend(template_at("sphere", 0.50, 0.50, 0.18));
    strokes.extend(template_at("continuous", 0.73, 0.48, 0.17));
    strokes.extend(template_at("safer", 0.50, 0.73, 0.15));
    strokes.extend(satellite_seals(8, 0.30, 0.038));
    strokes.extend(radial_spokes(8, 0.31));
    strokes.extend(perimeter_ticks(36, 0.39, 0.016));
    strokes.extend(perimeter_ticks(24, 0.34, 0.012));
    strokes.extend(script_marks(24, 0.20, 0.010));
    strokes.extend(script_marks(32, 0.27, 0.008));
    strokes
}

fn rough_circle(cx: f32, cy: f32, rx: f32, ry: f32, steps: usize) -> Vec<DrawnStroke> {
    let mut points = Vec::new();
    for index in 0..=steps {
        let angle = std::f32::consts::TAU * index as f32 / steps as f32;
        points.push(StrokePoint::new(
            cx + rx * angle.cos(),
            cy + ry * angle.sin(),
        ));
    }
    vec![DrawnStroke { points }]
}

fn satellite_seals(count: usize, orbit: f32, radius: f32) -> Vec<DrawnStroke> {
    (0..count)
        .flat_map(|index| {
            let angle =
                std::f32::consts::TAU * index as f32 / count as f32 + std::f32::consts::FRAC_PI_4;
            rough_circle(
                0.50 + orbit * angle.cos(),
                0.50 + orbit * angle.sin(),
                radius,
                radius * 0.92,
                16,
            )
        })
        .collect()
}

fn radial_spokes(count: usize, radius: f32) -> Vec<DrawnStroke> {
    (0..count)
        .map(|index| {
            let angle = std::f32::consts::TAU * index as f32 / count as f32;
            DrawnStroke {
                points: vec![
                    StrokePoint::new(0.50, 0.50),
                    StrokePoint::new(0.50 + radius * angle.cos(), 0.50 + radius * angle.sin()),
                ],
            }
        })
        .collect()
}

fn perimeter_ticks(count: usize, orbit: f32, half_len: f32) -> Vec<DrawnStroke> {
    (0..count)
        .map(|index| {
            let angle = std::f32::consts::TAU * index as f32 / count as f32;
            let center = StrokePoint::new(0.50 + orbit * angle.cos(), 0.50 + orbit * angle.sin());
            let tangent = angle + std::f32::consts::FRAC_PI_2;
            DrawnStroke {
                points: vec![
                    StrokePoint::new(
                        center.x - half_len * tangent.cos(),
                        center.y - half_len * tangent.sin(),
                    ),
                    StrokePoint::new(
                        center.x + half_len * tangent.cos(),
                        center.y + half_len * tangent.sin(),
                    ),
                ],
            }
        })
        .collect()
}

fn script_marks(count: usize, orbit: f32, half_len: f32) -> Vec<DrawnStroke> {
    (0..count)
        .map(|index| {
            let angle = std::f32::consts::TAU * (index as f32 + 0.35) / count as f32;
            let center = StrokePoint::new(0.50 + orbit * angle.cos(), 0.50 + orbit * angle.sin());
            let tangent = angle + std::f32::consts::FRAC_PI_2;
            let skew = if index % 2 == 0 { 0.55 } else { -0.55 };
            DrawnStroke {
                points: vec![
                    StrokePoint::new(
                        center.x - half_len * tangent.cos(),
                        center.y - half_len * tangent.sin(),
                    ),
                    StrokePoint::new(
                        center.x + half_len * tangent.cos(),
                        center.y + half_len * tangent.sin(),
                    ),
                    StrokePoint::new(
                        center.x
                            + half_len * (tangent + skew).cos()
                            + half_len * 0.35 * angle.cos(),
                        center.y
                            + half_len * (tangent + skew).sin()
                            + half_len * 0.35 * angle.sin(),
                    ),
                ],
            }
        })
        .collect()
}

fn rough_light(cx: f32, cy: f32, scale: f32) -> Vec<DrawnStroke> {
    vec![
        stroke_at(&[(0.27, 0.50), (0.75, 0.48)], cx, cy, scale),
        stroke_at(&[(0.50, 0.82), (0.50, 0.18)], cx, cy, scale),
        stroke_at(&[(0.67, 0.28), (0.34, 0.75)], cx, cy, scale),
        stroke_at(&[(0.30, 0.33), (0.70, 0.70)], cx, cy, scale),
        stroke_at(&[(0.42, 0.43), (0.58, 0.57)], cx, cy, scale),
    ]
}

fn rough_sphere(cx: f32, cy: f32, scale: f32) -> DrawnStroke {
    stroke_at(
        &[
            (0.50, 0.14),
            (0.76, 0.24),
            (0.86, 0.52),
            (0.68, 0.80),
            (0.38, 0.84),
            (0.16, 0.62),
            (0.22, 0.30),
            (0.50, 0.14),
        ],
        cx,
        cy,
        scale,
    )
}

fn rough_continuous_diamonds(cx: f32, cy: f32, scale: f32) -> Vec<DrawnStroke> {
    vec![
        stroke_at(
            &[
                (0.16, 0.50),
                (0.32, 0.22),
                (0.50, 0.50),
                (0.32, 0.78),
                (0.16, 0.50),
            ],
            cx,
            cy,
            scale,
        ),
        stroke_at(
            &[
                (0.50, 0.50),
                (0.68, 0.22),
                (0.84, 0.50),
                (0.68, 0.78),
                (0.50, 0.50),
            ],
            cx,
            cy,
            scale,
        ),
    ]
}

fn flat_rough_circle(cx: f32, cy: f32, scale: f32) -> DrawnStroke {
    stroke_at(
        &[
            (0.40, 0.18),
            (0.64, 0.18),
            (0.80, 0.30),
            (0.84, 0.56),
            (0.72, 0.76),
            (0.48, 0.84),
            (0.26, 0.74),
            (0.16, 0.52),
            (0.20, 0.32),
            (0.40, 0.18),
        ],
        cx,
        cy,
        scale,
    )
}

fn rough_touch_arrow(cx: f32, cy: f32, scale: f32) -> Vec<DrawnStroke> {
    vec![
        stroke_at(&[(0.52, 0.14), (0.52, 0.82)], cx, cy, scale),
        stroke_at(&[(0.30, 0.58), (0.52, 0.82)], cx, cy, scale),
        stroke_at(&[(0.72, 0.58), (0.52, 0.82)], cx, cy, scale),
    ]
}

fn outer_circle() -> Vec<DrawnStroke> {
    let mut points = Vec::new();
    for index in 0..=36 {
        let angle = std::f32::consts::TAU * index as f32 / 36.0;
        points.push(StrokePoint::new(
            0.50 + 0.42 * angle.cos(),
            0.50 + 0.40 * angle.sin(),
        ));
    }
    vec![DrawnStroke { points }]
}

fn stroke_at(points: &[(f32, f32)], cx: f32, cy: f32, scale: f32) -> DrawnStroke {
    DrawnStroke {
        points: points
            .iter()
            .map(|(x, y)| StrokePoint::new(cx + (x - 0.5) * scale, cy + (y - 0.5) * scale))
            .collect(),
    }
}

fn template_at(rune_id: &str, cx: f32, cy: f32, scale: f32) -> Vec<DrawnStroke> {
    template_strokes_for_rune(rune_id)
        .unwrap()
        .into_iter()
        .map(|stroke| DrawnStroke {
            points: stroke
                .points
                .into_iter()
                .map(|point| {
                    StrokePoint::new(cx + (point.x - 0.5) * scale, cy + (point.y - 0.5) * scale)
                })
                .collect(),
        })
        .collect()
}

/// Plan Phase 5 item 4: a degraded (but not sloppy) hand — roughly a 15% scale
/// wobble plus per-point noise — applied to a clean template before it is
/// placed on the slate. Reuses `perturb`, the same seeded jitter the
/// confusion-matrix gate (`rune_drawing::confusion_gate`) already exercises.
fn jittered_template_at(
    rune_id: &str,
    cx: f32,
    cy: f32,
    scale: f32,
    seed: u64,
) -> Vec<DrawnStroke> {
    let raw = template_strokes_for_rune(rune_id).unwrap();
    let degraded = perturb(&raw, 0.85, (0.02, -0.02), 0.015, seed);
    degraded
        .into_iter()
        .map(|stroke| DrawnStroke {
            points: stroke
                .points
                .into_iter()
                .map(|point| {
                    StrokePoint::new(cx + (point.x - 0.5) * scale, cy + (point.y - 0.5) * scale)
                })
                .collect(),
        })
        .collect()
}

fn degraded_circled_order(order: &CommissionDef) -> Vec<DrawnStroke> {
    let mut strokes = outer_circle();
    strokes.extend(jittered_template_at(
        &order.required_effect,
        0.26,
        0.50,
        0.18,
        11,
    ));
    strokes.extend(jittered_template_at(
        &order.required_shape,
        0.50,
        0.50,
        0.18,
        22,
    ));
    strokes.extend(jittered_template_at(
        &order.required_trigger,
        0.74,
        0.50,
        0.18,
        33,
    ));
    strokes
}

fn placed_ids(session: &GameSession) -> Vec<String> {
    session
        .board
        .placed
        .iter()
        .map(|rune| rune.rune_id.clone())
        .collect()
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

/// A circle whose quality (~0.30) sits between `Sandbox`'s 0.24 acceptance floor and
/// `Commission`'s 0.32 one — an off-center, elongated, incomplete arc.
fn weak_partial_circle() -> Vec<DrawnStroke> {
    let mut points = Vec::new();
    for index in 0..=28 {
        let angle = std::f32::consts::TAU * 0.6 * index as f32 / 28.0;
        points.push(StrokePoint::new(
            0.60 + 0.32 * angle.cos(),
            0.60 + 0.16 * angle.sin(),
        ));
    }
    vec![DrawnStroke { points }]
}
