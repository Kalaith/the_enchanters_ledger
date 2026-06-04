//! Immediate-mode UI for the workshop, rune palette, and ledger.

use crate::data::GameData;
use crate::rune_drawing::{DrawnStroke, StrokePoint};
use crate::rune_quality::RunePracticeReport;
use crate::state::{GamePhase, GameSession};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;

mod canvas;
mod commission;
mod drawing;
mod journal;
mod ledger;
mod practice;
mod rune_guide;
mod title;
mod widgets;

use commission::draw_commission_panel;
use drawing::draw_drawing_slate;
use journal::draw_journal_overlay;
use ledger::draw_ledger_panel;
use practice::draw_practice_overlay;
use rune_guide::draw_rune_palette;
use title::{draw_settings_window, draw_title_screen};
use widgets::*;

pub const LOGICAL_WIDTH: f32 = 1280.0;
pub const LOGICAL_HEIGHT: f32 = 720.0;

#[derive(Debug, Clone, Copy)]
pub struct UiSpace {
    logical_width: f32,
    logical_height: f32,
}

impl UiSpace {
    pub fn new(logical_width: f32, logical_height: f32) -> Self {
        Self {
            logical_width,
            logical_height,
        }
    }

    pub fn mouse_position(&self) -> Vec2 {
        let (screen_x, screen_y) = mouse_position();
        vec2(
            screen_x * self.logical_width / screen_width().max(1.0),
            screen_y * self.logical_height / screen_height().max(1.0),
        )
    }

    pub fn is_mouse_inside(&self) -> bool {
        let mouse = self.mouse_position();
        mouse.x >= 0.0
            && mouse.x <= self.logical_width
            && mouse.y >= 0.0
            && mouse.y <= self.logical_height
    }
}

pub fn begin_ui_frame() -> UiSpace {
    macroquad_toolkit::ui::set_ui_text_scale_for_screen(LOGICAL_WIDTH, LOGICAL_HEIGHT, 1.45);
    macroquad::camera::set_camera(&macroquad::camera::Camera2D {
        target: vec2(LOGICAL_WIDTH * 0.5, LOGICAL_HEIGHT * 0.5),
        zoom: vec2(2.0 / LOGICAL_WIDTH, 2.0 / LOGICAL_HEIGHT),
        ..Default::default()
    });
    UiSpace::new(LOGICAL_WIDTH, LOGICAL_HEIGHT)
}

pub fn end_ui_frame() {
    macroquad::camera::set_default_camera();
}

#[derive(Debug, Clone)]
pub enum UiAction {
    StartGame,
    NewGame,
    OpenSettings,
    CloseSettings,
    ToggleFullscreen,
    ExitGame,
    Save,
    Load,
    DeleteSave,
    ToggleJournal,
    CloseJournal,
    OpenPractice,
    ClosePractice,
    ClearPractice,
    ScorePractice,
    StartPracticeStroke(StrokePoint),
    ExtendPracticeStroke(StrokePoint),
    FinishPracticeStroke,
    ErasePracticeInk(StrokePoint, f32),
    SelectStoryWork,
    SelectTalismanWork(usize),
    SelectRune(String),
    SetRuneGuidePage(usize, usize),
    DeselectRune,
    PlaceRuneTemplate(StrokePoint),
    RemoveRuneTemplate(usize),
    MoveRuneTemplate(usize, StrokePoint),
    ToggleGuideEditMode,
    StartRuneStroke(StrokePoint),
    ExtendRuneStroke(StrokePoint),
    FinishRuneStroke,
    EraseRuneInk(StrokePoint, f32),
    ClearRuneDrawing,
    InterpretDiagram,
    CopyDiagnostics,
    ClearBoard,
    TestDesign,
    DeliverDesign,
    Research,
    SkipCommission,
}

pub struct UiContext<'a> {
    pub data: &'a GameData,
    pub session: &'a GameSession,
    pub save_exists: bool,
    pub save_slots: &'a [String],
    pub loaded_assets: usize,
    pub rune_guide_pages: &'a [usize; 4],
    pub journal_open: bool,
    pub settings_open: bool,
    pub fullscreen: bool,
    pub suppress_rune_erase: bool,
    pub guide_edit_mode: bool,
    pub interpretation_feedback_active: bool,
    pub title_texture: &'a Texture2D,
    pub practice: PracticeUi<'a>,
    pub ui: &'a UiSpace,
}

#[derive(Debug, Clone, Copy)]
pub struct PracticeUi<'a> {
    pub open: bool,
    pub strokes: &'a [DrawnStroke],
    pub active_stroke: Option<&'a DrawnStroke>,
    pub report: Option<&'a RunePracticeReport>,
}

pub fn draw_game_ui(ctx: UiContext<'_>) -> Vec<UiAction> {
    let mut actions = Vec::new();
    let mouse = ctx.ui.mouse_position();
    draw_workshop_backdrop();

    if ctx.session.phase == GamePhase::Title {
        let mut blocked_actions = Vec::new();
        let title_actions = if ctx.settings_open {
            &mut blocked_actions
        } else {
            &mut actions
        };
        draw_title_screen(&ctx, mouse, title_actions);
        if ctx.settings_open {
            draw_settings_window(&ctx, mouse, &mut actions);
        }
        return actions;
    }

    if ctx.session.phase == GamePhase::Naming {
        draw_name_screen(&ctx, mouse, &mut actions);
        return actions;
    }

    let modal_open = ctx.journal_open || ctx.practice.open;
    let mut blocked_actions = Vec::new();
    let background_actions = if modal_open {
        &mut blocked_actions
    } else {
        &mut actions
    };
    draw_header(&ctx, mouse, background_actions);
    draw_commission_panel(&ctx, mouse, background_actions);
    draw_drafting_panel(&ctx, mouse, background_actions);
    draw_rune_palette(&ctx, mouse, background_actions);
    draw_ledger_panel(&ctx, mouse, background_actions);
    if ctx.journal_open {
        draw_journal_overlay(&ctx, mouse, &mut actions);
    }
    if ctx.practice.open {
        draw_practice_overlay(&ctx, mouse, &mut actions);
    }
    actions
}

fn draw_workshop_backdrop() {
    draw_desk_backdrop(LOGICAL_WIDTH, LOGICAL_HEIGHT);
}

fn draw_name_screen(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let panel = Rect::new(322.0, 126.0, 636.0, 454.0);
    draw_surface(
        panel,
        &SurfaceStyle::new(panel_dark())
            .with_border(1.5, brass())
            .with_inner_border(7.0, 1.0, Color::new(0.95, 0.82, 0.52, 0.14)),
    );
    draw_corner_marks(panel, Color::new(0.82, 0.62, 0.30, 0.46));

    let sheet = Rect::new(panel.x + 56.0, panel.y + 94.0, panel.w - 112.0, 270.0);
    draw_parchment_sheet(sheet);

    draw_text_centered(
        "RUNEWRIGHT",
        panel.center().x,
        panel.y + 56.0,
        TextStyle::new(38.0, brass()),
    );
    draw_text_centered(
        "The Enchanter's Ledger",
        panel.center().x,
        panel.y + 82.0,
        TextStyle::new(20.0, parchment()),
    );
    draw_text_centered(
        "Name the ledger keeper",
        sheet.center().x,
        sheet.y + 54.0,
        TextStyle::new(18.0, ink()),
    );

    let name_rect = Rect::new(sheet.x + 64.0, sheet.y + 84.0, sheet.w - 128.0, 62.0);
    draw_surface(
        name_rect,
        &SurfaceStyle::new(Color::new(0.90, 0.82, 0.62, 1.0))
            .with_border(1.0, Color::new(0.28, 0.19, 0.12, 1.0)),
    );
    draw_text_centered_in_box_ex(
        &ctx.session.player.name,
        name_rect.x + 16.0,
        name_rect.y,
        name_rect.w - 32.0,
        name_rect.h,
        TextStyle::new(32.0, ink()),
    );

    let open_button = Rect::new(sheet.x + 154.0, sheet.y + 178.0, 216.0, 44.0);
    let mut start_requested = virtual_button(
        open_button,
        "Open Workshop",
        true,
        ButtonTone::Positive,
        mouse,
    );
    if !start_requested && is_mouse_button_released(MouseButton::Left) && ctx.ui.is_mouse_inside() {
        start_requested = true;
    }
    if start_requested {
        actions.push(UiAction::StartGame);
    }
    draw_text_centered(
        "Steve is written in the margin until you replace it.",
        sheet.center().x,
        sheet.y + 244.0,
        TextStyle::new(14.0, muted_ink()),
    );
}

fn draw_header(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let rect = Rect::new(6.0, 6.0, LOGICAL_WIDTH - 12.0, 82.0);
    draw_surface(
        rect,
        &SurfaceStyle::new(panel_dark())
            .with_border(1.0, brass_dim())
            .with_inner_border(5.0, 1.0, Color::new(0.95, 0.70, 0.28, 0.12))
            .with_top_highlight(2.0, Color::new(0.95, 0.76, 0.34, 0.42)),
    );
    draw_corner_marks(rect, Color::new(0.82, 0.62, 0.30, 0.46));

    draw_text_ex(
        "RUNEWRIGHT",
        rect.x + 24.0,
        rect.y + 34.0,
        TextStyle::new(34.0, brass()).params(),
    );
    draw_text_ex(
        &ctx.data.config.display_name,
        rect.x + 28.0,
        rect.y + 60.0,
        TextStyle::new(20.0, parchment()).params(),
    );

    let stats_y = rect.y + 20.0;
    draw_header_card(
        Rect::new(322.0, stats_y, 102.0, 42.0),
        "Day",
        &ctx.session.player.day.to_string(),
    );
    draw_header_card(
        Rect::new(436.0, stats_y, 122.0, 42.0),
        "Coins",
        &format_money(ctx.session.player.coins),
    );
    draw_header_card(
        Rect::new(570.0, stats_y, 116.0, 42.0),
        "Reputation",
        &format!("{:+}", ctx.session.player.reputation),
    );
    draw_header_card(
        Rect::new(698.0, stats_y, 116.0, 42.0),
        "Insight",
        &ctx.session.player.insight.to_string(),
    );

    meter(
        Rect::new(832.0, stats_y + 12.0, 184.0, 18.0),
        ctx.session.player.focus,
        ctx.data.config.max_focus,
        Color::new(0.36, 0.62, 0.82, 1.0),
        Some(&format!(
            "Focus {:.0}/{:.0}",
            ctx.session.player.focus, ctx.data.config.max_focus
        )),
    );
    draw_header_card(
        Rect::new(1032.0, stats_y, 92.0, 42.0),
        "Rank",
        &format!("{}", ctx.session.player.workshop_rank),
    );
    if virtual_button(
        Rect::new(rect.right() - 112.0, rect.y + 22.0, 86.0, 36.0),
        "Journal",
        true,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::ToggleJournal);
    }
}

fn draw_header_card(rect: Rect, label: &str, value: &str) {
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.078, 0.062, 0.043, 0.94))
            .with_border(1.0, Color::new(0.66, 0.48, 0.22, 0.48)),
    );
    draw_text_ex(
        label,
        rect.x + 10.0,
        rect.y + 16.0,
        TextStyle::new(12.0, Color::new(0.68, 0.62, 0.50, 1.0)).params(),
    );
    draw_text_right(
        value,
        rect.right() - 10.0,
        rect.y + 31.0,
        TextStyle::new(18.0, parchment()),
    );
}

fn draw_drafting_panel(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let rect = Rect::new(356.0, 104.0, 592.0, 498.0);
    draw_panel(rect, "Drawn Diagram");
    let page = Rect::new(rect.x + 18.0, rect.y + 48.0, rect.w - 36.0, rect.h - 64.0);
    draw_parchment_sheet(page);
    let mut slate_y = page.y + 12.0;
    if let Some(prompt) = ctx.session.tutorial_prompt() {
        let tutorial = Rect::new(page.x + 18.0, slate_y, page.w - 36.0, 64.0);
        draw_tutorial_banner(tutorial, prompt);
        slate_y = tutorial.bottom() + 12.0;
    }
    let slate = Rect::new(
        page.x + 18.0,
        slate_y,
        page.w - 36.0,
        page.bottom() - slate_y - 12.0,
    );
    draw_drawing_slate(ctx, slate, mouse, actions);
}

fn draw_tutorial_banner(rect: Rect, prompt: &str) {
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.93, 0.82, 0.55, 1.0))
            .with_border(1.5, Color::new(0.43, 0.24, 0.08, 0.92))
            .with_inner_border(5.0, 1.0, Color::new(0.20, 0.11, 0.04, 0.15)),
    );
    draw_text_ex(
        "Tutorial",
        rect.x + 14.0,
        rect.y + 21.0,
        TextStyle::new(16.0, ink()).params(),
    );
    draw_text_block(
        prompt,
        rect.x + 112.0,
        rect.y + 14.0,
        rect.w - 126.0,
        rect.h - 18.0,
        15.0,
        2.0,
        muted_ink(),
    );
}
