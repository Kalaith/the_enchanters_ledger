//! The reading, in the player's words.
//!
//! This is the only place the grammar is ever explained, and it explains it by
//! doing: the player draws, presses Interpret, and the game reads their
//! handwriting back to them. Pull two marks together and the sentence changes.
//! That is the whole teaching surface — there is no live feedback while the pen
//! is down, by design (`.project/placement-rules.md` §4.1).
//!
//! So the vocabulary of `reading.rs` — group, attached, reach — never appears
//! here. And the phrasing itself is data (`assets/data/runes.json`), not code:
//! adding a rune must not mean writing a sentence in Rust.

use super::{Mark, Reach, Reading};
use crate::data::{RuneCategory, RuneDef};

/// Reads a diagram back as plain sentences, one per thing the working does.
/// Every recognized mark appears in exactly one line — a mark that is silently
/// dropped from the reading is a mark the player cannot learn from.
pub fn read_aloud(reading: &Reading, defs: &[&RuneDef]) -> Vec<String> {
    let mut lines = Vec::new();
    for (index, mark) in reading.marks.iter().enumerate() {
        if mark.category != RuneCategory::Effect {
            continue;
        }
        lines.push(effect_line(reading, defs, index, mark));
    }
    // Shape, then modifiers, then the trigger — a fixed order, so moving a mark
    // around the circle changes what the reading *says* and never merely the
    // order it says it in.
    for category in [
        RuneCategory::Shape,
        RuneCategory::Modifier,
        RuneCategory::Trigger,
    ] {
        for (index, mark) in reading.marks.iter().enumerate() {
            if mark.reach != Reach::Working || mark.category != category {
                continue;
            }
            if let Some(line) = working_line(defs, mark, index, reading) {
                lines.push(line);
            }
        }
    }
    lines
}

/// One effect, with whatever was drawn against it folded in: "Fire is projected
/// as a beam, tempered."
fn effect_line(reading: &Reading, defs: &[&RuneDef], index: usize, mark: &Mark) -> String {
    let Some(def) = find(defs, &mark.rune_id) else {
        return String::new();
    };
    let shape = reading
        .attached_to(index)
        .find(|other| other.category == RuneCategory::Shape)
        .and_then(|other| find(defs, &other.rune_id));
    let modifiers = reading
        .attached_to(index)
        .filter(|other| other.category == RuneCategory::Modifier)
        .filter_map(|other| find(defs, &other.rune_id))
        .map(applied_phrase)
        .collect::<Vec<_>>();

    let mut line = match shape {
        Some(shape) => format!("{} is {}", def.name, applied_phrase(shape)),
        None => format!("{} {}", def.name, working_phrase(def)),
    };
    if !modifiers.is_empty() {
        line.push_str(&format!(", {}", join(&modifiers)));
    }
    line.push('.');
    line
}

/// One mark that acts on everything: the shape at the heart, a modifier drawn
/// on its own, the trigger.
fn working_line(defs: &[&RuneDef], mark: &Mark, index: usize, reading: &Reading) -> Option<String> {
    let def = find(defs, &mark.rune_id)?;
    // A shape or modifier that something else was drawn against is already
    // spoken for inside that mark's own line.
    if reading.attached_to(index).next().is_some() {
        return None;
    }
    Some(match mark.category {
        RuneCategory::Shape => format!("The {} {}.", def.name, working_phrase(def)),
        RuneCategory::Trigger => format!("The working {}.", working_phrase(def)),
        RuneCategory::Modifier => {
            format!("The whole working has been {}.", applied_phrase(def))
        }
        RuneCategory::Effect => return None,
    })
}

fn find<'a>(defs: &[&'a RuneDef], rune_id: &str) -> Option<&'a RuneDef> {
    defs.iter().copied().find(|def| def.id == rune_id)
}

/// How a rune reads when it acts on the whole working.
fn working_phrase(def: &RuneDef) -> &str {
    def.reads_as.as_deref().unwrap_or(match def.category {
        RuneCategory::Effect => "fills the working",
        RuneCategory::Shape => "surrounds the entire working",
        RuneCategory::Trigger => "activates as marked",
        RuneCategory::Modifier => "tempered",
    })
}

/// How a rune reads when it acts on one other mark.
fn applied_phrase(def: &RuneDef) -> String {
    def.applied_reads_as
        .clone()
        .unwrap_or_else(|| format!("marked with {}", def.name))
}

/// "a", "a and b", "a, b and c" — the reading is prose, not a list.
fn join(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [only] => only.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
