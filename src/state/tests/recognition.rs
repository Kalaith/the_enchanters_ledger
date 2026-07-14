//! Which runes a hand-drawn diagram reads as, through the full session
//! pipeline (`interpret_drawing`, not the recognizer in isolation).

use super::fixtures::*;

#[test]
fn rough_inner_circle_prefers_sphere_over_safer() {
    let data = data();
    let mut session = unlocked_session(&data);
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.42, 0.58, 0.18));
    strokes.push(flat_rough_circle(0.50, 0.34, 0.18));
    strokes.push(rough_continuous_pair(0.64, 0.58, 0.20));
    session.board.drawing_strokes = strokes;

    session.interpret_drawing(&data).unwrap();
    let ids = placed_ids(&session);

    assert!(ids.iter().any(|id| id == "sphere"), "{ids:?}");
    assert!(!ids.iter().any(|id| id == "safer"), "{ids:?}");
}

#[test]
fn screenshot_starter_runes_read_as_close_enough() {
    let data = data();
    let mut session = unlocked_session(&data);
    let mut strokes = rough_circle(0.50, 0.48, 0.38, 0.35, 34);
    strokes.extend(template_at("light", 0.34, 0.30, 0.18));
    strokes.extend(rough_continuous_diamonds(0.62, 0.32, 0.22));
    strokes.push(rough_sphere(0.49, 0.58, 0.18));
    session.board.drawing_strokes = strokes;

    session.interpret_drawing(&data).unwrap();
    let ids = placed_ids(&session);

    assert!(ids.iter().any(|id| id == "light"), "{ids:?}");
    assert!(ids.iter().any(|id| id == "continuous"), "{ids:?}");
    assert!(ids.iter().any(|id| id == "sphere"), "{ids:?}");
}

#[test]
fn tall_diagram_keeps_light_after_extracting_inner_sphere() {
    let data = data();
    let mut session = unlocked_session(&data);
    let mut strokes = rough_circle(0.445, 0.51, 0.205, 0.44, 36);
    strokes.extend(template_at("light", 0.36, 0.36, 0.16));
    strokes.push(rough_sphere(0.46, 0.45, 0.19));
    strokes.extend(template_at("continuous", 0.38, 0.62, 0.16));
    session.board.drawing_strokes = strokes;

    session.interpret_drawing(&data).unwrap();
    let ids = placed_ids(&session);

    assert!(ids.iter().any(|id| id == "light"), "{ids:?}");
    assert!(ids.iter().any(|id| id == "sphere"), "{ids:?}");
    assert!(ids.iter().any(|id| id == "continuous"), "{ids:?}");

    session.interpret_drawing(&data).unwrap();
    let second_ids = placed_ids(&session);

    assert_eq!(ids, second_ids);
}
