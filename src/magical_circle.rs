//! Structural interpretation for complex magical circles.

use crate::data::{RuneCategory, RuneDef};
use crate::rune_diagram::InterpretedRune;
use crate::rune_drawing::{DrawnStroke, StrokePoint};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagicalCircleSpell {
    pub name: String,
    pub tier_rank: u32,
    pub complexity: f32,
    pub intensity: f32,
    pub containment: f32,
    pub symmetry: f32,
    pub size_harmony: f32,
    pub ring_count: usize,
    pub satellite_count: usize,
    pub radial_count: usize,
    pub perimeter_mark_count: usize,
    #[serde(default)]
    pub script_mark_count: usize,
    pub dominant_effect: Option<String>,
    pub score_bonus: i32,
    pub power_bonus: i32,
    pub stability_bonus: i32,
    pub mana_cost_delta: i32,
    pub safety_bonus: i32,
}

impl MagicalCircleSpell {
    pub fn tier_label(&self) -> &'static str {
        match self.tier_rank {
            4.. => "grand",
            3 => "high",
            2 => "woven",
            _ => "simple",
        }
    }

    pub fn note(&self) -> String {
        format!(
            "{} {} circle: complexity {}%, rings {}, satellites {}, scripts {}, symmetry {}%",
            self.tier_label(),
            self.name,
            percent(self.complexity),
            self.ring_count,
            self.satellite_count,
            self.script_mark_count,
            percent(self.symmetry)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircleStrokeKind {
    RuneInk,
    ReinforcementRing,
    SatelliteSeal,
    PerimeterMark,
    RadialSpoke,
    ScriptMark,
}

impl CircleStrokeKind {
    pub fn is_circle_structure(self) -> bool {
        matches!(
            self,
            CircleStrokeKind::ReinforcementRing
                | CircleStrokeKind::PerimeterMark
                | CircleStrokeKind::RadialSpoke
                | CircleStrokeKind::ScriptMark
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CircleBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl CircleBounds {
    pub fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    fn from_stroke(stroke: &DrawnStroke) -> Option<Self> {
        let mut points = stroke.points.iter();
        let first = points.next()?;
        let mut bounds = Self::new(first.x, first.y, first.x, first.y);
        for point in points {
            bounds.min_x = bounds.min_x.min(point.x);
            bounds.min_y = bounds.min_y.min(point.y);
            bounds.max_x = bounds.max_x.max(point.x);
            bounds.max_y = bounds.max_y.max(point.y);
        }
        Some(bounds)
    }

    pub fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    pub fn height(self) -> f32 {
        self.max_y - self.min_y
    }

    pub fn center(self) -> StrokePoint {
        StrokePoint::new(
            self.min_x + self.width() * 0.5,
            self.min_y + self.height() * 0.5,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CircleMark {
    pub kind: CircleStrokeKind,
    pub scale: f32,
    pub orbit: f32,
    pub angle: f32,
    pub quality: f32,
}

pub fn classify_circle_stroke(stroke: &DrawnStroke, circle: CircleBounds) -> Option<CircleMark> {
    if !stroke.has_ink() {
        return None;
    }
    let bounds = CircleBounds::from_stroke(stroke)?;
    let center = bounds.center();
    let scale = (bounds.width() / circle.width().max(0.001))
        .max(bounds.height() / circle.height().max(0.001));
    let orbit = normalized_orbit(center, circle);
    let angle = angle_from_circle(center, circle);
    let closed = is_closed_stroke(stroke, bounds);
    let aspect = ratio_score(bounds.width() / bounds.height().max(0.001), 1.0);
    let circle_shape = if closed {
        radius_consistency(&stroke.points, center) * 0.72 + aspect * 0.28
    } else {
        0.0
    };
    let length_ratio = stroke_length(&stroke.points) / average_radius(circle).max(0.001);
    let radial = radial_score(stroke, circle);
    // Plan Phase 3 item 4 (C1/C2): a perimeter tick or script mark is a small, near-straight,
    // minimally-sampled flick (2-3 points; see the structure-mark test fixtures in
    // rune_diagram::tests) — `directness` and the point-count ceiling below are what actually
    // distinguish decorative structure ink from a small *rune* drawn at the same
    // orbit/scale/length, which usually has at least one real corner and more sampled shape
    // (e.g. `spark`'s 4-point zigzag). Without this, a diagram with many small legitimate runes
    // could see them swept into these structure-mark buckets (and excluded from rune recognition
    // entirely) purely by being small and numerous — exactly the "structure vs. rune ink by
    // size" failure mode the plan calls out.
    let directness = if stroke.points.len() >= 2 {
        distance(
            *stroke.points.first().unwrap(),
            *stroke.points.last().unwrap(),
        ) / stroke_length(&stroke.points).max(0.001)
    } else {
        0.0
    };

    // Perimeter ticks and script dashes have real drawn length — a
    // near-zero-length *dot* (e.g. the detached point some rune glyphs
    // carry) is not decorative writing and must stay rune ink, or the rune
    // it belongs to loses a piece and stops reading (plan Phase 3 item 4,
    // geometry over size).
    let has_dash_length = length_ratio >= 0.02;
    let kind = if closed && orbit <= 0.15 && (0.28..=0.92).contains(&scale) && circle_shape > 0.48 {
        CircleStrokeKind::ReinforcementRing
    } else if closed
        && stroke.points.len() >= 12
        && (0.22..=0.78).contains(&orbit)
        && (0.055..=0.24).contains(&scale)
        && circle_shape > 0.42
    {
        CircleStrokeKind::SatelliteSeal
    } else if (0.78..=1.10).contains(&orbit)
        && scale <= 0.13
        && length_ratio <= 0.30
        && has_dash_length
        && directness > 0.65
        && stroke.points.len() <= 3
    {
        CircleStrokeKind::PerimeterMark
    } else if (0.18..=0.88).contains(&orbit)
        && scale <= 0.075
        && length_ratio <= 0.20
        && has_dash_length
        && directness > 0.65
        && stroke.points.len() <= 3
    {
        CircleStrokeKind::ScriptMark
    } else if radial > 0.68 {
        CircleStrokeKind::RadialSpoke
    } else {
        CircleStrokeKind::RuneInk
    };
    let quality = match kind {
        CircleStrokeKind::ReinforcementRing | CircleStrokeKind::SatelliteSeal => circle_shape,
        CircleStrokeKind::RadialSpoke => radial,
        CircleStrokeKind::PerimeterMark => {
            let orbit_score = 1.0 - (orbit - 0.94).abs() / 0.18;
            let size_score = ratio_score(length_ratio.max(0.001), 0.11);
            (orbit_score.clamp(0.0, 1.0) * 0.58 + size_score * 0.42).clamp(0.0, 1.0)
        }
        CircleStrokeKind::ScriptMark => {
            let orbit_score = 1.0 - (orbit - 0.58).abs() / 0.46;
            let size_score = ratio_score(length_ratio.max(0.001), 0.07);
            (orbit_score.clamp(0.0, 1.0) * 0.50 + size_score * 0.50).clamp(0.0, 1.0)
        }
        CircleStrokeKind::RuneInk => circle_shape.max(radial),
    };

    Some(CircleMark {
        kind,
        scale,
        orbit,
        angle,
        quality,
    })
}

pub fn analyze_magical_circle(
    circle_quality: f32,
    marks: &[CircleMark],
    runes: &[InterpretedRune],
    rune_defs: &[&RuneDef],
    reading: &crate::reading::Reading,
) -> Option<MagicalCircleSpell> {
    let ring_count = count_kind(marks, CircleStrokeKind::ReinforcementRing);
    let satellite_count = count_kind(marks, CircleStrokeKind::SatelliteSeal);
    let radial_count = count_kind(marks, CircleStrokeKind::RadialSpoke);
    let perimeter_mark_count = count_kind(marks, CircleStrokeKind::PerimeterMark);
    let raw_script_mark_count = count_kind(marks, CircleStrokeKind::ScriptMark);
    let script_mark_count = if raw_script_mark_count >= 8 {
        raw_script_mark_count
    } else {
        0
    };
    let structural_count =
        ring_count + satellite_count + radial_count + perimeter_mark_count + script_mark_count;
    let max_tier = runes
        .iter()
        .filter_map(|rune| rune_def(rune_defs, &rune.rune_id).map(|def| def.tier))
        .max()
        .unwrap_or(1);

    if structural_count < 2 && max_tier < 4 {
        return None;
    }

    let average_rune_quality = if runes.is_empty() {
        0.0
    } else {
        runes.iter().map(|rune| rune.quality).sum::<f32>() / runes.len() as f32
    };
    let ring_score = diminishing_count(ring_count, 3);
    let satellite_score = diminishing_count(satellite_count, 5);
    let radial_score = diminishing_count(radial_count, 4);
    let perimeter_score = diminishing_count(perimeter_mark_count, 14);
    let script_score = diminishing_count(script_mark_count, 28);
    let rune_count_score = diminishing_count(runes.len(), 6);
    let tier_score = (max_tier as f32 / 4.0).clamp(0.25, 1.0);
    let structural_quality = structural_quality(marks);
    let satellite_placement = satellite_placement(marks);
    let symmetry = circular_symmetry(marks, runes, reading);
    let size_harmony = size_harmony(runes, rune_defs);
    let duplicate_effects = duplicate_effect_count(runes, rune_defs);
    let modifier_count = runes
        .iter()
        .filter(|rune| {
            rune_def(rune_defs, &rune.rune_id)
                .is_some_and(|def| def.category == RuneCategory::Modifier)
        })
        .count();

    let complexity = (circle_quality * 0.12
        + average_rune_quality * 0.12
        + rune_count_score * 0.10
        + tier_score * 0.12
        + ring_score * 0.16
        + satellite_score * 0.14
        + radial_score * 0.08
        + perimeter_score * 0.10
        + script_score * 0.07
        + structural_quality * 0.06
        + satellite_placement * 0.06
        + symmetry * 0.08
        + size_harmony * 0.06)
        .clamp(0.0, 1.0);
    let containment = (circle_quality * 0.30
        + ring_score * 0.22
        + perimeter_score * 0.15
        + script_score * 0.05
        + structural_quality * 0.12
        + symmetry * 0.21)
        .clamp(0.0, 1.0);
    let effect_scale = dominant_effect_scale(runes, rune_defs);
    let intensity = (tier_score * 0.24
        + average_rune_quality * 0.20
        + effect_scale * 1.35
        + satellite_score * 0.16
        + satellite_placement * 0.08
        + radial_score * 0.12
        + script_score * 0.04
        + duplicate_effects as f32 * 0.08)
        .clamp(0.0, 1.25);

    let tier_rank = if max_tier >= 4
        && complexity >= 0.72
        && ring_count >= 2
        && satellite_count >= 3
        && perimeter_mark_count >= 6
    {
        4
    } else if complexity >= 0.61 || max_tier >= 4 {
        3
    } else if complexity >= 0.48 {
        2
    } else {
        1
    };

    let dominant_effect = dominant_effect(runes, rune_defs);
    let name = spell_name(dominant_effect.as_deref(), tier_rank, rune_defs);
    let high_tier = if tier_rank >= 4 { 1 } else { 0 };
    let score_bonus =
        (complexity * 28.0 + symmetry * 8.0 + (tier_rank.saturating_sub(1) * 4) as f32).round()
            as i32
            + high_tier * 10;
    let power_bonus =
        (intensity * 24.0).round() as i32 + duplicate_effects as i32 * 4 + high_tier * 8;
    let stability_bonus = (containment * 30.0 + ring_count as f32 * 2.0 - intensity * 7.0).round()
        as i32
        + high_tier * 8;
    let mana_cost_delta = ring_count as i32 * 3
        + satellite_count as i32
        + radial_count as i32 * 2
        + perimeter_mark_count as i32 / 5
        + script_mark_count as i32 / 8
        + tier_rank as i32 * 2;
    let safety_bonus = (containment * 24.0
        + modifier_count as f32 * 4.0
        + ring_count as f32 * 2.0
        + satellite_count as f32
        - intensity * 8.0)
        .round() as i32
        + high_tier * 5;

    Some(MagicalCircleSpell {
        name,
        tier_rank,
        complexity,
        intensity,
        containment,
        symmetry,
        size_harmony,
        ring_count,
        satellite_count,
        radial_count,
        perimeter_mark_count,
        script_mark_count,
        dominant_effect,
        score_bonus,
        power_bonus,
        stability_bonus,
        mana_cost_delta,
        safety_bonus,
    })
}

fn count_kind(marks: &[CircleMark], kind: CircleStrokeKind) -> usize {
    marks.iter().filter(|mark| mark.kind == kind).count()
}

/// Structural-count score with diminishing but never-capped returns (plan Phase 4 item 1 / C6) —
/// replaces the old `ratio_count(count, target) = (count/target).clamp(0,1)`, which saturated at
/// 1.0 exactly at `target` and contributed nothing beyond it, so a 100-symbol diagram could never
/// out-score a well-drawn 30-symbol one no matter how much more structure it carried. This curve
/// approaches 1.0 asymptotically without ever reaching it: `count / (count + target * 0.4)` is
/// ~0.71 at `count == target` (close to the old curve's saturated 1.0, to limit how much tier
/// thresholds tuned against the old curve need retuning) and keeps climbing, sub-linearly, past
/// it — `0.83` at `2*target`, `0.88` at `3*target`, and so on.
pub(crate) fn diminishing_count(count: usize, target: usize) -> f32 {
    let count = count as f32;
    let target = (target.max(1) as f32) * 0.4;
    count / (count + target)
}

fn structural_quality(marks: &[CircleMark]) -> f32 {
    let structural = marks
        .iter()
        .filter(|mark| mark.kind != CircleStrokeKind::RuneInk)
        .collect::<Vec<_>>();
    if structural.is_empty() {
        0.0
    } else {
        structural.iter().map(|mark| mark.quality).sum::<f32>() / structural.len() as f32
    }
}

fn satellite_placement(marks: &[CircleMark]) -> f32 {
    let satellites = marks
        .iter()
        .filter(|mark| mark.kind == CircleStrokeKind::SatelliteSeal)
        .collect::<Vec<_>>();
    if satellites.is_empty() {
        return 0.0;
    }
    let total = satellites
        .iter()
        .map(|mark| {
            let size_score = ratio_score(mark.scale.max(0.01), 0.11);
            let orbit_score = (1.0 - (mark.orbit - 0.62).abs() / 0.34).clamp(0.0, 1.0);
            size_score * 0.46 + orbit_score * 0.54
        })
        .sum::<f32>();
    (total / satellites.len() as f32).clamp(0.0, 1.0)
}

fn rune_def<'a>(rune_defs: &[&'a RuneDef], rune_id: &str) -> Option<&'a RuneDef> {
    rune_defs.iter().copied().find(|rune| rune.id == rune_id)
}

/// Balance of the diagram: anchors whose directions cancel read as even.
///
/// Anchors are *groups*, not individual marks (`.project/placement-rules.md`
/// §5.1). Marks drawn together are one thing the eye sees in one place, so
/// three tight pairs spaced evenly are as balanced as six spread marks — and
/// using the placement grammar no longer costs the symmetry bonus, which would
/// have been exactly backwards.
fn circular_symmetry(
    marks: &[CircleMark],
    runes: &[InterpretedRune],
    reading: &crate::reading::Reading,
) -> f32 {
    let mut anchors = marks
        .iter()
        .filter(|mark| {
            matches!(
                mark.kind,
                CircleStrokeKind::SatelliteSeal | CircleStrokeKind::RadialSpoke
            )
        })
        .map(|mark| mark.angle)
        .collect::<Vec<_>>();
    let rune_angles = runes
        .iter()
        .map(|rune| angle_from_unit_center(rune.center))
        .collect::<Vec<_>>();
    let grouped = reading.group_angles(&rune_angles);
    if grouped.is_empty() {
        anchors.extend(
            runes
                .iter()
                .filter(|rune| rune.orbit > 0.18)
                .map(|rune| angle_from_unit_center(rune.center)),
        );
    } else {
        anchors.extend(grouped);
    }
    if anchors.len() < 2 {
        return 0.42;
    }
    let (x, y) = anchors.iter().fold((0.0, 0.0), |(x, y), angle| {
        (x + angle.cos(), y + angle.sin())
    });
    let balance = 1.0 - (x * x + y * y).sqrt() / anchors.len() as f32;
    let count_score = diminishing_count(anchors.len(), 8);
    (balance * 0.72 + count_score * 0.28).clamp(0.0, 1.0)
}

/// How well the marks are *sized*: each one measured against its category's
/// reference scale.
///
/// This used to carry a second, quieter opinion about where each mark sat —
/// shapes rewarded near the center, modifiers around orbit 0.55, everything
/// else around 0.42. That is gone (`.project/placement-rules.md` §5.3). Orbit
/// now carries *meaning* rather than quality: a shape at the heart shapes the
/// whole working and a shape drawn against one effect shapes only that effect,
/// and both are deliberate. Scoring the second one lower would have quietly
/// charged the player for using the grammar — exactly the silent position
/// penalty `prd.md` §4.4 deleted, grown back in a different place.
fn size_harmony(runes: &[InterpretedRune], rune_defs: &[&RuneDef]) -> f32 {
    if runes.is_empty() {
        return 0.0;
    }
    let mut total = 0.0;
    for rune in runes {
        let Some(def) = rune_def(rune_defs, &rune.rune_id) else {
            continue;
        };
        total += ratio_score(rune.scale.max(0.01), def.category.ideal_scale_in_circle());
    }
    (total / runes.len() as f32).clamp(0.0, 1.0)
}

fn duplicate_effect_count(runes: &[InterpretedRune], rune_defs: &[&RuneDef]) -> usize {
    let mut counts = HashMap::<&str, usize>::new();
    for rune in runes {
        if rune_def(rune_defs, &rune.rune_id)
            .is_some_and(|def| def.category == RuneCategory::Effect)
        {
            *counts.entry(rune.rune_id.as_str()).or_default() += 1;
        }
    }
    counts.values().map(|count| count.saturating_sub(1)).sum()
}

fn dominant_effect(runes: &[InterpretedRune], rune_defs: &[&RuneDef]) -> Option<String> {
    runes
        .iter()
        .filter_map(|rune| {
            let def = rune_def(rune_defs, &rune.rune_id)?;
            (def.category == RuneCategory::Effect).then_some((
                rune.rune_id.as_str(),
                def.tier as f32 + rune.quality + rune.scale,
            ))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(id, _)| id.to_owned())
}

fn dominant_effect_scale(runes: &[InterpretedRune], rune_defs: &[&RuneDef]) -> f32 {
    runes
        .iter()
        .filter(|rune| {
            rune_def(rune_defs, &rune.rune_id)
                .is_some_and(|def| def.category == RuneCategory::Effect)
        })
        .map(|rune| rune.scale)
        .fold(0.0, f32::max)
        .clamp(0.0, 0.35)
}

/// Generic, un-named fallback title — every *named* spell (the old hardcoded "Aegis of the
/// Floating City" / "Gravity Well" / etc. arms) is now a data-defined recipe instead
/// (`assets/data/recipes.json`, matched by `crate::recipes::match_recipe` in `evaluate()`, which
/// takes priority over this when a recipe matches). This is what's left for any diagram that
/// doesn't match a named recipe — plan Phase 4 item 3's "no code special-case" goal.
fn spell_name(dominant_effect: Option<&str>, tier_rank: u32, rune_defs: &[&RuneDef]) -> String {
    let prefix = match tier_rank {
        4.. => "Grand",
        3 => "High",
        2 => "Woven",
        _ => "Simple",
    };
    match dominant_effect {
        Some(effect) => {
            let name = rune_def(rune_defs, effect)
                .map(|rune| rune.name.as_str())
                .unwrap_or("Working");
            format!("{prefix} Circle of {name}")
        }
        None => format!("{prefix} Unbound Circle"),
    }
}

fn radial_score(stroke: &DrawnStroke, circle: CircleBounds) -> f32 {
    let Some(start) = stroke.points.first().copied() else {
        return 0.0;
    };
    let Some(end) = stroke.points.last().copied() else {
        return 0.0;
    };
    let directness = distance(start, end) / stroke_length(&stroke.points).max(0.001);
    let length_ratio = distance(start, end) / average_radius(circle).max(0.001);
    let start_orbit = normalized_orbit(start, circle);
    let end_orbit = normalized_orbit(end, circle);
    let reaches_center = start_orbit.min(end_orbit) <= 0.20;
    let reaches_ring = start_orbit.max(end_orbit) >= 0.48;
    if directness > 0.86 && length_ratio > 0.42 && reaches_center && reaches_ring {
        ((directness - 0.86) / 0.14 * 0.45 + (length_ratio - 0.42).min(0.58) / 0.58 * 0.55)
            .clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn normalized_orbit(point: StrokePoint, circle: CircleBounds) -> f32 {
    let center = circle.center();
    let rx = (circle.width() * 0.5).max(0.001);
    let ry = (circle.height() * 0.5).max(0.001);
    let x = (point.x - center.x) / rx;
    let y = (point.y - center.y) / ry;
    (x * x + y * y).sqrt()
}

fn angle_from_circle(point: StrokePoint, circle: CircleBounds) -> f32 {
    let center = circle.center();
    (point.y - center.y).atan2(point.x - center.x)
}

fn angle_from_unit_center(point: StrokePoint) -> f32 {
    (point.y - 0.5).atan2(point.x - 0.5)
}

fn average_radius(circle: CircleBounds) -> f32 {
    (circle.width() + circle.height()) * 0.25
}

fn is_closed_stroke(stroke: &DrawnStroke, bounds: CircleBounds) -> bool {
    let Some(first) = stroke.points.first().copied() else {
        return false;
    };
    let Some(last) = stroke.points.last().copied() else {
        return false;
    };
    let span = bounds.width().max(bounds.height()).max(0.001);
    stroke.points.len() >= 5 && distance(first, last) <= (span * 0.24).max(0.025)
}

fn radius_consistency(points: &[StrokePoint], center: StrokePoint) -> f32 {
    let radii = points
        .iter()
        .map(|point| distance(*point, center))
        .filter(|radius| *radius > 0.001)
        .collect::<Vec<_>>();
    if radii.is_empty() {
        return 0.0;
    }
    let mean = radii.iter().sum::<f32>() / radii.len() as f32;
    let variance = radii
        .iter()
        .map(|radius| {
            let delta = radius - mean;
            delta * delta
        })
        .sum::<f32>()
        / radii.len() as f32;
    (1.0 - variance.sqrt() / (mean * 0.46).max(0.001)).clamp(0.0, 1.0)
}

fn stroke_length(points: &[StrokePoint]) -> f32 {
    points
        .windows(2)
        .map(|segment| distance(segment[0], segment[1]))
        .sum()
}

fn ratio_score(candidate: f32, template: f32) -> f32 {
    let candidate = candidate.max(0.001);
    let template = template.max(0.001);
    (1.0 - (candidate / template).ln().abs() / 1.45).clamp(0.0, 1.0)
}

fn distance(a: StrokePoint, b: StrokePoint) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

fn percent(value: f32) -> i32 {
    (value.clamp(0.0, 1.0) * 100.0).round() as i32
}
