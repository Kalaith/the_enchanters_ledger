mod feedback;

use super::canvas::{
    draw_strokes, eraser_radius_in_rect, point_distance, point_in_rect, point_to_screen,
    ERASER_RADIUS_PIXELS,
};
use super::widgets::{mouse_over_rect, muted_ink, parchment_line, parchment_page, virtual_button};
use super::{UiAction, UiContext};
use crate::rune_drawing::{template_strokes_for_rune, StrokePoint};
use crate::state::{CircleGuide, GuideTemplate, RuneMastery};
use feedback::{draw_potency_tags, draw_reading, draw_spell_feedback};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use std::collections::HashMap;

const INK_THICKNESS: f32 = 5.0;

pub(super) fn draw_drawing_slate(
    ctx: &UiContext<'_>,
    rect: Rect,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    // The panel title ("Drawn Diagram") already names this surface, so the slate starts near the
    // top edge; height grows to keep the same bottom, leaving the controls strip unchanged.
    let slate = Rect::new(rect.x, rect.y + 4.0, rect.w, rect.h - 72.0);
    let controls = Rect::new(
        rect.x,
        slate.bottom() + 12.0,
        rect.w,
        rect.bottom() - slate.bottom() - 12.0,
    );
    draw_surface(
        slate,
        &SurfaceStyle::new(parchment_page())
            .with_border(1.5, Color::new(0.42, 0.27, 0.12, 0.84))
            .with_inner_border(5.0, 1.0, Color::new(0.42, 0.27, 0.12, 0.14)),
    );
    draw_slate_grid(slate);

    if let Some(circle) = &ctx.session.board.circle_guide {
        draw_circle_guide(circle, slate);
    }
    draw_strokes(
        &ctx.session.board.guide_structure,
        slate,
        Color::new(0.40, 0.27, 0.10, GUIDE_BASE_ALPHA),
        1.8,
    );
    draw_guide_templates(
        &ctx.session.board.guide_templates,
        slate,
        ctx.interpretation_feedback_active
            .then_some(ctx.session.board.last_diagram.as_ref())
            .flatten(),
        &ctx.session.player.rune_mastery,
    );
    if ctx.session.board.template_armed {
        if let Some(rune_id) = ctx.session.board.selected_rune.as_deref() {
            if mouse_over_rect(slate, mouse) {
                draw_guide_template_at(
                    rune_id,
                    point_in_rect(slate, mouse),
                    0.22,
                    slate,
                    Color::new(0.12, 0.26, 0.34, 0.42),
                    2.0,
                );
            }
        }
    }

    draw_strokes(
        &ctx.session.board.drawing_strokes,
        slate,
        Color::new(0.045, 0.028, 0.014, 0.96),
        INK_THICKNESS,
    );
    if let Some(active) = &ctx.session.board.active_stroke {
        draw_strokes(
            std::slice::from_ref(active),
            slate,
            Color::new(0.045, 0.028, 0.014, 0.96),
            INK_THICKNESS,
        );
    }
    if ctx.interpretation_feedback_active {
        if let Some(diagram) = &ctx.session.board.last_diagram {
            draw_spell_feedback(diagram, slate);
            draw_potency_tags(diagram, slate);
        }
        draw_reading(ctx.reading_lines, slate);
    }

    let drawing_active = ctx.session.board.active_stroke.is_some();
    let mouse_on_slate = mouse_over_rect(slate, mouse);
    let guide_edit_enabled =
        ctx.guide_edit_mode && !ctx.session.board.template_armed && !drawing_active;
    let hovered_template = if guide_edit_enabled && mouse_on_slate {
        guide_template_hit(&ctx.session.board.guide_templates, slate, mouse)
    } else {
        None
    };
    let hovered_remove_handle = if guide_edit_enabled && mouse_on_slate {
        guide_template_remove_handle_hit(&ctx.session.board.guide_templates, slate, mouse)
    } else {
        None
    };
    if let Some(index) = hovered_remove_handle.or(hovered_template) {
        if let Some(template) = ctx.session.board.guide_templates.get(index) {
            draw_guide_remove_handle(template, slate, hovered_remove_handle == Some(index));
        }
    }

    let cancel_template =
        ctx.session.board.template_armed && is_mouse_button_released(MouseButton::Right);
    if cancel_template {
        actions.push(UiAction::DeselectRune);
    }
    let remove_template_by_handle =
        hovered_remove_handle.filter(|_| is_mouse_button_pressed(MouseButton::Left));
    if let Some(index) = remove_template_by_handle {
        actions.push(UiAction::RemoveRuneTemplate(index));
    }
    if guide_edit_enabled
        && remove_template_by_handle.is_none()
        && is_mouse_button_down(MouseButton::Left)
    {
        if let Some(index) = hovered_template {
            actions.push(UiAction::MoveRuneTemplate(
                index,
                point_in_rect(slate, mouse),
            ));
        }
    }
    let erasing = !ctx.session.board.template_armed
        && mouse_on_slate
        && !ctx.guide_edit_mode
        && !ctx.suppress_rune_erase
        && is_mouse_button_down(MouseButton::Right);
    if mouse_on_slate {
        let eraser_color = if erasing {
            Color::new(0.38, 0.24, 0.12, 0.72)
        } else {
            Color::new(0.38, 0.24, 0.12, 0.34)
        };
        draw_circle_lines(
            mouse.x,
            mouse.y,
            ERASER_RADIUS_PIXELS,
            if erasing { 2.0 } else { 1.0 },
            eraser_color,
        );
    }

    if erasing {
        actions.push(UiAction::EraseRuneInk(
            point_in_rect(slate, mouse),
            eraser_radius_in_rect(slate),
        ));
    }

    let placing_template = ctx.session.board.template_armed && mouse_on_slate && !drawing_active;
    if !erasing && placing_template && is_mouse_button_pressed(MouseButton::Left) {
        actions.push(UiAction::PlaceRuneTemplate(point_in_rect(slate, mouse)));
    } else if !erasing
        && !ctx.guide_edit_mode
        && remove_template_by_handle.is_none()
        && mouse_on_slate
        && is_mouse_button_pressed(MouseButton::Left)
    {
        actions.push(UiAction::StartRuneStroke(point_in_rect(slate, mouse)));
    }
    if !erasing && drawing_active && is_mouse_button_down(MouseButton::Left) {
        actions.push(UiAction::ExtendRuneStroke(point_in_rect(slate, mouse)));
    }
    if drawing_active && is_mouse_button_released(MouseButton::Left) {
        actions.push(UiAction::FinishRuneStroke);
    }

    let has_ink =
        !ctx.session.board.drawing_strokes.is_empty() || ctx.session.board.active_stroke.is_some();
    let guide_label = ctx
        .session
        .board
        .template_armed
        .then(|| {
            ctx.session
                .board
                .selected_rune
                .as_deref()
                .map(|id| format!("Place {} guide", ctx.data.rune_name(id)))
        })
        .flatten()
        .unwrap_or_else(|| {
            if ctx.guide_edit_mode {
                "Guide edit: drag guides, click X to delete".to_owned()
            } else {
                "Circle + inner runes".to_owned()
            }
        });
    draw_text_block(
        &guide_label,
        controls.x,
        controls.y + 4.0,
        136.0,
        24.0,
        14.0,
        2.0,
        muted_ink(),
    );
    // Sits on the label row rather than the button strip below, which is already
    // full: this is a "show me what it should look like" aid, not a fifth verb.
    if virtual_button(
        Rect::new(controls.x + 150.0, controls.y + 2.0, 104.0, 26.0),
        if ctx.session.sandbox_mode {
            "Ink Ideal"
        } else {
            "Reference"
        },
        !drawing_active,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::ShowReferenceDiagram);
    }
    // Gameplay actions first; the interpret/test result now lives in the ledger status block, so
    // the strip is buttons only and no longer overflows the panel edge.
    if virtual_button(
        Rect::new(controls.x, controls.y + 34.0, 110.0, 26.0),
        "Interpret",
        has_ink && !drawing_active,
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::InterpretDiagram);
    }
    if virtual_button(
        Rect::new(controls.x + 120.0, controls.y + 34.0, 96.0, 26.0),
        "Clear Ink",
        has_ink && !drawing_active,
        ButtonTone::Muted,
        mouse,
    ) {
        actions.push(UiAction::ClearRuneDrawing);
    }
    if virtual_button(
        Rect::new(controls.x + 226.0, controls.y + 34.0, 92.0, 26.0),
        if ctx.guide_edit_mode {
            "Ink Mode"
        } else {
            "Guides"
        },
        !drawing_active,
        if ctx.guide_edit_mode {
            ButtonTone::Positive
        } else {
            ButtonTone::Muted
        },
        mouse,
    ) {
        actions.push(UiAction::ToggleGuideEditMode);
    }
    if virtual_button(
        Rect::new(controls.x + 328.0, controls.y + 34.0, 76.0, 26.0),
        if ctx.session.sandbox_mode {
            "Sandbox*"
        } else {
            "Sandbox"
        },
        !drawing_active,
        if ctx.session.sandbox_mode {
            ButtonTone::Positive
        } else {
            ButtonTone::Muted
        },
        mouse,
    ) {
        actions.push(UiAction::ToggleSandboxMode);
    }
    // Diagnostics is a developer aid; compile it out of release builds entirely.
    #[cfg(debug_assertions)]
    if virtual_button(
        Rect::new(controls.x + 414.0, controls.y + 34.0, 64.0, 26.0),
        "Diag",
        !drawing_active,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::CopyDiagnostics);
    }
}

fn draw_slate_grid(rect: Rect) {
    let cols = 9;
    let rows = 5;
    for col in 1..cols {
        let x = rect.x + rect.w * col as f32 / cols as f32;
        draw_line(
            x,
            rect.y + 10.0,
            x,
            rect.bottom() - 10.0,
            1.0,
            parchment_line(),
        );
    }
    for row in 1..rows {
        let y = rect.y + rect.h * row as f32 / rows as f32;
        draw_line(
            rect.x + 10.0,
            y,
            rect.right() - 10.0,
            y,
            1.0,
            parchment_line(),
        );
    }
}

fn guide_template_hit(templates: &[GuideTemplate], rect: Rect, mouse: Vec2) -> Option<usize> {
    templates
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, template)| {
            let center = point_to_screen(rect, template.center);
            let dx = mouse.x - center.x;
            let dy = mouse.y - center.y;
            let radius = guide_template_screen_radius(template, rect) * 1.08;
            (dx * dx + dy * dy <= radius * radius).then_some(index)
        })
}

fn guide_template_remove_handle_hit(
    templates: &[GuideTemplate],
    rect: Rect,
    mouse: Vec2,
) -> Option<usize> {
    templates
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, template)| {
            guide_template_remove_handle_rect(template, rect)
                .contains(mouse)
                .then_some(index)
        })
}

/// Base guide-line alpha at zero mastery — unchanged from the pre-Phase-5 hardcoded value, so a
/// rune drawn for the first time still gets the same full-strength guide it always has.
const GUIDE_BASE_ALPHA: f32 = 0.32;
/// How much mastery it takes to meaningfully fade a guide (plan Phase 5 item 2): the curve is
/// `1.0 / (1.0 + score / GUIDE_FADE_SCALE)`, so alpha is halved around `score ==
/// GUIDE_FADE_SCALE` and keeps approaching (never hard-cutting to) zero beyond it — the same
/// diminishing-not-capped shape as `magical_circle::diminishing_count`, reused here for the same
/// reason: no cliff a single extra practice rep could visibly jump across.
const GUIDE_FADE_SCALE: f32 = 6.0;

fn draw_guide_templates(
    templates: &[GuideTemplate],
    rect: Rect,
    diagram: Option<&crate::rune_diagram::DiagramInterpretation>,
    rune_mastery: &HashMap<String, RuneMastery>,
) {
    for template in templates {
        if let Some(diagram) = diagram {
            draw_guide_feedback(template, rect, guide_template_was_read(template, diagram));
        }
        let score = rune_mastery
            .get(&template.rune_id)
            .map_or(0.0, RuneMastery::score);
        let alpha = GUIDE_BASE_ALPHA / (1.0 + score / GUIDE_FADE_SCALE);
        draw_guide_template_at(
            &template.rune_id,
            template.center,
            template.scale,
            rect,
            Color::new(0.40, 0.27, 0.10, alpha),
            1.8,
        );
    }
}

/// Draws the working-circle tracing guide from the very points the reference ink
/// would be made of, rather than a screen-space `draw_circle_lines`. Slate
/// coordinates are normalized, so on a wider-than-tall slate a circle the
/// recognizer scores as perfectly round renders as an ellipse — tracing the
/// drawn line is what earns the score, so the drawn line has to be the real one.
fn draw_circle_guide(circle: &CircleGuide, rect: Rect) {
    let points = crate::perfect_diagram::circle_points(circle.center, circle.radius);
    let color = Color::new(0.40, 0.27, 0.10, GUIDE_BASE_ALPHA);
    for segment in points.windows(2) {
        let start = point_to_screen(rect, segment[0]);
        let end = point_to_screen(rect, segment[1]);
        draw_line(start.x, start.y, end.x, end.y, 1.8, color);
    }
}

fn draw_guide_feedback(template: &GuideTemplate, rect: Rect, read: bool) {
    let center = point_to_screen(rect, template.center);
    let radius = rect.w.min(rect.h) * template.scale * 0.42;
    let pulse = ((get_time() as f32 * 10.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
    let ring = if read {
        Color::new(0.95, 0.70, 0.18, 0.52 + pulse * 0.34)
    } else {
        Color::new(0.82, 0.16, 0.12, 0.48 + pulse * 0.34)
    };
    draw_circle_lines(center.x, center.y, radius, 2.4, ring);
    draw_circle_lines(
        center.x,
        center.y,
        radius + 4.0,
        1.2,
        Color::new(ring.r, ring.g, ring.b, ring.a * 0.42),
    );
}

fn draw_guide_remove_handle(template: &GuideTemplate, rect: Rect, highlighted: bool) {
    let handle = guide_template_remove_handle_rect(template, rect);
    let center = handle.center();
    let fill = if highlighted {
        Color::new(0.48, 0.14, 0.10, 0.92)
    } else {
        Color::new(0.20, 0.12, 0.08, 0.78)
    };
    let line = Color::new(0.94, 0.80, 0.54, 0.94);

    draw_circle(center.x, center.y, handle.w * 0.5, fill);
    draw_circle_lines(
        center.x,
        center.y,
        handle.w * 0.5 - 1.0,
        1.2,
        Color::new(0.94, 0.62, 0.30, 0.70),
    );
    draw_line(
        center.x - 4.2,
        center.y - 4.2,
        center.x + 4.2,
        center.y + 4.2,
        1.8,
        line,
    );
    draw_line(
        center.x + 4.2,
        center.y - 4.2,
        center.x - 4.2,
        center.y + 4.2,
        1.8,
        line,
    );
}

fn guide_template_remove_handle_rect(template: &GuideTemplate, rect: Rect) -> Rect {
    let center = point_to_screen(rect, template.center);
    let radius = guide_template_screen_radius(template, rect);
    let size = 17.0;
    Rect::new(
        (center.x + radius * 0.50).clamp(rect.x + 2.0, rect.right() - size - 2.0),
        (center.y - radius * 0.70 - size * 0.5).clamp(rect.y + 2.0, rect.bottom() - size - 2.0),
        size,
        size,
    )
}

fn guide_template_screen_radius(template: &GuideTemplate, rect: Rect) -> f32 {
    rect.w.min(rect.h) * template.scale * 0.48
}

fn guide_template_was_read(
    template: &GuideTemplate,
    diagram: &crate::rune_diagram::DiagramInterpretation,
) -> bool {
    diagram.runes.iter().any(|rune| {
        rune.rune_id == template.rune_id && point_distance(rune.center, template.center) <= 0.20
    })
}

fn draw_guide_template_at(
    rune_id: &str,
    center: StrokePoint,
    scale: f32,
    rect: Rect,
    color: Color,
    thickness: f32,
) {
    let Some(strokes) = template_strokes_for_rune(rune_id) else {
        return;
    };
    for stroke in strokes {
        for segment in stroke.points.windows(2) {
            let start = guide_point_to_screen(rect, center, scale, segment[0]);
            let end = guide_point_to_screen(rect, center, scale, segment[1]);
            draw_line(start.x, start.y, end.x, end.y, thickness, color);
        }
        for point in &stroke.points {
            let point = guide_point_to_screen(rect, center, scale, *point);
            draw_circle(point.x, point.y, thickness * 0.5, color);
        }
    }
}

fn guide_point_to_screen(rect: Rect, center: StrokePoint, scale: f32, point: StrokePoint) -> Vec2 {
    let base = rect.w.min(rect.h);
    let center = point_to_screen(rect, center);
    vec2(
        center.x + (point.x - 0.5) * scale * base,
        center.y + (point.y - 0.5) * scale * base,
    )
}
