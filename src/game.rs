//! High-level game loop, state transitions, and toolkit integration.

use crate::browser_clipboard::{copy_text, ClipboardCopy};
use crate::data::GameData;
use crate::rune_diagnostics::diagnose_session;
use crate::rune_drawing::{erase_strokes_at, DrawnStroke};
use crate::rune_quality::{practice_report_for_rune, RunePracticeReport};
use crate::state::{migrate_save_value, CraftReport, GamePhase, GameSession, SaveData};
use crate::ui::{self, PracticeUi, UiAction, UiContext};
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

pub struct Game {
    data: GameData,
    session: GameSession,
    assets: AssetManager,
    notifications: NotificationManager,
    events: EventBus<UiAction>,
    title_texture: Texture2D,
    save_exists: bool,
    save_slots: Vec<String>,
    rune_guide_pages: [usize; 4],
    journal_open: bool,
    settings_open: bool,
    fullscreen: bool,
    suppress_rune_erase: bool,
    practice: PracticeState,
}

#[derive(Debug, Default)]
struct PracticeState {
    open: bool,
    strokes: Vec<DrawnStroke>,
    active_stroke: Option<DrawnStroke>,
    report: Option<RunePracticeReport>,
}

impl Game {
    pub async fn new() -> Self {
        let data = GameData::load().unwrap_or_else(|err| {
            panic!(
                "The Enchanter's Ledger embedded data failed to load: {}",
                err
            );
        });

        let mut assets = AssetManager::new();
        let placeholder = Image::gen_image_color(16, 16, Color::new(0.72, 0.55, 0.28, 1.0));
        assets.set_placeholder_texture_direct(Texture2D::from_image(&placeholder));
        let loaded_assets = assets.load_texture_configs(&data.texture_manifest).await;
        let title_texture = Texture2D::from_file_with_format(
            include_bytes!("../ledger_title.png"),
            Some(ImageFormat::Png),
        );
        title_texture.set_filter(FilterMode::Linear);

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
            title_texture,
            save_exists: false,
            save_slots: Vec::new(),
            rune_guide_pages: [0; 4],
            journal_open: false,
            settings_open: false,
            fullscreen: false,
            suppress_rune_erase: false,
            practice: PracticeState::default(),
        };
        game.refresh_save_state();
        game
    }

    pub fn update(&mut self, dt: f32) {
        self.notifications.update(dt);
        self.session.update_focus(&self.data.config, dt);
        if self.suppress_rune_erase && !is_mouse_button_down(MouseButton::Right) {
            self.suppress_rune_erase = false;
        }
        if let Some(msg) = self.session.ensure_playable_work(&self.data) {
            self.notifications.info(msg);
        }
        self.handle_keyboard();

        let actions: Vec<UiAction> = self.events.drain().collect();
        for action in actions {
            self.apply_action(action);
        }
    }

    pub fn draw(&mut self) {
        clear_background(Color::new(0.045, 0.05, 0.06, 1.0));

        let virtual_ui = ui::begin_ui_frame();
        let ctx = UiContext {
            data: &self.data,
            session: &self.session,
            save_exists: self.save_exists,
            save_slots: &self.save_slots,
            loaded_assets: self.assets.len(),
            rune_guide_pages: &self.rune_guide_pages,
            journal_open: self.journal_open,
            settings_open: self.settings_open,
            fullscreen: self.fullscreen,
            suppress_rune_erase: self.suppress_rune_erase,
            title_texture: &self.title_texture,
            practice: PracticeUi {
                open: self.practice.open,
                strokes: &self.practice.strokes,
                active_stroke: self.practice.active_stroke.as_ref(),
                report: self.practice.report.as_ref(),
            },
            ui: &virtual_ui,
        };

        let actions = ui::draw_game_ui(ctx);
        ui::end_ui_frame();

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
        if self.settings_open {
            if is_key_pressed(KeyCode::Escape) {
                self.events.push(UiAction::CloseSettings);
            }
            if is_key_pressed(KeyCode::F11) {
                self.events.push(UiAction::ToggleFullscreen);
            }
            return;
        }

        if self.session.phase == GamePhase::Title {
            if is_key_pressed(KeyCode::Enter) {
                if self.save_exists {
                    self.events.push(UiAction::Load);
                } else {
                    self.events.push(UiAction::NewGame);
                }
            }
            if is_key_pressed(KeyCode::N) {
                self.events.push(UiAction::NewGame);
            }
            if is_key_pressed(KeyCode::C) && self.save_exists {
                self.events.push(UiAction::Load);
            }
            if is_key_pressed(KeyCode::S) {
                self.events.push(UiAction::OpenSettings);
            }
            if is_key_pressed(KeyCode::Escape) {
                self.events.push(UiAction::ExitGame);
            }
            return;
        }

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
        if is_key_pressed(KeyCode::J) {
            self.events.push(UiAction::ToggleJournal);
        }
        if is_key_pressed(KeyCode::Escape) {
            if self.practice.open {
                self.events.push(UiAction::ClosePractice);
            } else if self.journal_open {
                self.events.push(UiAction::CloseJournal);
            } else {
                self.events.push(UiAction::ClearBoard);
            }
        }
    }

    fn apply_action(&mut self, action: UiAction) {
        match action {
            UiAction::StartGame => {
                self.session.start_playing();
                self.notifications
                    .success(format!("{} opens the shop.", self.session.player.name));
                self.save_game();
            }
            UiAction::NewGame => {
                self.session = GameSession::new(&self.data.config);
                self.session.phase = GamePhase::Naming;
                self.rune_guide_pages = [0; 4];
                self.journal_open = false;
                self.settings_open = false;
                self.suppress_rune_erase = false;
                self.practice = PracticeState::default();
                self.notifications.info("Started a fresh ledger.");
            }
            UiAction::OpenSettings => {
                self.settings_open = true;
                self.journal_open = false;
                self.practice.open = false;
            }
            UiAction::CloseSettings => {
                self.settings_open = false;
            }
            UiAction::ToggleFullscreen => {
                self.fullscreen = !self.fullscreen;
                set_fullscreen(self.fullscreen);
            }
            UiAction::ExitGame => self.exit_game(),
            UiAction::ToggleJournal => {
                self.journal_open = !self.journal_open;
            }
            UiAction::CloseJournal => {
                self.journal_open = false;
            }
            UiAction::OpenPractice => {
                self.practice.open = true;
                self.journal_open = false;
                self.practice.report = None;
                self.notifications
                    .info("Practice slate opened for the selected rune.");
            }
            UiAction::ClosePractice => {
                self.practice.open = false;
            }
            UiAction::ClearPractice => {
                self.practice.strokes.clear();
                self.practice.active_stroke = None;
                self.practice.report = None;
            }
            UiAction::ScorePractice => self.score_practice(),
            UiAction::StartPracticeStroke(point) => {
                self.practice.active_stroke = Some(DrawnStroke::new(point));
                self.practice.report = None;
            }
            UiAction::ExtendPracticeStroke(point) => {
                if let Some(stroke) = &mut self.practice.active_stroke {
                    stroke.push(point);
                }
            }
            UiAction::FinishPracticeStroke => {
                if let Some(stroke) = self.practice.active_stroke.take() {
                    if stroke.has_ink() {
                        self.practice.strokes.push(stroke);
                    }
                }
            }
            UiAction::ErasePracticeInk(point, radius) => {
                erase_strokes_at(&mut self.practice.strokes, point, radius);
                self.practice.report = None;
            }
            UiAction::SelectStoryWork => match self.session.select_story_work(&self.data) {
                Ok(msg) => {
                    self.journal_open = false;
                    self.notifications.info(msg);
                }
                Err(err) => self.notifications.warning(err),
            },
            UiAction::SelectTalismanWork(index) => {
                match self.session.select_talisman_work(index, &self.data) {
                    Ok(msg) => {
                        self.journal_open = false;
                        self.notifications.info(msg);
                    }
                    Err(err) => self.notifications.warning(err),
                }
            }
            UiAction::Save => self.save_game(),
            UiAction::Load => self.load_game(),
            UiAction::DeleteSave => self.delete_save(),
            UiAction::SetRuneGuidePage(category_index, page) => {
                if let Some(slot) = self.rune_guide_pages.get_mut(category_index) {
                    *slot = page;
                }
            }
            UiAction::SelectRune(id) => match self.session.select_rune(&id, &self.data) {
                Ok(()) => {
                    let name = self.data.rune_name(&id);
                    self.practice.report = None;
                    self.notifications.info(format!(
                        "Selected {}; click the slate to place a guide.",
                        name
                    ));
                }
                Err(err) => self.notifications.warning(err),
            },
            UiAction::DeselectRune => {
                self.session.deselect_rune();
                self.notifications.info("Cleared the rune guide selection.");
            }
            UiAction::PlaceRuneTemplate(point) => {
                match self.session.place_guide_template(point, &self.data) {
                    Ok(msg) => self.notifications.info(msg),
                    Err(err) => self.notifications.warning(err),
                }
            }
            UiAction::RemoveRuneTemplate(index) => {
                match self.session.remove_guide_template(index, &self.data) {
                    Ok(msg) => {
                        self.suppress_rune_erase = is_mouse_button_down(MouseButton::Right);
                        self.notifications.info(msg);
                    }
                    Err(err) => self.notifications.warning(err),
                }
            }
            UiAction::StartRuneStroke(point) => self.session.start_drawing_stroke(point),
            UiAction::ExtendRuneStroke(point) => self.session.extend_drawing_stroke(point),
            UiAction::FinishRuneStroke => self.session.finish_drawing_stroke(),
            UiAction::EraseRuneInk(point, radius) => {
                self.session.erase_drawing_at(point, radius);
            }
            UiAction::ClearRuneDrawing => {
                self.session.clear_drawing();
                self.notifications.info("Cleared the rune slate.");
            }
            UiAction::InterpretDiagram => match self.session.interpret_drawing(&self.data) {
                Ok(msg) => {
                    self.notifications.info(msg);
                    if let Some(tutorial_msg) = self.session.advance_tutorial_after_interpret() {
                        self.notifications.success(tutorial_msg);
                        self.save_game();
                    }
                }
                Err(err) => self.notifications.warning(err),
            },
            UiAction::CopyDiagnostics => self.copy_diagnostics(),
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
            UiAction::Research => match self.session.research(&self.data) {
                Ok(msg) => self.notifications.success(msg),
                Err(err) => self.notifications.warning(err),
            },
            UiAction::SkipCommission => {
                self.session.skip_commission(&self.data);
                self.notifications
                    .warning("Declined the commission. Reputation -1.");
            }
        }
    }

    fn report_result(&mut self, label: &str, report: CraftReport) {
        if let Some(discovery) = &report.discovery {
            self.notifications.success(format!(
                "New enchantment discovered: {} (+{} insight)",
                discovery.name, discovery.insight
            ));
        }

        let mut payouts = Vec::new();
        if report.reward > 0 {
            payouts.push(format!("+{} coins", report.reward));
        }
        if report.reputation != 0 {
            payouts.push(format!("{:+} rep", report.reputation));
        }
        if report.insight > 0 {
            payouts.push(format!("+{} insight", report.insight));
        }
        let payout = if payouts.is_empty() {
            String::new()
        } else {
            format!(" {}", payouts.join(", "))
        };
        let notes = if report.notes.is_empty() {
            String::new()
        } else {
            format!(" | {}", report.notes.join("; "))
        };
        let message = format!(
            "{}: {} ({}, score {}){}{}",
            label,
            report.result.title,
            report.result.grade.label(),
            report.result.score,
            payout,
            notes
        );

        match report.result.grade {
            crate::state::EnchantGrade::Brilliant => self.notifications.success(message),
            crate::state::EnchantGrade::Reliable => self.notifications.success(message),
            crate::state::EnchantGrade::Unstable => self.notifications.warning(message),
            crate::state::EnchantGrade::Failed => self.notifications.danger(message),
        }
    }

    fn score_practice(&mut self) {
        let Some(rune_id) = self.session.board.selected_rune.as_deref() else {
            self.notifications
                .warning("Select a rune in the guide before practicing.");
            return;
        };
        if self.practice.strokes.is_empty() && self.practice.active_stroke.is_none() {
            self.notifications.warning("The practice slate is blank.");
            return;
        }
        if self.practice.active_stroke.is_some() {
            self.notifications
                .warning("Lift the pen before checking the practice mark.");
            return;
        }
        let runes = self
            .data
            .runes
            .iter()
            .filter(|rune| self.session.can_use_rune(rune));
        let Some(report) = practice_report_for_rune(rune_id, &self.practice.strokes, runes) else {
            self.notifications
                .warning("The practice mark is too faint to score.");
            return;
        };
        let quality = report.quality;
        self.practice.report = Some(report);
        if quality >= 0.78 {
            self.notifications
                .success(format!("Practice quality {:.0}%.", quality * 100.0));
        } else {
            self.notifications
                .info(format!("Practice quality {:.0}%.", quality * 100.0));
        }
    }

    fn copy_diagnostics(&mut self) {
        match diagnose_session(&self.session, &self.data) {
            Ok(log) => {
                let line_count = log.lines().count();
                let copy_result = copy_text(&log);
                println!("{log}");
                self.session.board.last_diagnostic_log = Some(log.clone());
                self.session.board.last_interpretation_note =
                    Some(diagnostic_preview(&log, line_count));
                match copy_result {
                    ClipboardCopy::Copied => self.notifications.info(format!(
                        "Copied diagram diagnostic log ({} lines).",
                        line_count
                    )),
                    ClipboardCopy::Requested => self.notifications.info(format!(
                        "Requested browser copy; diagnostic panel opened ({} lines).",
                        line_count
                    )),
                    ClipboardCopy::Failed => self.notifications.warning(format!(
                        "Clipboard was blocked; diagnostic panel opened ({} lines).",
                        line_count
                    )),
                }
            }
            Err(err) => self.notifications.warning(err),
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
                if let Some(msg) = self.session.ensure_playable_work(&self.data) {
                    self.notifications.info(msg);
                }
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

    fn exit_game(&mut self) {
        if self.session.phase != GamePhase::Title {
            self.save_game();
        }
        macroquad::miniquad::window::quit();
    }

    fn refresh_save_state(&mut self) {
        self.save_exists = slot_exists(&self.data.config.game_name, &self.data.config.save_slot);
        self.save_slots = get_save_slots(&self.data.config.game_name);
    }
}

fn diagnostic_preview(log: &str, line_count: usize) -> String {
    let preview = log.lines().take(4).collect::<Vec<_>>().join(" | ");
    format!("Diagnostic log copied ({} lines). {}", line_count, preview)
}
