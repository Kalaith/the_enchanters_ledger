//! The Enchanter's Ledger.

use macroquad::prelude::*;

mod data;
mod game;
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
    let mut game = Game::new().await;

    loop {
        let dt = get_frame_time().min(0.1);
        game.update(dt);
        game.draw();
        next_frame().await;
    }
}
