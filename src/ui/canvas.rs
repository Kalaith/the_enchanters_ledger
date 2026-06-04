use crate::rune_drawing::{DrawnStroke, StrokePoint};
use macroquad::prelude::*;

pub(super) const ERASER_RADIUS_PIXELS: f32 = 18.0;

pub(super) fn point_in_rect(rect: Rect, point: Vec2) -> StrokePoint {
    StrokePoint::new((point.x - rect.x) / rect.w, (point.y - rect.y) / rect.h)
}

pub(super) fn point_to_screen(rect: Rect, point: StrokePoint) -> Vec2 {
    vec2(rect.x + point.x * rect.w, rect.y + point.y * rect.h)
}

pub(super) fn eraser_radius_in_rect(rect: Rect) -> f32 {
    ERASER_RADIUS_PIXELS / rect.w.min(rect.h).max(1.0)
}

pub(super) fn point_distance(a: StrokePoint, b: StrokePoint) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

pub(super) fn draw_strokes(strokes: &[DrawnStroke], rect: Rect, color: Color, thickness: f32) {
    draw_strokes_with_point_scale(strokes, rect, color, thickness, 0.45);
}

pub(super) fn draw_strokes_with_point_scale(
    strokes: &[DrawnStroke],
    rect: Rect,
    color: Color,
    thickness: f32,
    point_radius_scale: f32,
) {
    for stroke in strokes {
        for segment in stroke.points.windows(2) {
            let start = point_to_screen(rect, segment[0]);
            let end = point_to_screen(rect, segment[1]);
            draw_line(start.x, start.y, end.x, end.y, thickness, color);
        }
        for point in &stroke.points {
            let point = point_to_screen(rect, *point);
            draw_circle(point.x, point.y, thickness * point_radius_scale, color);
        }
    }
}
