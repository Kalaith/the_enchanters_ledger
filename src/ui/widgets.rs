use crate::state::EnchantGrade;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::RectExt;

pub(super) fn draw_panel(rect: Rect, title: &str) {
    draw_surface_with_title(
        rect,
        Some(title),
        &SurfaceStyle::new(panel_dark())
            .with_border(1.0, brass_dim())
            .with_inner_border(5.0, 1.0, Color::new(0.95, 0.72, 0.32, 0.10))
            .with_header(40.0, panel_header())
            .with_header_divider(1.0, Color::new(0.64, 0.50, 0.30, 0.36)),
        TextStyle::new(18.0, parchment()),
    );
    draw_corner_marks(rect, Color::new(0.82, 0.62, 0.30, 0.42));
}

pub(super) fn draw_desk_backdrop(width: f32, height: f32) {
    clear_background(desk_dark());
    draw_rectangle(
        0.0,
        0.0,
        width,
        height,
        Color::new(0.055, 0.047, 0.037, 1.0),
    );
    for i in 0..10 {
        let y = i as f32 * 78.0 - 18.0;
        draw_rectangle(
            0.0,
            y,
            width,
            72.0,
            if i % 2 == 0 {
                Color::new(0.080, 0.055, 0.036, 0.58)
            } else {
                Color::new(0.060, 0.044, 0.033, 0.46)
            },
        );
        draw_line(0.0, y + 72.0, width, y + 72.0, 1.0, wood_grain());
    }
    for i in 0..18 {
        let x = i as f32 * 82.0 - 30.0;
        draw_line(
            x,
            0.0,
            x + 78.0,
            height,
            1.0,
            Color::new(0.17, 0.105, 0.054, 0.12),
        );
    }
    draw_rectangle(
        0.0,
        604.0,
        width,
        height - 604.0,
        Color::new(0.050, 0.038, 0.029, 0.72),
    );
    draw_line(
        0.0,
        604.0,
        width,
        604.0,
        2.0,
        Color::new(0.23, 0.15, 0.07, 0.45),
    );
}

pub(super) fn draw_parchment_sheet(rect: Rect) {
    draw_surface(
        rect,
        &SurfaceStyle::new(parchment_page())
            .with_border(1.5, Color::new(0.33, 0.21, 0.10, 0.72))
            .with_inner_border(6.0, 1.0, Color::new(0.48, 0.30, 0.13, 0.20)),
    );
    for i in 0..7 {
        let y = rect.y + 22.0 + i as f32 * (rect.h - 44.0) / 6.0;
        draw_line(
            rect.x + 14.0,
            y,
            rect.right() - 14.0,
            y,
            1.0,
            Color::new(0.38, 0.24, 0.12, 0.08),
        );
    }
}

pub(super) fn draw_corner_marks(rect: Rect, color: Color) {
    let len = 16.0;
    let gap = 5.0;
    draw_line(
        rect.x + gap,
        rect.y + gap,
        rect.x + gap + len,
        rect.y + gap,
        1.0,
        color,
    );
    draw_line(
        rect.x + gap,
        rect.y + gap,
        rect.x + gap,
        rect.y + gap + len,
        1.0,
        color,
    );
    draw_line(
        rect.right() - gap - len,
        rect.y + gap,
        rect.right() - gap,
        rect.y + gap,
        1.0,
        color,
    );
    draw_line(
        rect.right() - gap,
        rect.y + gap,
        rect.right() - gap,
        rect.y + gap + len,
        1.0,
        color,
    );
    draw_line(
        rect.x + gap,
        rect.bottom() - gap,
        rect.x + gap + len,
        rect.bottom() - gap,
        1.0,
        color,
    );
    draw_line(
        rect.x + gap,
        rect.bottom() - gap - len,
        rect.x + gap,
        rect.bottom() - gap,
        1.0,
        color,
    );
    draw_line(
        rect.right() - gap - len,
        rect.bottom() - gap,
        rect.right() - gap,
        rect.bottom() - gap,
        1.0,
        color,
    );
    draw_line(
        rect.right() - gap,
        rect.bottom() - gap - len,
        rect.right() - gap,
        rect.bottom() - gap,
        1.0,
        color,
    );
}

pub(super) fn virtual_button(
    rect: Rect,
    text: &str,
    enabled: bool,
    tone: ButtonTone,
    mouse: Vec2,
) -> bool {
    let hovered = enabled && mouse_over_rect(rect, mouse);
    let pressed = hovered && is_mouse_button_down(MouseButton::Left);
    let activated = hovered && is_mouse_button_released(MouseButton::Left);
    draw_button_plaque(rect, tone, enabled, hovered, pressed, false);

    let palette = button_palette(tone);
    let font_size = (rect.h * 0.42).clamp(10.0, 17.0);
    draw_text_centered_in_box_ex(
        text,
        rect.x + 6.0,
        rect.y + if pressed { 1.0 } else { 0.0 } - 1.0,
        rect.w - 12.0,
        rect.h,
        TextStyle::new(
            font_size,
            if enabled {
                palette.text
            } else {
                Color::new(0.42, 0.40, 0.36, 1.0)
            },
        ),
    );
    activated
}

pub(super) fn draw_button_plaque(
    rect: Rect,
    tone: ButtonTone,
    enabled: bool,
    hovered: bool,
    pressed: bool,
    selected: bool,
) {
    let palette = button_palette(tone);
    let inset = rect.inset(3.0);
    let face = if !enabled {
        palette.disabled
    } else if pressed {
        palette.pressed
    } else if hovered || selected {
        palette.hovered
    } else {
        palette.normal
    };
    let border = if selected {
        brass()
    } else if enabled {
        palette.border
    } else {
        Color::new(0.30, 0.28, 0.24, 0.82)
    };

    draw_rectangle(
        rect.x + 2.0,
        rect.y + 3.0,
        rect.w,
        rect.h,
        Color::new(0.015, 0.012, 0.010, 0.58),
    );
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.040, 0.036, 0.030, 1.0))
            .with_border(1.0, Color::new(0.16, 0.11, 0.055, 1.0))
            .with_inner_border(2.0, 1.0, Color::new(0.70, 0.50, 0.22, 0.26)),
    );
    draw_surface(
        inset,
        &SurfaceStyle::new(face)
            .with_border(if selected { 1.8 } else { 1.0 }, border)
            .with_inner_border(4.0, 1.0, Color::new(0.96, 0.74, 0.34, 0.12)),
    );

    let top = Color::new(
        (face.r + 0.050).min(1.0),
        (face.g + 0.050).min(1.0),
        (face.b + 0.050).min(1.0),
        face.a,
    );
    draw_rectangle(
        inset.x + 2.0,
        inset.y + 2.0,
        inset.w - 4.0,
        inset.h * 0.38,
        top,
    );
    draw_line(
        inset.x + 4.0,
        inset.y + 4.0,
        inset.right() - 4.0,
        inset.y + 4.0,
        1.0,
        Color::new(1.0, 0.82, 0.44, if enabled { 0.28 } else { 0.10 }),
    );
    draw_line(
        inset.x + 4.0,
        inset.bottom() - 4.0,
        inset.right() - 4.0,
        inset.bottom() - 4.0,
        1.0,
        Color::new(0.0, 0.0, 0.0, if enabled { 0.30 } else { 0.16 }),
    );
    draw_small_corner_marks(
        inset,
        Color::new(0.92, 0.66, 0.26, if enabled { 0.62 } else { 0.22 }),
    );
}

fn draw_small_corner_marks(rect: Rect, color: Color) {
    let len = 8.0;
    let gap = 3.0;
    draw_line(
        rect.x + gap,
        rect.y + gap,
        rect.x + gap + len,
        rect.y + gap,
        1.0,
        color,
    );
    draw_line(
        rect.x + gap,
        rect.y + gap,
        rect.x + gap,
        rect.y + gap + len,
        1.0,
        color,
    );
    draw_line(
        rect.right() - gap - len,
        rect.y + gap,
        rect.right() - gap,
        rect.y + gap,
        1.0,
        color,
    );
    draw_line(
        rect.right() - gap,
        rect.y + gap,
        rect.right() - gap,
        rect.y + gap + len,
        1.0,
        color,
    );
    draw_line(
        rect.x + gap,
        rect.bottom() - gap,
        rect.x + gap + len,
        rect.bottom() - gap,
        1.0,
        color,
    );
    draw_line(
        rect.x + gap,
        rect.bottom() - gap - len,
        rect.x + gap,
        rect.bottom() - gap,
        1.0,
        color,
    );
    draw_line(
        rect.right() - gap - len,
        rect.bottom() - gap,
        rect.right() - gap,
        rect.bottom() - gap,
        1.0,
        color,
    );
    draw_line(
        rect.right() - gap,
        rect.bottom() - gap - len,
        rect.right() - gap,
        rect.bottom() - gap,
        1.0,
        color,
    );
}

#[derive(Debug, Clone, Copy)]
struct ButtonPalette {
    normal: Color,
    hovered: Color,
    pressed: Color,
    disabled: Color,
    border: Color,
    text: Color,
}

fn button_palette(tone: ButtonTone) -> ButtonPalette {
    match tone {
        ButtonTone::Primary => ButtonPalette {
            normal: Color::new(0.16, 0.23, 0.34, 1.0),
            hovered: Color::new(0.21, 0.31, 0.46, 1.0),
            pressed: Color::new(0.10, 0.16, 0.26, 1.0),
            disabled: Color::new(0.08, 0.10, 0.13, 1.0),
            border: Color::new(0.58, 0.64, 0.74, 0.90),
            text: parchment(),
        },
        ButtonTone::Secondary => ButtonPalette {
            normal: Color::new(0.16, 0.13, 0.22, 1.0),
            hovered: Color::new(0.22, 0.18, 0.30, 1.0),
            pressed: Color::new(0.11, 0.09, 0.17, 1.0),
            disabled: Color::new(0.09, 0.08, 0.11, 1.0),
            border: Color::new(0.58, 0.50, 0.72, 0.78),
            text: parchment(),
        },
        ButtonTone::Positive => ButtonPalette {
            normal: Color::new(0.12, 0.30, 0.17, 1.0),
            hovered: Color::new(0.18, 0.40, 0.23, 1.0),
            pressed: Color::new(0.08, 0.21, 0.12, 1.0),
            disabled: Color::new(0.08, 0.13, 0.09, 1.0),
            border: Color::new(0.55, 0.72, 0.42, 0.85),
            text: parchment(),
        },
        ButtonTone::Warning => ButtonPalette {
            normal: Color::new(0.38, 0.18, 0.10, 1.0),
            hovered: Color::new(0.50, 0.25, 0.13, 1.0),
            pressed: Color::new(0.26, 0.11, 0.07, 1.0),
            disabled: Color::new(0.16, 0.10, 0.07, 1.0),
            border: Color::new(0.82, 0.55, 0.26, 0.82),
            text: parchment(),
        },
        ButtonTone::Danger => ButtonPalette {
            normal: Color::new(0.34, 0.13, 0.11, 1.0),
            hovered: Color::new(0.48, 0.17, 0.14, 1.0),
            pressed: Color::new(0.22, 0.08, 0.07, 1.0),
            disabled: Color::new(0.14, 0.08, 0.08, 1.0),
            border: Color::new(0.78, 0.42, 0.34, 0.80),
            text: parchment(),
        },
        ButtonTone::Muted => ButtonPalette {
            normal: Color::new(0.095, 0.100, 0.105, 1.0),
            hovered: Color::new(0.14, 0.145, 0.155, 1.0),
            pressed: Color::new(0.060, 0.064, 0.070, 1.0),
            disabled: Color::new(0.060, 0.060, 0.064, 1.0),
            border: Color::new(0.54, 0.48, 0.38, 0.70),
            text: parchment(),
        },
    }
}

pub(super) fn mouse_over_rect(rect: Rect, mouse: Vec2) -> bool {
    rect.contains_point(mouse)
}

pub(super) fn grade_color(grade: EnchantGrade) -> Color {
    match grade {
        EnchantGrade::Brilliant => Color::new(0.30, 0.55, 0.42, 1.0),
        EnchantGrade::Reliable => Color::new(0.18, 0.36, 0.52, 1.0),
        EnchantGrade::Unstable => Color::new(0.66, 0.45, 0.16, 1.0),
        EnchantGrade::Failed => Color::new(0.55, 0.20, 0.18, 1.0),
    }
}

pub(super) fn risk_color(risk: &str) -> Color {
    match risk {
        "Routine" => Color::new(0.18, 0.30, 0.26, 1.0),
        "Low" => Color::new(0.18, 0.34, 0.22, 1.0),
        "Medium" => Color::new(0.38, 0.31, 0.14, 1.0),
        "High" => Color::new(0.52, 0.23, 0.16, 1.0),
        "Suspicious" => Color::new(0.34, 0.21, 0.40, 1.0),
        "Forbidden" | "Legendary" => Color::new(0.50, 0.17, 0.28, 1.0),
        _ => Color::new(0.24, 0.25, 0.28, 1.0),
    }
}

pub(super) fn ink() -> Color {
    Color::new(0.11, 0.065, 0.035, 1.0)
}

pub(super) fn muted_ink() -> Color {
    Color::new(0.25, 0.18, 0.11, 1.0)
}

pub(super) fn brass() -> Color {
    Color::new(0.74, 0.57, 0.30, 1.0)
}

pub(super) fn brass_dim() -> Color {
    Color::new(0.64, 0.48, 0.24, 0.62)
}

pub(super) fn parchment() -> Color {
    Color::new(0.92, 0.86, 0.70, 1.0)
}

pub(super) fn parchment_page() -> Color {
    Color::new(0.77, 0.66, 0.45, 1.0)
}

pub(super) fn parchment_line() -> Color {
    Color::new(0.38, 0.25, 0.12, 0.22)
}

pub(super) fn panel_dark() -> Color {
    Color::new(0.055, 0.058, 0.052, 0.98)
}

pub(super) fn panel_header() -> Color {
    Color::new(0.095, 0.073, 0.048, 1.0)
}

pub(super) fn desk_dark() -> Color {
    Color::new(0.045, 0.035, 0.028, 1.0)
}

fn wood_grain() -> Color {
    Color::new(0.20, 0.12, 0.06, 0.30)
}
