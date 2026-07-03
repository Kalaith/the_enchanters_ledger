use super::EnchantGrade;
use crate::data::{CommissionDef, GameData, RuneCategory, RuneDef};
use std::collections::HashMap;

pub(super) fn result_title(
    data: &GameData,
    commission: &CommissionDef,
    by_category: &HashMap<RuneCategory, Vec<&RuneDef>>,
    grade: EnchantGrade,
) -> String {
    let effect = by_category
        .get(&RuneCategory::Effect)
        .and_then(|runes| runes.first())
        .map(|rune| rune.name.as_str())
        .unwrap_or("Uncertain");
    let modifier = by_category
        .get(&RuneCategory::Modifier)
        .and_then(|runes| runes.first())
        .map(|rune| rune.name.as_str());

    match grade {
        EnchantGrade::Brilliant => match modifier {
            Some(modifier) => format!(
                "{} of {} {}",
                title_case(&commission.item),
                modifier,
                effect
            ),
            None => format!("Perfect {} of {}", title_case(&commission.item), effect),
        },
        EnchantGrade::Reliable => format!("{} of {}", title_case(&commission.item), effect),
        EnchantGrade::Unstable => format!("Volatile {} of {}", commission.item, effect),
        EnchantGrade::Failed => {
            let required = data.rune_name(&commission.required_effect);
            format!("{} of Mild {}", title_case(&commission.item), required)
        }
    }
}

/// A cause-specific backfire message (plan Phase 4 item 4): "failures teach the grammar" — when
/// something concrete and nameable went wrong (uncontained potency, a specific missing
/// requirement, conflicting duplicate effects), say so instead of the generic grade-based
/// message `side_effect` falls back to. Priority: an accident already gets its own message from
/// `side_effect` (returns `None` here so callers fall through to it, avoiding a double message);
/// then uncontained potency; then a specifically-named missing requirement; then duplicate
/// effects. Returns `None` when none of these apply, so callers should fall back to `side_effect`.
pub(super) fn backfire_cause(
    data: &GameData,
    accident: bool,
    missing_requirement: Option<(&str, &str)>,
    potency_excess: f32,
    duplicate_effect_count: usize,
) -> Option<String> {
    if accident {
        return None;
    }
    if potency_excess > 0.5 {
        return Some(format!(
            "The working circle can't hold everything drawn — about {:.1} potency worth of \
             effect spills past the rings and wards, and the margins hiss with the overflow.",
            potency_excess
        ));
    }
    if let Some((kind, rune_id)) = missing_requirement {
        return Some(format!(
            "The diagram never drew a {kind} — the commission calls for {}.",
            data.rune_name(rune_id)
        ));
    }
    if duplicate_effect_count > 0 {
        return Some(
            "Two effect runes compete for the same working circle, and their conflicting pulls \
             bleed mana into waste."
                .to_owned(),
        );
    }
    None
}

pub(super) fn side_effect(
    data: &GameData,
    commission: &CommissionDef,
    grade: EnchantGrade,
    matched_request: bool,
    accident: bool,
    average_quality: f32,
    weak_marks: i32,
) -> String {
    if !matched_request {
        return format!(
            "The diagram misses part of the request for {} + {} + {}.",
            data.rune_name(&commission.required_effect),
            data.rune_name(&commission.required_shape),
            data.rune_name(&commission.required_trigger)
        );
    }
    if accident {
        return "The test bench coughs up smoke and the ink keeps glowing after you lift the quill."
            .to_owned();
    }
    if weak_marks > 0 || average_quality < 0.68 {
        return "The rune identities hold, but ragged strokes leak value into heat, hum, and wasted mana."
            .to_owned();
    }
    match grade {
        EnchantGrade::Brilliant => {
            "Clean output, tidy mana draw, and a margin note worth publishing.".to_owned()
        }
        EnchantGrade::Reliable => {
            "The enchantment works as commissioned and should hold under normal use.".to_owned()
        }
        EnchantGrade::Unstable => {
            "It works, but the margins hiss whenever someone says the activation phrase.".to_owned()
        }
        EnchantGrade::Failed => {
            "The result is more conversation piece than enchantment.".to_owned()
        }
    }
}

pub(super) fn percent(value: f32) -> i32 {
    (value.clamp(0.0, 1.0) * 100.0).round() as i32
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
