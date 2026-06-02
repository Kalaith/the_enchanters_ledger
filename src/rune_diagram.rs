//! Whole-diagram interpretation built on top of rune stroke recognition.

use crate::data::{RuneCategory, RuneDef};
use crate::magical_circle::{
    analyze_magical_circle, classify_circle_stroke, CircleBounds, CircleMark, CircleStrokeKind,
    MagicalCircleSpell,
};
use crate::rune_drawing::{recognize_rune, DrawnStroke, RecognitionOutcome, StrokePoint};
use serde::{Deserialize, Serialize};

pub const MIN_CIRCLE_QUALITY: f32 = 0.32;
pub const MIN_DIAGRAM_RUNE_CONFIDENCE: f32 = 0.32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DiagramInterpretation {
    pub circle_quality: f32,
    pub circle_found: bool,
    pub runes: Vec<InterpretedRune>,
    pub rejected_marks: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spell: Option<MagicalCircleSpell>,
}

impl DiagramInterpretation {
    pub fn accepted(&self) -> bool {
        self.circle_found && self.circle_quality >= MIN_CIRCLE_QUALITY && !self.runes.is_empty()
    }

    pub fn average_rune_quality(&self) -> f32 {
        if self.runes.is_empty() {
            0.0
        } else {
            self.runes.iter().map(|rune| rune.quality).sum::<f32>() / self.runes.len() as f32
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterpretedRune {
    pub rune_id: String,
    pub confidence: f32,
    pub quality: f32,
    pub center: StrokePoint,
    #[serde(default)]
    pub scale: f32,
    #[serde(default)]
    pub orbit: f32,
}

pub fn interpret_diagram<'a>(
    strokes: &[DrawnStroke],
    runes: impl IntoIterator<Item = &'a RuneDef>,
) -> DiagramInterpretation {
    let useful = strokes
        .iter()
        .enumerate()
        .filter(|(_, stroke)| stroke.has_ink())
        .collect::<Vec<_>>();

    let circle_candidates = useful
        .iter()
        .filter_map(|(index, stroke)| {
            let bounds = StrokeBounds::from_stroke(stroke)?;
            circle_quality(stroke, bounds).map(|score| (*index, score, bounds))
        })
        .collect::<Vec<_>>();
    let Some((circle_index, circle_quality, circle_bounds)) =
        select_working_circle(&circle_candidates)
    else {
        return DiagramInterpretation {
            circle_found: false,
            ..Default::default()
        };
    };

    let circle_found = circle_quality >= MIN_CIRCLE_QUALITY;
    let available_runes = runes.into_iter().collect::<Vec<_>>();
    let spell_bounds = CircleBounds::new(
        circle_bounds.min_x,
        circle_bounds.min_y,
        circle_bounds.max_x,
        circle_bounds.max_y,
    );
    let classified_marks = useful
        .iter()
        .filter(|(index, _)| *index != circle_index)
        .filter_map(|(index, stroke)| {
            classify_circle_stroke(stroke, spell_bounds).map(|mark| (*index, mark))
        })
        .collect::<Vec<_>>();
    let inner_strokes = useful
        .into_iter()
        .filter(|(index, stroke)| {
            *index != circle_index
                && is_inside_working_circle(stroke, circle_bounds)
                && !is_circle_structure(*index, &classified_marks)
        })
        .map(|(index, stroke)| (index, stroke.clone()))
        .collect::<Vec<_>>();

    let clusters = cluster_strokes(&inner_strokes);
    let mut interpreted = Vec::new();
    let mut rejected_marks = 0;
    for cluster in clusters {
        if extract_overlapped_spheres(
            &cluster,
            &available_runes,
            circle_bounds,
            circle_quality,
            &mut interpreted,
            &mut rejected_marks,
        ) {
            continue;
        }

        let Some(recognized) = recognize_rune(&cluster.strokes, available_runes.iter().copied())
        else {
            rejected_marks += 1;
            continue;
        };
        push_recognized_rune(
            recognized,
            cluster.bounds,
            circle_bounds,
            circle_quality,
            &available_runes,
            &mut interpreted,
            &mut rejected_marks,
        );
    }

    interpreted.sort_by(|a, b| {
        a.center
            .y
            .total_cmp(&b.center.y)
            .then(a.center.x.total_cmp(&b.center.x))
    });
    let circle_marks = classified_marks
        .into_iter()
        .map(|(_, mark)| mark)
        .collect::<Vec<_>>();
    let spell = if circle_found {
        analyze_magical_circle(
            circle_quality,
            &circle_marks,
            &interpreted,
            &available_runes,
        )
    } else {
        None
    };

    DiagramInterpretation {
        circle_quality,
        circle_found,
        runes: interpreted,
        rejected_marks,
        spell,
    }
}

pub(crate) fn select_working_circle(
    candidates: &[(usize, f32, StrokeBounds)],
) -> Option<(usize, f32, StrokeBounds)> {
    candidates
        .iter()
        .filter(|(_, quality, _)| *quality >= MIN_CIRCLE_QUALITY)
        .max_by(|a, b| {
            let a_span = a.2.width().max(a.2.height());
            let b_span = b.2.width().max(b.2.height());
            a_span.total_cmp(&b_span).then_with(|| a.1.total_cmp(&b.1))
        })
        .copied()
        .or_else(|| {
            candidates
                .iter()
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .copied()
        })
}

pub(crate) fn is_circle_structure(index: usize, marks: &[(usize, CircleMark)]) -> bool {
    let reinforcement_count = marks
        .iter()
        .filter(|(_, mark)| mark.kind == CircleStrokeKind::ReinforcementRing && mark.quality > 0.48)
        .count();
    let satellite_count = marks
        .iter()
        .filter(|(_, mark)| mark.kind == CircleStrokeKind::SatelliteSeal && mark.quality > 0.68)
        .count();
    let script_count = marks
        .iter()
        .filter(|(_, mark)| mark.kind == CircleStrokeKind::ScriptMark && mark.quality > 0.42)
        .count();
    let radial_count = marks
        .iter()
        .filter(|(_, mark)| mark.kind == CircleStrokeKind::RadialSpoke && mark.quality > 0.68)
        .count();
    marks.iter().any(|(mark_index, mark)| {
        *mark_index == index
            && if mark.kind == CircleStrokeKind::SatelliteSeal {
                satellite_count >= 3 && mark.quality > 0.68
            } else if mark.kind == CircleStrokeKind::ReinforcementRing {
                reinforcement_count >= 2 && mark.quality > 0.48
            } else if mark.kind == CircleStrokeKind::ScriptMark {
                script_count >= 8 && mark.quality > 0.42
            } else if mark.kind == CircleStrokeKind::RadialSpoke {
                radial_count >= 6 && mark.quality > 0.68
            } else {
                mark.kind.is_circle_structure()
            }
    })
}

fn extract_overlapped_spheres(
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

    let remaining = cluster
        .strokes
        .iter()
        .enumerate()
        .filter(|(index, _)| !sphere_indices.contains(index))
        .map(|(_, stroke)| stroke.clone())
        .collect::<Vec<_>>();
    if remaining.is_empty() {
        return true;
    }

    let Some(bounds) = StrokeBounds::from_strokes(&remaining) else {
        *rejected_marks += 1;
        return true;
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

    true
}

fn push_recognized_rune(
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

pub(crate) fn circle_quality(stroke: &DrawnStroke, bounds: StrokeBounds) -> Option<f32> {
    if stroke.points.len() < 8 {
        return None;
    }
    let width = bounds.width();
    let height = bounds.height();
    let span = width.max(height).max(0.001);
    if span < 0.22 || width < 0.15 || height < 0.15 {
        return None;
    }

    let closure = 1.0 - distance(stroke.points[0], *stroke.points.last()?) / (span * 0.50);
    let aspect = ratio_score(width / height.max(0.001), 1.0);
    let center = bounds.center();
    let center_score = 1.0 - distance(center, StrokePoint::new(0.5, 0.5)) / 0.62;
    let radius_score = radius_consistency(&stroke.points, center);
    let coverage = angle_coverage(&stroke.points, center);
    let top_start = circle_start_score(stroke.points[0], center, bounds);

    let score = closure.clamp(0.0, 1.0) * 0.32
        + aspect * 0.18
        + center_score.clamp(0.0, 1.0) * 0.04
        + radius_score * 0.22
        + coverage * 0.18
        + top_start * 0.08;
    Some(score.clamp(0.0, 1.0))
}

fn circle_start_score(start: StrokePoint, center: StrokePoint, bounds: StrokeBounds) -> f32 {
    let top = StrokePoint::new(center.x, bounds.min_y);
    let span = bounds.width().max(bounds.height()).max(0.001);
    (1.0 - distance(start, top) / (span * 0.80)).clamp(0.0, 1.0)
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

pub(crate) fn is_inside_working_circle(stroke: &DrawnStroke, circle_bounds: StrokeBounds) -> bool {
    let Some(bounds) = StrokeBounds::from_stroke(stroke) else {
        return false;
    };
    let center = bounds.center();
    let circle_center = circle_bounds.center();
    let rx = (circle_bounds.width() * 0.5).max(0.05);
    let ry = (circle_bounds.height() * 0.5).max(0.05);
    let nx = (center.x - circle_center.x) / rx;
    let ny = (center.y - circle_center.y) / ry;
    nx * nx + ny * ny <= 1.25
        && bounds.width() < circle_bounds.width() * 0.92
        && bounds.height() < circle_bounds.height() * 0.92
}

fn normalized_orbit(center: StrokePoint, circle_bounds: StrokeBounds) -> f32 {
    let circle_center = circle_bounds.center();
    let rx = (circle_bounds.width() * 0.5).max(0.001);
    let ry = (circle_bounds.height() * 0.5).max(0.001);
    let nx = (center.x - circle_center.x) / rx;
    let ny = (center.y - circle_center.y) / ry;
    (nx * nx + ny * ny).sqrt()
}

#[derive(Debug, Clone)]
pub(crate) struct StrokeCluster {
    pub(crate) indices: Vec<usize>,
    pub(crate) strokes: Vec<DrawnStroke>,
    pub(crate) bounds: StrokeBounds,
}

pub(crate) fn cluster_strokes(strokes: &[(usize, DrawnStroke)]) -> Vec<StrokeCluster> {
    let mut clusters = Vec::<StrokeCluster>::new();
    for (stroke_index, stroke) in strokes {
        let Some(bounds) = StrokeBounds::from_stroke(stroke) else {
            continue;
        };
        let mut target = None;
        for (index, cluster) in clusters.iter().enumerate() {
            if cluster
                .bounds
                .expanded(0.035)
                .intersects(&bounds.expanded(0.035))
                || distance(cluster.bounds.center(), bounds.center()) < 0.14
            {
                target = Some(index);
                break;
            }
        }

        if let Some(index) = target {
            clusters[index].bounds.include(&bounds);
            clusters[index].strokes.push(stroke.clone());
            clusters[index].indices.push(*stroke_index);
        } else {
            clusters.push(StrokeCluster {
                indices: vec![*stroke_index],
                strokes: vec![stroke.clone()],
                bounds,
            });
        }
    }
    clusters
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StrokeBounds {
    pub(crate) min_x: f32,
    pub(crate) min_y: f32,
    pub(crate) max_x: f32,
    pub(crate) max_y: f32,
}

impl StrokeBounds {
    pub(crate) fn from_stroke(stroke: &DrawnStroke) -> Option<Self> {
        let mut points = stroke.points.iter();
        let first = points.next()?;
        let mut bounds = Self {
            min_x: first.x,
            min_y: first.y,
            max_x: first.x,
            max_y: first.y,
        };
        for point in points {
            bounds.min_x = bounds.min_x.min(point.x);
            bounds.min_y = bounds.min_y.min(point.y);
            bounds.max_x = bounds.max_x.max(point.x);
            bounds.max_y = bounds.max_y.max(point.y);
        }
        Some(bounds)
    }

    fn from_strokes(strokes: &[DrawnStroke]) -> Option<Self> {
        let mut strokes = strokes.iter().filter_map(Self::from_stroke);
        let mut bounds = strokes.next()?;
        for stroke_bounds in strokes {
            bounds.include(&stroke_bounds);
        }
        Some(bounds)
    }

    pub(crate) fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    pub(crate) fn height(self) -> f32 {
        self.max_y - self.min_y
    }

    pub(crate) fn center(self) -> StrokePoint {
        StrokePoint::new(
            self.min_x + self.width() * 0.5,
            self.min_y + self.height() * 0.5,
        )
    }

    fn scale_relative(self, circle: Self) -> f32 {
        (self.width() / circle.width().max(0.001)).max(self.height() / circle.height().max(0.001))
    }

    fn expanded(self, amount: f32) -> Self {
        Self {
            min_x: self.min_x - amount,
            min_y: self.min_y - amount,
            max_x: self.max_x + amount,
            max_y: self.max_y + amount,
        }
    }

    fn intersects(self, other: &Self) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }

    fn include(&mut self, other: &Self) {
        self.min_x = self.min_x.min(other.min_x);
        self.min_y = self.min_y.min(other.min_y);
        self.max_x = self.max_x.max(other.max_x);
        self.max_y = self.max_y.max(other.max_y);
    }
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
    (1.0 - variance.sqrt() / (mean * 0.42).max(0.001)).clamp(0.0, 1.0)
}

fn angle_coverage(points: &[StrokePoint], center: StrokePoint) -> f32 {
    let mut bins = [false; 8];
    for point in points {
        let angle = (point.y - center.y).atan2(point.x - center.x);
        let normalized = (angle + std::f32::consts::TAU) % std::f32::consts::TAU;
        let index = ((normalized / std::f32::consts::TAU) * bins.len() as f32) as usize;
        bins[index.min(bins.len() - 1)] = true;
    }
    bins.iter().filter(|filled| **filled).count() as f32 / bins.len() as f32
}

fn ratio_score(candidate: f32, template: f32) -> f32 {
    let candidate = candidate.max(0.001);
    let template = template.max(0.001);
    (1.0 - (candidate / template).ln().abs() / 1.15).clamp(0.0, 1.0)
}

fn distance(a: StrokePoint, b: StrokePoint) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::rune_drawing::template_strokes_for_rune;

    fn rank_one(data: &GameData) -> Vec<&RuneDef> {
        data.runes.iter().filter(|rune| rune.tier == 1).collect()
    }

    fn all_runes(data: &GameData) -> Vec<&RuneDef> {
        data.runes.iter().collect()
    }

    #[test]
    fn rejects_diagram_without_outer_circle() {
        let data = GameData::load().unwrap();
        let strokes = template_at("light", 0.5, 0.5, 0.22);

        let interpretation = interpret_diagram(&strokes, rank_one(&data));

        assert!(!interpretation.circle_found);
        assert!(!interpretation.accepted());
    }

    #[test]
    fn interprets_multiple_runes_inside_enclosing_circle() {
        let data = GameData::load().unwrap();
        let mut strokes = outer_circle();
        strokes.extend(template_at("light", 0.26, 0.50, 0.18));
        strokes.extend(template_at("sphere", 0.50, 0.50, 0.18));
        strokes.extend(template_at("continuous", 0.74, 0.50, 0.18));

        let interpretation = interpret_diagram(&strokes, rank_one(&data));
        let ids = interpretation
            .runes
            .iter()
            .map(|rune| rune.rune_id.as_str())
            .collect::<Vec<_>>();

        assert!(interpretation.accepted(), "{interpretation:?}");
        assert!(ids.contains(&"light"), "{ids:?}");
        assert!(ids.contains(&"sphere"), "{ids:?}");
        assert!(ids.contains(&"continuous"), "{ids:?}");
    }

    #[test]
    fn overlapped_sphere_still_reads_inside_working_circle() {
        let data = GameData::load().unwrap();
        let mut strokes = outer_circle();
        strokes.extend(template_at("light", 0.50, 0.50, 0.18));
        strokes.push(rough_sphere(0.50, 0.50, 0.20));

        let interpretation = interpret_diagram(&strokes, rank_one(&data));
        let ids = interpretation
            .runes
            .iter()
            .map(|rune| rune.rune_id.as_str())
            .collect::<Vec<_>>();

        assert!(interpretation.accepted(), "{interpretation:?}");
        assert!(ids.contains(&"light"), "{ids:?}");
        assert!(ids.contains(&"sphere"), "{ids:?}");
    }

    #[test]
    fn lone_large_centered_inner_circle_reads_as_sphere() {
        let data = GameData::load().unwrap();
        let mut strokes = outer_circle();
        strokes.extend(template_at("light", 0.26, 0.50, 0.18));
        strokes.extend(rough_circle(0.50, 0.50, 0.17, 0.16, 20));

        let interpretation = interpret_diagram(&strokes, rank_one(&data));
        let ids = interpretation
            .runes
            .iter()
            .map(|rune| rune.rune_id.as_str())
            .collect::<Vec<_>>();

        assert!(interpretation.accepted(), "{interpretation:?}");
        assert!(ids.contains(&"sphere"), "{ids:?}");
    }

    #[test]
    fn screenshot_clear_light_and_sphere_read_together() {
        let data = GameData::load().unwrap();
        let mut strokes = rough_circle(0.30, 0.38, 0.25, 0.27, 28);
        strokes.push(flat_rough_circle(0.20, 0.28, 0.12));
        strokes.extend(template_at("light", 0.31, 0.36, 0.18));

        let interpretation = interpret_diagram(&strokes, rank_one(&data));
        let ids = interpretation
            .runes
            .iter()
            .map(|rune| rune.rune_id.as_str())
            .collect::<Vec<_>>();

        assert!(interpretation.accepted(), "{interpretation:?}");
        assert!(ids.contains(&"light"), "{ids:?}");
        assert!(ids.contains(&"sphere"), "{ids:?}");
    }

    #[test]
    fn accepts_smaller_off_center_outer_circle() {
        let data = GameData::load().unwrap();
        let mut strokes = rough_circle(0.34, 0.42, 0.23, 0.19, 24);
        strokes.extend(template_at("light", 0.34, 0.42, 0.12));

        let interpretation = interpret_diagram(&strokes, rank_one(&data));
        let ids = interpretation
            .runes
            .iter()
            .map(|rune| rune.rune_id.as_str())
            .collect::<Vec<_>>();

        assert!(interpretation.accepted(), "{interpretation:?}");
        assert!(ids.contains(&"light"), "{ids:?}");
    }

    #[test]
    fn rejects_simple_cross_inside_circle() {
        let data = GameData::load().unwrap();
        let mut strokes = outer_circle();
        strokes.push(stroke_at(&[(0.50, 0.18), (0.50, 0.82)], 0.50, 0.50, 0.20));
        strokes.push(stroke_at(&[(0.26, 0.50), (0.74, 0.50)], 0.50, 0.50, 0.20));

        let interpretation = interpret_diagram(&strokes, rank_one(&data));

        assert!(!interpretation.accepted(), "{interpretation:?}");
        assert!(interpretation.runes.is_empty(), "{interpretation:?}");
        assert_eq!(interpretation.rejected_marks, 1);
    }

    #[test]
    fn centered_shape_rune_scores_better_than_off_center_shape() {
        let data = GameData::load().unwrap();
        let centered = touch_quality_at(0.50, 0.50, &data);
        let off_center = touch_quality_at(0.26, 0.50, &data);

        assert!(
            centered > off_center + 0.03,
            "centered={centered} off_center={off_center}"
        );
    }

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

    fn touch_quality_at(x: f32, y: f32, data: &GameData) -> f32 {
        let mut strokes = outer_circle();
        strokes.extend(template_at("touch", x, y, 0.18));
        let interpretation = interpret_diagram(&strokes, rank_one(data));
        interpretation
            .runes
            .iter()
            .find(|rune| rune.rune_id == "touch")
            .map(|rune| rune.quality)
            .unwrap_or(0.0)
    }

    fn outer_circle() -> Vec<DrawnStroke> {
        rough_circle(0.50, 0.50, 0.42, 0.40, 36)
    }

    fn high_tier_city_circle() -> Vec<DrawnStroke> {
        let mut strokes = outer_circle();
        strokes.extend(rough_circle(0.50, 0.50, 0.36, 0.34, 48));
        strokes.extend(rough_circle(0.50, 0.50, 0.30, 0.29, 44));
        strokes.extend(rough_circle(0.50, 0.50, 0.23, 0.22, 38));
        strokes.extend(rough_circle(0.50, 0.50, 0.16, 0.15, 32));
        strokes.extend(template_at("gravity", 0.28, 0.48, 0.22));
        strokes.extend(template_at("sphere", 0.50, 0.50, 0.18));
        strokes.extend(template_at("continuous", 0.73, 0.48, 0.17));
        strokes.extend(template_at("safer", 0.50, 0.73, 0.15));
        strokes.extend(satellite_seals(8, 0.30, 0.038));
        strokes.extend(radial_spokes(8, 0.31));
        strokes.extend(perimeter_ticks(36, 0.39, 0.016));
        strokes.extend(perimeter_ticks(24, 0.34, 0.012));
        strokes.extend(script_marks(24, 0.20, 0.010));
        strokes.extend(script_marks(32, 0.27, 0.008));
        strokes
    }

    fn rough_circle(cx: f32, cy: f32, rx: f32, ry: f32, steps: usize) -> Vec<DrawnStroke> {
        let mut points = Vec::new();
        for index in 0..=steps {
            let angle = std::f32::consts::TAU * index as f32 / steps as f32;
            let wobble = if index % 5 == 0 { 0.015 } else { 0.0 };
            points.push(StrokePoint::new(
                cx + (rx + wobble) * angle.cos(),
                cy + (ry - wobble * 0.5) * angle.sin(),
            ));
        }
        vec![DrawnStroke { points }]
    }

    fn rough_sphere(cx: f32, cy: f32, scale: f32) -> DrawnStroke {
        stroke_at(
            &[
                (0.50, 0.14),
                (0.76, 0.24),
                (0.86, 0.52),
                (0.68, 0.80),
                (0.38, 0.84),
                (0.16, 0.62),
                (0.22, 0.30),
                (0.50, 0.14),
            ],
            cx,
            cy,
            scale,
        )
    }

    fn flat_rough_circle(cx: f32, cy: f32, scale: f32) -> DrawnStroke {
        stroke_at(
            &[
                (0.30, 0.24),
                (0.58, 0.20),
                (0.76, 0.32),
                (0.80, 0.58),
                (0.66, 0.76),
                (0.38, 0.78),
                (0.20, 0.60),
                (0.18, 0.36),
                (0.30, 0.24),
            ],
            cx,
            cy,
            scale,
        )
    }

    fn satellite_seals(count: usize, orbit: f32, radius: f32) -> Vec<DrawnStroke> {
        (0..count)
            .flat_map(|index| {
                let angle = std::f32::consts::TAU * index as f32 / count as f32
                    + std::f32::consts::FRAC_PI_4;
                rough_circle(
                    0.50 + orbit * angle.cos(),
                    0.50 + orbit * angle.sin(),
                    radius,
                    radius * 0.92,
                    16,
                )
            })
            .collect()
    }

    fn radial_spokes(count: usize, radius: f32) -> Vec<DrawnStroke> {
        (0..count)
            .map(|index| {
                let angle = std::f32::consts::TAU * index as f32 / count as f32;
                DrawnStroke {
                    points: vec![
                        StrokePoint::new(0.50, 0.50),
                        StrokePoint::new(0.50 + radius * angle.cos(), 0.50 + radius * angle.sin()),
                    ],
                }
            })
            .collect()
    }

    fn perimeter_ticks(count: usize, orbit: f32, half_len: f32) -> Vec<DrawnStroke> {
        (0..count)
            .map(|index| {
                let angle = std::f32::consts::TAU * index as f32 / count as f32;
                let center =
                    StrokePoint::new(0.50 + orbit * angle.cos(), 0.50 + orbit * angle.sin());
                let tangent = angle + std::f32::consts::FRAC_PI_2;
                DrawnStroke {
                    points: vec![
                        StrokePoint::new(
                            center.x - half_len * tangent.cos(),
                            center.y - half_len * tangent.sin(),
                        ),
                        StrokePoint::new(
                            center.x + half_len * tangent.cos(),
                            center.y + half_len * tangent.sin(),
                        ),
                    ],
                }
            })
            .collect()
    }

    fn script_marks(count: usize, orbit: f32, half_len: f32) -> Vec<DrawnStroke> {
        (0..count)
            .map(|index| {
                let angle = std::f32::consts::TAU * (index as f32 + 0.35) / count as f32;
                let center =
                    StrokePoint::new(0.50 + orbit * angle.cos(), 0.50 + orbit * angle.sin());
                let tangent = angle + std::f32::consts::FRAC_PI_2;
                let skew = if index % 2 == 0 { 0.55 } else { -0.55 };
                DrawnStroke {
                    points: vec![
                        StrokePoint::new(
                            center.x - half_len * tangent.cos(),
                            center.y - half_len * tangent.sin(),
                        ),
                        StrokePoint::new(
                            center.x + half_len * tangent.cos(),
                            center.y + half_len * tangent.sin(),
                        ),
                        StrokePoint::new(
                            center.x
                                + half_len * (tangent + skew).cos()
                                + half_len * 0.35 * angle.cos(),
                            center.y
                                + half_len * (tangent + skew).sin()
                                + half_len * 0.35 * angle.sin(),
                        ),
                    ],
                }
            })
            .collect()
    }

    fn template_at(rune_id: &str, cx: f32, cy: f32, scale: f32) -> Vec<DrawnStroke> {
        template_strokes_for_rune(rune_id)
            .unwrap()
            .into_iter()
            .map(|stroke| DrawnStroke {
                points: stroke
                    .points
                    .into_iter()
                    .map(|point| {
                        StrokePoint::new(cx + (point.x - 0.5) * scale, cy + (point.y - 0.5) * scale)
                    })
                    .collect(),
            })
            .collect()
    }

    fn stroke_at(points: &[(f32, f32)], cx: f32, cy: f32, scale: f32) -> DrawnStroke {
        DrawnStroke {
            points: points
                .iter()
                .map(|(x, y)| StrokePoint::new(cx + (x - 0.5) * scale, cy + (y - 0.5) * scale))
                .collect(),
        }
    }
}
