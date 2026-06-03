use super::widgets::{
    ink, mouse_over_rect, muted_ink, parchment_line, parchment_page, virtual_button,
};
use super::{UiAction, UiContext};
use crate::rune_drawing::{template_strokes_for_rune, DrawnStroke, StrokePoint};
use crate::state::GuideTemplate;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;

const INK_THICKNESS: f32 = 5.0;
const ERASER_RADIUS_PIXELS: f32 = 18.0;

pub(super) fn draw_drawing_slate(
    ctx: &UiContext<'_>,
    rect: Rect,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    let slate = Rect::new(rect.x, rect.y + 24.0, rect.w, rect.h - 92.0);
    let controls = Rect::new(
        rect.x,
        slate.bottom() + 12.0,
        rect.w,
        rect.bottom() - slate.bottom() - 12.0,
    );
    draw_text_ex(
        "Diagram Slate",
        rect.x,
        rect.y + 16.0,
        TextStyle::new(15.0, ink()).params(),
    );
    draw_surface(
        slate,
        &SurfaceStyle::new(parchment_page())
            .with_border(1.5, Color::new(0.42, 0.27, 0.12, 0.84))
            .with_inner_border(5.0, 1.0, Color::new(0.42, 0.27, 0.12, 0.14)),
    );
    draw_slate_grid(slate);

    draw_guide_templates(
        &ctx.session.board.guide_templates,
        slate,
        ctx.interpretation_feedback_active
            .then_some(ctx.session.board.last_diagram.as_ref())
            .flatten(),
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
        }
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
        Rect::new(controls.x + 328.0, controls.y + 34.0, 64.0, 26.0),
        "Diag",
        !drawing_active,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::CopyDiagnostics);
    }

    let note_x = controls.x + 404.0;
    if let Some(note) = &ctx.session.board.last_interpretation_note {
        draw_text_block(
            note,
            note_x,
            controls.y + 4.0,
            controls.right() - note_x,
            controls.h - 4.0,
            12.0,
            2.0,
            ink(),
        );
    } else if let Some(diagram) = &ctx.session.board.last_diagram {
        let names = diagram
            .runes
            .iter()
            .take(3)
            .map(|rune| ctx.data.rune_name(&rune.rune_id).to_owned())
            .collect::<Vec<_>>()
            .join(" + ");
        let names = if names.is_empty() {
            "No clear runes".to_owned()
        } else {
            names
        };
        draw_text_block(
            &format!(
                "Read: {} | circle {}%",
                names,
                (diagram.circle_quality * 100.0).round() as i32
            ),
            note_x,
            controls.y + 4.0,
            controls.right() - note_x,
            controls.h - 4.0,
            13.0,
            2.0,
            if diagram.accepted() {
                Color::new(0.14, 0.40, 0.22, 1.0)
            } else {
                Color::new(0.52, 0.20, 0.13, 1.0)
            },
        );
    } else {
        draw_text_block(
            "Awaiting interpretation.",
            note_x,
            controls.y + 4.0,
            controls.right() - note_x,
            controls.h - 4.0,
            13.0,
            2.0,
            muted_ink(),
        );
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

fn point_in_rect(rect: Rect, point: Vec2) -> StrokePoint {
    StrokePoint::new((point.x - rect.x) / rect.w, (point.y - rect.y) / rect.h)
}

fn eraser_radius_in_rect(rect: Rect) -> f32 {
    ERASER_RADIUS_PIXELS / rect.w.min(rect.h).max(1.0)
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

fn draw_guide_templates(
    templates: &[GuideTemplate],
    rect: Rect,
    diagram: Option<&crate::rune_diagram::DiagramInterpretation>,
) {
    for template in templates {
        if let Some(diagram) = diagram {
            draw_guide_feedback(template, rect, guide_template_was_read(template, diagram));
        }
        draw_guide_template_at(
            &template.rune_id,
            template.center,
            template.scale,
            rect,
            Color::new(0.40, 0.27, 0.10, 0.32),
            1.8,
        );
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

fn point_distance(a: StrokePoint, b: StrokePoint) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

fn guide_point_to_screen(rect: Rect, center: StrokePoint, scale: f32, point: StrokePoint) -> Vec2 {
    let base = rect.w.min(rect.h);
    let center = point_to_screen(rect, center);
    vec2(
        center.x + (point.x - 0.5) * scale * base,
        center.y + (point.y - 0.5) * scale * base,
    )
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

fn draw_spell_feedback(diagram: &crate::rune_diagram::DiagramInterpretation, rect: Rect) {
    let Some(spell) = &diagram.spell else {
        return;
    };
    let center = rect.center();
    let base_radius = rect.w.min(rect.h) * 0.42;
    let alpha = (0.14 + spell.complexity * 0.30).clamp(0.16, 0.48);
    let color = if spell.tier_rank >= 4 {
        Color::new(0.38, 0.82, 0.95, alpha)
    } else if spell.tier_rank >= 3 {
        Color::new(0.86, 0.64, 0.20, alpha)
    } else {
        Color::new(0.44, 0.70, 0.44, alpha)
    };
    for index in 0..spell.ring_count.clamp(1, 4) {
        let radius = base_radius * (0.88 - index as f32 * 0.12);
        draw_circle_lines(center.x, center.y, radius, 2.0, color);
    }
    for index in 0..spell.satellite_count.min(10) {
        let angle = std::f32::consts::TAU * index as f32 / spell.satellite_count.max(1) as f32;
        let point = vec2(
            center.x + angle.cos() * base_radius * 0.62,
            center.y + angle.sin() * base_radius * 0.62,
        );
        draw_circle_lines(point.x, point.y, 8.0 + spell.intensity * 5.0, 1.7, color);
    }
    for index in 0..spell.radial_count.min(8) {
        let angle = std::f32::consts::TAU * index as f32 / spell.radial_count.max(1) as f32;
        draw_line(
            center.x,
            center.y,
            center.x + angle.cos() * base_radius * 0.72,
            center.y + angle.sin() * base_radius * 0.72,
            1.2,
            Color::new(color.r, color.g, color.b, color.a * 0.55),
        );
    }
    for index in 0..spell.script_mark_count.min(36) {
        let angle = std::f32::consts::TAU * index as f32 / spell.script_mark_count.max(1) as f32;
        let radius = base_radius * (0.40 + (index % 3) as f32 * 0.10);
        let point = vec2(
            center.x + angle.cos() * radius,
            center.y + angle.sin() * radius,
        );
        draw_circle(
            point.x,
            point.y,
            1.6,
            Color::new(color.r, color.g, color.b, color.a * 0.70),
        );
    }
}

fn point_to_screen(rect: Rect, point: StrokePoint) -> Vec2 {
    vec2(rect.x + point.x * rect.w, rect.y + point.y * rect.h)
}
