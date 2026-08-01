//! The quest diagram manual: one entry per shipped commission and day
//! talisman, pairing the order's text with the diagram that fills it.
//!
//! This is the single source both manual surfaces read — the in-game overlay
//! (`ui::manual`) and the generated HTML page (`--manual <dir>`, see
//! `html::render_manual_page`) — so a change to how a quest's diagram is laid
//! out shows up in both without either knowing about the other.
//!
//! Entries are built, not authored: the runes come from `assets/data`, the
//! layout from `crate::perfect_diagram`, and the "does this actually read?"
//! caveat from running the generated ink back through the real recognizer. A
//! manual that claimed a diagram works when the recognizer disagrees would be
//! worse than no manual at all.

use crate::data::{CommissionDef, GameData, RuneDef};
use crate::perfect_diagram::{perfect_diagram_for, DiagramRequest, PerfectDiagram, StructurePlan};
use crate::rune_diagram::interpret_diagram;
use crate::state::WorkOrderKind;

mod html;
mod svg;
#[cfg(test)]
mod tests;

pub use html::render_manual_page;
pub use svg::diagram_svg;

/// One rune a quest calls for, with the slot it fills.
#[derive(Debug, Clone)]
pub struct ManualRune {
    pub label: &'static str,
    pub id: String,
    pub name: String,
}

/// One quest's manual page.
#[derive(Debug, Clone)]
pub struct ManualEntry {
    pub id: String,
    pub kind: &'static str,
    /// Heading this entry files under in the generated page.
    pub section: &'static str,
    pub item: String,
    pub customer: String,
    pub request: String,
    pub risk: String,
    pub reward: i64,
    pub insight: i64,
    pub notation: Vec<ManualRune>,
    /// How to read the picture: what the sizes mean, and anything about this
    /// page in particular. Written per entry rather than by the renderer, since
    /// a quest diagram and a ladder rung are drawn to different rules.
    pub note: String,
    /// Plain-language lines for whatever structural work the order demands
    /// beyond its runes; empty for the many orders that demand none.
    pub structure: Vec<String>,
    pub diagram: PerfectDiagram,
    /// This entry's own diagram, read aloud exactly as the slate reads a
    /// player's — see `crate::reading::sentences`. The manual has to speak the
    /// same language the game does, or it teaches a dialect nobody else uses.
    pub reading: Vec<String>,
    /// Names of runes this entry's own diagram does not read back as — the
    /// recognizer's tracked gaps (see TODO.md), surfaced rather than hidden.
    pub unreadable: Vec<String>,
}

impl ManualEntry {
    pub fn title(&self) -> String {
        let mut title = self.item.clone();
        if let Some(first) = title.get_mut(0..1) {
            first.make_ascii_uppercase();
        }
        title
    }
}

/// Every page in the manual: story commissions, then day talismans, then the
/// practice ladder.
pub fn manual_entries(data: &GameData) -> Vec<ManualEntry> {
    data.commissions
        .iter()
        .map(|job| entry(job, WorkOrderKind::Story, data))
        .chain(
            data.talisman_jobs
                .iter()
                .map(|job| entry(job, WorkOrderKind::Talisman, data)),
        )
        .chain(ladder_entries(data))
        .collect()
}

/// The practice ladder as manual pages — see `crate::ladder`. These are drills,
/// not orders, so they carry no customer, coins or insight; the complexity word
/// takes the risk badge's place.
fn ladder_entries(data: &GameData) -> impl Iterator<Item = ManualEntry> + '_ {
    crate::ladder::ladder_levels().iter().map(move |level| {
        let diagram = crate::ladder::diagram_for_level(level, data);
        let notation = ladder_notation(&level.runes, data);
        ManualEntry {
            id: format!("ladder_{}", level.level),
            kind: "Practice Ladder",
            section: "Practice Ladder",
            item: level.title.clone(),
            customer: format!(
                "Level {} of 10 - {} - {} mark{}",
                level.level,
                level.complexity,
                level.runes.len(),
                if level.runes.len() == 1 { "" } else { "s" }
            ),
            request: level.brief.clone(),
            risk: level.complexity.clone(),
            reward: 0,
            insight: 0,
            unreadable: unreadable_in(&notation, &diagram, data),
            reading: read_diagram_aloud(&diagram, data),
            note: ladder_note(level),
            notation,
            structure: ladder_structure_notes(level),
            diagram,
        }
    })
}

/// A ladder level names each mark it wants, repeats included; the manual lists
/// each distinct rune once with a count, so ten marks do not become ten rows.
fn ladder_notation(runes: &[String], data: &GameData) -> Vec<ManualRune> {
    let mut notation: Vec<ManualRune> = Vec::new();
    let mut counts: Vec<usize> = Vec::new();
    for id in runes {
        match notation.iter().position(|rune| rune.id == *id) {
            Some(index) => counts[index] += 1,
            None => {
                let Some(def) = data.rune(id) else { continue };
                notation.push(ManualRune {
                    label: def.category.label(),
                    id: id.clone(),
                    name: def.name.clone(),
                });
                counts.push(1);
            }
        }
    }
    for (rune, count) in notation.iter_mut().zip(counts) {
        if count > 1 {
            rune.name = format!("{} x{count}", rune.name);
        }
    }
    notation
}

fn ladder_structure_notes(level: &crate::ladder::LadderLevel) -> Vec<String> {
    let wants = level.structure;
    structure_lines(&[
        (wants.rings, "reinforcement ring"),
        (wants.satellites, "satellite seal"),
        (wants.radials, "radial spoke"),
        (wants.perimeter, "perimeter tick"),
        (wants.scripts, "script mark"),
        (wants.sub_scopes, "sub-circle"),
    ])
}

fn entry(job: &CommissionDef, kind: WorkOrderKind, data: &GameData) -> ManualEntry {
    let diagram = diagram_for_job(job, data, |_| true);
    let notation = notation_for(job, data);
    let unreadable = unreadable_in(&notation, &diagram, data);

    ManualEntry {
        id: job.id.clone(),
        kind: kind.panel_title(),
        section: match kind {
            WorkOrderKind::Story => "Story Quests",
            WorkOrderKind::Talisman => "Day Talismans",
        },
        item: job.item.clone(),
        customer: job.customer.clone(),
        request: job.request.clone(),
        risk: job.risk.clone(),
        reward: job.reward,
        insight: job.insight,
        notation,
        structure: structure_notes(job),
        note: format!("{REFERENCE_SIZE_NOTE} {SLATE_SHAPE_NOTE}"),
        reading: read_diagram_aloud(&diagram, data),
        diagram,
        unreadable,
    }
}

/// Interprets a generated diagram and reads it back, so the manual page says
/// what the slate would say for the same ink.
fn read_diagram_aloud(diagram: &PerfectDiagram, data: &GameData) -> Vec<String> {
    let interpretation = interpret_diagram(&diagram.strokes(), data.runes.iter());
    let defs = data.runes.iter().collect::<Vec<_>>();
    let reading = crate::reading::read(&interpretation.runes, interpretation.circle_center, &defs);
    crate::reading::read_aloud(&reading, &defs)
}

/// Said on every page that draws its runes at reference size.
const REFERENCE_SIZE_NOTE: &str =
    "Runes are drawn at their category's reference size, which reads at full potency.";
/// Said on every page, because the ellipse surprises everyone once.
const SLATE_SHAPE_NOTE: &str = "The circle is drawn in slate coordinates, so it is wider than it \
     is tall - that is the shape that scores as round.";

/// A rung that packs many marks into one circle has to draw them under
/// reference size, which the reading notices; say so rather than repeating the
/// full-potency line that is true of the quest pages.
fn ladder_note(level: &crate::ladder::LadderLevel) -> String {
    if level.rune_scale >= 1.0 {
        return format!("{REFERENCE_SIZE_NOTE} {SLATE_SHAPE_NOTE}");
    }
    format!(
        "Marks here are drawn at {}% of their category's reference size - small enough to keep \
         {} of them apart inside one circle, which means they read at reduced potency. {}",
        (level.rune_scale * 100.0).round(),
        level.runes.len(),
        SLATE_SHAPE_NOTE
    )
}

/// The runes a quest calls for: its required trio, then its optional bonus
/// modifier if it has one.
pub fn notation_for(job: &CommissionDef, data: &GameData) -> Vec<ManualRune> {
    let mut notation = [
        ("Effect", job.required_effect.as_str()),
        ("Shape", job.required_shape.as_str()),
        ("Trigger", job.required_trigger.as_str()),
    ]
    .into_iter()
    .map(|(label, id)| ManualRune {
        label,
        id: id.to_owned(),
        name: data.rune_name(id).to_owned(),
    })
    .collect::<Vec<_>>();
    if let Some(id) = job.optional_modifier.as_deref() {
        notation.push(ManualRune {
            label: "Bonus",
            id: id.to_owned(),
            name: data.rune_name(id).to_owned(),
        });
    }
    notation
}

/// Lays out the diagram that fills `job`: its notation, plus whatever
/// structural work it demands. `allow` filters the notation down to runes the
/// reader may actually use — the manual passes everything, the in-game slate
/// reference passes only unlocked runes.
pub fn diagram_for_job(
    job: &CommissionDef,
    data: &GameData,
    allow: impl Fn(&RuneDef) -> bool,
) -> PerfectDiagram {
    let runes = notation_for(job, data)
        .into_iter()
        .filter_map(|rune| data.rune(&rune.id))
        .filter(|rune| allow(rune))
        .collect::<Vec<_>>();
    // A sub-scope needs ink of its own to read as a scope rather than as plain
    // reinforcement decoration. The order's own effect is the natural thing to
    // put in one: a vent carries the same working the circle does.
    let sub_scope_runes = data
        .rune(&job.required_effect)
        .filter(|rune| allow(rune))
        .into_iter()
        .collect();

    perfect_diagram_for(&DiagramRequest {
        runes,
        structure: structure_plan(job),
        sub_scope_runes,
        ..Default::default()
    })
}

pub fn structure_plan(job: &CommissionDef) -> StructurePlan {
    let needs = &job.required_structure;
    StructurePlan {
        rings: needs.rings,
        satellites: needs.satellites,
        radials: needs.radials,
        perimeter: needs.perimeter,
        scripts: needs.scripts,
        sub_scopes: job.required_sub_scopes,
    }
}

fn structure_notes(job: &CommissionDef) -> Vec<String> {
    let needs = &job.required_structure;
    structure_lines(&[
        (needs.rings, "reinforcement ring"),
        (needs.satellites, "satellite seal"),
        (needs.radials, "radial spoke"),
        (needs.perimeter, "perimeter tick"),
        (needs.scripts, "script mark"),
        (job.required_sub_scopes, "sub-circle"),
    ])
}

fn structure_lines(counts: &[(usize, &str)]) -> Vec<String> {
    counts
        .iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, noun)| format!("{count} {noun}{}", if *count == 1 { "" } else { "s" }))
        .collect()
}

/// Which of `notation`'s runes the recognizer does not read back out of
/// `diagram` - the manual's own honesty check, run against the real reader.
fn unreadable_in(
    notation: &[ManualRune],
    diagram: &PerfectDiagram,
    data: &GameData,
) -> Vec<String> {
    let read_back = interpret_diagram(&diagram.strokes(), data.runes.iter());
    notation
        .iter()
        .filter(|rune| !read_back.runes.iter().any(|found| found.rune_id == rune.id))
        .map(|rune| rune.name.clone())
        .collect()
}
