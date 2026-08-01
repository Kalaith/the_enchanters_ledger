//! The placement rules, tested as rules — see `.project/placement-rules.md` §9.
//!
//! Marks are built directly rather than drawn, so a failure here is a failure of
//! the grammar and not of the recognizer. `state::tests` covers the path from
//! real ink.

use super::*;
use crate::data::GameData;
use crate::rune_diagram::InterpretedRune;
use std::f32::consts::TAU;

const CENTER: StrokePoint = StrokePoint { x: 0.5, y: 0.5 };

/// A mark at `turns` around the circle (0.0 = right, 0.25 = down), `orbit` out
/// from the middle.
fn at(rune_id: &str, turns: f32, orbit: f32) -> InterpretedRune {
    let angle = turns * TAU;
    InterpretedRune {
        rune_id: rune_id.to_owned(),
        confidence: 1.0,
        quality: 1.0,
        center: StrokePoint::new(
            CENTER.x + orbit * 0.4 * angle.cos(),
            CENTER.y + orbit * 0.4 * angle.sin(),
        ),
        scale: 0.18,
        orbit,
        scope_depth: 0,
        potency: 1.0,
    }
}

/// Evenly spaced around the ring, in the order given.
fn spread(ids: &[&str]) -> Vec<InterpretedRune> {
    ids.iter()
        .enumerate()
        .map(|(index, id)| at(id, index as f32 / ids.len() as f32, 0.55))
        .collect()
}

fn reach_of(reading: &Reading, rune_id: &str) -> Reach {
    reading
        .marks
        .iter()
        .find(|mark| mark.rune_id == rune_id)
        .unwrap_or_else(|| panic!("no {rune_id} in the reading"))
        .reach
}

fn target_of<'a>(reading: &'a Reading, rune_id: &str) -> Option<&'a str> {
    match reach_of(reading, rune_id) {
        Reach::Working => None,
        Reach::Mark(index) => Some(reading.marks[index].rune_id.as_str()),
    }
}

#[test]
fn evenly_spaced_marks_join_nothing() {
    // The compatibility guarantee: a diagram drawn without knowing the rule
    // reads exactly as it always did.
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    for count in 3..=9 {
        let ids = [
            "light",
            "sphere",
            "continuous",
            "safer",
            "warmth",
            "touch",
            "spark",
            "stronger",
            "frost",
        ];
        let runes = spread(&ids[..count]);
        let reading = read(&runes, CENTER, &defs);

        assert_eq!(reading.groups.len(), count, "{count} marks");
        for mark in &reading.marks {
            assert_eq!(
                mark.reach,
                Reach::Working,
                "{} at {count} marks",
                mark.rune_id
            );
        }
    }
}

#[test]
fn an_unevenly_drawn_spread_still_joins_nothing() {
    // Exact even spacing is a weak guarantee — hands wobble. A ring drawn by
    // eye has uneven gaps, and none of them should read as deliberate.
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    let wobble = [0.0, 0.28, 0.47, 0.72];
    let ids = ["light", "sphere", "safer", "continuous"];
    let runes = ids
        .iter()
        .zip(wobble)
        .map(|(id, turns)| at(id, turns, 0.55))
        .collect::<Vec<_>>();

    let reading = read(&runes, CENTER, &defs);

    for mark in &reading.marks {
        assert_eq!(mark.reach, Reach::Working, "{}", mark.rune_id);
    }
}

#[test]
fn a_modifier_pulled_beside_an_effect_tempers_that_effect() {
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    // Light and Safer close together; Sphere and Continuous left spread.
    let runes = vec![
        at("light", 0.0, 0.55),
        at("safer", 0.04, 0.55),
        at("sphere", 0.4, 0.55),
        at("continuous", 0.7, 0.55),
    ];
    let reading = read(&runes, CENTER, &defs);

    assert_eq!(target_of(&reading, "safer"), Some("light"));
    assert_eq!(reach_of(&reading, "sphere"), Reach::Working);
    assert_eq!(reach_of(&reading, "continuous"), Reach::Working);
}

#[test]
fn moving_the_modifier_changes_what_it_tempers() {
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    let beside_light = read(
        &[
            at("light", 0.0, 0.55),
            at("safer", 0.04, 0.55),
            at("warmth", 0.4, 0.55),
            at("continuous", 0.7, 0.55),
        ],
        CENTER,
        &defs,
    );
    let beside_warmth = read(
        &[
            at("light", 0.0, 0.55),
            at("warmth", 0.4, 0.55),
            at("safer", 0.44, 0.55),
            at("continuous", 0.7, 0.55),
        ],
        CENTER,
        &defs,
    );

    assert_eq!(target_of(&beside_light, "safer"), Some("light"));
    assert_eq!(target_of(&beside_warmth, "safer"), Some("warmth"));
}

#[test]
fn several_modifiers_crowded_round_one_mark_all_reach_it() {
    // A mark carries any number of modifiers; each attaches to exactly one mark,
    // reaching past its fellow modifiers rather than chaining off them.
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    let runes = vec![
        at("light", 0.0, 0.55),
        at("safer", 0.03, 0.55),
        at("stronger", 0.06, 0.55),
        at("longer_duration", 0.09, 0.55),
        at("continuous", 0.5, 0.55),
    ];
    let reading = read(&runes, CENTER, &defs);

    for modifier in ["safer", "stronger", "longer_duration"] {
        assert_eq!(target_of(&reading, modifier), Some("light"), "{modifier}");
    }
}

#[test]
fn a_shape_drawn_against_one_effect_shapes_only_that_effect() {
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    let runes = vec![
        at("fire", 0.0, 0.55),
        at("beam", 0.04, 0.55),
        at("frost", 0.45, 0.55),
        at("on_impact", 0.72, 0.55),
    ];
    let reading = read(&runes, CENTER, &defs);

    assert_eq!(target_of(&reading, "beam"), Some("fire"));
    assert_eq!(reach_of(&reading, "frost"), Reach::Working);
}

#[test]
fn a_shape_in_the_heart_shapes_everything() {
    // Heart marks define defaults: they take no partners however close the ring
    // marks orbiting them happen to sit.
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    let runes = vec![
        at("sphere", 0.0, 0.05),
        at("fire", 0.0, 0.55),
        at("frost", 0.03, 0.55),
        at("on_impact", 0.5, 0.55),
    ];
    let reading = read(&runes, CENTER, &defs);

    assert_eq!(reach_of(&reading, "sphere"), Reach::Working);
    assert_eq!(
        reading
            .marks
            .iter()
            .find(|mark| mark.rune_id == "sphere")
            .unwrap()
            .band,
        Band::Heart
    );
}

#[test]
fn a_trigger_never_attaches_however_close_it_is_drawn() {
    // Position is free for triggers: there is only one working, and the trigger
    // says when it acts.
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    let runes = vec![
        at("light", 0.0, 0.55),
        at("continuous", 0.03, 0.55),
        at("sphere", 0.4, 0.55),
        at("warmth", 0.7, 0.55),
    ];
    let reading = read(&runes, CENTER, &defs);

    assert_eq!(reach_of(&reading, "continuous"), Reach::Working);
}

#[test]
fn rotating_a_diagram_changes_nothing() {
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    let base = vec![
        at("light", 0.0, 0.55),
        at("safer", 0.04, 0.55),
        at("sphere", 0.4, 0.55),
        at("continuous", 0.7, 0.55),
    ];
    let reference = read(&base, CENTER, &defs);

    for turn in [0.1, 0.25, 0.5, 0.77, 0.99] {
        let rotated = base
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let ids = ["light", "safer", "sphere", "continuous"];
                let turns = [0.0, 0.04, 0.4, 0.7];
                at(ids[index], (turns[index] + turn) % 1.0, 0.55)
            })
            .collect::<Vec<_>>();
        let reading = read(&rotated, CENTER, &defs);

        assert_eq!(reading.groups.len(), reference.groups.len(), "turn {turn}");
        for mark in &reference.marks {
            assert_eq!(
                target_of(&reading, &mark.rune_id),
                target_of(&reference, &mark.rune_id),
                "{} at turn {turn}",
                mark.rune_id
            );
        }
    }
}

#[test]
fn marks_in_a_sub_scope_never_join_the_marks_outside_it() {
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    let mut runes = spread(&["light", "sphere", "continuous"]);
    let mut vent = at("fire", 0.02, 0.55);
    vent.scope_depth = 1;
    runes.push(vent);
    let reading = read(&runes, CENTER, &defs);

    assert_eq!(
        reading.marks.len(),
        3,
        "a vent's mark joined the outer ring"
    );
}

#[test]
fn a_group_of_only_modifiers_tempers_nothing() {
    // There is nothing there to modify, so both stay working-wide.
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    let runes = vec![
        at("safer", 0.0, 0.55),
        at("stronger", 0.03, 0.55),
        at("light", 0.4, 0.55),
        at("continuous", 0.7, 0.55),
    ];
    let reading = read(&runes, CENTER, &defs);

    assert_eq!(reach_of(&reading, "safer"), Reach::Working);
    assert_eq!(reach_of(&reading, "stronger"), Reach::Working);
}

#[test]
fn the_reading_speaks_plainly_and_accounts_for_every_mark() {
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    let runes = vec![
        at("light", 0.0, 0.55),
        at("safer", 0.04, 0.55),
        at("sphere", 0.4, 0.55),
        at("continuous", 0.7, 0.55),
    ];
    let reading = read(&runes, CENTER, &defs);
    let lines = read_aloud(&reading, &defs);

    assert_eq!(
        lines,
        vec![
            "Light lights the whole working, tempered.".to_owned(),
            "The Sphere surrounds the entire working.".to_owned(),
            "The working runs continuously.".to_owned(),
        ]
    );
    // The player never meets the vocabulary this module thinks in.
    for line in &lines {
        for jargon in ["bind", "group", "adjacen", "attach", "orbit", "reach"] {
            assert!(
                !line.to_lowercase().contains(jargon),
                "{jargon} leaked into {line:?}"
            );
        }
    }
}

#[test]
fn pulling_a_shape_in_changes_the_sentence() {
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    let spread_reading = read(&spread(&["fire", "beam", "on_impact"]), CENTER, &defs);
    let together = read(
        &[
            at("fire", 0.0, 0.55),
            at("beam", 0.04, 0.55),
            at("on_impact", 0.5, 0.55),
        ],
        CENTER,
        &defs,
    );

    assert_eq!(
        read_aloud(&spread_reading, &defs)[0],
        "Fire burns through the whole working."
    );
    assert_eq!(
        read_aloud(&together, &defs)[0],
        "Fire is projected as a beam."
    );
}

#[test]
fn the_reading_says_things_in_a_fixed_order() {
    // Moving a mark around the circle must change what the reading says, never
    // merely the order it says it in — otherwise the player learns noise.
    let data = GameData::load().unwrap();
    let defs = data.runes.iter().collect::<Vec<_>>();
    let ids = ["continuous", "sphere", "light", "safer"];
    let mut previous: Option<Vec<String>> = None;

    for turn in [0.0, 0.3, 0.6] {
        let runes = ids
            .iter()
            .enumerate()
            .map(|(index, id)| at(id, (index as f32 / 4.0 + turn) % 1.0, 0.55))
            .collect::<Vec<_>>();
        let lines = read_aloud(&read(&runes, CENTER, &defs), &defs);
        if let Some(previous) = &previous {
            assert_eq!(previous, &lines, "the order moved with the diagram");
        }
        previous = Some(lines);
    }
}
