//! Recipe discovery as a single piece of bookkeeping.
//!
//! `progression` already checks that a first test pays `DISCOVERY_INSIGHT` once.
//! What was missing is the rest of the ledger moving *with* it: the recipe row
//! itself, the journal, coins and reputation on delivery, and the archive tier
//! that the accumulated insight eventually buys. Each of those is written by a
//! different method, so a change to one can silently stop agreeing with the
//! others — these fixtures assert the whole set against one run.

use super::fixtures::*;
use crate::state::{EnchantGrade, GameSession, WorkOrderKind, DISCOVERY_INSIGHT};

/// The three rank-one marks every fixture below discovers with. Kept in one
/// place so "the same design" means the same signature everywhere.
fn lantern_diagram() -> Vec<crate::rune_drawing::DrawnStroke> {
    circled_diagram(&[
        ("light", 0.26, 0.50),
        ("sphere", 0.50, 0.50),
        ("continuous", 0.74, 0.50),
    ])
}

fn session_with_read_diagram(data: &crate::data::GameData) -> GameSession {
    let mut session = unlocked_session(data);
    session.board.drawing_strokes = lantern_diagram();
    session.interpret_drawing(data).unwrap();
    session
}

/// A first test writes exactly one ledger row and pays insight for it; a second
/// test of the same design writes no new row, pays nothing again, and only
/// updates that row's running counters. `best_score` is a max, not a last-write.
#[test]
fn testing_a_design_twice_writes_one_ledger_row_and_pays_once() {
    let data = data();
    let mut session = session_with_read_diagram(&data);
    let insight_before = session.player.insight;
    let coins_before = session.player.coins;
    let reputation_before = session.player.reputation;

    let first = session.test_design(&data);

    assert_eq!(session.discoveries.len(), 1, "{:?}", session.discoveries);
    let row = session.discoveries[0].clone();
    assert_eq!(row.uses, 1, "{row:?}");
    assert_eq!(row.best_score, first.result.score, "{row:?}");
    assert_eq!(row.name, first.result.title, "{row:?}");
    assert_eq!(session.player.insight, insight_before + DISCOVERY_INSIGHT);
    // Testing is bench work: it costs and earns no money or standing.
    assert_eq!(session.player.coins, coins_before);
    assert_eq!(session.player.reputation, reputation_before);
    assert_eq!(first.reward, 0);
    assert_eq!(first.reputation, 0);

    let second = session.test_design(&data);

    assert_eq!(session.discoveries.len(), 1, "{:?}", session.discoveries);
    let row = session.discoveries[0].clone();
    assert_eq!(row.uses, 2, "{row:?}");
    assert_eq!(row.best_score, first.result.score.max(second.result.score));
    assert!(second.discovery.is_none(), "{second:?}");
    assert_eq!(second.insight, 0);
    assert_eq!(session.player.insight, insight_before + DISCOVERY_INSIGHT);
}

/// The journal is the player's only record of *why* their insight moved. A
/// discovery has to leave one, and re-testing the same design must not leave a
/// second one claiming insight that was never paid.
#[test]
fn a_discovery_leaves_exactly_one_journal_entry() {
    let data = data();
    let mut session = session_with_read_diagram(&data);
    let entries_before = session.journal.len();

    let report = session.test_design(&data);
    session.test_design(&data);

    let name = report
        .discovery
        .as_ref()
        .expect("first test discovers")
        .name
        .clone();
    let insight_entries = session
        .journal
        .iter()
        .skip(entries_before)
        .filter(|entry| entry.title == "Insight gained")
        .collect::<Vec<_>>();

    assert_eq!(insight_entries.len(), 1, "{insight_entries:?}");
    assert!(
        insight_entries[0].body.contains(&name),
        "{insight_entries:?}"
    );
    assert!(
        insight_entries[0]
            .body
            .contains(&DISCOVERY_INSIGHT.to_string()),
        "{insight_entries:?}"
    );
    assert_eq!(insight_entries[0].day, session.player.day);
}

/// Delivery is where the other three currencies move. Discovery insight is paid
/// on top of the client's own insight, and every figure the report claims has to
/// be the figure the player's ledger actually moved by.
#[test]
fn delivering_a_discovery_moves_coins_reputation_and_insight_as_reported() {
    let data = data();
    let mut session = session_with_read_diagram(&data);
    let before = session.player.clone();

    let report = session.deliver_design(&data);

    assert_eq!(session.player.coins, before.coins + report.reward);
    assert_eq!(
        session.player.reputation,
        before.reputation + report.reputation
    );
    assert_eq!(session.player.insight, before.insight + report.insight);
    assert_eq!(session.player.completed_orders, before.completed_orders + 1);
    assert_eq!(session.player.day, before.day + 1);

    // The delivery discovered the recipe itself, so its insight is the client's
    // plus the discovery's — never the discovery's alone.
    let discovery = report.discovery.as_ref().expect("first delivery discovers");
    assert_eq!(discovery.insight, DISCOVERY_INSIGHT);
    assert!(report.insight >= DISCOVERY_INSIGHT, "{report:?}");
    assert_eq!(session.discoveries.len(), 1, "{:?}", session.discoveries);
    assert!(
        session
            .journal
            .iter()
            .any(|entry| entry.title.starts_with("Delivered")),
        "{:?}",
        session.journal
    );
}

/// A failed grade is not a discovery. Nothing may be written to the ledger and
/// no insight may be paid — otherwise a player could farm insight off ink that
/// never worked.
#[test]
fn a_failed_design_records_no_recipe_and_pays_no_insight() {
    let data = data();
    let mut session = unlocked_session(&data);
    session.board.drawing_strokes = weak_partial_circle();
    let _ = session.interpret_drawing(&data);
    let insight_before = session.player.insight;

    let report = session.test_design(&data);

    if report.result.grade == EnchantGrade::Failed {
        assert!(session.discoveries.is_empty(), "{:?}", session.discoveries);
        assert!(report.discovery.is_none(), "{report:?}");
        assert_eq!(report.insight, 0);
        assert_eq!(session.player.insight, insight_before);
    } else {
        // The fixture is meant to fail; if scoring ever makes it pass, the test
        // is no longer covering what it claims to.
        panic!("weak_partial_circle no longer fails: {report:?}");
    }
}

/// The whole loop, in one run: discoveries accumulate insight, insight and coins
/// buy the next archive tier, and the tier is what opens a rune that was locked
/// a moment ago. `research` has to debit exactly its quoted price.
#[test]
fn accumulated_discovery_insight_buys_the_next_archive_tier() {
    let data = data();
    let mut session = session_with_read_diagram(&data);

    let fire = data.rune("fire").expect("fire is a rank-two rune");
    assert_eq!(session.player.workshop_rank, 1);
    assert!(!session.can_use_rune(fire), "fire starts locked");
    assert!(session.select_rune("fire", &data).is_err());

    // Bench work alone cannot pay for research: it earns insight, never coins.
    session.test_design(&data);
    assert!(session.player.insight >= DISCOVERY_INSIGHT);
    let coins_from_testing = session.player.coins;

    // Deliveries are where the coins come from. Take talisman work so the run
    // does not depend on which story commission happens to be pinned.
    session.player.active_work = WorkOrderKind::Talisman;
    for _ in 0..6 {
        session.board.drawing_strokes = lantern_diagram();
        session.interpret_drawing(&data).unwrap();
        session.deliver_design(&data);
    }
    assert!(
        session.player.coins > coins_from_testing,
        "deliveries earned nothing: {:?}",
        session.player
    );

    let coins_before = session.player.coins;
    let insight_before = session.player.insight;
    let day_before = session.player.day;
    // Rank 2 costs 24 + 2*16 coins and 6 + 2*5 insight (see `research`).
    let (coin_cost, insight_cost) = (24 + 2 * 16, 6 + 2 * 5);
    assert!(
        coins_before >= coin_cost && insight_before >= insight_cost,
        "run did not earn enough to research: {:?}",
        session.player
    );

    let message = session.research(&data).expect("research affordable");

    assert_eq!(session.player.workshop_rank, 2);
    assert_eq!(session.player.coins, coins_before - coin_cost);
    assert_eq!(session.player.insight, insight_before - insight_cost);
    assert_eq!(session.player.day, day_before + 1);
    assert!(message.contains("Fire"), "{message}");
    assert!(
        session.can_use_rune(fire),
        "fire still locked after research"
    );
    assert!(session.select_rune("fire", &data).is_ok());
    assert!(
        session
            .journal
            .iter()
            .any(|entry| entry.title == "Research completed"),
        "{:?}",
        session.journal
    );
}
