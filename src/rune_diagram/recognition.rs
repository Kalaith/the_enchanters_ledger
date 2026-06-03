use super::geometry::{distance, StrokeBounds, StrokeCluster};
use super::{InterpretedRune, MIN_DIAGRAM_RUNE_CONFIDENCE};
use crate::data::{RuneCategory, RuneDef};
use crate::rune_drawing::{recognize_rune, RecognitionOutcome, StrokePoint};

pub(super) fn extract_overlapped_spheres(
    cluster: &StrokeCluster,
    available_runes: &[&RuneDef],
    circle_bounds: StrokeBounds,
    circle_quality: f32,
    interpreted: &mut Vec<InterpretedRune>,
    rejected_marks: &mut usize,
) -> bool {
    if cluster.strokes.len() <= 1 || !available_runes.iter().any(|rune| rune.id == "sphere") {
        return false;
    }

    let whole = recognize_rune(&cluster.strokes, available_runes.iter().copied());
    if whole
        .as_ref()
        .is_some_and(|recognized| recognized.accepted && recognized.rune_id == "healing")
    {
        return false;
    }

    let mut sphere_indices = Vec::new();
    for (index, stroke) in cluster.strokes.iter().enumerate() {
        let Some(bounds) = StrokeBounds::from_stroke(stroke) else {
            continue;
        };
        let Some(recognized) = recognize_rune(
            std::slice::from_ref(stroke),
            available_runes.iter().copied(),
        ) else {
            continue;
        };
        if recognized.rune_id == "sphere" && recognized.confidence >= MIN_DIAGRAM_RUNE_CONFIDENCE {
            push_recognized_rune(
                recognized,
                bounds,
                circle_bounds,
                circle_quality,
                available_runes,
                interpreted,
                rejected_marks,
            );
            sphere_indices.push(index);
        }
    }

    if sphere_indices.is_empty() {
        return false;
    }

    let remaining_groups = remaining_stroke_groups(cluster, &sphere_indices);
    if remaining_groups.is_empty() {
        return true;
    }

    for remaining in remaining_groups {
        let Some(bounds) = StrokeBounds::from_strokes(&remaining) else {
            *rejected_marks += 1;
            continue;
        };
        if let Some(recognized) = recognize_rune(&remaining, available_runes.iter().copied()) {
            push_recognized_rune(
                recognized,
                bounds,
                circle_bounds,
                circle_quality,
                available_runes,
                interpreted,
                rejected_marks,
            );
        } else {
            *rejected_marks += 1;
        }
    }

    true
}

fn remaining_stroke_groups(
    cluster: &StrokeCluster,
    removed_local_indices: &[usize],
) -> Vec<Vec<crate::rune_drawing::DrawnStroke>> {
    let mut groups = Vec::new();
    let mut current = Vec::new();
    let mut last_original_index = None;

    for (local_index, (original_index, stroke)) in
        cluster.indices.iter().zip(&cluster.strokes).enumerate()
    {
        if removed_local_indices.contains(&local_index) {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            last_original_index = None;
            continue;
        }
        if last_original_index.is_some_and(|last| *original_index != last + 1)
            && !current.is_empty()
        {
            groups.push(std::mem::take(&mut current));
        }
        current.push(stroke.clone());
        last_original_index = Some(*original_index);
    }

    if !current.is_empty() {
        groups.push(current);
    }
    groups
}

pub(super) fn push_recognized_rune(
    mut recognized: RecognitionOutcome,
    bounds: StrokeBounds,
    circle_bounds: StrokeBounds,
    circle_quality: f32,
    available_runes: &[&RuneDef],
    interpreted: &mut Vec<InterpretedRune>,
    rejected_marks: &mut usize,
) {
    if recognized.confidence < MIN_DIAGRAM_RUNE_CONFIDENCE {
        *rejected_marks += 1;
        return;
    }
    let center = bounds.center();
    let scale = bounds.scale_relative(circle_bounds);
    let orbit = normalized_orbit(center, circle_bounds);
    let category = available_runes
        .iter()
        .find(|rune| rune.id == recognized.rune_id)
        .map(|rune| rune.category);
    let layout = category.map_or(1.0, |category| {
        layout_quality(category, center, circle_bounds)
    });
    recognized.quality = (recognized.quality * circle_quality * layout).clamp(0.20, 1.0);
    interpreted.push(InterpretedRune {
        rune_id: recognized.rune_id,
        confidence: recognized.confidence,
        quality: recognized.quality,
        center,
        scale,
        orbit,
    });
}

fn layout_quality(category: RuneCategory, center: StrokePoint, circle_bounds: StrokeBounds) -> f32 {
    let relative = relative_to_circle(center, circle_bounds);
    let target = match category {
        RuneCategory::Effect => StrokePoint::new(0.30, 0.50),
        RuneCategory::Shape => StrokePoint::new(0.50, 0.50),
        RuneCategory::Trigger => StrokePoint::new(0.70, 0.50),
        RuneCategory::Modifier => StrokePoint::new(0.50, 0.72),
    };
    let score = (1.0 - distance(relative, target) / 0.48).clamp(0.0, 1.0);
    (0.76 + score * 0.24).clamp(0.0, 1.0)
}

fn relative_to_circle(center: StrokePoint, circle_bounds: StrokeBounds) -> StrokePoint {
    StrokePoint::new(
        (center.x - circle_bounds.min_x) / circle_bounds.width().max(0.001),
        (center.y - circle_bounds.min_y) / circle_bounds.height().max(0.001),
    )
}

fn normalized_orbit(center: StrokePoint, circle_bounds: StrokeBounds) -> f32 {
    let circle_center = circle_bounds.center();
    let rx = (circle_bounds.width() * 0.5).max(0.001);
    let ry = (circle_bounds.height() * 0.5).max(0.001);
    let nx = (center.x - circle_center.x) / rx;
    let ny = (center.y - circle_center.y) / ry;
    (nx * nx + ny * ny).sqrt()
}
