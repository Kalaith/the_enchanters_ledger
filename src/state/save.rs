use super::{GameSession, SaveData};
use crate::data::GameConfig;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct LegacySave {
    points: Option<i64>,
    energy: Option<f32>,
    turn: Option<u32>,
}

pub fn migrate_save_value(
    detected_version: Option<String>,
    value: Value,
    config: &GameConfig,
) -> Result<SaveData, String> {
    let payload = value.get("data").cloned().unwrap_or(value);

    if let Ok(mut current) = serde_json::from_value::<SaveData>(payload.clone()) {
        current.version = config.version.clone();
        return Ok(current);
    }

    let legacy: LegacySave = serde_json::from_value(payload)
        .map_err(|err| format!("Unsupported save format {:?}: {}", detected_version, err))?;

    let mut session = GameSession::new(config);
    session.phase = super::GamePhase::Playing;
    session.player.tutorial_stage = super::TutorialStage::Complete;
    if let Some(points) = legacy.points {
        session.player.coins = points;
    }
    if let Some(energy) = legacy.energy {
        session.player.focus = energy.clamp(0.0, config.max_focus);
    }
    if let Some(turn) = legacy.turn {
        session.player.day = turn.max(1);
    }

    Ok(session.to_save(&config.version))
}
