use super::*;

fn points(raw: &[(f32, f32)]) -> Vec<StrokePoint> {
    raw.iter().map(|(x, y)| StrokePoint::new(*x, *y)).collect()
}

fn rounded_corners(points: &[StrokePoint]) -> i32 {
    corner_count(points).round() as i32
}

#[test]
fn open_straight_line_has_no_corners() {
    let line = points(&[(0.15, 0.20), (0.85, 0.80)]);

    assert_eq!(rounded_corners(&line), 0);
}

#[test]
fn open_bend_counts_a_single_corner() {
    let bend = points(&[(0.20, 0.20), (0.50, 0.80), (0.80, 0.20)]);

    assert_eq!(rounded_corners(&bend), 1);
}

#[test]
fn closed_square_still_counts_four_corners() {
    let square = points(&[
        (0.20, 0.20),
        (0.80, 0.20),
        (0.80, 0.80),
        (0.20, 0.80),
        (0.20, 0.20),
    ]);

    assert_eq!(rounded_corners(&square), 4);
}

#[test]
fn closed_hexagon_reads_closer_to_six_corners_than_four() {
    // The "safer" rune template: a hand-authored hexagon whose corners
    // aren't all equal-angle. Two of them (turn ~0.638 rad) fall just
    // short of the closed-shape threshold (0.60 was chosen so these
    // clear it; the *open*-stroke threshold is 0.68, which would have
    // missed them). corner_count is now continuous (corner_confidence
    // is a sigmoid, not a hard cutoff), so those two soft corners
    // contribute partial weight rather than either flipping fully in or
    // vanishing outright — the total lands near, not exactly at, 6.0.
    // What matters for recognition is that it reads far closer to a
    // hexagon (6) than a diamond (4); see safer_template_still_recognizes_safer
    // and the confusion-matrix gate for the actual recognition outcome.
    let hexagon = points(&[
        (0.50, 0.12),
        (0.80, 0.26),
        (0.74, 0.66),
        (0.50, 0.88),
        (0.26, 0.66),
        (0.20, 0.26),
        (0.50, 0.12),
    ]);

    let corners = corner_count(&hexagon);
    assert!(
        (5.0..=6.2).contains(&corners),
        "corners={corners}, expected close to 6"
    );
}

#[test]
fn closed_diamond_still_counts_four_corners() {
    let diamond = points(&[
        (0.50, 0.12),
        (0.86, 0.50),
        (0.50, 0.88),
        (0.14, 0.50),
        (0.50, 0.12),
    ]);

    assert_eq!(rounded_corners(&diamond), 4);
}
