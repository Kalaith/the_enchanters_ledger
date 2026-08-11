use super::*;
use crate::data::GameData;
use crate::rune_drawing::{template_strokes_for_rune, StrokePoint};

#[test]
fn diagnostic_log_mentions_circle_clusters_and_final_runes() {
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.34, 0.32, 0.18));
    strokes.extend(template_at("sphere", 0.50, 0.58, 0.18));

    let log = diagnose_diagram(&strokes, data.runes.iter().filter(|rune| rune.tier == 1));

    assert!(log.contains("selected circle"), "{log}");
    assert!(log.contains("clusters sent to rune recognition"), "{log}");
    assert!(log.contains("final interpretation"), "{log}");
    assert!(log.contains("light"), "{log}");
    assert!(log.contains("sphere"), "{log}");
}

#[test]
fn player_hint_flags_missing_circle() {
    let data = GameData::load().unwrap();
    let strokes = template_at("light", 0.50, 0.50, 0.20);

    let hint = player_hint(&strokes, data.runes.iter().filter(|rune| rune.tier == 1));

    assert!(hint.unwrap().contains("circle"), "expected a circle hint");
}

#[test]
fn player_hint_flags_a_weak_circle() {
    let data = GameData::load().unwrap();
    // Same weak partial-arc circle used to prove Sandbox is more lenient than Commission
    // (`state::tests::sandbox_mode_accepts_a_weak_circle_commission_mode_rejects`) — quality
    // ~0.30, below `MIN_CIRCLE_QUALITY` (0.32).
    let mut points = Vec::new();
    for index in 0..=28 {
        let angle = std::f32::consts::TAU * 0.6 * index as f32 / 28.0;
        points.push(StrokePoint::new(
            0.60 + 0.32 * angle.cos(),
            0.60 + 0.16 * angle.sin(),
        ));
    }
    let strokes = vec![DrawnStroke { points }];

    let hint = player_hint(&strokes, data.runes.iter());

    assert!(
        hint.as_deref().unwrap().contains("only reads at"),
        "{hint:?}"
    );
}

#[test]
fn player_hint_is_none_for_a_clean_diagram() {
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.34, 0.32, 0.18));
    strokes.extend(template_at("sphere", 0.50, 0.58, 0.18));

    let hint = player_hint(&strokes, data.runes.iter().filter(|rune| rune.tier == 1));

    assert_eq!(hint, None, "{hint:?}");
}

#[test]
fn blank_diagnostic_log_explains_empty_slate() {
    let data = GameData::load().unwrap();
    let session = GameSession::new(&data.config);

    let log = diagnose_session(&session, &data).unwrap();

    assert!(log.contains("stroke count: 0 useful: 0"), "{log}");
    assert!(log.contains("circle candidates: 0"), "{log}");
    assert!(log.contains("circle_found=false"), "{log}");
}

fn outer_circle() -> Vec<DrawnStroke> {
    let mut points = Vec::new();
    for index in 0..=32 {
        let angle = std::f32::consts::TAU * index as f32 / 32.0;
        points.push(StrokePoint::new(
            0.50 + 0.40 * angle.cos(),
            0.50 + 0.38 * angle.sin(),
        ));
    }
    vec![DrawnStroke { points }]
}

fn template_at(rune_id: &str, cx: f32, cy: f32, scale: f32) -> Vec<DrawnStroke> {
    template_strokes_for_rune(rune_id)
        .unwrap()
        .into_iter()
        .map(|stroke| DrawnStroke {
            points: stroke
                .points
                .into_iter()
                .map(|point| {
                    StrokePoint::new(cx + (point.x - 0.5) * scale, cy + (point.y - 0.5) * scale)
                })
                .collect(),
        })
        .collect()
}
