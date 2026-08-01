//! A rung nobody can draw is not a rung, so every level's diagram goes back
//! through the real recognizer here.

use super::*;
use crate::rune_diagram::interpret_diagram;
use std::collections::HashMap;

/// Counts of each rune id, so a level that asks for two `light` marks is only
/// satisfied by reading two of them back.
fn tally<'a>(ids: impl Iterator<Item = &'a str>) -> HashMap<&'a str, usize> {
    let mut counts = HashMap::new();
    for id in ids {
        *counts.entry(id).or_insert(0) += 1;
    }
    counts
}

#[test]
fn the_ladder_descends_one_mark_at_a_time() {
    // The shape of the whole thing: level 1 has ten marks, each rung after it
    // has one fewer, and level 10 has a single mark. Levels are added one per
    // pass, so this checks the run built so far is a prefix of that ladder.
    let levels = ladder_levels();
    assert!(!levels.is_empty(), "the ladder is empty");
    assert!(levels.len() <= 10, "the ladder has more than ten rungs");

    for (index, level) in levels.iter().enumerate() {
        let expected = index as u32 + 1;
        assert_eq!(level.level, expected, "rungs are out of order");
        assert_eq!(
            level.runes.len(),
            11 - expected as usize,
            "level {expected} should carry {} marks",
            11 - expected
        );
        assert!(!level.title.is_empty(), "level {expected} has no title");
        assert!(!level.brief.is_empty(), "level {expected} has no brief");
        assert!(
            !level.complexity.is_empty(),
            "level {expected} has no complexity"
        );
    }
}

#[test]
fn every_rung_reads_back_exactly_what_it_asks_for() {
    let data = GameData::load().unwrap();
    for level in ladder_levels() {
        let diagram = diagram_for_level(level, &data);
        let read_back = interpret_diagram(&diagram.strokes(), data.runes.iter());

        assert!(
            read_back.circle_found,
            "level {}: no circle ({read_back:?})",
            level.level
        );
        assert_eq!(
            tally(read_back.runes.iter().map(|rune| rune.rune_id.as_str())),
            tally(level.runes.iter().map(String::as_str)),
            "level {} misread",
            level.level
        );
    }
}

#[test]
fn every_rung_carries_the_structure_it_asks_for() {
    let data = GameData::load().unwrap();
    for level in ladder_levels() {
        let wants = level.structure;
        if StructurePlan::from(wants).is_empty() {
            continue;
        }
        let diagram = diagram_for_level(level, &data);
        let read_back = interpret_diagram(&diagram.strokes(), data.runes.iter());
        let tree = read_back
            .scope_spell
            .as_ref()
            .unwrap_or_else(|| panic!("level {}: no scope tree", level.level));

        assert!(tree.ring_count >= wants.rings, "level {}", level.level);
        assert!(
            tree.satellite_count >= wants.satellites,
            "level {}",
            level.level
        );
        assert!(tree.radial_count >= wants.radials, "level {}", level.level);
        assert!(
            tree.perimeter_mark_count >= wants.perimeter,
            "level {}",
            level.level
        );
        assert!(
            tree.script_mark_count >= wants.scripts,
            "level {}",
            level.level
        );
        assert!(
            tree.sub_scopes.len() >= wants.sub_scopes,
            "level {}",
            level.level
        );
    }
}
