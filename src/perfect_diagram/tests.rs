//! The whole point of a "perfect" diagram is that the recognizer reads it back
//! exactly as laid out, so every test here is a round trip through the real
//! `interpret_diagram` — no shortcuts through internals.

use super::*;
use crate::data::{CommissionDef, GameData};
use crate::rune_diagram::interpret_diagram;

/// Runes whose canonical template does not survive diagram-level *clustering* at
/// its category's reference size: drawn inside a working circle, its component
/// strokes land far enough apart (relative to their own size) that
/// `rune_diagram::geometry::cluster_strokes` never groups them, so recognition
/// only ever sees the pieces. `sound`'s three chevrons split three ways;
/// `larger`'s box and its four detached arrow ticks split five ways, and the box
/// alone reads as `sphere`.
///
/// This is a pre-existing recognizer/template gap, not a layout one — the
/// clustering thresholds are all relative to stroke size, so no scale a
/// generator (or a player) could draw these at changes the outcome. Tracked in
/// TODO.md; `fragmenting_runes_still_fragment` fails the moment one is fixed, so
/// this list cannot quietly rot.
const RUNES_THAT_FRAGMENT_IN_A_DIAGRAM: &[&str] = &["larger", "sound"];

fn runes<'a>(data: &'a GameData, ids: &[&str]) -> Vec<&'a RuneDef> {
    ids.iter()
        .map(|id| data.rune(id).unwrap_or_else(|| panic!("unknown rune {id}")))
        .collect()
}

fn required_runes(job: &CommissionDef) -> Vec<&str> {
    vec![
        job.required_effect.as_str(),
        job.required_shape.as_str(),
        job.required_trigger.as_str(),
    ]
}

fn first_commission_runes(data: &GameData) -> Vec<&str> {
    required_runes(&data.commissions[0])
}

fn round_trip_ids(interpretation: &crate::rune_diagram::DiagramInterpretation) -> Vec<String> {
    let mut found = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.clone())
        .collect::<Vec<_>>();
    found.sort();
    found
}

/// Reads `ids` back out of a generated diagram, sorted.
fn round_trip(data: &GameData, ids: &[&str]) -> Vec<String> {
    let diagram = perfect_diagram(runes(data, ids).iter().copied());
    let interpretation = interpret_diagram(&diagram.strokes(), data.runes.iter());
    let mut found = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.clone())
        .collect::<Vec<_>>();
    found.sort();
    found
}

fn sorted(ids: &[&str]) -> Vec<String> {
    let mut owned = ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();
    owned.sort();
    owned
}

#[test]
fn first_commission_reference_reads_back_exactly() {
    // The opening quest — a circle with Light, Sphere and Continuous inside it.
    // This is the diagram the in-game "Reference" guide lays out, so it has to
    // survive the same commission-band interpretation a player's traced ink does.
    let data = GameData::load().unwrap();
    let wanted = first_commission_runes(&data);
    assert_eq!(
        wanted,
        vec!["light", "sphere", "continuous"],
        "first commission changed; this test's expectations describe the old one"
    );

    let diagram = perfect_diagram(runes(&data, &wanted).iter().copied());
    let interpretation = interpret_diagram(&diagram.strokes(), data.runes.iter());

    assert!(interpretation.circle_found, "{interpretation:?}");
    assert!(
        interpretation.circle_quality > 0.90,
        "circle read at {:.3}",
        interpretation.circle_quality
    );
    assert_eq!(interpretation.rejected_marks, 0, "{interpretation:?}");
    let mut found = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();
    found.sort_unstable();
    assert_eq!(found, vec!["continuous", "light", "sphere"]);
}

#[test]
fn reference_runes_land_at_reference_magnitude() {
    // Placement sizes each rune at its category's `ideal_scale_in_circle`, the
    // point `potency_for_rune` scores as 1.0 — a perfect diagram should read as
    // neither over- nor under-powered.
    let data = GameData::load().unwrap();
    let wanted = first_commission_runes(&data);
    let diagram = perfect_diagram(runes(&data, &wanted).iter().copied());
    let interpretation = interpret_diagram(&diagram.strokes(), data.runes.iter());

    assert_eq!(interpretation.runes.len(), wanted.len());
    for rune in &interpretation.runes {
        assert!(
            (rune.potency - 1.0).abs() < 0.05,
            "{} read at potency {:.3}",
            rune.rune_id,
            rune.potency
        );
        assert!(
            rune.quality > 0.90,
            "{} read at quality {:.3}",
            rune.rune_id,
            rune.quality
        );
    }
}

#[test]
fn every_job_reference_reads_back() {
    // The reference is offered for whatever work is pinned, not just the opening
    // quest, so every shipped commission and talisman has to round-trip — both
    // its required trio and that trio plus its optional modifier, since a fourth
    // rune tightens the ring and the layout still has to keep them separable.
    let data = GameData::load().unwrap();
    for job in data.commissions.iter().chain(&data.talisman_jobs) {
        let mut wanted = required_runes(job);
        if wanted
            .iter()
            .any(|id| RUNES_THAT_FRAGMENT_IN_A_DIAGRAM.contains(id))
        {
            continue;
        }
        assert_eq!(round_trip(&data, &wanted), sorted(&wanted), "{}", job.id);

        let Some(modifier) = job.optional_modifier.as_deref() else {
            continue;
        };
        if RUNES_THAT_FRAGMENT_IN_A_DIAGRAM.contains(&modifier) {
            continue;
        }
        wanted.push(modifier);
        assert_eq!(
            round_trip(&data, &wanted),
            sorted(&wanted),
            "{} + modifier",
            job.id
        );
    }
}

#[test]
fn fragmenting_runes_still_fragment() {
    // Keeps `RUNES_THAT_FRAGMENT_IN_A_DIAGRAM` honest: when the recognizer
    // learns to cluster one of these, this fails and the exclusion goes away.
    let data = GameData::load().unwrap();
    for id in RUNES_THAT_FRAGMENT_IN_A_DIAGRAM {
        assert_ne!(
            round_trip(&data, &[id]),
            sorted(&[id]),
            "{id} now reads back from a generated diagram — drop it from \
             RUNES_THAT_FRAGMENT_IN_A_DIAGRAM and from TODO.md"
        );
    }
}

#[test]
fn a_lone_rune_sits_at_the_center_and_reads_back() {
    // The tutorial's first step only has Light unlocked, so the reference has to
    // work for a single-rune diagram too.
    let data = GameData::load().unwrap();
    let diagram = perfect_diagram(runes(&data, &["light"]).iter().copied());

    assert_eq!(diagram.runes.len(), 1);
    let placement = &diagram.runes[0];
    assert!(
        (placement.center.x - CIRCLE_CENTER.x).abs() < 0.01
            && (placement.center.y - CIRCLE_CENTER.y).abs() < 0.01,
        "lone rune placed at {:?}",
        placement.center
    );
    assert_eq!(round_trip(&data, &["light"]), sorted(&["light"]));
}

#[test]
fn generation_is_deterministic() {
    // A guide the player can re-summon mid-trace must not shift under them.
    let data = GameData::load().unwrap();
    let wanted = runes(&data, &first_commission_runes(&data));
    assert_eq!(
        perfect_diagram(wanted.iter().copied()),
        perfect_diagram(wanted.iter().copied())
    );
}

#[test]
fn an_empty_rune_set_still_produces_a_readable_circle() {
    let diagram = perfect_diagram(std::iter::empty());
    assert!(diagram.runes.is_empty());
    assert!(diagram.circle_stroke().has_ink());
}

#[test]
fn a_requested_grouping_reads_back_as_marks_drawn_together() {
    // The generator has to be able to say what the grammar can express: a
    // modifier drawn against one effect, close enough for `crate::reading` to
    // join them and no closer, so the recognizer still reads two marks.
    let data = GameData::load().unwrap();
    let wanted = runes(&data, &["light", "sphere", "continuous", "safer"]);
    let diagram = perfect_diagram_for(&DiagramRequest {
        runes: wanted.clone(),
        // Light and Safer drawn together.
        groups: vec![vec![0, 3]],
        ..Default::default()
    });

    let interpretation = interpret_diagram(&diagram.strokes(), data.runes.iter());
    assert_eq!(
        round_trip_ids(&interpretation),
        sorted(&["continuous", "light", "safer", "sphere"]),
        "the marks stopped being four separate marks"
    );

    let defs = data.runes.iter().collect::<Vec<_>>();
    let reading = crate::reading::read(&interpretation.runes, interpretation.circle_center, &defs);
    let safer = reading
        .marks
        .iter()
        .position(|mark| mark.rune_id == "safer")
        .expect("safer");
    let target = match reading.marks[safer].reach {
        crate::reading::Reach::Mark(index) => reading.marks[index].rune_id.clone(),
        crate::reading::Reach::Working => panic!("safer was not read as drawn against anything"),
    };
    assert_eq!(target, "light");
}

#[test]
fn an_ungrouped_diagram_still_reads_as_one_whole_working() {
    // The compatibility guarantee, checked at the generator: every quest
    // reference lays its marks out evenly, and nothing joins.
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    for job in data.commissions.iter().chain(&data.talisman_jobs) {
        let diagram = crate::manual::diagram_for_job(job, &data, |_| true);
        let interpretation = interpret_diagram(&diagram.strokes(), data.runes.iter());
        let reading =
            crate::reading::read(&interpretation.runes, interpretation.circle_center, &defs);
        for mark in &reading.marks {
            assert_eq!(
                mark.reach,
                crate::reading::Reach::Working,
                "{}: {} was read as drawn against another mark",
                job.id,
                mark.rune_id
            );
        }
    }
}
