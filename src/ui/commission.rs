use super::widgets::{ink, muted_ink, parchment, parchment_line, risk_color, virtual_button};
use super::{UiAction, UiContext};
use crate::state::WorkOrderKind;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;
use macroquad_toolkit::ui::RectExt;

pub(super) fn draw_commission_panel(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let rect = Rect::new(10.0, 104.0, 338.0, 498.0);
    super::widgets::draw_panel(rect, ctx.session.active_work_kind().panel_title());
    let content = rect.inset(16.0);
    let sheet = Rect::new(content.x, rect.y + 52.0, content.w, 300.0);
    let commission = ctx.session.current_commission(ctx.data);
    super::widgets::draw_parchment_sheet(sheet);

    draw_text_block(
        &commission.customer,
        sheet.x + 16.0,
        sheet.y + 34.0,
        sheet.w - 32.0,
        32.0,
        20.0,
        2.0,
        ink(),
    );
    draw_text_block(
        &commission.request,
        sheet.x + 16.0,
        sheet.y + 70.0,
        sheet.w - 32.0,
        58.0,
        16.0,
        4.0,
        muted_ink(),
    );
    draw_line(
        sheet.x + 14.0,
        sheet.y + 134.0,
        sheet.right() - 14.0,
        sheet.y + 134.0,
        1.0,
        parchment_line(),
    );

    let mut y = sheet.y + 157.0;
    draw_ui_text_ex(
        "Required notation",
        sheet.x + 16.0,
        y,
        TextStyle::new(16.0, ink()).params(),
    );
    y += 14.0;
    let required = [
        ("Effect", commission.required_effect.as_str()),
        ("Shape", commission.required_shape.as_str()),
        ("Trigger", commission.required_trigger.as_str()),
    ];
    for (label, id) in required {
        draw_requirement(ctx, sheet.x + 16.0, y, sheet.w - 32.0, label, id);
        y += 26.0;
    }
    if let Some(modifier) = &commission.optional_modifier {
        draw_requirement(ctx, sheet.x + 16.0, y, sheet.w - 32.0, "Bonus", modifier);
    }

    let badge_y = sheet.bottom() - 32.0;
    draw_badge(
        Rect::new(sheet.x + 16.0, badge_y, 84.0, 24.0),
        &format!("Risk {}", commission.risk),
        risk_color(&commission.risk),
        parchment(),
    );
    draw_badge(
        Rect::new(sheet.x + 108.0, badge_y, 84.0, 24.0),
        &format_money(commission.reward),
        Color::new(0.16, 0.28, 0.19, 1.0),
        parchment(),
    );
    draw_badge(
        Rect::new(sheet.x + 200.0, badge_y, 78.0, 24.0),
        &format!("+{} insight", commission.insight),
        Color::new(0.19, 0.17, 0.30, 1.0),
        parchment(),
    );

    let button_y = rect.bottom() - 138.0;
    if virtual_button(
        Rect::new(content.x, button_y, content.w, 38.0),
        "Test Diagram",
        true,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::TestDesign);
    }
    let deliver_label = if ctx.session.active_work_kind() == WorkOrderKind::Story {
        "Deliver Product"
    } else {
        "Deliver Talisman"
    };
    if virtual_button(
        Rect::new(content.x, button_y + 48.0, content.w, 38.0),
        deliver_label,
        true,
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::DeliverDesign);
    }
    if ctx.session.active_work_kind() == WorkOrderKind::Story {
        if virtual_button(
            Rect::new(content.x, button_y + 96.0, content.w, 32.0),
            "Research",
            true,
            ButtonTone::Secondary,
            mouse,
        ) {
            actions.push(UiAction::Research);
        }
    } else {
        if virtual_button(
            Rect::new(content.x, button_y + 96.0, (content.w - 10.0) * 0.5, 32.0),
            "Next Job",
            true,
            ButtonTone::Warning,
            mouse,
        ) {
            actions.push(UiAction::SkipCommission);
        }
        if virtual_button(
            Rect::new(
                content.x + (content.w + 10.0) * 0.5,
                button_y + 96.0,
                (content.w - 10.0) * 0.5,
                32.0,
            ),
            "Research",
            true,
            ButtonTone::Secondary,
            mouse,
        ) {
            actions.push(UiAction::Research);
        }
    }
}

fn draw_requirement(ctx: &UiContext<'_>, x: f32, y: f32, w: f32, label: &str, id: &str) {
    let locked = ctx
        .data
        .rune(id)
        .is_some_and(|rune| !ctx.session.can_use_rune(rune));
    let color = if locked {
        Color::new(0.54, 0.28, 0.22, 0.20)
    } else {
        Color::new(0.42, 0.27, 0.12, 0.10)
    };
    draw_surface(
        Rect::new(x, y, w, 23.0),
        &SurfaceStyle::new(color).with_border(1.0, parchment_line()),
    );
    draw_ui_text_ex(
        label,
        x + 9.0,
        y + 16.0,
        TextStyle::new(13.0, muted_ink()).params(),
    );
    let suffix = if locked { " (locked)" } else { "" };
    draw_text_right(
        &format!("{}{}", ctx.data.rune_name(id), suffix),
        x + w - 9.0,
        y + 16.0,
        TextStyle::new(14.0, ink()),
    );
}
