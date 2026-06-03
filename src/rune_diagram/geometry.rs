use crate::rune_drawing::{DrawnStroke, StrokePoint};

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

pub(crate) fn distance(a: StrokePoint, b: StrokePoint) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}
