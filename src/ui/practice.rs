use super::widgets::{
    brass, brass_dim, ink, mouse_over_rect, panel_dark, parchment, parchment_line, parchment_page,
    virtual_button,
};
use super::{UiAction, UiContext, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use crate::rune_drawing::{template_strokes_for_rune, DrawnStroke, StrokePoint};
use crate::rune_quality::{quality_label, RunePracticeReport};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::RectExt;

const PRACTICE_INK: f32 = 5.0;
const ERASER_RADIUS_PIXELS: f32 = 18.0;

pub(super) fn draw_practice_overlay(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    draw_rectangle(
        0.0,
        0.0,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT,
        Color::new(0.0, 0.0, 0.0, 0.60),
    );
    let rect = Rect::new(18.0, 18.0, LOGICAL_WIDTH - 36.0, LOGICAL_HEIGHT - 36.0);
    super::widgets::draw_panel(rect, "Rune Practice");
    if virtual_button(
        Rect::new(rect.right() - 94.0, rect.y + 10.0, 72.0, 28.0),
        "Close",
        true,
        ButtonTone::Muted,
        mouse,
    ) {
        actions.push(UiAction::ClosePractice);
    }

    let Some(rune_id) = ctx.session.board.selected_rune.as_deref() else {
        draw_text_centered(
            "Select a rune in the guide to practice it.",
            rect.center().x,
            rect.center().y,
            TextStyle::new(18.0, parchment()),
        );
        return;
    };
    let rune_name = ctx.data.rune_name(rune_id);
    let content = rect.inset(24.0);
    let gap = 20.0;
    let section_top = content.y + 54.0;
    let section_h = 500.0;
    let reference_w = 274.0;
    let feedback_w = 262.0;
    let slate_w = content.w - reference_w - feedback_w - gap * 2.0;
    let reference = Rect::new(content.x, section_top, reference_w, section_h);
    let slate = Rect::new(reference.right() + gap, section_top, slate_w, section_h);
    let feedback = Rect::new(slate.right() + gap, section_top, feedback_w, section_h);
    draw_reference(ctx, rune_id, rune_name, reference);
    draw_practice_slate(ctx, rune_id, slate, mouse, actions);
    draw_feedback(ctx.practice.report, feedback, ctx);

    let controls = Rect::new(slate.x, slate.bottom() + 16.0, slate.w, 42.0);
    if virtual_button(
        Rect::new(controls.x, controls.y, 130.0, 32.0),
        "Check Mark",
        !ctx.practice.strokes.is_empty() && ctx.practice.active_stroke.is_none(),
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::ScorePractice);
    }
    if virtual_button(
        Rect::new(controls.x + 146.0, controls.y, 118.0, 32.0),
        "Clear",
        !ctx.practice.strokes.is_empty() && ctx.practice.active_stroke.is_none(),
        ButtonTone::Muted,
        mouse,
    ) {
        actions.push(UiAction::ClearPractice);
    }
    draw_text_block(
        "Quality affects rewards, stability, mana waste, and final ratings.",
        controls.x + 282.0,
        controls.y + 6.0,
        controls.w - 282.0,
        32.0,
        13.0,
        2.0,
        parchment(),
    );
}

fn draw_reference(ctx: &UiContext<'_>, rune_id: &str, rune_name: &str, rect: Rect) {
    draw_section(rect, "Reference");
    draw_text_centered(
        rune_name,
        rect.center().x,
        rect.y + 58.0,
        TextStyle::new(22.0, parchment()),
    );
    let preview = Rect::new(rect.x + 28.0, rect.y + 84.0, rect.w - 56.0, 228.0);
    draw_surface(
        preview,
        &SurfaceStyle::new(Color::new(0.10, 0.075, 0.052, 1.0))
            .with_border(1.0, Color::new(0.72, 0.58, 0.34, 0.38)),
    );
    draw_template_with_marks(
        rune_id,
        preview.inset(18.0),
        Color::new(0.92, 0.86, 0.70, 0.72),
    );
    let description = ctx
        .data
        .rune(rune_id)
        .map(|rune| rune.description.as_str())
        .unwrap_or("");
    draw_text_block(
        description,
        rect.x + 18.0,
        rect.y + 336.0,
        rect.w - 36.0,
        86.0,
        13.0,
        3.0,
        Color::new(0.68, 0.72, 0.66, 1.0),
    );
}

fn draw_practice_slate(
    ctx: &UiContext<'_>,
    rune_id: &str,
    rect: Rect,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    draw_section(rect, "Practice Slate");
    let page = Rect::new(rect.x + 20.0, rect.y + 48.0, rect.w - 40.0, rect.h - 68.0);
    draw_surface(
        page,
        &SurfaceStyle::new(parchment_page())
            .with_border(1.5, Color::new(0.42, 0.27, 0.12, 0.84))
            .with_inner_border(5.0, 1.0, Color::new(0.42, 0.27, 0.12, 0.14)),
    );
    draw_practice_grid(page);
    draw_template_with_marks(
        rune_id,
        page.inset(64.0),
        Color::new(0.35, 0.23, 0.12, 0.28),
    );
    draw_strokes(
        ctx.practice.strokes,
        page,
        Color::new(0.045, 0.028, 0.014, 0.96),
        PRACTICE_INK,
    );
    if let Some(active) = ctx.practice.active_stroke {
        draw_strokes(
            std::slice::from_ref(active),
            page,
            Color::new(0.045, 0.028, 0.014, 0.96),
            PRACTICE_INK,
        );
    }

    let drawing_active = ctx.practice.active_stroke.is_some();
    let erasing = mouse_over_rect(page, mouse) && is_mouse_button_down(MouseButton::Right);
    if mouse_over_rect(page, mouse) {
        draw_circle_lines(
            mouse.x,
            mouse.y,
            ERASER_RADIUS_PIXELS,
            if erasing { 2.0 } else { 1.0 },
            Color::new(0.38, 0.24, 0.12, if erasing { 0.72 } else { 0.34 }),
        );
    }
    if erasing {
        actions.push(UiAction::ErasePracticeInk(
            point_in_rect(page, mouse),
            eraser_radius_in_rect(page),
        ));
    } else if mouse_over_rect(page, mouse) && is_mouse_button_pressed(MouseButton::Left) {
        actions.push(UiAction::StartPracticeStroke(point_in_rect(page, mouse)));
    }
    if !erasing && drawing_active && is_mouse_button_down(MouseButton::Left) {
        actions.push(UiAction::ExtendPracticeStroke(point_in_rect(page, mouse)));
    }
    if drawing_active && is_mouse_button_released(MouseButton::Left) {
        actions.push(UiAction::FinishPracticeStroke);
    }
}

fn draw_feedback(report: Option<&RunePracticeReport>, rect: Rect, ctx: &UiContext<'_>) {
    draw_section(rect, "Score");
    let Some(report) = report else {
        draw_text_block(
            "Practice marks are scored for shape, start point, and stroke order.",
            rect.x + 16.0,
            rect.y + 56.0,
            rect.w - 32.0,
            72.0,
            14.0,
            3.0,
            parchment(),
        );
        return;
    };
    let read_name = report
        .read_rune_id
        .as_deref()
        .map(|id| ctx.data.rune_name(id))
        .unwrap_or("Unread");
    let target_name = ctx.data.rune_name(&report.target_rune_id);
    draw_text_block(
        &format!(
            "Target: {}\nRead: {} | Confidence {}%",
            target_name,
            read_name,
            (report.confidence * 100.0).round() as i32
        ),
        rect.x + 16.0,
        rect.y + 56.0,
        rect.w - 32.0,
        42.0,
        13.0,
        2.0,
        parchment(),
    );
    draw_score_bar(
        rect.x + 16.0,
        rect.y + 124.0,
        rect.w - 32.0,
        "Quality",
        report.quality,
    );
    draw_score_bar(
        rect.x + 16.0,
        rect.y + 168.0,
        rect.w - 32.0,
        "Shape",
        report.shape_score,
    );
    draw_score_bar(
        rect.x + 16.0,
        rect.y + 212.0,
        rect.w - 32.0,
        "Start",
        report.start_score,
    );
    draw_score_bar(
        rect.x + 16.0,
        rect.y + 256.0,
        rect.w - 32.0,
        "Order",
        report.stroke_order_score,
    );
    draw_text_block(
        &format!("{}: {}", quality_label(report.quality), report.feedback),
        rect.x + 16.0,
        rect.y + 304.0,
        rect.w - 32.0,
        52.0,
        13.0,
        3.0,
        if report.accepted {
            Color::new(0.68, 0.86, 0.64, 1.0)
        } else {
            parchment()
        },
    );
}

fn draw_score_bar(x: f32, y: f32, w: f32, label: &str, score: f32) {
    draw_text_ex(label, x, y, TextStyle::new(13.0, parchment()).params());
    let bar = Rect::new(x, y + 8.0, w, 14.0);
    draw_surface(
        bar,
        &SurfaceStyle::new(Color::new(0.050, 0.045, 0.038, 1.0)).with_border(1.0, brass_dim()),
    );
    draw_rectangle(
        bar.x + 2.0,
        bar.y + 2.0,
        (bar.w - 4.0) * score.clamp(0.0, 1.0),
        bar.h - 4.0,
        Color::new(0.62, 0.44, 0.18, 1.0),
    );
    draw_text_right(
        &format!("{}%", (score * 100.0).round() as i32),
        bar.right(),
        y,
        TextStyle::new(12.0, parchment()),
    );
}

fn draw_template_with_marks(rune_id: &str, rect: Rect, color: Color) {
    let Some(strokes) = template_strokes_for_rune(rune_id) else {
        return;
    };
    draw_strokes(&strokes, rect, color, 2.0);
    for (index, stroke) in strokes.iter().enumerate() {
        let Some(start) = stroke.points.first() else {
            continue;
        };
        let point = point_to_screen(rect, *start);
        draw_circle(point.x, point.y, 6.0, Color::new(0.90, 0.64, 0.18, 0.88));
        draw_text_centered(
            &(index + 1).to_string(),
            point.x,
            point.y + 4.0,
            TextStyle::new(9.0, ink()),
        );
    }
}

fn draw_practice_grid(rect: Rect) {
    for i in 1..5 {
        let x = rect.x + rect.w * i as f32 / 5.0;
        draw_line(
            x,
            rect.y + 12.0,
            x,
            rect.bottom() - 12.0,
            1.0,
            parchment_line(),
        );
    }
    for i in 1..4 {
        let y = rect.y + rect.h * i as f32 / 4.0;
        draw_line(
            rect.x + 12.0,
            y,
            rect.right() - 12.0,
            y,
            1.0,
            parchment_line(),
        );
    }
}

fn draw_section(rect: Rect, title: &str) {
    draw_surface(
        rect,
        &SurfaceStyle::new(panel_dark())
            .with_border(1.0, brass_dim())
            .with_inner_border(5.0, 1.0, Color::new(0.95, 0.72, 0.32, 0.08)),
    );
    draw_text_centered(
        title,
        rect.center().x,
        rect.y + 25.0,
        TextStyle::new(17.0, brass()),
    );
}

fn point_in_rect(rect: Rect, point: Vec2) -> StrokePoint {
    StrokePoint::new((point.x - rect.x) / rect.w, (point.y - rect.y) / rect.h)
}

fn eraser_radius_in_rect(rect: Rect) -> f32 {
    ERASER_RADIUS_PIXELS / rect.w.min(rect.h).max(1.0)
}

fn draw_strokes(strokes: &[DrawnStroke], rect: Rect, color: Color, thickness: f32) {
    for stroke in strokes {
        for segment in stroke.points.windows(2) {
            let start = point_to_screen(rect, segment[0]);
            let end = point_to_screen(rect, segment[1]);
            draw_line(start.x, start.y, end.x, end.y, thickness, color);
        }
        for point in &stroke.points {
            let point = point_to_screen(rect, *point);
            draw_circle(point.x, point.y, thickness * 0.45, color);
        }
    }
}

fn point_to_screen(rect: Rect, point: StrokePoint) -> Vec2 {
    vec2(rect.x + point.x * rect.w, rect.y + point.y * rect.h)
}
