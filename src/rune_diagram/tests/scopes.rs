//! Scope structure: the circle spell a diagram's marks add up to, the
//! reference frame a nested ring gives its own ink, and the recipes a scope
//! tree matches against.

use super::fixtures::*;
use crate::data::GameData;
use crate::rune_diagram::interpret_diagram;
use crate::rune_drawing::DrawnStroke;

#[test]
fn interprets_high_tier_structured_circle_spell() {
    let data = GameData::load().unwrap();
    let interpretation = interpret_diagram(&high_tier_city_circle(), all_runes(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();
    let spell = interpretation.spell.as_ref().unwrap();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"gravity"), "{ids:?}");
    assert!(ids.contains(&"sphere"), "{ids:?}");
    assert!(ids.contains(&"continuous"), "{ids:?}");
    assert_eq!(spell.dominant_effect.as_deref(), Some("gravity"));
    assert_eq!(spell.tier_rank, 4, "{spell:?}");
    assert!(spell.complexity >= 0.72, "{spell:?}");
    assert!(spell.ring_count >= 4, "{spell:?}");
    assert!(spell.satellite_count >= 6, "{spell:?}");
    assert!(spell.perimeter_mark_count >= 32, "{spell:?}");
    assert!(spell.script_mark_count >= 32, "{spell:?}");
}

#[test]
fn nested_off_center_ring_reads_its_own_effect_rune_relative_to_its_own_scope() {
    // Phase 3 item 2 exit criterion: an off-center sub-circle (a "vent" in a volcano-style
    // diagram) enclosing its own ink is interpreted as its own scope, so the rune inside it is
    // scored relative to the *sub-circle*, not the outer working circle. Drawn at 0.16 slate
    // units, "light" reads at roughly reference size (scale ~0.19, ratio ~1.0) against the huge
    // outer circle but oversized (scale ~0.5+, ratio ~2.9) against the much smaller sub-circle —
    // so a large reported scale/potency here is direct evidence recursion, not the outer circle,
    // supplied the reference frame.
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(rough_circle(0.30, 0.30, 0.15, 0.14, 24));
    strokes.extend(template_at("light", 0.30, 0.30, 0.16));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let light = interpretation
        .runes
        .iter()
        .find(|rune| rune.rune_id == "light");

    assert!(interpretation.accepted(), "{interpretation:?}");
    let light = light.unwrap_or_else(|| panic!("{interpretation:?}"));
    assert!(light.scale > 0.35, "{light:?}");
    assert!(light.potency > 1.5, "{light:?}");
}

#[test]
fn fireball_recipe_recognized_purely_from_data() {
    // Plan Phase 4 exit criterion: "fireball ... defined purely in data" — one scope, no
    // sub-scopes, matched against `assets/data/recipes.json` with no rune-id-specific Rust
    // involved in the naming decision.
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("fire", 0.50, 0.30, 0.20));
    strokes.extend(template_at("sphere", 0.28, 0.62, 0.18));
    strokes.extend(template_at("on_command", 0.72, 0.62, 0.16));

    let interpretation = interpret_diagram(&strokes, all_runes(&data));
    assert!(interpretation.accepted(), "{interpretation:?}");
    let tree = interpretation.scope_spell.as_ref().expect("scope_spell");
    let matched = crate::recipes::match_recipe(tree, &data.recipes).unwrap_or_else(|| {
        panic!(
            "no recipe matched: tree={tree:?} runes={:?}",
            interpretation.runes
        )
    });

    assert_eq!(matched.id, "fireball", "{tree:?}");
}

#[test]
fn volcano_recipe_recognized_purely_from_data() {
    // Plan Phase 4 exit criterion: "volcano ... defined purely in data" — a root scope
    // (fire + cone + continuous + rings + satellites) enclosing two off-center vent sub-scopes
    // (each its own force + fire), matched against `assets/data/recipes.json` with no
    // rune-id-specific Rust involved. Also exercises `ScopeSpell::total_potency` summing across
    // the tree, since `volcano`'s `min_potency` is only reached by root + vent fire combined.
    let data = GameData::load().unwrap();
    let strokes = volcano_diagram();

    let interpretation = interpret_diagram(&strokes, all_runes(&data));
    assert!(interpretation.accepted(), "{interpretation:?}");
    let tree = interpretation.scope_spell.as_ref().expect("scope_spell");
    assert_eq!(
        tree.sub_scopes.len(),
        2,
        "{tree:?} runes={:?}",
        interpretation.runes
    );

    let matched = crate::recipes::match_recipe(tree, &data.recipes).unwrap_or_else(|| {
        panic!(
            "no recipe matched: tree={tree:?} runes={:?}",
            interpretation.runes
        )
    });
    assert_eq!(matched.id, "volcano", "{tree:?}");
}

fn volcano_diagram() -> Vec<DrawnStroke> {
    let mut strokes = outer_circle();
    strokes.extend(template_at("fire", 0.50, 0.24, 0.28));
    strokes.extend(template_at("continuous", 0.18, 0.50, 0.16));
    strokes.extend(template_at("cone", 0.50, 0.86, 0.16));
    // Concentric reinforcement rings, near the center — proven safe from being reinterpreted
    // as their own sub-scopes (orbit ~0, below `NESTED_RING_MIN_ORBIT`).
    strokes.extend(rough_circle(0.50, 0.50, 0.14, 0.133, 32));
    strokes.extend(rough_circle(0.50, 0.50, 0.125, 0.119, 28));
    // 3 satellite seals, clustered on the right so they stay clear of the rings (orbit floor)
    // and the two vents (below).
    strokes.extend(rough_circle(0.74, 0.50, 0.045, 0.0414, 16));
    strokes.extend(rough_circle(0.7175, 0.6015, 0.045, 0.0414, 16));
    strokes.extend(rough_circle(0.7175, 0.3985, 0.045, 0.0414, 16));
    strokes.extend(vent(0.22, 0.78));
    strokes.extend(vent(0.78, 0.78));
    strokes
}

/// One volcano vent: a ring enclosing its own `force` and `fire` runes, offset from each other
/// so they stay separate clusters.
fn vent(cx: f32, cy: f32) -> Vec<DrawnStroke> {
    let mut strokes = rough_circle(cx, cy, 0.125, 0.119, 32);
    strokes.extend(template_at("force", cx - 0.07, cy, 0.17));
    strokes.extend(template_at("fire", cx + 0.07, cy, 0.17));
    strokes
}
