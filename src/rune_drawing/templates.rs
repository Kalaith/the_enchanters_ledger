use super::{DrawnStroke, StrokePoint};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

const RUNE_TEMPLATES_JSON: &str = include_str!("../../assets/data/rune_templates.json");

#[derive(Debug, Deserialize)]
struct TemplateEntry {
    id: String,
    strokes: Vec<Vec<(f32, f32)>>,
    #[serde(default)]
    variants: Vec<Vec<Vec<(f32, f32)>>>,
}

/// Rune shape data — one canonical stroke layout per rune, plus any extra
/// accepted stroke layouts ("variants") — lives in
/// `assets/data/rune_templates.json`, not in this file. Adding a rune's
/// drawable shape is a JSON edit; only its *structural* checks (in
/// `shape.rs`, still bespoke per rune) need Rust.
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

pub(crate) fn template_variants_for_rune(rune_id: &str) -> Vec<Vec<DrawnStroke>> {
    let Some(entry) = template_table().get(rune_id) else {
        return Vec::new();
    };
    std::iter::once(strokes_from_points(&entry.strokes))
        .chain(entry.variants.iter().map(|variant| strokes_from_points(variant)))
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
