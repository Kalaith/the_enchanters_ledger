//! Data-defined spell recipes (plan Phase 4 item 3): named spells are predicates evaluated
//! against a diagram's `rune_diagram::ScopeSpell` tree, not hand-written Rust `match` arms — see
//! `assets/data/recipes.json` for the actual roster and prd.md's Phase 4 section for the schema.

use crate::data::{RecipeDef, RecipeRequirements};
use crate::rune_diagram::ScopeSpell;

/// A rune counts as "present" once its total potency clears this — every recognized rune's
/// potency is clamped to `[0.35, 2.2]` (prd.md §4.2), so any real occurrence clears it easily;
/// this only exists to tell "never drawn" (`total_potency == 0.0`) apart from "drawn, but an
/// explicit `min_potency` requirement wasn't met".
const MIN_EFFECT_PRESENCE: f32 = 0.001;

/// The best-matching recipe for `tree`, or `None` if nothing matches. Among matches, the highest
/// `tier` wins (a more specific/elaborate recipe should always be named over a broader one it
/// happens to also satisfy — see `assets/data/recipes.json`'s `floating_city` vs. `gravity_well`
/// for why every recipe roster needs to keep this invariant: a strictly-more-specific recipe
/// should carry a strictly-higher tier); ties break on the lower recipe id, for determinism.
pub fn match_recipe<'a>(tree: &ScopeSpell, recipes: &'a [RecipeDef]) -> Option<&'a RecipeDef> {
    recipes
        .iter()
        .filter(|recipe| requirements_met(tree, &recipe.requires))
        .max_by(|a, b| a.tier.cmp(&b.tier).then_with(|| b.id.cmp(&a.id)))
}

fn requirements_met(tree: &ScopeSpell, requires: &RecipeRequirements) -> bool {
    requires.effect.iter().all(|(id, requirement)| {
        tree.total_potency(id) >= requirement.min_potency.max(MIN_EFFECT_PRESENCE)
    }) && requires
        .shape
        .as_deref()
        .is_none_or(|id| tree.shape.as_deref() == Some(id))
        && requires
            .trigger
            .as_deref()
            .is_none_or(|id| tree.trigger.as_deref() == Some(id))
        && requires
            .modifier
            .as_deref()
            .is_none_or(|id| tree.modifiers.iter().any(|modifier| modifier == id))
        && tree.ring_count >= requires.structure.rings
        && tree.satellite_count >= requires.structure.satellites
        && tree.radial_count >= requires.structure.radials
        && tree.perimeter_mark_count >= requires.structure.perimeter
        && tree.script_mark_count >= requires.structure.scripts
        && requires.sub_scopes.iter().all(|sub_requirement| {
            let matching = tree
                .sub_scopes
                .iter()
                .filter(|sub| {
                    sub.effects
                        .iter()
                        .any(|(id, _)| id == &sub_requirement.effect)
                })
                .count();
            matching >= sub_requirement.count
        })
}

#[cfg(test)]
mod tests {
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
            sub_scopes: vec![scope_with_effect("fire", 1.0), scope_with_effect("fire", 1.0)],
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
            effect: HashMap::from([(
                "fire".to_owned(),
                EffectRequirement { min_potency: 2.0 },
            )]),
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
}
