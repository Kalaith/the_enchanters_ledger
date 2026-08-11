use super::{DrawnStroke, StrokePoint};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

const RUNE_TEMPLATES_JSON: &str =
    macroquad_toolkit::include_json_str!("../../assets/data/rune_templates.json");

#[derive(Debug, Deserialize)]
struct TemplateEntry {
    id: String,
    strokes: Vec<Vec<(f32, f32)>>,
    #[serde(default)]
    variants: Vec<Vec<Vec<(f32, f32)>>>,
    /// Declarative structural profile (plan Phase 1 item 3) — evaluated by
    /// the generic checker in `shape.rs`; runes without one are scored by
    /// template matching alone.
    #[serde(default)]
    structure: Option<super::shape::StructureSpec>,
    /// Runes this one is known to sit geometrically close to. Consumed by
    /// the confusion-matrix gate (test-only) as its tracked-confusion
    /// allowlist.
    #[serde(default)]
    #[cfg_attr(not(test), allow(dead_code))]
    confusable_with: Vec<String>,
}

/// Rune shape data — one canonical stroke layout per rune, any extra
/// accepted stroke layouts ("variants"), and the rune's declarative
/// structural profile — all live in `assets/data/rune_templates.json`, not
/// in this file. Adding a rune, including one with structural character, is
/// a JSON edit; no recognition code branches on rune ids.
fn template_table() -> &'static HashMap<String, TemplateEntry> {
    static TABLE: OnceLock<HashMap<String, TemplateEntry>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let entries: Vec<TemplateEntry> = serde_json::from_str(RUNE_TEMPLATES_JSON)
            .expect("assets/data/rune_templates.json should be valid");
        entries
            .into_iter()
            .map(|entry| (entry.id.clone(), entry))
            .collect()
    })
}

/// Every rune id that has template data — the id set both normalized-template
/// caches (identity scoring's and strict quality's) are prebuilt over.
pub(crate) fn all_template_rune_ids() -> impl Iterator<Item = &'static str> {
    template_table().keys().map(String::as_str)
}

/// The rune's declared structural profile, if it has one.
pub(crate) fn structure_spec_for_rune(
    rune_id: &str,
) -> Option<&'static super::shape::StructureSpec> {
    template_table().get(rune_id)?.structure.as_ref()
}

/// All (truth, confusable) pairs declared in the template data — the
/// confusion gate's tracked-confusion allowlist, kept in data next to the
/// runes it describes instead of hardcoded in test code.
#[cfg(test)]
pub(crate) fn known_confusions() -> Vec<(&'static str, &'static str)> {
    let mut pairs = template_table()
        .values()
        .flat_map(|entry| {
            entry
                .confusable_with
                .iter()
                .map(|other| (entry.id.as_str(), other.as_str()))
        })
        .collect::<Vec<_>>();
    pairs.sort_unstable();
    pairs
}

pub(crate) fn template_variants_for_rune(rune_id: &str) -> Vec<Vec<DrawnStroke>> {
    let Some(entry) = template_table().get(rune_id) else {
        return Vec::new();
    };
    std::iter::once(strokes_from_points(&entry.strokes))
        .chain(
            entry
                .variants
                .iter()
                .map(|variant| strokes_from_points(variant)),
        )
        .collect()
}

pub fn template_strokes_for_rune(rune_id: &str) -> Option<Vec<DrawnStroke>> {
    template_table()
        .get(rune_id)
        .map(|entry| strokes_from_points(&entry.strokes))
}

fn strokes_from_points(strokes: &[Vec<(f32, f32)>]) -> Vec<DrawnStroke> {
    strokes
        .iter()
        .map(|points| DrawnStroke {
            points: points
                .iter()
                .map(|(x, y)| StrokePoint::new(*x, *y))
                .collect(),
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn raw(strokes: &[&[(f32, f32)]]) -> Vec<DrawnStroke> {
    strokes
        .iter()
        .map(|stroke| DrawnStroke {
            points: stroke
                .iter()
                .map(|(x, y)| StrokePoint::new(*x, *y))
                .collect(),
        })
        .collect()
}
