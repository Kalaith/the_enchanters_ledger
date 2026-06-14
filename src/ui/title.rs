use super::widgets::{brass, brass_dim, draw_corner_marks, panel_dark, parchment, virtual_button};
use super::{UiAction, UiContext, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

pub(super) fn draw_title_screen(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    draw_title_art(ctx.title_texture);
    draw_title_scrim();

    let button_w = 158.0;
    let button_h = 38.0;
    let button_gap = 16.0;
    let total_w = button_w * 4.0 + button_gap * 3.0;
    let x = (LOGICAL_WIDTH - total_w) * 0.5;
    let y = 500.0;
    if virtual_button(
        Rect::new(x, y, button_w, button_h),
        "Continue",
        ctx.save_exists,
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::Load);
    }
    if virtual_button(
        Rect::new(x + (button_w + button_gap), y, button_w, button_h),
        "New Game",
        true,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::NewGame);
    }
    if virtual_button(
        Rect::new(x + (button_w + button_gap) * 2.0, y, button_w, button_h),
        "Settings",
        true,
        ButtonTone::Muted,
        mouse,
    ) {
        actions.push(UiAction::OpenSettings);
    }
    if virtual_button(
        Rect::new(x + (button_w + button_gap) * 3.0, y, button_w, button_h),
        "Exit",
        true,
        ButtonTone::Danger,
        mouse,
    ) {
        actions.push(UiAction::ExitGame);
    }
}

pub(super) fn draw_settings_window(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    draw_rectangle(
        0.0,
        0.0,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT,
        Color::new(0.0, 0.0, 0.0, 0.62),
    );
    let rect = Rect::new(18.0, 18.0, LOGICAL_WIDTH - 36.0, LOGICAL_HEIGHT - 36.0);
    draw_surface(
        rect,
        &SurfaceStyle::new(panel_dark())
            .with_border(1.0, brass_dim())
            .with_inner_border(6.0, 1.0, Color::new(0.95, 0.72, 0.32, 0.14))
            .with_header(62.0, Color::new(0.088, 0.064, 0.042, 1.0))
            .with_header_divider(1.0, Color::new(0.64, 0.50, 0.30, 0.36)),
    );
    draw_corner_marks(rect, Color::new(0.84, 0.62, 0.28, 0.52));
    draw_text_centered(
        "Settings",
        rect.center().x,
        rect.y + 40.0,
        TextStyle::new(26.0, parchment()),
    );

    if virtual_button(
        Rect::new(rect.right() - 146.0, rect.y + 17.0, 112.0, 34.0),
        "Close",
        true,
        ButtonTone::Muted,
        mouse,
    ) {
        actions.push(UiAction::CloseSettings);
    }

    let display = Rect::new(rect.x + 66.0, rect.y + 112.0, 420.0, 254.0);
    draw_settings_section(display, "Display");
    draw_ui_text_ex(
        "Fullscreen",
        display.x + 28.0,
        display.y + 72.0,
        TextStyle::new(20.0, parchment()).params(),
    );
    let toggle_text = if ctx.fullscreen {
        "Fullscreen: On"
    } else {
        "Fullscreen: Off"
    };
    if virtual_button(
        Rect::new(display.x + 28.0, display.y + 96.0, 224.0, 40.0),
        toggle_text,
        true,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::ToggleFullscreen);
    }
}

fn draw_title_art(texture: &Texture2D) {
    let scale = (LOGICAL_WIDTH / texture.width()).max(LOGICAL_HEIGHT / texture.height());
    let width = texture.width() * scale;
    let height = texture.height() * scale;
    draw_texture_ex(
        texture,
        (LOGICAL_WIDTH - width) * 0.5,
        (LOGICAL_HEIGHT - height) * 0.5,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(width, height)),
            ..Default::default()
        },
    );
}

fn draw_title_scrim() {
    draw_rectangle(
        0.0,
        0.0,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT,
        Color::new(0.02, 0.014, 0.008, 0.16),
    );
}

fn draw_settings_section(rect: Rect, title: &str) {
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.030, 0.034, 0.032, 0.96))
            .with_border(1.0, brass_dim())
            .with_inner_border(6.0, 1.0, Color::new(0.95, 0.72, 0.32, 0.10)),
    );
    draw_corner_marks(rect, Color::new(0.84, 0.62, 0.28, 0.38));
    draw_ui_text_ex(
        title,
        rect.x + 28.0,
        rect.y + 36.0,
        TextStyle::new(22.0, brass()).params(),
    );
}
