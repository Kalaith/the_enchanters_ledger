//! The Enchanter's Ledger.

use macroquad::prelude::*;

mod browser_clipboard;
mod corpus;
mod data;
mod game;
mod magical_circle;
mod recipes;
mod rune_diagnostics;
mod rune_diagram;
mod rune_drawing;
mod rune_quality;
mod state;
mod ui;

use game::Game;

fn window_conf() -> Conf {
    Conf {
        window_title: "The Enchanter's Ledger".to_owned(),
        window_width: ui::LOGICAL_WIDTH as i32,
        window_height: ui::LOGICAL_HEIGHT as i32,
        window_resizable: true,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    macroquad_toolkit::ui::ensure_default_ui_font().expect("toolkit UI font should load");
    macroquad_toolkit::ui::set_min_ui_font_size(16.0);

    let mut game = Game::new().await;

    loop {
        let dt = get_frame_time().min(0.1);
        game.update(dt);
        game.draw();
        next_frame().await;
    }
}
