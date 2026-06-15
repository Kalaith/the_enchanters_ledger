use super::{template_strokes_for_rune, DrawnStroke, StrokePoint};

#[derive(Debug, Clone)]
pub(crate) struct RuneSample {
    pub(crate) name: &'static str,
    pub(crate) rune_id: &'static str,
    pub(crate) strokes: Vec<DrawnStroke>,
}

impl RuneSample {
    fn new(name: &'static str, rune_id: &'static str, strokes: Vec<DrawnStroke>) -> Self {
        Self {
            name,
            rune_id,
            strokes,
        }
    }
}

pub(crate) fn structural_rune_samples() -> Vec<RuneSample> {
    vec![
        RuneSample::new("sphere_template", "sphere", template("sphere")),
        RuneSample::new("sphere_rough", "sphere", rough_sphere()),
        RuneSample::new("touch_template", "touch", template("touch")),
        RuneSample::new("touch_rough_arrow", "touch", rough_touch()),
        RuneSample::new("beam_template", "beam", template("beam")),
        RuneSample::new("beam_rough_arrow", "beam", rough_beam()),
        RuneSample::new("aura_template", "aura", template("aura")),
        RuneSample::new("aura_rough_hex", "aura", rough_aura()),
        RuneSample::new("burst_template", "burst", template("burst")),
        RuneSample::new("burst_rough_cross", "burst", rough_burst()),
        RuneSample::new("cone_template", "cone", template("cone")),
        RuneSample::new("cone_rough_triangle", "cone", rough_cone()),
        RuneSample::new("safer_template", "safer", template("safer")),
        RuneSample::new("safer_rough_hex", "safer", rough_safer()),
    ]
}

pub(crate) fn ambiguous_shape_samples() -> Vec<RuneSample> {
    vec![
        RuneSample::new("sphere_safer_round_hex", "sphere", round_hex()),
        RuneSample::new("touch_beam_diagonal_arrow", "touch", diagonal_arrow()),
    ]
}

pub(crate) fn circled_sample(
    sample: &RuneSample,
    cx: f32,
    cy: f32,
    scale: f32,
) -> Vec<DrawnStroke> {
    let mut strokes = outer_circle();
    strokes.extend(strokes_at(&sample.strokes, cx, cy, scale));
    strokes
}

pub(crate) fn outer_circle() -> Vec<DrawnStroke> {
    let mut points = Vec::new();
    for index in 0..=40 {
        let angle = std::f32::consts::TAU * index as f32 / 40.0;
        let wobble = if index % 6 == 0 { 0.010 } else { 0.0 };
        points.push(StrokePoint::new(
            0.50 + (0.42 + wobble) * angle.cos(),
            0.50 + (0.40 - wobble * 0.5) * angle.sin(),
        ));
    }
    vec![DrawnStroke { points }]
}

pub(crate) fn strokes_at(
    strokes: &[DrawnStroke],
    cx: f32,
    cy: f32,
    scale: f32,
) -> Vec<DrawnStroke> {
    strokes
        .iter()
        .map(|stroke| DrawnStroke {
            points: stroke
                .points
                .iter()
                .map(|point| {
                    StrokePoint::new(cx + (point.x - 0.5) * scale, cy + (point.y - 0.5) * scale)
                })
                .collect(),
        })
        .collect()
}

fn template(rune_id: &str) -> Vec<DrawnStroke> {
    template_strokes_for_rune(rune_id).unwrap_or_default()
}

fn raw(strokes: &[&[(f32, f32)]]) -> Vec<DrawnStroke> {
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

fn rough_sphere() -> Vec<DrawnStroke> {
    raw(&[&[
        (0.50, 0.14),
        (0.74, 0.22),
        (0.86, 0.48),
        (0.74, 0.76),
        (0.48, 0.86),
        (0.22, 0.70),
        (0.18, 0.42),
        (0.30, 0.22),
        (0.50, 0.14),
    ]])
}

fn rough_touch() -> Vec<DrawnStroke> {
    raw(&[
        &[(0.52, 0.14), (0.50, 0.84)],
        &[(0.30, 0.60), (0.50, 0.84)],
        &[(0.72, 0.60), (0.50, 0.84)],
    ])
}

fn rough_beam() -> Vec<DrawnStroke> {
    raw(&[
        &[(0.14, 0.52), (0.84, 0.50)],
        &[(0.64, 0.34), (0.84, 0.50)],
        &[(0.64, 0.66), (0.84, 0.50)],
    ])
}

fn rough_aura() -> Vec<DrawnStroke> {
    raw(&[
        &[
            (0.50, 0.16),
            (0.80, 0.36),
            (0.78, 0.64),
            (0.50, 0.84),
            (0.20, 0.64),
            (0.18, 0.38),
            (0.50, 0.16),
        ],
        &[(0.30, 0.52), (0.70, 0.50)],
    ])
}

fn rough_burst() -> Vec<DrawnStroke> {
    raw(&[
        &[(0.50, 0.14), (0.50, 0.86)],
        &[(0.14, 0.50), (0.86, 0.50)],
        &[(0.25, 0.26), (0.76, 0.74)],
        &[(0.74, 0.24), (0.25, 0.76)],
    ])
}

fn rough_cone() -> Vec<DrawnStroke> {
    raw(&[&[(0.18, 0.78), (0.52, 0.20), (0.82, 0.78), (0.18, 0.78)]])
}

fn rough_safer() -> Vec<DrawnStroke> {
    raw(&[&[
        (0.50, 0.12),
        (0.78, 0.28),
        (0.74, 0.66),
        (0.50, 0.88),
        (0.26, 0.66),
        (0.22, 0.28),
        (0.50, 0.12),
    ]])
}

fn round_hex() -> Vec<DrawnStroke> {
    raw(&[&[
        (0.50, 0.14),
        (0.76, 0.25),
        (0.84, 0.50),
        (0.74, 0.75),
        (0.50, 0.86),
        (0.24, 0.75),
        (0.16, 0.50),
        (0.26, 0.25),
        (0.50, 0.14),
    ]])
}

fn diagonal_arrow() -> Vec<DrawnStroke> {
    raw(&[
        &[(0.24, 0.22), (0.76, 0.78)],
        &[(0.52, 0.76), (0.76, 0.78)],
        &[(0.75, 0.54), (0.76, 0.78)],
    ])
}
