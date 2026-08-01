//! Stroke builders for the decorative structure a diagram carries beyond its
//! runes — reinforcement rings, satellite seals, radial spokes, perimeter ticks
//! and script marks.
//!
//! Every shape here is drawn to land in a specific `magical_circle::
//! classify_circle_stroke` bucket, and the sizes are the ones the existing
//! grand-circle test fixtures already prove classify correctly; they are just
//! parameterized by the scope they belong to instead of hardcoded around a
//! 0.42-radius circle.

use super::circle_points;
use crate::rune_drawing::{DrawnStroke, StrokePoint};
use std::f32::consts::{FRAC_PI_2, TAU};

/// A closed ring: a satellite seal, a reinforcement ring, or a sub-scope's own
/// circle, depending on how big it is and where it sits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiagramRing {
    pub center: StrokePoint,
    pub radius: f32,
}

impl DiagramRing {
    pub fn stroke(&self) -> DrawnStroke {
        DrawnStroke {
            points: circle_points(self.center, self.radius),
        }
    }
}

/// A spoke from the scope's center outward to `radius` — `classify_circle_stroke`
/// scores these by how radial they are, so they start exactly at the center.
pub fn radial_spoke(center: StrokePoint, angle: f32, radius: f32) -> DrawnStroke {
    DrawnStroke {
        points: vec![
            center,
            StrokePoint::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            ),
        ],
    }
}

/// A short tangential dash on the given orbit — a perimeter tick out near the
/// rim, or (drawn smaller, further in) a script mark. Two points, straight, so
/// it clears the `directness > 0.65` and `points <= 3` tests that separate
/// decorative writing from small rune ink.
pub fn tangential_tick(
    center: StrokePoint,
    angle: f32,
    orbit: f32,
    half_length: f32,
) -> DrawnStroke {
    let at = StrokePoint::new(
        center.x + orbit * angle.cos(),
        center.y + orbit * angle.sin(),
    );
    let tangent = angle + FRAC_PI_2;
    DrawnStroke {
        points: vec![
            StrokePoint::new(
                at.x - half_length * tangent.cos(),
                at.y - half_length * tangent.sin(),
            ),
            StrokePoint::new(
                at.x + half_length * tangent.cos(),
                at.y + half_length * tangent.sin(),
            ),
        ],
    }
}

/// `count` angles spread evenly around a full turn, starting at the top.
pub fn even_angles(count: usize) -> Vec<f32> {
    (0..count)
        .map(|index| -FRAC_PI_2 + TAU * index as f32 / count.max(1) as f32)
        .collect()
}

/// Angles that sit in the gaps between `occupied`, so small marks never land on
/// top of the runes and sub-scopes already placed around the ring. Gaps take
/// items round-robin, and a gap hosting several spreads them evenly inside
/// itself, so this stays collision-free however many are asked for.
pub fn gap_angles(occupied: &[f32], count: usize) -> Vec<f32> {
    if count == 0 {
        return Vec::new();
    }
    if occupied.len() < 2 {
        return even_angles(count)
            .into_iter()
            .map(|angle| angle + if occupied.is_empty() { 0.0 } else { TAU * 0.5 })
            .collect();
    }

    let gaps = occupied.len();
    let mut per_gap = vec![0usize; gaps];
    for item in 0..count {
        per_gap[item % gaps] += 1;
    }

    let mut angles = Vec::with_capacity(count);
    for (index, hosted) in per_gap.iter().enumerate() {
        let start = occupied[index];
        let end = occupied[(index + 1) % gaps] + if index + 1 == gaps { TAU } else { 0.0 };
        for slot in 0..*hosted {
            let fraction = (slot + 1) as f32 / (*hosted + 1) as f32;
            angles.push(start + (end - start) * fraction);
        }
    }
    angles
}
