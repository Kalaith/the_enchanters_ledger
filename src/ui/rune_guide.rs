use super::canvas::draw_strokes_with_point_scale;
use super::widgets::{draw_button_plaque, parchment, virtual_button};
use super::{UiAction, UiContext};
use crate::data::{RuneCategory, RuneDef};
use crate::rune_drawing::template_strokes_for_rune;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::RectExt;

pub(super) fn draw_rune_palette(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let rect = Rect::new(956.0, 104.0, 314.0, 498.0);
    super::widgets::draw_panel(rect, "Rune Guide");
    let content = rect.inset(16.0);
    let reference = ctx
        .session
        .board
        .selected_rune
        .as_deref()
        .and_then(|id| ctx.data.rune(id))
        .or_else(|| {
            ctx.data
                .runes
                .iter()
                .find(|rune| ctx.session.can_use_rune(rune))
        });
    if let Some(rune) = reference {
        if virtual_button(
            Rect::new(rect.right() - 96.0, rect.y + 10.0, 76.0, 26.0),
            "Practice",
            ctx.session.can_use_rune(rune),
            ButtonTone::Secondary,
            mouse,
        ) {
            actions.push(UiAction::OpenPractice);
        }
        draw_rune_reference_card(
            ctx,
            rune,
            Rect::new(content.x, content.y + 48.0, content.w, 148.0),
        );
    }

    let col_w = content.w / RuneCategory::ALL.len() as f32;
    let grid_top = content.y + 206.0;
    let known_by_category = RuneCategory::ALL
        .iter()
        .map(|category| {
            ctx.data
                .runes_in_category(*category)
                .filter(|rune| ctx.session.can_use_rune(rune))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let row_step = 39.0;
    let row_h = 36.0;
    let visible_rows = ((content.bottom() - grid_top - 44.0) / row_step)
        .floor()
        .max(1.0) as usize;
    let mut hovered_rune = None;
    for (category_index, category) in RuneCategory::ALL.iter().enumerate() {
        let x = content.x + category_index as f32 * col_w;
        draw_text_centered(
            category.label(),
            x + col_w * 0.5,
            grid_top,
            TextStyle::new(15.0, parchment()),
        );
        let mut y = grid_top + 13.0;
        let known = &known_by_category[category_index];
        let page_count = known.len().div_ceil(visible_rows).max(1);
        let page = ctx.rune_guide_pages[category_index].min(page_count - 1);
        for rune in known.iter().skip(page * visible_rows).take(visible_rows) {
            let selected = ctx
                .session
                .board
                .selected_rune
                .as_deref()
                .is_some_and(|id| id == rune.id);
            let button_rect = Rect::new(x + 4.0, y, col_w - 8.0, row_h);
            if rune_tool_button(button_rect, rune, selected, true, mouse) {
                actions.push(UiAction::SelectRune(rune.id.clone()));
            }
            if selected
                && button_rect.contains_point(mouse)
                && is_mouse_button_released(MouseButton::Right)
            {
                actions.push(UiAction::DeselectRune);
            }
            if button_rect.contains_point(mouse) {
                hovered_rune = Some(rune);
            }
            y += row_step;
        }
        let locked_count = ctx
            .data
            .runes_in_category(*category)
            .filter(|rune| !ctx.session.can_use_rune(rune))
            .count();
        if page_count > 1 {
            draw_page_controls(
                category_index,
                page,
                page_count,
                Rect::new(x + 5.0, content.bottom() - 34.0, col_w - 10.0, 18.0),
                mouse,
                actions,
            );
        }
        if locked_count > 0 {
            draw_text_centered(
                &format!("+{} locked", locked_count),
                x + col_w * 0.5,
                content.bottom() - 7.0,
                TextStyle::new(10.0, Color::new(0.42, 0.46, 0.44, 1.0)),
            );
        }
    }

    if let Some(rune) = hovered_rune {
        draw_tooltip(
            &format!("{}: {}", rune.name, rune.description),
            vec2(mouse.x, mouse.y + 8.0),
        );
    }
}

fn draw_page_controls(
    category_index: usize,
    page: usize,
    page_count: usize,
    rect: Rect,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    if virtual_button(
        Rect::new(rect.x, rect.y, 18.0, rect.h),
        "<",
        page > 0,
        ButtonTone::Muted,
        mouse,
    ) {
        actions.push(UiAction::SetRuneGuidePage(category_index, page - 1));
    }
    if virtual_button(
        Rect::new(rect.right() - 18.0, rect.y, 18.0, rect.h),
        ">",
        page + 1 < page_count,
        ButtonTone::Muted,
        mouse,
    ) {
        actions.push(UiAction::SetRuneGuidePage(category_index, page + 1));
    }
    draw_text_centered(
        &format!("{}/{}", page + 1, page_count),
        rect.center().x,
        rect.y + 13.0,
        TextStyle::new(10.0, Color::new(0.72, 0.66, 0.52, 1.0)),
    );
}

fn draw_rune_reference_card(ctx: &UiContext<'_>, rune: &RuneDef, rect: Rect) {
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.060, 0.065, 0.062, 1.0))
            .with_border(1.0, Color::new(0.72, 0.58, 0.34, 0.45)),
    );

    let preview = Rect::new(rect.x + 10.0, rect.y + 14.0, 88.0, 88.0);
    draw_surface(
        preview,
        &SurfaceStyle::new(Color::new(0.10, 0.075, 0.052, 1.0))
            .with_border(1.0, Color::new(0.72, 0.58, 0.34, 0.38)),
    );
    draw_rune_template_preview(
        &rune.id,
        preview.inset(10.0),
        parchment(),
        2.4,
        ctx.session.can_use_rune(rune),
    );

    draw_text_block(
        &rune.name,
        rect.x + 112.0,
        rect.y + 22.0,
        rect.w - 122.0,
        20.0,
        18.0,
        1.0,
        parchment(),
    );
    draw_text_block(
        &format!(
            "{} | Tier {} | {}",
            rune.category.label(),
            rune.tier,
            if ctx.session.can_use_rune(rune) {
                "Known"
            } else {
                "Locked"
            }
        ),
        rect.x + 112.0,
        rect.y + 45.0,
        rect.w - 122.0,
        18.0,
        13.0,
        1.0,
        Color::new(0.70, 0.75, 0.72, 1.0),
    );
    draw_text_block(
        &rune.description,
        rect.x + 112.0,
        rect.y + 68.0,
        rect.w - 122.0,
        42.0,
        12.0,
        2.0,
        Color::new(0.58, 0.64, 0.61, 1.0),
    );
    draw_text_block(
        "Magnitude: drawn near reference size with a full stroke gives \
         normal potency. Bigger, fully-traced ink raises it; a short or \
         under-sized mark still reads, but weaker.",
        rect.x + 112.0,
        rect.y + 108.0,
        rect.w - 122.0,
        40.0,
        11.0,
        2.0,
        Color::new(0.50, 0.56, 0.53, 1.0),
    );
}

fn rune_tool_button(
    rect: Rect,
    rune: &RuneDef,
    selected: bool,
    enabled: bool,
    mouse: Vec2,
) -> bool {
    let hovered = enabled && rect.contains_point(mouse);
    draw_button_plaque(
        rect,
        ButtonTone::Muted,
        enabled,
        hovered,
        hovered && is_mouse_button_down(MouseButton::Left),
        selected,
    );
    draw_rune_template_preview(
        &rune.id,
        centered_square(rect.inset(5.0)),
        if enabled {
            parchment()
        } else {
            Color::new(0.34, 0.36, 0.35, 1.0)
        },
        1.25,
        enabled,
    );
    hovered && is_mouse_button_released(MouseButton::Left)
}

fn centered_square(rect: Rect) -> Rect {
    let size = rect.w.min(rect.h);
    Rect::new(
        rect.center().x - size * 0.5,
        rect.center().y - size * 0.5,
        size,
        size,
    )
}

fn draw_rune_template_preview(
    rune_id: &str,
    rect: Rect,
    color: Color,
    thickness: f32,
    enabled: bool,
) {
    let Some(strokes) = template_strokes_for_rune(rune_id) else {
        return;
    };
    let alpha = if enabled { color.a } else { color.a * 0.65 };
    let color = Color::new(color.r, color.g, color.b, alpha);
    draw_strokes_with_point_scale(&strokes, rect, color, thickness, 0.42);
}
