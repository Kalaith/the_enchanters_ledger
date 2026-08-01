//! A manual whose diagrams do not fill the quests they document is worse than
//! none, so the central test here runs every generated diagram through the real
//! session pipeline — interpret, then evaluate — and demands the commission
//! read as matched.

use super::*;
use crate::state::{EnchantGrade, GameSession, TutorialStage, WorkOrderKind};

/// Quests whose notation the recognizer cannot currently read inside a diagram
/// at all — the `sound` / `larger` gap tracked in TODO.md and in
/// `perfect_diagram::tests::RUNES_THAT_FRAGMENT_IN_A_DIAGRAM`. Their manual
/// entries carry the caveat instead of a working diagram, which
/// `flagged_entries_are_exactly_the_known_gaps` pins down.
const QUESTS_BLOCKED_BY_RECOGNIZER_GAPS: &[&str] = &["spark_sign", "alarm_bell"];

fn session_for(data: &GameData, job_id: &str) -> GameSession {
    let mut session = GameSession::new(&data.config);
    session.start_playing();
    session.player.tutorial_stage = TutorialStage::Complete;
    // The manual documents every quest, including ones gated behind research —
    // so the reader is assumed to have the notation open.
    session.player.workshop_rank = 4;
    if let Some(index) = data.commissions.iter().position(|job| job.id == job_id) {
        session.player.active_work = WorkOrderKind::Story;
        session.player.current_commission = index;
    } else {
        let index = data
            .talisman_jobs
            .iter()
            .position(|job| job.id == job_id)
            .unwrap_or_else(|| panic!("no quest {job_id}"));
        session.player.active_work = WorkOrderKind::Talisman;
        session.player.current_talisman = index;
    }
    session.player.focus = 1_000.0;
    session
}

/// The manual also carries practice-ladder rungs, which are drills rather than
/// orders — `crate::ladder`'s own tests cover those.
fn quest_entries(data: &GameData) -> Vec<ManualEntry> {
    manual_entries(data)
        .into_iter()
        .filter(|entry| entry.kind != "Practice Ladder")
        .collect()
}

#[test]
fn every_manual_diagram_fills_the_quest_it_documents() {
    let data = GameData::load().unwrap();
    for entry in quest_entries(&data) {
        if QUESTS_BLOCKED_BY_RECOGNIZER_GAPS.contains(&entry.id.as_str()) {
            continue;
        }
        let mut session = session_for(&data, &entry.id);
        session.board.drawing_strokes = entry.diagram.strokes();
        session
            .interpret_drawing(&data)
            .unwrap_or_else(|err| panic!("{}: {err}", entry.id));

        let report = session.test_design(&data);
        assert!(
            report.result.matched_request,
            "{}: diagram does not match the request ({:?}, placed {:?})",
            entry.id,
            report.result,
            session
                .board
                .placed
                .iter()
                .map(|placed| placed.rune_id.as_str())
                .collect::<Vec<_>>()
        );
        assert_ne!(
            report.result.grade,
            EnchantGrade::Failed,
            "{}: {:?}",
            entry.id,
            report.result
        );
    }
}

#[test]
fn structural_quests_get_the_structure_they_demand() {
    // The five commissions that ask for rings, satellite seals or sub-circles
    // are the reason the layout has a structured mode at all — check the counts
    // the recognizer reads back, not just that the diagram matched.
    let data = GameData::load().unwrap();
    let structural = data
        .commissions
        .iter()
        .filter(|job| !structure_plan(job).is_empty())
        .collect::<Vec<_>>();
    assert!(!structural.is_empty(), "no structural commissions in data");

    for job in structural {
        let needs = &job.required_structure;
        let diagram = diagram_for_job(job, &data, |_| true);
        let read_back = interpret_diagram(&diagram.strokes(), data.runes.iter());
        let tree = read_back
            .scope_spell
            .as_ref()
            .unwrap_or_else(|| panic!("{}: no scope tree ({read_back:?})", job.id));

        assert!(tree.ring_count >= needs.rings, "{}: {tree:?}", job.id);
        assert!(
            tree.satellite_count >= needs.satellites,
            "{}: {tree:?}",
            job.id
        );
        assert!(tree.radial_count >= needs.radials, "{}: {tree:?}", job.id);
        assert!(
            tree.perimeter_mark_count >= needs.perimeter,
            "{}: {tree:?}",
            job.id
        );
        assert!(
            tree.script_mark_count >= needs.scripts,
            "{}: {tree:?}",
            job.id
        );
        assert!(
            tree.sub_scopes.len() >= job.required_sub_scopes,
            "{}: {tree:?}",
            job.id
        );
    }
}

#[test]
fn structure_never_adds_runes_the_quest_did_not_ask_for() {
    // Decoration below its structure threshold falls through to rune
    // recognition, where a lone satellite seal reads as a `sphere`. Every rune
    // read back has to be one the manual entry actually lists.
    let data = GameData::load().unwrap();
    for entry in manual_entries(&data) {
        if QUESTS_BLOCKED_BY_RECOGNIZER_GAPS.contains(&entry.id.as_str()) {
            continue;
        }
        let read_back = interpret_diagram(&entry.diagram.strokes(), data.runes.iter());
        for found in &read_back.runes {
            assert!(
                entry.notation.iter().any(|rune| rune.id == found.rune_id),
                "{}: diagram also reads a {} nobody asked for",
                entry.id,
                found.rune_id
            );
        }
    }
}

#[test]
fn flagged_entries_are_exactly_the_known_gaps() {
    // Keeps the caveat honest in both directions: no quest quietly starts
    // failing, and the list shrinks when the recognizer improves.
    let data = GameData::load().unwrap();
    let flagged = manual_entries(&data)
        .into_iter()
        .filter(|entry| !entry.unreadable.is_empty())
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    assert_eq!(flagged, QUESTS_BLOCKED_BY_RECOGNIZER_GAPS);
}

#[test]
fn the_manual_covers_every_quest_and_every_ladder_rung() {
    let data = GameData::load().unwrap();
    let entries = manual_entries(&data);

    assert_eq!(
        entries.len(),
        data.commissions.len() + data.talisman_jobs.len() + crate::ladder::ladder_levels().len()
    );
    assert_eq!(quest_entries(&data).len(), 18);
    for entry in &entries {
        assert!(!entry.customer.is_empty(), "{}", entry.id);
        assert!(!entry.request.is_empty(), "{}", entry.id);
        assert!(!entry.notation.is_empty(), "{}", entry.id);
        assert!(!entry.diagram.runes.is_empty(), "{}", entry.id);
    }
    for entry in quest_entries(&data) {
        assert!(entry.notation.len() >= 3, "{}", entry.id);
    }
    // Ladder rungs are drills: complexity in place of risk, and no payout.
    for entry in entries.iter().filter(|e| e.kind == "Practice Ladder") {
        assert_eq!((entry.reward, entry.insight), (0, 0), "{}", entry.id);
        assert!(!entry.risk.is_empty(), "{}", entry.id);
    }
}

#[test]
fn the_page_is_self_contained_and_escapes_quest_text() {
    let data = GameData::load().unwrap();
    let entries = manual_entries(&data);
    let page = render_manual_page(&entries, &data);

    assert!(page.starts_with("<!doctype html>"));
    for entry in &entries {
        assert!(
            page.contains(&format!("id=\"{}\"", entry.id)),
            "{}",
            entry.id
        );
    }
    assert!(
        page.matches("<svg").count() >= entries.len(),
        "one diagram per entry"
    );
    // No external requests: the page has to work from disk with no network.
    // (`xmlns="http://www.w3.org/2000/svg"` is a namespace name, never fetched.)
    for fetched in ["<script", "<link", "<img", "@import", "url(http"] {
        assert!(!page.contains(fetched), "external reference: {fetched}");
    }
    // `Mira`'s request has no markup in it, but the data is text a designer
    // edits — the renderer must not be the thing that stops being safe.
    let mut sharp = data.clone();
    sharp.commissions[0].customer = "<script>alert(1)</script>".to_owned();
    let page = render_manual_page(&manual_entries(&sharp), &sharp);
    assert!(!page.contains("<script>"), "unescaped quest text");
}

#[test]
fn the_svg_draws_every_stroke_of_the_diagram() {
    let data = GameData::load().unwrap();
    let entry = &manual_entries(&data)[0];
    let svg = diagram_svg(&entry.diagram);

    assert_eq!(
        svg.matches("<polyline").count(),
        entry.diagram.strokes().len()
    );
    assert!(svg.contains("viewBox"));
}

#[test]
fn every_page_reads_its_own_diagram_aloud() {
    // The manual has to speak the same language the slate does, or it teaches a
    // dialect nobody else uses. Its sentences come from the same reader, run
    // over its own generated ink.
    let data = GameData::load().unwrap();
    for entry in manual_entries(&data) {
        assert!(
            !entry.reading.is_empty(),
            "{} says nothing about its own diagram",
            entry.id
        );
        // Marks laid out evenly act on the whole working, so no page should
        // claim something was drawn against something else.
        for line in &entry.reading {
            assert!(line.ends_with('.'), "{}: {line:?}", entry.id);
        }
    }
}

#[test]
fn the_page_shows_what_the_diagram_says() {
    let data = GameData::load().unwrap();
    let entries = manual_entries(&data);
    let page = render_manual_page(&entries, &data);

    for line in &entries[0].reading {
        assert!(
            page.contains(line.as_str()),
            "{line:?} missing from the page"
        );
    }
}
