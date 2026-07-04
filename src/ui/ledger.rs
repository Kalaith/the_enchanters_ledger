use super::widgets::{
    brass_dim, grade_color, ink, muted_ink, panel_dark, parchment, parchment_page, virtual_button,
};
use super::{UiAction, UiContext, LOGICAL_WIDTH};
use crate::state::GameSession;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

pub(super) fn draw_ledger_panel(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let rect = Rect::new(10.0, 612.0, LOGICAL_WIDTH - 20.0, 104.0);
    draw_surface(
        rect,
        &SurfaceStyle::new(panel_dark()).with_border(1.0, brass_dim()),
    );

    let pad = 18.0;
    let gap = 18.0;
    let left = Rect::new(rect.x + pad, rect.y + 12.0, 460.0, 84.0);
    draw_diagram_status(ctx, left);

    let right_w = 294.0;
    let right = Rect::new(rect.right() - pad - right_w, rect.y + 16.0, right_w, 72.0);
    let middle = Rect::new(
        left.right() + gap,
        rect.y + 16.0,
        right.x - left.right() - gap * 2.0,
        72.0,
    );
    draw_discoveries(ctx.session, middle);

    let button_gap = 10.0;
    let bw = (right.w - button_gap * 2.0) / 3.0;
    if virtual_button(
        Rect::new(right.x, right.y, bw, 28.0),
        "Save",
        true,
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::Save);
    }
    if virtual_button(
        Rect::new(right.x + bw + button_gap, right.y, bw, 28.0),
        "Load",
        ctx.save_exists,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::Load);
    }
    if virtual_button(
        Rect::new(right.x + (bw + button_gap) * 2.0, right.y, bw, 28.0),
        "Delete",
        ctx.save_exists,
        ButtonTone::Danger,
        mouse,
    ) {
        actions.push(UiAction::DeleteSave);
    }
    draw_text_block(
        &format!(
            "Assets: {} | Save slots: {}",
            ctx.loaded_assets,
            if ctx.save_slots.is_empty() {
                "none".to_owned()
            } else {
                ctx.save_slots.join(", ")
            }
        ),
        right.x,
        right.y + 42.0,
        right.w,
        22.0,
        13.0,
        2.0,
        Color::new(0.62, 0.66, 0.64, 1.0),
    );
}

/// Single home for "how good is my current diagram": the Interpret read (top row) and the last
/// Test result (bottom rows). Previously the Interpret read lived inline in the drafting panel,
/// where it overlapped the button strip; consolidating both here removes that duplication.
fn draw_diagram_status(ctx: &UiContext<'_>, rect: Rect) {
    let board = &ctx.session.board;

    // Row A — Interpretation (result of the "Interpret" button).
    draw_ui_text_ex(
        "Interpretation",
        rect.x,
        rect.y + 14.0,
        TextStyle::new(14.0, parchment()).params(),
    );
    if let Some(note) = &board.last_interpretation_note {
        draw_text_block(
            note,
            rect.x + 120.0,
            rect.y + 3.0,
            rect.w - 120.0,
            20.0,
            13.0,
            2.0,
            Color::new(0.78, 0.83, 0.80, 1.0),
        );
    } else if let Some(diagram) = &board.last_diagram {
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
                "{} | circle {}% | potency {}%",
                names,
                (diagram.circle_quality * 100.0).round() as i32,
                (diagram.average_rune_potency() * 100.0).round() as i32
            ),
            rect.x + 120.0,
            rect.y + 3.0,
            rect.w - 120.0,
            20.0,
            13.0,
            2.0,
            if diagram.accepted() {
                Color::new(0.42, 0.72, 0.50, 1.0)
            } else {
                Color::new(0.80, 0.52, 0.44, 1.0)
            },
        );
    } else {
        draw_text_block(
            "Awaiting interpretation.",
            rect.x + 120.0,
            rect.y + 3.0,
            rect.w - 120.0,
            20.0,
            13.0,
            2.0,
            Color::new(0.62, 0.66, 0.64, 1.0),
        );
    }

    // Row B — Last Test (result of the "Test Diagram" button), with side-effect flavor wrapped
    // onto its own line so it no longer truncates mid-sentence.
    draw_ui_text_ex(
        "Last Test",
        rect.x,
        rect.y + 40.0,
        TextStyle::new(14.0, parchment()).params(),
    );
    if let Some(result) = &board.last_evaluation {
        draw_badge(
            Rect::new(rect.x + 82.0, rect.y + 27.0, 84.0, 22.0),
            result.grade.label(),
            grade_color(result.grade),
            parchment(),
        );
        draw_text_block(
            &format!(
                "Power {} · Stability {} · Mana {}",
                result.power, result.stability, result.mana_cost
            ),
            rect.x + 176.0,
            rect.y + 29.0,
            rect.w - 176.0,
            20.0,
            13.0,
            2.0,
            Color::new(0.78, 0.83, 0.80, 1.0),
        );
        draw_text_block(
            &result.side_effect,
            rect.x,
            rect.y + 52.0,
            rect.w,
            30.0,
            12.0,
            2.0,
            muted_ink(),
        );
    } else {
        draw_text_block(
            "No diagram has been tested today.",
            rect.x + 82.0,
            rect.y + 31.0,
            rect.w - 82.0,
            24.0,
            13.0,
            2.0,
            Color::new(0.62, 0.66, 0.64, 1.0),
        );
    }
}

fn draw_discoveries(session: &GameSession, rect: Rect) {
    draw_ui_text_ex(
        &format!("Discoveries ({})", session.discoveries.len()),
        rect.x,
        rect.y + 15.0,
        TextStyle::new(15.0, parchment()).params(),
    );
    let card_w = 92.0;
    let card_y = rect.y + 24.0;
    for (index, recipe) in session.discoveries.iter().rev().take(4).enumerate() {
        let card = Rect::new(
            rect.x + index as f32 * (card_w + 10.0),
            card_y,
            card_w,
            48.0,
        );
        draw_surface(
            card,
            &SurfaceStyle::new(parchment_page())
                .with_border(1.0, Color::new(0.37, 0.24, 0.12, 0.70)),
        );
        draw_text_block(
            &recipe.name,
            card.x + 7.0,
            card.y + 20.0,
            card.w - 14.0,
            22.0,
            12.0,
            2.0,
            ink(),
        );
        draw_text_right(
            &format!("{}", recipe.best_score),
            card.right() - 6.0,
            card.bottom() - 5.0,
            TextStyle::new(10.0, muted_ink()),
        );
    }
    if session.discoveries.is_empty() {
        draw_text_block(
            "Experiment with complete diagrams to record recipes.",
            rect.x,
            rect.y + 34.0,
            rect.w,
            28.0,
            14.0,
            2.0,
            Color::new(0.78, 0.83, 0.80, 1.0),
        );
    }
}
