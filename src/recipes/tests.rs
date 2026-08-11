use super::*;
use crate::data::{EffectRequirement, StructureRequirement, SubScopeRequirement};
use std::collections::HashMap;

fn recipe(id: &str, tier: u32, requires: RecipeRequirements) -> RecipeDef {
    RecipeDef {
        id: id.to_owned(),
        name: id.to_owned(),
        tier,
        requires,
    }
}

fn effect_req(id: &str, min_potency: f32) -> RecipeRequirements {
    RecipeRequirements {
        effect: HashMap::from([(id.to_owned(), EffectRequirement { min_potency })]),
        ..Default::default()
    }
}

fn scope_with_effect(id: &str, potency: f32) -> ScopeSpell {
    ScopeSpell {
        effects: vec![(id.to_owned(), potency)],
        ..Default::default()
    }
}

#[test]
fn bare_effect_presence_matches_regardless_of_potency() {
    let tree = scope_with_effect("gravity", 0.4);
    let recipes = [recipe("gravity_well", 1, effect_req("gravity", 0.0))];

    assert!(match_recipe(&tree, &recipes).is_some());
}

#[test]
fn absent_effect_does_not_match() {
    let tree = scope_with_effect("light", 1.0);
    let recipes = [recipe("gravity_well", 1, effect_req("gravity", 0.0))];

    assert!(match_recipe(&tree, &recipes).is_none());
}

#[test]
fn min_potency_requirement_rejects_a_weak_effect() {
    let tree = scope_with_effect("fire", 1.0);
    let recipes = [recipe("fireball", 1, effect_req("fire", 2.0))];

    assert!(match_recipe(&tree, &recipes).is_none());
}

#[test]
fn potency_sums_across_sub_scopes_for_min_potency() {
    let root = ScopeSpell {
        effects: vec![("fire".to_owned(), 1.0)],
        sub_scopes: vec![
            scope_with_effect("fire", 1.0),
            scope_with_effect("fire", 1.0),
        ],
        ..Default::default()
    };
    let recipes = [recipe("hot", 1, effect_req("fire", 2.5))];

    assert!(match_recipe(&root, &recipes).is_some());
}

#[test]
fn higher_tier_recipe_wins_when_both_match() {
    let tree = ScopeSpell {
        effects: vec![("gravity".to_owned(), 1.0)],
        shape: Some("sphere".to_owned()),
        trigger: Some("continuous".to_owned()),
        ..Default::default()
    };
    let recipes = [
        recipe("gravity_well", 1, effect_req("gravity", 0.0)),
        recipe(
            "floating_city",
            4,
            RecipeRequirements {
                effect: HashMap::from([("gravity".to_owned(), EffectRequirement::default())]),
                shape: Some("sphere".to_owned()),
                trigger: Some("continuous".to_owned()),
                ..Default::default()
            },
        ),
    ];

    let matched = match_recipe(&tree, &recipes).unwrap();
    assert_eq!(matched.id, "floating_city");
}

#[test]
fn tie_breaks_on_lowest_id() {
    let tree = scope_with_effect("gravity", 1.0);
    let recipes = [
        recipe("zzz", 1, effect_req("gravity", 0.0)),
        recipe("aaa", 1, effect_req("gravity", 0.0)),
    ];

    let matched = match_recipe(&tree, &recipes).unwrap();
    assert_eq!(matched.id, "aaa");
}

#[test]
fn structure_and_sub_scope_requirements_gate_a_volcano_style_recipe() {
    let requires = RecipeRequirements {
        effect: HashMap::from([("fire".to_owned(), EffectRequirement { min_potency: 2.0 })]),
        shape: Some("cone".to_owned()),
        trigger: Some("continuous".to_owned()),
        structure: StructureRequirement {
            rings: 2,
            satellites: 3,
            ..Default::default()
        },
        sub_scopes: vec![SubScopeRequirement {
            effect: "force".to_owned(),
            count: 2,
        }],
        ..Default::default()
    };
    let recipes = [recipe("volcano", 4, requires)];

    let incomplete = ScopeSpell {
        effects: vec![("fire".to_owned(), 2.0)],
        shape: Some("cone".to_owned()),
        trigger: Some("continuous".to_owned()),
        ring_count: 2,
        satellite_count: 3,
        sub_scopes: vec![scope_with_effect("force", 1.0)],
        ..Default::default()
    };
    assert!(
        match_recipe(&incomplete, &recipes).is_none(),
        "only one force sub-scope should not satisfy count: 2"
    );

    let complete = ScopeSpell {
        sub_scopes: vec![
            scope_with_effect("force", 1.0),
            scope_with_effect("force", 1.0),
        ],
        ..incomplete
    };
    assert!(match_recipe(&complete, &recipes).is_some());
}
