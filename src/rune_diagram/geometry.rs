use crate::rune_drawing::{DrawnStroke, StrokePoint};

const MAX_CLUSTER_STROKE_DISTANCE: f32 = 0.045;
const MAX_CLUSTER_CENTER_DISTANCE: f32 = 0.09;

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
            if cluster_should_accept(&cluster.strokes, stroke) {
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

fn cluster_should_accept(cluster_strokes: &[DrawnStroke], stroke: &DrawnStroke) -> bool {
    let Some(bounds) = StrokeBounds::from_stroke(stroke) else {
        return false;
    };
    cluster_strokes.iter().any(|cluster_stroke| {
        let Some(cluster_bounds) = StrokeBounds::from_stroke(cluster_stroke) else {
            return false;
        };
        distance(cluster_bounds.center(), bounds.center()) <= MAX_CLUSTER_CENTER_DISTANCE
            && stroke_distance(cluster_stroke, stroke) <= MAX_CLUSTER_STROKE_DISTANCE
    })
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
