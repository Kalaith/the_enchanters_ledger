//! Deterministic sample generation shared by the confusion-matrix gate and
//! the property tests. Perturbations are computed about each template's own
//! bounding box and clamped so they never push a point into the [0,1] edge
//! clamp in `StrokePoint::new` — an out-of-range point would silently distort
//! the shape instead of exercising real translate/scale/jitter behavior.

use super::{DrawnStroke, StrokePoint};

fn dist(a: StrokePoint, b: StrokePoint) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

/// A tiny xorshift64* PRNG so jitter is seeded and reproducible without a
/// `rand` dependency.
pub(crate) struct Lcg(u64);

impl Lcg {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E3779B97F4A7C15) ^ 0xD1B54A32D192ED03)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 32) as u32
    }

    /// Uniform sample in [-1, 1].
    fn next_unit(&mut self) -> f32 {
        (self.next_u32() as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Clamps into `[lo, hi]`, or `[hi, lo]` if the requested scale left no safe
/// translate room at all — falls back to the tightest-fit midpoint rather
/// than panicking on an inverted range.
fn clamp_to_valid_range(value: f32, lo: f32, hi: f32) -> f32 {
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    value.clamp(lo, hi)
}

pub(crate) fn bounding_box(strokes: &[DrawnStroke]) -> (f32, f32, f32, f32) {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for point in strokes.iter().flat_map(|stroke| &stroke.points) {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }
    (min_x, max_x, min_y, max_y)
}

/// Scales and translates `strokes` about their bounding-box center, then adds
/// seeded per-point jitter. `translate` is clamped to whatever room is left
/// after scaling so the result stays inside a safe [margin, 1-margin] box.
pub(crate) fn perturb(
    strokes: &[DrawnStroke],
    scale: f32,
    translate: (f32, f32),
    jitter_amp: f32,
    seed: u64,
) -> Vec<DrawnStroke> {
    let (min_x, max_x, min_y, max_y) = bounding_box(strokes);
    let cx = (min_x + max_x) * 0.5;
    let cy = (min_y + max_y) * 0.5;
    let half_w = (max_x - min_x) * 0.5 * scale;
    let half_h = (max_y - min_y) * 0.5 * scale;
    let margin = 0.02 + jitter_amp;
    let dx = clamp_to_valid_range(
        translate.0,
        margin - (cx - half_w),
        (1.0 - margin) - (cx + half_w),
    );
    let dy = clamp_to_valid_range(
        translate.1,
        margin - (cy - half_h),
        (1.0 - margin) - (cy + half_h),
    );

    let mut rng = Lcg::new(seed);
    strokes
        .iter()
        .map(|stroke| DrawnStroke {
            points: stroke
                .points
                .iter()
                .map(|point| {
                    StrokePoint::new(
                        cx + (point.x - cx) * scale + dx + rng.next_unit() * jitter_amp,
                        cy + (point.y - cy) * scale + dy + rng.next_unit() * jitter_amp,
                    )
                })
                .collect(),
        })
        .collect()
}

/// Re-samples every stroke to `target_count` evenly arc-length-spaced points,
/// simulating a different capture frame rate / mouse speed.
pub(crate) fn resample_density(strokes: &[DrawnStroke], target_count: usize) -> Vec<DrawnStroke> {
    strokes
        .iter()
        .map(|stroke| DrawnStroke {
            points: resample_points(&stroke.points, target_count),
        })
        .collect()
}

fn resample_points(points: &[StrokePoint], target_count: usize) -> Vec<StrokePoint> {
    if points.len() < 2 || target_count < 2 {
        return points.to_vec();
    }
    let total: f32 = points.windows(2).map(|pair| dist(pair[0], pair[1])).sum();
    if total <= 0.0001 {
        return vec![points[0]; target_count];
    }
    (0..target_count)
        .map(|index| {
            let target = total * index as f32 / (target_count - 1) as f32;
            point_at_distance(points, target)
        })
        .collect()
}

fn point_at_distance(points: &[StrokePoint], target: f32) -> StrokePoint {
    let mut walked = 0.0;
    for pair in points.windows(2) {
        let segment = dist(pair[0], pair[1]);
        if segment <= 0.0 {
            continue;
        }
        if walked + segment >= target {
            let t = ((target - walked) / segment).clamp(0.0, 1.0);
            return StrokePoint::new(
                pair[0].x + (pair[1].x - pair[0].x) * t,
                pair[0].y + (pair[1].y - pair[0].y) * t,
            );
        }
        walked += segment;
    }
    *points.last().unwrap()
}
