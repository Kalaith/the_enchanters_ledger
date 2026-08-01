//! The Enchanter's Ledger.

use macroquad::prelude::*;
use macroquad_toolkit::capture;

mod browser_clipboard;
mod corpus;
mod data;
mod game;
mod ladder;
mod magical_circle;
mod manual;
mod perfect_diagram;
mod reading;
mod recipes;
mod rune_diagnostics;
mod rune_diagram;
mod rune_drawing;
mod rune_quality;
mod state;
mod ui;

use game::Game;

fn window_conf() -> Conf {
    capture::capture_window_conf(
        "THE_ENCHANTERS_LEDGER",
        "The Enchanter's Ledger",
        ui::LOGICAL_WIDTH as i32,
        ui::LOGICAL_HEIGHT as i32,
    )
}

/// Phase 6 item 3: a fast tuning-loop entry point that replays a corpus/diagram JSON file
/// through `rune_diagnostics::diagnose_diagram` and prints the log, without opening the game
/// window. The crate is binary-only (no `src/lib.rs`), so a real `cargo run --example` would
/// need a lib/bin split; this CLI branch reuses `main.rs` as the crate root instead, at zero
/// extra visibility cost. `#[macroquad::main]` is intentionally not used on `main` itself (its
/// expansion creates the game window before any user code runs, which this branch must skip
/// entirely) — `main` is a plain `fn` that either handles `--diagnose` and returns, or falls
/// through to the same `Window::from_config(window_conf(), amain())` call the attribute would
/// have generated.
fn main() {
    if let Some(path) = cli_path_after("--diagnose") {
        match run_diagnose_cli(&path) {
            Ok(log) => println!("{log}"),
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        return;
    }
    if let Some(dir) = cli_path_after("--manual") {
        match run_manual_cli(&dir) {
            Ok(written) => println!("wrote {}", written.display()),
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        return;
    }
    macroquad::Window::from_config(window_conf(), amain());
}

fn cli_path_after(flag: &str) -> Option<std::path::PathBuf> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == flag {
            return args.next().map(std::path::PathBuf::from);
        }
    }
    None
}

/// Writes the quest diagram manual (`manual::render_manual_page`) as one
/// self-contained HTML file. Same rationale as `--diagnose` above: a
/// windowless entry point into the crate, so the manual can be regenerated in
/// CI or from a shell without booting the game.
fn run_manual_cli(dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let data = data::GameData::load()?;
    let entries = manual::manual_entries(&data);
    let page = manual::render_manual_page(&entries, &data);
    std::fs::create_dir_all(dir)
        .map_err(|err| format!("failed to create {}: {err}", dir.display()))?;
    let path = dir.join("index.html");
    std::fs::write(&path, page)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(path)
}

fn run_diagnose_cli(path: &std::path::Path) -> Result<String, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let strokes = parse_diagram_json(&text)?;
    let data = data::GameData::load()?;
    Ok(rune_diagnostics::diagnose_diagram(&strokes, &data.runes))
}

/// Accepts either a `corpus::CorpusSample` (`{label, strokes}`, the shape
/// `capture_corpus_sample` writes to `tests/corpus/`) or a bare `Vec<DrawnStroke>` diagram, so
/// this tool replays both a labeled single-rune sample and a full drawn diagram.
fn parse_diagram_json(text: &str) -> Result<Vec<rune_drawing::DrawnStroke>, String> {
    if let Ok(sample) = serde_json::from_str::<corpus::CorpusSample>(text) {
        return Ok(sample.strokes);
    }
    serde_json::from_str::<Vec<rune_drawing::DrawnStroke>>(text)
        .map_err(|err| format!("not a CorpusSample or a Vec<DrawnStroke>: {err}"))
}

async fn amain() {
    macroquad_toolkit::ui::ensure_default_ui_font().expect("toolkit UI font should load");
    macroquad_toolkit::ui::set_min_ui_font_size(16.0);

    let mut game = Game::new().await;

    // Screenshot harness: when THE_ENCHANTERS_LEDGER_CAPTURE_PATH is set, seed
    // a scene, simulate deterministic frames, write a PNG, and exit.
    if let Some(config) = capture::CaptureConfig::from_env("THE_ENCHANTERS_LEDGER") {
        game.begin_capture_scene(&config.scene);
        capture::run_capture(&config, |dt| {
            game.update(dt);
            game.draw();
        })
        .await;
        return;
    }

    loop {
        let dt = get_frame_time().min(0.1);
        game.update(dt);
        game.draw();
        next_frame().await;
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn run_diagnose_cli_reads_a_bare_stroke_list_and_reports_the_recognized_rune() {
        let strokes = rune_drawing::template_strokes_for_rune("spark").unwrap();
        let json = serde_json::to_string(&strokes).unwrap();
        let path = std::env::temp_dir().join(format!(
            "enchanters_ledger_cli_test_bare_{}.json",
            std::process::id()
        ));
        std::fs::write(&path, json).unwrap();

        let log = run_diagnose_cli(&path);
        std::fs::remove_file(&path).ok();

        let log = log.unwrap();
        assert!(log.contains("spark"), "{log}");
    }

    #[test]
    fn run_diagnose_cli_reads_a_corpus_sample_and_reports_the_recognized_rune() {
        let strokes = rune_drawing::template_strokes_for_rune("spark").unwrap();
        let sample = corpus::CorpusSample {
            label: "spark".to_owned(),
            strokes,
        };
        let json = serde_json::to_string(&sample).unwrap();
        let path = std::env::temp_dir().join(format!(
            "enchanters_ledger_cli_test_sample_{}.json",
            std::process::id()
        ));
        std::fs::write(&path, json).unwrap();

        let log = run_diagnose_cli(&path);
        std::fs::remove_file(&path).ok();

        let log = log.unwrap();
        assert!(log.contains("spark"), "{log}");
    }

    #[test]
    fn run_diagnose_cli_reports_an_error_for_a_missing_file() {
        let path = std::env::temp_dir().join("enchanters_ledger_cli_test_missing_file.json");
        std::fs::remove_file(&path).ok();

        assert!(run_diagnose_cli(&path).is_err());
    }
}
