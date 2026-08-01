//! The in-game diagram manual: one page per quest, showing the diagram that
//! fills it drawn on a slate the same way the player's own ink is drawn.
//!
//! Content comes from `crate::manual`, the same builder the generated HTML page
//! reads, so the two can never drift apart.

use super::canvas::draw_strokes;
use super::widgets::{
    brass, muted_ink, parchment, parchment_line, parchment_page, risk_color, virtual_button,
};
use super::{UiAction, UiContext, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use crate::manual::ManualEntry;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

pub(super) fn draw_manual_overlay(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    draw_rectangle(
        0.0,
        0.0,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT,
        Color::new(0.0, 0.0, 0.0, 0.58),
    );
    let rect = Rect::new(146.0, 64.0, 988.0, 566.0);
    super::widgets::draw_panel(rect, "Diagram Manual");

    if virtual_button(
        Rect::new(rect.right() - 94.0, rect.y + 10.0, 72.0, 28.0),
        "Close",
        true,
        ButtonTone::Muted,
        mouse,
    ) {
        actions.push(UiAction::CloseManual);
    }

    let entries = ctx.manual;
    if entries.is_empty() {
        return;
    }
    let page = ctx.manual_page.min(entries.len() - 1);
    let entry = &entries[page];

    let content = Rect::new(rect.x + 22.0, rect.y + 52.0, rect.w - 44.0, rect.h - 74.0);
    // Same proportions as the drawing slate, so the diagram here is the shape
    // the player ends up drawing there. The picture yields height to the reading
    // underneath it rather than the other way round: a rung of nine marks says
    // nine things, and a reading that stops halfway teaches nothing.
    let reading_height = entry.reading.len() as f32 * READING_LINE_HEIGHT;
    let slate_height = (content.h - 34.0 - 8.0 - reading_height).clamp(180.0, 363.0);
    let slate_width = (slate_height * SLATE_ASPECT).min(460.0);
    let slate = Rect::new(
        content.right() - slate_width,
        content.y + 34.0,
        slate_width,
        slate_height,
    );
    let text = Rect::new(
        content.x,
        content.y + 34.0,
        content.w - slate.w - 24.0,
        content.h - 34.0,
    );

    draw_pager(entries, page, content, mouse, actions);
    // The badges sit just above the "Lay Out" button; everything above them has
    // to share what is left.
    draw_entry_text(ctx, entry, text, content.bottom() - 44.0);
    draw_entry_diagram(entry, slate);
    draw_entry_reading(
        entry,
        Rect::new(
            slate.x,
            slate.bottom() + 8.0,
            slate.w,
            content.bottom() - slate.bottom() - 8.0,
        ),
    );

    if virtual_button(
        Rect::new(text.x, content.bottom() - 34.0, 208.0, 30.0),
        "Lay Out On Slate",
        true,
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::LayOutManualDiagram(page));
    }
}

fn draw_pager(
    entries: &[ManualEntry],
    page: usize,
    rect: Rect,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    if virtual_button(
        Rect::new(rect.x, rect.y, 30.0, 26.0),
        "<",
        page > 0,
        ButtonTone::Muted,
        mouse,
    ) {
        actions.push(UiAction::SetManualPage(page.saturating_sub(1)));
    }
    if virtual_button(
        Rect::new(rect.x + 36.0, rect.y, 30.0, 26.0),
        ">",
        page + 1 < entries.len(),
        ButtonTone::Muted,
        mouse,
    ) {
        actions.push(UiAction::SetManualPage(page + 1));
    }
    draw_ui_text_ex(
        &format!("{} / {}", page + 1, entries.len()),
        rect.x + 76.0,
        rect.y + 18.0,
        TextStyle::new(14.0, muted_ink()).params(),
    );
    draw_ui_text_ex(
        entries[page].kind,
        rect.right() - 130.0,
        rect.y + 18.0,
        TextStyle::new(13.0, Color::new(0.62, 0.56, 0.42, 1.0)).params(),
    );
}

/// Height the risk/payout badge strip below the notation rows needs.
const BADGE_STRIP_HEIGHT: f32 = 30.0;
/// One line of the diagram read aloud.
const READING_LINE_HEIGHT: f32 = 15.0;
/// The drawing slate's own width-to-height ratio (`ui::draw_drafting_panel`).
const SLATE_ASPECT: f32 = 520.0 / 410.0;

fn draw_entry_text(ctx: &UiContext<'_>, entry: &ManualEntry, rect: Rect, bottom: f32) {
    draw_ui_text_ex(
        &entry.title(),
        rect.x,
        rect.y + 22.0,
        TextStyle::new(22.0, brass()).params(),
    );
    draw_text_block(
        &entry.customer,
        rect.x,
        rect.y + 32.0,
        rect.w,
        20.0,
        14.0,
        1.0,
        Color::new(0.66, 0.62, 0.52, 1.0),
    );
    draw_text_block(
        &entry.request,
        rect.x,
        rect.y + 54.0,
        rect.w,
        100.0,
        14.0,
        3.0,
        parchment(),
    );

    // A rung can name nine marks where a commission names three, so the rows
    // share whatever height is left once the notes below them are reserved —
    // tightening rather than running off the bottom of the panel.
    let mut row_y = rect.y + 162.0;
    let reserved = BADGE_STRIP_HEIGHT
        + if entry.structure.is_empty() {
            0.0
        } else {
            52.0
        }
        + if entry.unreadable.is_empty() {
            0.0
        } else {
            58.0
        };
    let row_step =
        ((bottom - reserved - row_y) / entry.notation.len().max(1) as f32).clamp(20.0, 28.0);
    for rune in &entry.notation {
        let locked = ctx
            .data
            .rune(&rune.id)
            .is_some_and(|def| !ctx.session.can_use_rune(def));
        draw_surface(
            Rect::new(rect.x, row_y, rect.w, row_step - 5.0),
            &SurfaceStyle::new(if locked {
                Color::new(0.28, 0.12, 0.09, 0.56)
            } else {
                Color::new(0.12, 0.19, 0.14, 0.56)
            })
            .with_border(1.0, parchment_line()),
        );
        let baseline = row_y + row_step * 0.55;
        draw_ui_text_ex(
            rune.label,
            rect.x + 8.0,
            baseline,
            TextStyle::new(12.0, muted_ink()).params(),
        );
        draw_text_right(
            &format!("{}{}", rune.name, if locked { " (locked)" } else { "" }),
            rect.right() - 8.0,
            baseline,
            TextStyle::new(13.0, parchment()),
        );
        row_y += row_step;
    }

    if !entry.structure.is_empty() {
        draw_text_block(
            &format!("Also wants: {}.", entry.structure.join(", ")),
            rect.x,
            row_y + 6.0,
            rect.w,
            46.0,
            13.0,
            2.0,
            Color::new(0.72, 0.66, 0.50, 1.0),
        );
        row_y += 52.0;
    }
    if !entry.unreadable.is_empty() {
        draw_text_block(
            &format!(
                "The workshop cannot yet read {} inside a diagram, however carefully drawn.",
                entry.unreadable.join(", ")
            ),
            rect.x,
            row_y + 6.0,
            rect.w,
            52.0,
            13.0,
            3.0,
            Color::new(0.86, 0.56, 0.44, 1.0),
        );
        row_y += 58.0;
    }

    // Ladder rungs pay nothing — they are drills, so the payout badges would
    // read as a reward of zero rather than as no reward at all.
    draw_badge(
        Rect::new(rect.x, row_y + 8.0, 84.0, 22.0),
        &entry.risk,
        risk_color(&entry.risk),
        parchment(),
    );
    if entry.reward > 0 {
        draw_badge(
            Rect::new(rect.x + 94.0, row_y + 8.0, 78.0, 22.0),
            &format!("{}c", entry.reward),
            Color::new(0.16, 0.28, 0.19, 1.0),
            parchment(),
        );
    }
    if entry.insight > 0 {
        draw_badge(
            Rect::new(rect.x + 182.0, row_y + 8.0, 92.0, 22.0),
            &format!("+{} insight", entry.insight),
            Color::new(0.19, 0.17, 0.30, 1.0),
            parchment(),
        );
    }
}

/// What the diagram says, in the same words the slate uses after Interpret.
fn draw_entry_reading(entry: &ManualEntry, rect: Rect) {
    if entry.reading.is_empty() || rect.h < 20.0 {
        return;
    }
    for (index, line) in entry.reading.iter().enumerate() {
        draw_text_block(
            line,
            rect.x,
            rect.y + index as f32 * READING_LINE_HEIGHT,
            rect.w,
            READING_LINE_HEIGHT,
            13.0,
            1.0,
            Color::new(0.78, 0.74, 0.60, 1.0),
        );
    }
}

fn draw_entry_diagram(entry: &ManualEntry, rect: Rect) {
    draw_surface(
        rect,
        &SurfaceStyle::new(parchment_page())
            .with_border(1.5, Color::new(0.42, 0.27, 0.12, 0.84))
            .with_inner_border(5.0, 1.0, Color::new(0.42, 0.27, 0.12, 0.14)),
    );
    // Same normalized-to-slate mapping the drawing slate uses, so the manual's
    // picture is the shape the player is meant to end up with, ellipse and all.
    draw_strokes(
        &entry.diagram.strokes(),
        rect,
        Color::new(0.045, 0.028, 0.014, 0.96),
        3.0,
    );
}
