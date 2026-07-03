//! Human-readable diagnostics for diagram interpretation.

use crate::data::{GameData, RuneDef};
use crate::magical_circle::{classify_circle_stroke, CircleBounds};
use crate::rune_diagram::{
    cluster_strokes, gather_circle_candidates, interpret_diagram, is_circle_structure,
    is_inside_working_circle, select_working_circle_for_strokes, StrokeBounds, MIN_CIRCLE_QUALITY,
    MIN_DIAGRAM_RUNE_CONFIDENCE,
};
use crate::rune_drawing::{recognize_rune, DrawnStroke};
use crate::state::GameSession;
use std::fmt::Write;

pub fn diagnose_session(session: &GameSession, data: &GameData) -> Result<String, String> {
    let commission = session.current_commission(data);
    let unlocked = data
        .runes
        .iter()
        .filter(|rune| session.can_use_rune(rune))
        .collect::<Vec<_>>();
    let mut log = String::new();
    writeln!(log, "The Enchanter's Ledger diagram diagnostic").ok();
    writeln!(log, "version: {}", data.config.version).ok();
    writeln!(
        log,
        "day: {} | rank: {} | work: {} ({})",
        session.player.day,
        session.player.workshop_rank,
        session.active_work_kind().panel_title(),
        commission.id
    )
    .ok();
    writeln!(
        log,
        "required: effect={} shape={} trigger={} optional={}",
        commission.required_effect,
        commission.required_shape,
        commission.required_trigger,
        commission.optional_modifier.as_deref().unwrap_or("none")
    )
    .ok();
    writeln!(
        log,
        "unlocked runes: {}",
        unlocked
            .iter()
            .map(|rune| rune.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
    .ok();
    log.push('\n');
    log.push_str(&diagnose_diagram(
        &session.board.drawing_strokes,
        unlocked.iter().copied(),
    ));
    Ok(log)
}

pub fn diagnose_diagram<'a>(
    strokes: &[DrawnStroke],
    runes: impl IntoIterator<Item = &'a RuneDef>,
) -> String {
    let available_runes = runes.into_iter().collect::<Vec<_>>();
    let useful = strokes
        .iter()
        .enumerate()
        .filter(|(_, stroke)| stroke.has_ink())
        .collect::<Vec<_>>();
    let mut log = String::new();
    writeln!(
        log,
        "stroke count: {} useful: {}",
        strokes.len(),
        useful.len()
    )
    .ok();
    writeln!(
        log,
        "thresholds: circle>={} rune_confidence>={}",
        pct(MIN_CIRCLE_QUALITY),
        pct(MIN_DIAGRAM_RUNE_CONFIDENCE)
    )
    .ok();

    let circle_candidates = gather_circle_candidates(&useful);
    writeln!(log, "\ncircle candidates: {}", circle_candidates.len()).ok();
    for (indices, score, bounds) in &circle_candidates {
        writeln!(
            log,
            "  strokes {indices:?}: circle={} bounds={}",
            pct(*score),
            fmt_bounds(*bounds)
        )
        .ok();
    }

    let Some((circle_member_indices, circle_score, circle_bounds)) =
        select_working_circle_for_strokes(&circle_candidates, &useful, MIN_CIRCLE_QUALITY)
    else {
        writeln!(log, "\nselected circle: none").ok();
        write_stroke_guesses(&mut log, &useful, &available_runes);
        write_final_interpretation(&mut log, strokes, &available_runes);
        return log;
    };

    writeln!(
        log,
        "\nselected circle: strokes {circle_member_indices:?} quality={} bounds={}",
        pct(circle_score),
        fmt_bounds(circle_bounds)
    )
    .ok();

    let spell_bounds = CircleBounds::new(
        circle_bounds.min_x,
        circle_bounds.min_y,
        circle_bounds.max_x,
        circle_bounds.max_y,
    );
    let classified_marks = useful
        .iter()
        .filter(|(index, _)| !circle_member_indices.contains(index))
        .filter_map(|(index, stroke)| {
            classify_circle_stroke(stroke, spell_bounds).map(|mark| (*index, mark))
        })
        .collect::<Vec<_>>();
    writeln!(log, "\ninner stroke filtering:").ok();
    for (index, stroke) in &useful {
        if circle_member_indices.contains(index) {
            continue;
        }
        let inside = is_inside_working_circle(stroke, circle_bounds);
        let structure = is_circle_structure(*index, &classified_marks);
        let mark = classified_marks
            .iter()
            .find(|(mark_index, _)| mark_index == index)
            .map(|(_, mark)| {
                format!(
                    "{:?} q={} scale={:.2} orbit={:.2}",
                    mark.kind,
                    pct(mark.quality),
                    mark.scale,
                    mark.orbit
                )
            })
            .unwrap_or_else(|| "unclassified".to_owned());
        writeln!(
            log,
            "  stroke #{index}: inside={} structure_filtered={} mark={}",
            inside, structure, mark
        )
        .ok();
    }

    let inner_strokes = useful
        .iter()
        .filter(|(index, stroke)| {
            !circle_member_indices.contains(index)
                && is_inside_working_circle(stroke, circle_bounds)
                && !is_circle_structure(*index, &classified_marks)
        })
        .map(|(index, stroke)| (*index, (*stroke).clone()))
        .collect::<Vec<_>>();
    let clusters = cluster_strokes(&inner_strokes);
    writeln!(
        log,
        "\nclusters sent to rune recognition: {}",
        clusters.len()
    )
    .ok();
    for (cluster_index, cluster) in clusters.iter().enumerate() {
        writeln!(
            log,
            "  cluster #{cluster_index}: strokes={:?} bounds={}",
            cluster.indices,
            fmt_bounds(cluster.bounds)
        )
        .ok();
        write_guess(
            &mut log,
            "    combined",
            recognize_rune(&cluster.strokes, available_runes.iter().copied()),
        );
        if cluster.strokes.len() > 1 {
            for (stroke_index, stroke) in cluster.indices.iter().zip(&cluster.strokes) {
                write_guess(
                    &mut log,
                    &format!("    stroke #{stroke_index} alone"),
                    recognize_rune(
                        std::slice::from_ref(stroke),
                        available_runes.iter().copied(),
                    ),
                );
            }
        }
    }

    write_final_interpretation(&mut log, strokes, &available_runes);
    log
}

/// One templated sentence for the single most relevant issue in `strokes` (plan Phase 5 item 3)
/// — a player-facing sibling to `diagnose_diagram`'s verbose dev log, built from the exact same
/// typed calls (`gather_circle_candidates`, `select_working_circle_for_strokes`,
/// `classify_circle_stroke`, `cluster_strokes`, `recognize_rune`) instead of duplicating any
/// recognition logic. Priority, most fundamental problem first: no circle → circle too weak →
/// the most ambiguous rune cluster → a rune-sized mark that read as circle structure instead →
/// a generic "reads, but soft" note when everything above is clean but quality is still low.
/// Returns `None` when there's nothing worth flagging.
pub fn player_hint<'a>(
    strokes: &[DrawnStroke],
    runes: impl IntoIterator<Item = &'a RuneDef>,
) -> Option<String> {
    let available_runes = runes.into_iter().collect::<Vec<_>>();
    let useful = strokes
        .iter()
        .enumerate()
        .filter(|(_, stroke)| stroke.has_ink())
        .collect::<Vec<_>>();
    if useful.is_empty() {
        return None;
    }

    let circle_candidates = gather_circle_candidates(&useful);
    let Some((circle_member_indices, circle_score, circle_bounds)) =
        select_working_circle_for_strokes(&circle_candidates, &useful, 0.0)
    else {
        return Some(
            "No closed shape reads as a circle yet — draw one continuous loop around your runes."
                .to_owned(),
        );
    };
    if circle_score < MIN_CIRCLE_QUALITY {
        return Some(format!(
            "The circle only reads at {} — close the loop where it started and keep the curve \
             even.",
            pct(circle_score)
        ));
    }

    let spell_bounds = CircleBounds::new(
        circle_bounds.min_x,
        circle_bounds.min_y,
        circle_bounds.max_x,
        circle_bounds.max_y,
    );
    let classified_marks = useful
        .iter()
        .filter(|(index, _)| !circle_member_indices.contains(index))
        .filter_map(|(index, stroke)| {
            classify_circle_stroke(stroke, spell_bounds).map(|mark| (*index, mark))
        })
        .collect::<Vec<_>>();

    let inner_strokes = useful
        .iter()
        .filter(|(index, stroke)| {
            !circle_member_indices.contains(index)
                && is_inside_working_circle(stroke, circle_bounds)
                && !is_circle_structure(*index, &classified_marks)
        })
        .map(|(index, stroke)| (*index, (*stroke).clone()))
        .collect::<Vec<_>>();
    let clusters = cluster_strokes(&inner_strokes);

    // Worst-ambiguity cluster: the accepted-but-close-to-a-rival, or rejected-but-close, read.
    let worst_ambiguity = clusters
        .iter()
        .filter_map(|cluster| {
            let guess = recognize_rune(&cluster.strokes, available_runes.iter().copied())?;
            let alt = guess.alternatives.first()?;
            (guess.ambiguous || !guess.accepted).then_some((
                guess.rune_id,
                alt.rune_id.clone(),
                guess.score_gap,
            ))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2));
    if let Some((best, alt, gap)) = worst_ambiguity {
        return Some(format!(
            "One mark could be {best} or {alt} — only {} apart. Make the difference between \
             them clearer.",
            pct(gap)
        ));
    }

    // A rune-sized stroke that got swept into circle structure (ring/satellite/perimeter/
    // script) instead of staying candidate rune ink, per C1/C2's "small runes look like
    // decoration" failure mode (prd.md §5.2).
    let swallowed_as_structure = useful.iter().find(|(index, _)| {
        !circle_member_indices.contains(index) && is_circle_structure(*index, &classified_marks)
    });
    if let Some((_, stroke)) = swallowed_as_structure {
        if recognize_rune(
            std::slice::from_ref(stroke),
            available_runes.iter().copied(),
        )
        .is_some_and(|guess| guess.accepted)
        {
            return Some(
                "A mark near the ring reads as decoration, not a rune — draw it larger or \
                 farther from the other structure marks if it was meant to be a symbol."
                    .to_owned(),
            );
        }
    }

    let final_read = interpret_diagram(strokes, available_runes.iter().copied());
    if final_read.accepted() && final_read.average_rune_quality() < 0.68 {
        return Some(
            "The diagram reads, but precision is soft overall — check start points and stroke \
             order against the guides."
                .to_owned(),
        );
    }

    None
}

fn write_final_interpretation(log: &mut String, strokes: &[DrawnStroke], runes: &[&RuneDef]) {
    let final_read = interpret_diagram(strokes, runes.iter().copied());
    writeln!(log, "\nfinal interpretation:").ok();
    writeln!(
        log,
        "  accepted={} circle_found={} circle={} rejected_marks={}",
        final_read.accepted(),
        final_read.circle_found,
        pct(final_read.circle_quality),
        final_read.rejected_marks
    )
    .ok();
    let read = if final_read.runes.is_empty() {
        "none".to_owned()
    } else {
        final_read
            .runes
            .iter()
            .map(|rune| {
                format!(
                    "{} conf={} quality={} center=({:.2},{:.2})",
                    rune.rune_id,
                    pct(rune.confidence),
                    pct(rune.quality),
                    rune.center.x,
                    rune.center.y
                )
            })
            .collect::<Vec<_>>()
            .join(" | ")
    };
    writeln!(log, "  runes: {read}").ok();
}

fn write_stroke_guesses(log: &mut String, useful: &[(usize, &DrawnStroke)], runes: &[&RuneDef]) {
    writeln!(log, "\nstroke guesses without selected circle:").ok();
    for (index, stroke) in useful {
        write_guess(
            log,
            &format!("  stroke #{index}"),
            recognize_rune(std::slice::from_ref(*stroke), runes.iter().copied()),
        );
    }
}

fn write_guess(
    log: &mut String,
    label: &str,
    guess: Option<crate::rune_drawing::RecognitionOutcome>,
) {
    if let Some(guess) = guess {
        let status = if guess.accepted {
            "accepted"
        } else {
            "below threshold"
        };
        let ambiguity = if guess.ambiguous {
            format!(" ambiguous gap={}", pct(guess.score_gap))
        } else {
            format!(" gap={}", pct(guess.score_gap))
        };
        let alternatives = if guess.alternatives.is_empty() {
            String::new()
        } else {
            format!(
                " alternatives=[{}]",
                guess
                    .alternatives
                    .iter()
                    .map(|candidate| format!("{}:{}", candidate.rune_id, pct(candidate.confidence)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        writeln!(
            log,
            "{label}: best={} confidence={} quality={}{}{} {status}",
            guess.rune_id,
            pct(guess.confidence),
            pct(guess.quality),
            ambiguity,
            alternatives
        )
        .ok();
    } else {
        writeln!(log, "{label}: no recognizable ink").ok();
    }
}

fn fmt_bounds(bounds: StrokeBounds) -> String {
    format!(
        "x={:.2}..{:.2} y={:.2}..{:.2} w={:.2} h={:.2}",
        bounds.min_x,
        bounds.max_x,
        bounds.min_y,
        bounds.max_y,
        bounds.width(),
        bounds.height()
    )
}

fn pct(value: f32) -> String {
    format!("{}%", (value.clamp(0.0, 1.0) * 100.0).round() as i32)
}

#[cfg(test)]
mod tests {
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
}
