//! High-level game loop, state transitions, and toolkit integration.

use crate::data::GameData;
use crate::state::{migrate_save_value, CraftReport, GamePhase, GameSession, SaveData};
use crate::ui::{self, UiAction, UiContext};
use macroquad::prelude::*;
use macroquad_toolkit::assets::AssetManager;
use macroquad_toolkit::events::EventBus;
use macroquad_toolkit::notifications::{
    NotificationAnchor, NotificationManager, NotificationRenderConfig,
};
use macroquad_toolkit::persistence::{
    delete_slot, get_save_slots, load_from_slot_with_migration, save_to_slot_with_version,
    slot_exists,
};
use macroquad_toolkit::prelude::{begin_virtual_ui_frame, end_virtual_ui_frame};

pub struct Game {
    data: GameData,
    session: GameSession,
    assets: AssetManager,
    notifications: NotificationManager,
    events: EventBus<UiAction>,
    save_exists: bool,
    save_slots: Vec<String>,
}

impl Game {
    pub async fn new() -> Self {
        let data = GameData::load().unwrap_or_else(|err| {
            panic!("The Enchanter's Ledger embedded data failed to load: {}", err);
        });

        let mut assets = AssetManager::new();
        let placeholder = Image::gen_image_color(16, 16, Color::new(0.72, 0.55, 0.28, 1.0));
        assets.set_placeholder_texture_direct(Texture2D::from_image(&placeholder));
        let loaded_assets = assets.load_texture_configs(&data.texture_manifest).await;

        let mut notifications = NotificationManager::new();
        notifications.info(format!(
            "Opened the workshop ledger; {} textures loaded.",
            loaded_assets
        ));

        let session = GameSession::new(&data.config);
        let mut game = Self {
            data,
            session,
            assets,
            notifications,
            events: EventBus::new(),
            save_exists: false,
            save_slots: Vec::new(),
        };
        game.refresh_save_state();
        game
    }

    pub fn update(&mut self, dt: f32) {
        self.notifications.update(dt);
        self.session.update_focus(&self.data.config, dt);
        self.handle_keyboard();

        let actions: Vec<UiAction> = self.events.drain().collect();
        for action in actions {
            self.apply_action(action);
        }
    }

    pub fn draw(&mut self) {
        clear_background(Color::new(0.045, 0.05, 0.06, 1.0));

        let virtual_ui = begin_virtual_ui_frame(ui::LOGICAL_WIDTH, ui::LOGICAL_HEIGHT);
        let ctx = UiContext {
            data: &self.data,
            session: &self.session,
            save_exists: self.save_exists,
            save_slots: &self.save_slots,
            loaded_assets: self.assets.len(),
            ui: &virtual_ui,
        };

        let actions = ui::draw_game_ui(ctx);
        end_virtual_ui_frame();

        for action in actions {
            self.events.push(action);
        }

        self.notifications
            .draw_with_config(&NotificationRenderConfig {
                anchor: NotificationAnchor::BottomRight,
                ..Default::default()
            });
    }

    fn handle_keyboard(&mut self) {
        if self.session.phase == GamePhase::Naming {
            while let Some(c) = get_char_pressed() {
                self.session.append_name_char(c);
            }
            if is_key_pressed(KeyCode::Backspace) {
                self.session.pop_name_char();
            }
            if is_key_pressed(KeyCode::Enter) {
                self.events.push(UiAction::StartGame);
            }
            return;
        }

        if is_key_pressed(KeyCode::S) {
            self.events.push(UiAction::Save);
        }
        if is_key_pressed(KeyCode::L) {
            self.events.push(UiAction::Load);
        }
        if is_key_pressed(KeyCode::T) {
            self.events.push(UiAction::TestDesign);
        }
        if is_key_pressed(KeyCode::D) {
            self.events.push(UiAction::DeliverDesign);
        }
        if is_key_pressed(KeyCode::R) {
            self.events.push(UiAction::Research);
        }
        if is_key_pressed(KeyCode::N) {
            self.events.push(UiAction::SkipCommission);
        }
        if is_key_pressed(KeyCode::Escape) {
            self.events.push(UiAction::ClearBoard);
        }
    }

    fn apply_action(&mut self, action: UiAction) {
        match action {
            UiAction::StartGame => {
                self.session.start_playing();
                self.notifications.success(format!(
                    "{} opens the shop.",
                    self.session.player.name
                ));
            }
            UiAction::NewGame => {
                self.session = GameSession::new(&self.data.config);
                self.notifications.info("Started a fresh ledger.");
            }
            UiAction::Save => self.save_game(),
            UiAction::Load => self.load_game(),
            UiAction::DeleteSave => self.delete_save(),
            UiAction::SelectRune(id) => match self.session.select_rune(&id, &self.data) {
                Ok(()) => {
                    let name = self.data.rune_name(&id);
                    self.notifications.info(format!("Selected {} rune.", name));
                }
                Err(err) => self.notifications.warning(err),
            },
            UiAction::SelectLinkTool => {
                self.session.select_link_tool();
                self.notifications.info("Selected ink link tool.");
            }
            UiAction::UseBoardNode(node) => match self.session.use_board_node(node, &self.data) {
                Ok(msg) => self.notifications.info(msg),
                Err(err) => self.notifications.warning(err),
            },
            UiAction::EraseBoardNode(node) => match self.session.erase_node(node) {
                Ok(msg) => self.notifications.info(msg),
                Err(err) => self.notifications.warning(err),
            },
            UiAction::ClearBoard => {
                self.session.clear_board();
                self.notifications.info("Cleared the drafting page.");
            }
            UiAction::TestDesign => {
                let report = self.session.test_design(&self.data);
                self.report_result("Test", report);
            }
            UiAction::DeliverDesign => {
                let report = self.session.deliver_design(&self.data);
                self.report_result("Delivered", report);
            }
            UiAction::Research => match self.session.research() {
                Ok(msg) => self.notifications.success(msg),
                Err(err) => self.notifications.warning(err),
            },
            UiAction::SkipCommission => {
                self.session.skip_commission(&self.data);
                self.notifications.warning("Declined the commission. Reputation -1.");
            }
        }
    }

    fn report_result(&mut self, label: &str, report: CraftReport) {
        if let Some(name) = report.discovery {
            self.notifications.success(format!("New enchantment discovered: {}", name));
        }

        let payout = if report.reward > 0 {
            format!(
                " +{} coins, {:+} rep, +{} insight",
                report.reward, report.reputation, report.insight
            )
        } else {
            String::new()
        };
        let message = format!(
            "{}: {} ({}, score {}){}",
            label,
            report.result.title,
            report.result.grade.label(),
            report.result.score,
            payout
        );

        match report.result.grade {
            crate::state::EnchantGrade::Brilliant => self.notifications.success(message),
            crate::state::EnchantGrade::Reliable => self.notifications.success(message),
            crate::state::EnchantGrade::Unstable => self.notifications.warning(message),
            crate::state::EnchantGrade::Failed => self.notifications.danger(message),
        }
    }

    fn save_game(&mut self) {
        let save = self.session.to_save(&self.data.config.version);
        match save_to_slot_with_version(
            &self.data.config.game_name,
            &self.data.config.save_slot,
            &save,
            &self.data.config.version,
        ) {
            Ok(()) => {
                self.notifications.success("Saved the workshop ledger.");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.danger(format!("Save failed: {}", err)),
        }
    }

    fn load_game(&mut self) {
        let loaded: Result<SaveData, String> = load_from_slot_with_migration(
            &self.data.config.game_name,
            &self.data.config.save_slot,
            &self.data.config.version,
            |version, value| migrate_save_value(version, value, &self.data.config),
        );

        match loaded {
            Ok(save) => {
                self.session = GameSession::from_save(save);
                self.notifications.success("Loaded the saved ledger.");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.warning(format!("Load failed: {}", err)),
        }
    }

    fn delete_save(&mut self) {
        match delete_slot(&self.data.config.game_name, &self.data.config.save_slot) {
            Ok(()) => {
                self.notifications.info("Deleted the saved ledger slot.");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.danger(format!("Delete failed: {}", err)),
        }
    }

    fn refresh_save_state(&mut self) {
        self.save_exists = slot_exists(&self.data.config.game_name, &self.data.config.save_slot);
        self.save_slots = get_save_slots(&self.data.config.game_name);
    }
}
