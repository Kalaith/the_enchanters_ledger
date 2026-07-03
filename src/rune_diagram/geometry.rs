use crate::rune_drawing::{DrawnStroke, StrokePoint};
use std::collections::HashMap;

/// Cluster-distance thresholds are computed per stroke-pair (see `cluster_thresholds`) from the
/// pair's own size instead of these old fixed constants (plan Phase 3 item 3 / C3) — a dense
/// diagram of small runes now clusters at small-rune scale instead of merging into one blob at
/// the scale these constants were originally tuned for.
///
/// Two different pair shapes need two different reaches, so there are two factor sets, not one:
/// a multi-stroke rune's *open* component strokes (an arrow's shaft and its chevron head, say)
/// are often quite different sizes and don't literally touch — bridging that gap needs a
/// generous reach scaled off the *larger* of the two. A pair where either stroke is already a
/// *closed*, self-contained shape (a circle rune like `sphere`) is different: that stroke is
/// already a complete glyph on its own, so its reach must stay conservative (scaled off the
/// *smaller* of the two) or it swallows unrelated ink drawn nearby — a big circle's own bounding
/// diagonal is not a sensible "how far can I reach for a neighbor" radius.
const OPEN_CENTER_DISTANCE_FACTOR: f32 = 0.65;
const OPEN_STROKE_DISTANCE_FACTOR: f32 = 0.35;
const CLOSED_CENTER_DISTANCE_FACTOR: f32 = 0.42;
const CLOSED_STROKE_DISTANCE_FACTOR: f32 = 0.21;
const MIN_CLUSTER_CENTER_DISTANCE: f32 = 0.018;
const MIN_CLUSTER_STROKE_DISTANCE: f32 = 0.009;

/// Per-pair cluster-adjacency thresholds, scaled by the pair's own bounding-box diagonal —
/// replaces the old fixed absolute `MAX_CLUSTER_*_DISTANCE` constants (C3): two small runes
/// drawn close together cluster at small-rune scale, two large runes at large-rune scale,
/// instead of everything sharing one slate-absolute distance. See the factor constants above for
/// why the open/closed split exists and why each picks a different one of the pair's diagonals.
/// Floored so degenerate near-zero-size strokes don't collapse the threshold to 0.
fn cluster_thresholds(
    a: &DrawnStroke,
    a_bounds: StrokeBounds,
    b: &DrawnStroke,
    b_bounds: StrokeBounds,
) -> (f32, f32) {
    let either_closed = is_closed_stroke(a, a_bounds) || is_closed_stroke(b, b_bounds);
    // A closed shape enclosing the other stroke (e.g. `aura`'s hexagon ring around its own
    // crossbar, `fire`'s loop around its own inner squiggle) is still one multi-stroke rune, not
    // a closed glyph with an unrelated neighbor — nesting overrides the closed-conservative path
    // back to the generous one. Likewise two closed shapes that share a drawn vertex (e.g.
    // `continuous`'s two diamonds, touching bounding boxes at a shared point) are one rune, not
    // two coincidentally-adjacent ones — an unrelated closed shape's bounding box merely being
    // *near* another's (like `sphere` next to `light`) is not the same as their boxes actually
    // touching, so this stays a safe, narrow override.
    let nested = bounds_contains(a_bounds, b_bounds)
        || bounds_contains(b_bounds, a_bounds)
        || bounds_gap(a_bounds, b_bounds) <= 0.004;
    let (center_factor, stroke_factor, local_scale) = if either_closed && !nested {
        (
            CLOSED_CENTER_DISTANCE_FACTOR,
            CLOSED_STROKE_DISTANCE_FACTOR,
            a_bounds.diagonal().min(b_bounds.diagonal()),
        )
    } else {
        (
            OPEN_CENTER_DISTANCE_FACTOR,
            OPEN_STROKE_DISTANCE_FACTOR,
            a_bounds.diagonal().max(b_bounds.diagonal()),
        )
    };
    let center_limit = (local_scale * center_factor).max(MIN_CLUSTER_CENTER_DISTANCE);
    let stroke_limit = (local_scale * stroke_factor).max(MIN_CLUSTER_STROKE_DISTANCE);
    (center_limit, stroke_limit)
}

/// A closed, self-contained shape (e.g. a circle rune like `sphere`) already reads as a complete
/// glyph on its own — see `cluster_thresholds`. Mirrors the closure heuristic used elsewhere
/// (`magical_circle::is_closed_stroke`, `rune_drawing::is_effectively_closed`), tuned separately
/// here since clustering cares about "is this a complete shape", not circle-quality or
/// continuation-merge specifically.
fn is_closed_stroke(stroke: &DrawnStroke, bounds: StrokeBounds) -> bool {
    let (Some(first), Some(last)) = (stroke.points.first(), stroke.points.last()) else {
        return false;
    };
    let span = bounds.width().max(bounds.height()).max(0.001);
    stroke.points.len() >= 5 && distance(*first, *last) <= (span * 0.24).max(0.025)
}

/// Whether `inner`'s extent sits within `outer`'s extent (with a small tolerance for hand-drawn
/// jitter) — see `cluster_thresholds`'s `nested` check.
fn bounds_contains(outer: StrokeBounds, inner: StrokeBounds) -> bool {
    let tolerance_x = outer.width() * 0.08;
    let tolerance_y = outer.height() * 0.08;
    inner.min_x >= outer.min_x - tolerance_x
        && inner.max_x <= outer.max_x + tolerance_x
        && inner.min_y >= outer.min_y - tolerance_y
        && inner.max_y <= outer.max_y + tolerance_y
}

#[derive(Debug, Clone)]
pub(crate) struct StrokeCluster {
    pub(crate) indices: Vec<usize>,
    pub(crate) strokes: Vec<DrawnStroke>,
    pub(crate) bounds: StrokeBounds,
}

/// Groups strokes into connected clusters by spatial proximity (never by original draw order —
/// see `crate::rune_diagram::recognition`'s callers for the order-independence this backs, plan
/// item 5 / A2). Uses a spatial hash grid so a diagram with hundreds of small strokes stays
/// roughly O(n·k) instead of testing every O(n²) pair (plan item 6 / C4): a stroke can only ever
/// cluster with a neighbor within its own (size-relative) threshold, so only strokes in the same
/// or adjacent grid cell are ever compared.
pub(crate) fn cluster_strokes(strokes: &[(usize, DrawnStroke)]) -> Vec<StrokeCluster> {
    let items = strokes
        .iter()
        .filter_map(|(index, stroke)| {
            StrokeBounds::from_stroke(stroke).map(|bounds| (*index, stroke.clone(), bounds))
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        return Vec::new();
    }

    let edges = candidate_pairs(&items)
        .into_iter()
        .filter(|(left, right)| {
            strokes_should_cluster(
                &items[*left].1,
                items[*left].2,
                &items[*right].1,
                items[*right].2,
            )
        })
        .fold(
            vec![Vec::<usize>::new(); items.len()],
            |mut edges, (left, right)| {
                edges[left].push(right);
                edges[right].push(left);
                edges
            },
        );

    let mut visited = vec![false; items.len()];
    let mut clusters = Vec::<StrokeCluster>::new();
    for start in 0..items.len() {
        if visited[start] {
            continue;
        }
        let mut stack = vec![start];
        let mut component = Vec::new();
        visited[start] = true;
        while let Some(index) = stack.pop() {
            component.push(index);
            for neighbor in &edges[index] {
                if !visited[*neighbor] {
                    visited[*neighbor] = true;
                    stack.push(*neighbor);
                }
            }
        }
        component.sort_by_key(|index| items[*index].0);

        let first = component[0];
        let mut bounds = items[first].2;
        let mut indices = Vec::with_capacity(component.len());
        let mut cluster_strokes = Vec::with_capacity(component.len());
        for item_index in component {
            bounds.include(&items[item_index].2);
            indices.push(items[item_index].0);
            cluster_strokes.push(items[item_index].1.clone());
        }
        clusters.push(StrokeCluster {
            indices,
            strokes: cluster_strokes,
            bounds,
        });
    }

    clusters.sort_by_key(|cluster| cluster.indices.first().copied().unwrap_or(usize::MAX));
    clusters
}

/// Candidate (left, right) item-index pairs worth a full `strokes_should_cluster` check: items
/// bucketed into a grid sized to the largest plausible per-pair threshold, so only same/adjacent
/// cell pairs are ever considered. Falls back to the full O(n²) pair set for tiny inputs, where
/// the grid's bookkeeping overhead isn't worth it.
fn candidate_pairs(items: &[(usize, DrawnStroke, StrokeBounds)]) -> Vec<(usize, usize)> {
    if items.len() < 24 {
        let mut pairs = Vec::with_capacity(items.len() * items.len() / 2);
        for left in 0..items.len() {
            for right in (left + 1)..items.len() {
                pairs.push((left, right));
            }
        }
        return pairs;
    }

    let max_diagonal = items
        .iter()
        .map(|(_, _, bounds)| bounds.diagonal())
        .fold(0.0f32, f32::max);
    // Any pair's `center_limit` is at most `max_diagonal * OPEN_CENTER_DISTANCE_FACTOR` (the
    // larger of the two factor sets, applied to the largest single diagonal present — an upper
    // bound on both the open-pair "max diagonal" and closed-pair "min diagonal" cases), so a
    // cell this size guarantees a same-or-adjacent-cell pair for anything within that threshold
    // — the standard uniform-grid neighbor-search invariant.
    let cell_size = (max_diagonal * OPEN_CENTER_DISTANCE_FACTOR).max(0.03);

    let mut buckets: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (item_index, (_, _, bounds)) in items.iter().enumerate() {
        buckets
            .entry(cell_of(bounds.center(), cell_size))
            .or_default()
            .push(item_index);
    }

    let mut pairs = Vec::new();
    for (item_index, (_, _, bounds)) in items.iter().enumerate() {
        let (cx, cy) = cell_of(bounds.center(), cell_size);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let Some(neighbors) = buckets.get(&(cx + dx, cy + dy)) else {
                    continue;
                };
                for &neighbor in neighbors {
                    if neighbor > item_index {
                        pairs.push((item_index, neighbor));
                    }
                }
            }
        }
    }
    pairs
}

fn cell_of(point: StrokePoint, cell_size: f32) -> (i32, i32) {
    (
        (point.x / cell_size).floor() as i32,
        (point.y / cell_size).floor() as i32,
    )
}

fn strokes_should_cluster(
    a: &DrawnStroke,
    a_bounds: StrokeBounds,
    b: &DrawnStroke,
    b_bounds: StrokeBounds,
) -> bool {
    let (center_limit, stroke_limit) = cluster_thresholds(a, a_bounds, b, b_bounds);
    distance(a_bounds.center(), b_bounds.center()) <= center_limit
        && bounds_gap(a_bounds, b_bounds) <= stroke_limit
        && stroke_distance(a, b) <= stroke_limit
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

    pub(crate) fn from_strokes(strokes: &[DrawnStroke]) -> Option<Self> {
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

    pub(crate) fn diagonal(self) -> f32 {
        (self.width() * self.width() + self.height() * self.height()).sqrt()
    }

    pub(crate) fn center(self) -> StrokePoint {
        StrokePoint::new(
            self.min_x + self.width() * 0.5,
            self.min_y + self.height() * 0.5,
        )
    }

    pub(crate) fn scale_relative(self, circle: Self) -> f32 {
        (self.width() / circle.width().max(0.001)).max(self.height() / circle.height().max(0.001))
    }

    fn include(&mut self, other: &Self) {
        self.min_x = self.min_x.min(other.min_x);
        self.min_y = self.min_y.min(other.min_y);
        self.max_x = self.max_x.max(other.max_x);
        self.max_y = self.max_y.max(other.max_y);
    }
}

pub(crate) fn distance(a: StrokePoint, b: StrokePoint) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

/// Euclidean gap between two axis-aligned bounding boxes (0 if they overlap) — a cheap lower
/// bound on the true minimum distance between the strokes they contain, so `strokes_should_cluster`
/// can skip `stroke_distance`'s O(p²) segment-pair scan whenever the boxes alone are already too
/// far apart to possibly cluster (plan item 6 / C4).
fn bounds_gap(a: StrokeBounds, b: StrokeBounds) -> f32 {
    let dx = (a.min_x.max(b.min_x) - a.max_x.min(b.max_x)).max(0.0);
    let dy = (a.min_y.max(b.min_y) - a.max_y.min(b.max_y)).max(0.0);
    (dx * dx + dy * dy).sqrt()
}

fn stroke_distance(a: &DrawnStroke, b: &DrawnStroke) -> f32 {
    let mut best = f32::INFINITY;
    for a_segment in a.points.windows(2) {
        for b_segment in b.points.windows(2) {
            best = best.min(segment_distance(
                a_segment[0],
                a_segment[1],
                b_segment[0],
                b_segment[1],
            ));
        }
    }
    best
}

fn segment_distance(a0: StrokePoint, a1: StrokePoint, b0: StrokePoint, b1: StrokePoint) -> f32 {
    if segments_intersect(a0, a1, b0, b1) {
        return 0.0;
    }
    point_segment_distance(a0, b0, b1)
        .min(point_segment_distance(a1, b0, b1))
        .min(point_segment_distance(b0, a0, a1))
        .min(point_segment_distance(b1, a0, a1))
}

fn point_segment_distance(point: StrokePoint, start: StrokePoint, end: StrokePoint) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= f32::EPSILON {
        return distance(point, start);
    }
    let t = (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_sq).clamp(0.0, 1.0);
    distance(point, StrokePoint::new(start.x + dx * t, start.y + dy * t))
}

fn segments_intersect(a0: StrokePoint, a1: StrokePoint, b0: StrokePoint, b1: StrokePoint) -> bool {
    let d1 = orientation(a0, a1, b0);
    let d2 = orientation(a0, a1, b1);
    let d3 = orientation(b0, b1, a0);
    let d4 = orientation(b0, b1, a1);
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

fn orientation(a: StrokePoint, b: StrokePoint, c: StrokePoint) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}
