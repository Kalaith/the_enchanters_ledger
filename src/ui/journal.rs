use super::widgets::{
    brass, brass_dim, ink, muted_ink, panel_dark, parchment, parchment_line, risk_color,
    virtual_button,
};
use super::{UiAction, UiContext, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use crate::data::CommissionDef;
use crate::state::WorkOrderKind;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;
use macroquad_toolkit::ui::RectExt;

pub(super) fn draw_journal_overlay(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    draw_rectangle(
        0.0,
        0.0,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT,
        Color::new(0.0, 0.0, 0.0, 0.58),
    );
    let rect = Rect::new(146.0, 64.0, 988.0, 566.0);
    super::widgets::draw_panel(rect, "Journal");

    if virtual_button(
        Rect::new(rect.right() - 94.0, rect.y + 10.0, 72.0, 28.0),
        "Close",
        true,
        ButtonTone::Muted,
        mouse,
    ) {
        actions.push(UiAction::CloseJournal);
    }

    let content = rect.inset(22.0);
    let story = Rect::new(content.x, content.y + 28.0, 302.0, 410.0);
    let jobs = Rect::new(story.right() + 18.0, story.y, 388.0, 410.0);
    let notes = Rect::new(jobs.right() + 18.0, story.y, 300.0, 410.0);
    draw_story_section(ctx, story, mouse, actions);
    draw_talisman_section(ctx, jobs, mouse, actions);
    draw_notes_section(ctx, notes);
}

fn draw_story_section(ctx: &UiContext<'_>, rect: Rect, mouse: Vec2, actions: &mut Vec<UiAction>) {
    draw_section(rect, "Story Quest");
    let commission = ctx.session.story_commission(ctx.data);
    draw_order_summary(ctx, commission, rect.x + 14.0, rect.y + 48.0, rect.w - 28.0);
    let ready = required_unlocked(ctx, commission);
    let status = if ready {
        "Ready for the workbench."
    } else {
        "Filed until research unlocks the missing runes."
    };
    draw_text_block(
        status,
        rect.x + 14.0,
        rect.bottom() - 92.0,
        rect.w - 28.0,
        28.0,
        14.0,
        2.0,
        if ready { ink() } else { muted_ink() },
    );
    let active = ctx.session.active_work_kind() == WorkOrderKind::Story;
    if virtual_button(
        Rect::new(rect.x + 14.0, rect.bottom() - 48.0, rect.w - 28.0, 32.0),
        if active { "Pinned" } else { "Pin Story" },
        ready && !active,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::SelectStoryWork);
    }
}

fn draw_talisman_section(
    ctx: &UiContext<'_>,
    rect: Rect,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    draw_section(rect, "Day Talismans");
    let jobs = ctx.session.available_talisman_jobs(ctx.data);
    if jobs.is_empty() {
        draw_text_block(
            "No routine talisman packets are readable at this rank.",
            rect.x + 16.0,
            rect.y + 58.0,
            rect.w - 32.0,
            48.0,
            14.0,
            2.0,
            parchment(),
        );
        return;
    }

    for (slot, (index, job)) in jobs.into_iter().take(4).enumerate() {
        let card = Rect::new(
            rect.x + 14.0,
            rect.y + 46.0 + slot as f32 * 88.0,
            rect.w - 28.0,
            76.0,
        );
        draw_surface(
            card,
            &SurfaceStyle::new(Color::new(0.078, 0.062, 0.043, 0.96))
                .with_border(1.0, Color::new(0.58, 0.42, 0.20, 0.58)),
        );
        draw_ui_text_ex(
            &job.item,
            card.x + 12.0,
            card.y + 20.0,
            TextStyle::new(16.0, parchment()).params(),
        );
        draw_text_block(
            &job.request,
            card.x + 12.0,
            card.y + 30.0,
            card.w - 108.0,
            34.0,
            12.0,
            2.0,
            Color::new(0.74, 0.77, 0.70, 1.0),
        );
        draw_badge(
            Rect::new(card.right() - 92.0, card.y + 10.0, 76.0, 20.0),
            &format!("{}c", job.reward),
            risk_color(&job.risk),
            parchment(),
        );
        draw_badge(
            Rect::new(card.right() - 92.0, card.y + 36.0, 76.0, 20.0),
            &format!("+{} insight", job.insight),
            Color::new(0.19, 0.17, 0.30, 1.0),
            parchment(),
        );
        let active = ctx.session.active_work_kind() == WorkOrderKind::Talisman
            && ctx.session.player.current_talisman == index;
        if virtual_button(
            Rect::new(card.right() - 92.0, card.bottom() - 28.0, 76.0, 24.0),
            if active { "Pinned" } else { "Pin" },
            !active,
            ButtonTone::Positive,
            mouse,
        ) {
            actions.push(UiAction::SelectTalismanWork(index));
        }
    }
}

fn draw_notes_section(ctx: &UiContext<'_>, rect: Rect) {
    draw_section(rect, "Ledger Notes");
    if ctx.session.journal.is_empty() {
        draw_text_block(
            "Deliveries, research, and new recipe insights will be recorded here.",
            rect.x + 16.0,
            rect.y + 58.0,
            rect.w - 32.0,
            72.0,
            14.0,
            3.0,
            parchment(),
        );
        return;
    }

    for (index, entry) in ctx.session.journal.iter().rev().take(5).enumerate() {
        let y = rect.y + 50.0 + index as f32 * 70.0;
        draw_ui_text_ex(
            &format!("Day {} - {}", entry.day, entry.title),
            rect.x + 16.0,
            y,
            TextStyle::new(14.0, parchment()).params(),
        );
        draw_text_block(
            &entry.body,
            rect.x + 16.0,
            y + 10.0,
            rect.w - 32.0,
            44.0,
            12.0,
            2.0,
            Color::new(0.66, 0.69, 0.63, 1.0),
        );
    }
}

fn draw_order_summary(ctx: &UiContext<'_>, commission: &CommissionDef, x: f32, y: f32, width: f32) {
    draw_text_block(
        &commission.customer,
        x,
        y,
        width,
        32.0,
        17.0,
        2.0,
        parchment(),
    );
    draw_text_block(
        &commission.request,
        x,
        y + 42.0,
        width,
        64.0,
        13.0,
        3.0,
        Color::new(0.70, 0.73, 0.68, 1.0),
    );
    let mut row_y = y + 128.0;
    for (label, id) in [
        ("Effect", commission.required_effect.as_str()),
        ("Shape", commission.required_shape.as_str()),
        ("Trigger", commission.required_trigger.as_str()),
    ] {
        draw_requirement(ctx, x, row_y, width, label, id);
        row_y += 28.0;
    }
    if let Some(modifier) = &commission.optional_modifier {
        draw_requirement(ctx, x, row_y, width, "Bonus", modifier);
    }
    draw_badge(
        Rect::new(x, y + 260.0, 84.0, 22.0),
        &commission.risk,
        risk_color(&commission.risk),
        parchment(),
    );
    draw_badge(
        Rect::new(x + 94.0, y + 260.0, 78.0, 22.0),
        &format!("{}c", commission.reward),
        Color::new(0.16, 0.28, 0.19, 1.0),
        parchment(),
    );
    draw_badge(
        Rect::new(x + 182.0, y + 260.0, 92.0, 22.0),
        &format!("+{} insight", commission.insight),
        Color::new(0.19, 0.17, 0.30, 1.0),
        parchment(),
    );
}

fn draw_requirement(ctx: &UiContext<'_>, x: f32, y: f32, w: f32, label: &str, id: &str) {
    let locked = ctx
        .data
        .rune(id)
        .is_some_and(|rune| !ctx.session.can_use_rune(rune));
    draw_surface(
        Rect::new(x, y, w, 23.0),
        &SurfaceStyle::new(if locked {
            Color::new(0.28, 0.12, 0.09, 0.56)
        } else {
            Color::new(0.12, 0.19, 0.14, 0.56)
        })
        .with_border(1.0, parchment_line()),
    );
    draw_ui_text_ex(
        label,
        x + 8.0,
        y + 16.0,
        TextStyle::new(12.0, muted_ink()).params(),
    );
    let suffix = if locked { " (locked)" } else { "" };
    draw_text_right(
        &format!("{}{}", ctx.data.rune_name(id), suffix),
        x + w - 8.0,
        y + 16.0,
        TextStyle::new(13.0, parchment()),
    );
}

fn required_unlocked(ctx: &UiContext<'_>, commission: &CommissionDef) -> bool {
    [
        commission.required_effect.as_str(),
        commission.required_shape.as_str(),
        commission.required_trigger.as_str(),
    ]
    .into_iter()
    .all(|id| {
        ctx.data
            .rune(id)
            .is_some_and(|rune| ctx.session.can_use_rune(rune))
    })
}

fn draw_section(rect: Rect, title: &str) {
    draw_surface(
        rect,
        &SurfaceStyle::new(panel_dark())
            .with_border(1.0, brass_dim())
            .with_inner_border(5.0, 1.0, Color::new(0.95, 0.72, 0.32, 0.08)),
    );
    draw_line(
        rect.x + 10.0,
        rect.y + 36.0,
        rect.right() - 10.0,
        rect.y + 36.0,
        1.0,
        Color::new(0.64, 0.50, 0.30, 0.30),
    );
    draw_text_centered(
        title,
        rect.center().x,
        rect.y + 24.0,
        TextStyle::new(17.0, brass()),
    );
}
