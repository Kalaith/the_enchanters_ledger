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
mod tests;
