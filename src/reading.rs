//! How a diagram *reads* — the layer between "which runes are on the slate"
//! and "what the working does".
//!
//! Recognition (`rune_diagram`) answers what each mark is and where it sits.
//! This module answers what those marks say together: which mark acts on the
//! whole working, which acts on one other mark, and what sentence that comes
//! out as. See `.project/placement-rules.md` for the design and the reasoning
//! behind each threshold.
//!
//! Three rules, and everything here follows from them:
//!
//! 1. **Marks drawn deliberately close are read together.** Two ring marks join
//!    when their angular gap is under half the mean gap in the scope — a test
//!    that is scale-free, visible to the eye, and *opt-in*: evenly spaced marks
//!    join nothing, so a diagram drawn without knowing this rule reads exactly
//!    as it always did.
//! 2. **Heart marks define defaults; ring marks drawn together create
//!    exceptions.** A shape at the center shapes everything; a shape pulled in
//!    beside one effect shapes only that effect.
//! 3. **A mark may carry any number of modifiers; a modifier attaches to
//!    exactly one mark** — the nearest non-modifier in its group.
//!
//! Nothing here depends on absolute direction: rotating a diagram produces the
//! identical reading. The vocabulary in this module ("group", "attached") is
//! internal — see `sentences`, which is all the player ever sees.

use crate::data::{RuneCategory, RuneDef};
use crate::rune_diagram::InterpretedRune;
use crate::rune_drawing::StrokePoint;
use std::f32::consts::TAU;

mod sentences;
#[cfg(test)]
mod tests;

pub use sentences::read_aloud;

/// Orbit at or below which a mark sits in the working's heart — the band that
/// defines defaults. Matches the band `classify_circle_stroke` already treats
/// as "central" for structural marks.
const HEART_ORBIT: f32 = 0.25;
/// Fraction of the scope's mean angular gap under which two ring marks count as
/// drawn together.
///
/// Two forces set this. Below it, marks have to be visibly closer to each other
/// than to everything else, so an unevenly drawn spread does not read as
/// deliberate. Above it, the rule has to stay *reachable*: two marks cannot be
/// drawn closer than about 1.6 of their own spans without
/// `rune_diagram::geometry::cluster_strokes` merging them into one mark, which
/// on a four-mark ring at reference size is 0.92 rad against a mean gap of 1.57
/// — a floor of 0.58. A half-gap rule would have been geometrically impossible
/// to satisfy for every diagram of four marks or more.
///
/// Two thirds clears that floor for three and four marks at reference size.
/// Five or more marks leave no room at full size, and have to be drawn smaller
/// before they can say anything about grouping — the same trade a crowded
/// diagram already makes.
const TOGETHER_GAP_FRACTION: f32 = 0.65;

/// Which band a mark was drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// The working's default — applies to everything, takes no partners.
    Heart,
    /// Takes part in the reading and can be drawn together with its neighbours.
    Ring,
}

/// What a mark acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// The whole working — a heart mark, a trigger, or a ring mark drawn on its
    /// own.
    Working,
    /// One other mark, by index into `Reading::marks`.
    Mark(usize),
}

/// One recognized mark, with where it sits and what it acts on.
#[derive(Debug, Clone)]
pub struct Mark {
    pub rune_id: String,
    pub category: RuneCategory,
    pub band: Band,
    pub reach: Reach,
    /// Index into `Reading::groups`; marks drawn alone get their own group.
    pub group: usize,
}

/// A whole scope's worth of marks, read.
#[derive(Debug, Clone, Default)]
pub struct Reading {
    pub marks: Vec<Mark>,
    /// Indices into `marks`, one entry per set of marks drawn together. Groups
    /// are what balance and relation scoring reason about — a diagram of three
    /// tight pairs has three anchors, not six.
    pub groups: Vec<Vec<usize>>,
}

impl Reading {
    /// Marks that act on `index`, if any.
    pub fn attached_to(&self, index: usize) -> impl Iterator<Item = &Mark> {
        self.marks
            .iter()
            .filter(move |mark| mark.reach == Reach::Mark(index))
    }

    /// Every (actor, acted-on) pair the reading establishes, as category pairs —
    /// what `state::evaluate`'s relation scoring reads. A mark that reaches the
    /// whole working relates to every other mark in it, which is why an evenly
    /// spread diagram still reads as a coherent working.
    pub fn relations(&self) -> Vec<(usize, usize)> {
        let mut relations = Vec::new();
        for (index, mark) in self.marks.iter().enumerate() {
            match mark.reach {
                Reach::Mark(target) => relations.push((index, target)),
                Reach::Working => {
                    for other in 0..self.marks.len() {
                        if other != index {
                            relations.push((index, other));
                        }
                    }
                }
            }
        }
        relations
    }

    /// Mean direction of each group, for balance scoring — see
    /// `magical_circle::circular_symmetry`.
    pub fn group_angles(&self, angles: &[f32]) -> Vec<f32> {
        self.groups
            .iter()
            .filter_map(|group| {
                let (x, y) = group.iter().fold((0.0, 0.0), |(x, y), index| {
                    let angle = angles.get(*index).copied().unwrap_or(0.0);
                    (x + angle.cos(), y + angle.sin())
                });
                (x.abs() > f32::EPSILON || y.abs() > f32::EPSILON).then(|| y.atan2(x))
            })
            .collect()
    }
}

/// Reads one scope's recognized runes. `runes` is the flat list for a single
/// scope, in any order; nested scopes are read separately, since a scope is a
/// sentence and marks in different sentences never join.
pub fn read(runes: &[InterpretedRune], center: StrokePoint, defs: &[&RuneDef]) -> Reading {
    let mut marks = runes
        .iter()
        // A scope is a sentence, and marks in different sentences never join —
        // a vent's contents are read on their own terms, not against the marks
        // orbiting the working circle outside it.
        .filter(|rune| rune.scope_depth == 0)
        .filter_map(|rune| {
            let def = defs.iter().find(|def| def.id == rune.rune_id)?;
            Some(Mark {
                rune_id: rune.rune_id.clone(),
                category: def.category,
                band: if rune.orbit <= HEART_ORBIT {
                    Band::Heart
                } else {
                    Band::Ring
                },
                reach: Reach::Working,
                group: 0,
            })
        })
        .collect::<Vec<_>>();
    if marks.is_empty() {
        return Reading::default();
    }

    let angles = runes
        .iter()
        .filter(|rune| rune.scope_depth == 0)
        .map(|rune| angle_of(rune, center))
        .collect::<Vec<_>>();
    let groups = group_ring_marks(&marks, &angles);
    for (group_index, group) in groups.iter().enumerate() {
        for index in group {
            marks[*index].group = group_index;
        }
    }
    for group in &groups {
        attach_within(&mut marks, group);
    }

    Reading { marks, groups }
}

/// Angle of a mark around its scope, measured from the scope's center. Only
/// differences between these ever matter, so the zero direction is arbitrary.
fn angle_of(rune: &InterpretedRune, center: StrokePoint) -> f32 {
    let angle = (rune.center.y - center.y).atan2(rune.center.x - center.x);
    (angle + TAU) % TAU
}

/// Partitions ring marks into runs drawn deliberately close, leaving heart
/// marks (and lone ring marks) in groups of their own. Works on the cyclic gap
/// sequence, so the result survives rotation.
fn group_ring_marks(marks: &[Mark], angles: &[f32]) -> Vec<Vec<usize>> {
    let mut ring = (0..marks.len())
        .filter(|index| marks[*index].band == Band::Ring)
        .collect::<Vec<_>>();
    ring.sort_by(|a, b| angles[*a].total_cmp(&angles[*b]));

    let mut groups = marks
        .iter()
        .enumerate()
        .filter(|(_, mark)| mark.band == Band::Heart)
        .map(|(index, _)| vec![index])
        .collect::<Vec<_>>();
    if ring.is_empty() {
        return groups;
    }
    if ring.len() < 3 {
        // Two marks have one gap between them and nothing to compare it
        // against; there is no "closer than the rest" to measure. One mark has
        // no gap at all.
        groups.extend(ring.into_iter().map(|index| vec![index]));
        return groups;
    }

    let gaps = (0..ring.len())
        .map(|position| {
            let next = angles[ring[(position + 1) % ring.len()]];
            let here = angles[ring[position]];
            (next - here + TAU) % TAU
        })
        .collect::<Vec<_>>();
    let mean = gaps.iter().sum::<f32>() / gaps.len() as f32;
    let together = mean * TOGETHER_GAP_FRACTION;

    // Start the walk after the widest gap, so a run that straddles the zero
    // angle is not split in half by where the measurement happens to begin.
    let start = gaps
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(position, _)| (position + 1) % ring.len())
        .unwrap_or(0);

    let mut current = vec![ring[start]];
    for step in 0..ring.len() - 1 {
        let position = (start + step) % ring.len();
        let next = ring[(position + 1) % ring.len()];
        if gaps[position] < together {
            current.push(next);
        } else {
            groups.push(std::mem::replace(&mut current, vec![next]));
        }
    }
    groups.push(current);
    groups
}

/// Applies the reach rules inside one group. A group of one changes nothing —
/// its mark keeps working-wide reach.
fn attach_within(marks: &mut [Mark], group: &[usize]) {
    if group.len() < 2 {
        return;
    }
    let anchors = group
        .iter()
        .copied()
        .filter(|index| marks[*index].category != RuneCategory::Modifier)
        .collect::<Vec<_>>();

    for index in group.iter().copied() {
        let category = marks[index].category;
        // A trigger says when the whole working acts, and there is only one
        // working — its position is free, so it never attaches to anything.
        if category == RuneCategory::Trigger {
            continue;
        }
        let target = match category {
            // A modifier reaches past its fellow modifiers to the mark they are
            // all crowded around, so several stacked outward all attach to that
            // one mark rather than chaining off each other.
            RuneCategory::Modifier => nearest(&anchors, index),
            // A shape drawn against one mark shapes that mark alone. Effects
            // drawn together stay independent for now — direction between them
            // is reserved, see placement-rules.md §7.
            RuneCategory::Shape => nearest(
                &anchors
                    .iter()
                    .copied()
                    .filter(|other| marks[*other].category == RuneCategory::Effect)
                    .collect::<Vec<_>>(),
                index,
            ),
            _ => None,
        };
        if let Some(target) = target {
            marks[index].reach = Reach::Mark(target);
        }
    }
}

/// Nearest candidate to `index` by position in the group's own ordering, which
/// is already angular order. Ties break toward the later mark — clockwise, the
/// direction the circle is read in.
fn nearest(candidates: &[usize], index: usize) -> Option<usize> {
    candidates
        .iter()
        .copied()
        .filter(|candidate| *candidate != index)
        .min_by_key(|candidate| candidate.abs_diff(index))
}
