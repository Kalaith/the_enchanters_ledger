//! Synthetic grand-diagram round trips: many symbols packed inside one
//! working circle must all be found, none merged or eaten, regardless of the
//! order they were drawn in.

use super::fixtures::*;
use crate::data::GameData;
use crate::rune_diagram::{interpret_diagram, InterpretedRune};
use crate::rune_drawing::test_support::perturb;
use crate::rune_drawing::DrawnStroke;

#[test]
fn hundred_plus_symbol_diagram_round_trips_within_perf_budget_and_order_independence() {
    // Phase 3 exit criterion: a synthetic ~120-symbol diagram (small "spark" runes packed on a
    // grid inside the working circle) round-trips — every symbol found, none merged/eaten — and
    // completes well inside a generous sanity budget. This is a debug `cargo test` run, so the
    // time bound is a "doesn't regress asymptotically / hang" check, not the plan doc's
    // native-release ~100ms target. Also exercises items 2/5's order-independence together:
    // reversing the draw order must not change which runes are found.
    let data = GameData::load().unwrap();
    let sparks = small_rune_grid("spark", 120, 0.055, 0.028);
    assert!(
        sparks.len() >= 100,
        "grid only placed {} sparks",
        sparks.len()
    );

    let mut strokes = outer_circle();
    strokes.extend(sparks.clone());

    let start = std::time::Instant::now();
    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let elapsed = start.elapsed();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert_eq!(
        interpretation.runes.len(),
        sparks.len(),
        "found {} of {} sparks: {:?}",
        interpretation.runes.len(),
        sparks.len(),
        interpretation.runes
    );
    assert!(
        interpretation
            .runes
            .iter()
            .all(|rune| rune.rune_id == "spark"),
        "{:?}",
        interpretation.runes
    );
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "interpret_diagram took {elapsed:?} for a {}-stroke diagram",
        strokes.len()
    );

    let mut shuffled = strokes;
    shuffled[1..].reverse();
    let shuffled_interpretation = interpret_diagram(&shuffled, rank_one(&data));
    assert_eq!(
        shuffled_interpretation.runes.len(),
        interpretation.runes.len(),
        "reversing draw order changed how many runes were found"
    );
}

#[test]
fn synthetic_diagram_round_trips_across_sizes_with_jitter() {
    // Phase 6 item 1 exit criterion: a synthetic diagram generator, jittered per symbol, must
    // round-trip across a size sweep from a handful of symbols up toward the plan's 300-symbol
    // target — not just the single fixed ~120 clean-template case
    // `hundred_plus_symbol_diagram_round_trips_within_perf_budget_and_order_independence`
    // already covers. Spacing/scale tighten at larger sizes so more instances fit inside the
    // working circle while staying distinct clusters (§5.6's scale-relative clustering floors).
    let data = GameData::load().unwrap();
    let runes = rank_one(&data);

    for (size, spacing, scale) in [
        (3usize, 0.10, 0.030),
        (10, 0.075, 0.028),
        (30, 0.062, 0.026),
        (60, 0.058, 0.024),
        (120, 0.055, 0.022),
        (200, 0.046, 0.018),
        (300, 0.039, 0.015),
    ] {
        let sparks = jittered_rune_grid("spark", size, spacing, scale, size as u64 * 1000);
        let placed = sparks.len();
        assert!(
            placed >= size,
            "size={size}: grid only placed {placed} sparks"
        );

        let mut strokes = outer_circle();
        strokes.extend(sparks);

        let start = std::time::Instant::now();
        let interpretation = interpret_diagram(&strokes, runes.iter().copied());
        let elapsed = start.elapsed();

        assert!(interpretation.accepted(), "size={size}: {interpretation:?}");
        assert_eq!(
            interpretation.runes.len(),
            placed,
            "size={size}: found {} of {placed} sparks: {:?}",
            interpretation.runes.len(),
            interpretation.runes
        );
        assert!(
            interpretation
                .runes
                .iter()
                .all(|rune| rune.rune_id == "spark"),
            "size={size}: {:?}",
            interpretation.runes
        );
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "size={size}: interpret_diagram took {elapsed:?} for a {}-stroke diagram",
            strokes.len()
        );
    }
}

#[test]
fn heterogeneous_diagram_round_trips_and_order_shuffle_changes_nothing() {
    // Phase 3 item 7 exit criterion, heterogeneous leg: ~150 *mixed* symbols
    // (adjacent neighbors are different runes) round-trip — every symbol
    // found, none merged or eaten — and a draw-order shuffle produces the
    // *identical* interpretation (same runes at the same places with the
    // same potencies), not merely the same count.
    let data = GameData::load().unwrap();
    let runes = rank_one(&data);
    let (symbols, expected) =
        mixed_jittered_rune_grid(&["spark", "warmth", "light"], 150, 0.052, 0.020, 42_000);
    assert!(expected.len() >= 150, "grid only placed {}", expected.len());

    let mut strokes = outer_circle();
    strokes.extend(symbols.clone());
    let interpretation = interpret_diagram(&strokes, runes.iter().copied());

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert_eq!(
        interpretation.runes.len(),
        expected.len(),
        "found {} of {} mixed symbols",
        interpretation.runes.len(),
        expected.len()
    );
    let mut found: Vec<&str> = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect();
    let mut wanted = expected.clone();
    found.sort_unstable();
    wanted.sort_unstable();
    assert_eq!(found, wanted, "some symbols were misread");

    // Full-interpretation order independence: reverse every symbol stroke
    // (keep the circle first so the same working circle is found) and demand
    // the same reading rune-for-rune, not just the same count.
    let mut shuffled = outer_circle();
    shuffled.extend(symbols.iter().rev().cloned());
    let reshuffled = interpret_diagram(&shuffled, runes.iter().copied());

    let key = |rune: &InterpretedRune| {
        (
            rune.rune_id.clone(),
            (rune.center.x * 1000.0).round() as i32,
            (rune.center.y * 1000.0).round() as i32,
        )
    };
    // `quality` is deliberately absent from this comparison: identity,
    // placement, scale and potency must be order-independent, but quality
    // *rewards* canonical stroke order by design (prd.md §3), so reversing
    // the strokes legitimately lowers it.
    let mut original: Vec<_> = interpretation
        .runes
        .iter()
        .map(|rune| (key(rune), rune.scale, rune.potency))
        .collect();
    let mut swapped: Vec<_> = reshuffled
        .runes
        .iter()
        .map(|rune| (key(rune), rune.scale, rune.potency))
        .collect();
    original.sort_by(|a, b| a.0.cmp(&b.0));
    swapped.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        original.len(),
        swapped.len(),
        "shuffle changed the rune count"
    );
    for (a, b) in original.iter().zip(&swapped) {
        assert_eq!(a.0, b.0, "shuffle moved or renamed a rune");
        assert!(
            (a.1 - b.1).abs() < 1e-4 && (a.2 - b.2).abs() < 1e-4,
            "shuffle changed a rune's numbers: {a:?} vs {b:?}"
        );
    }
}

/// Walks a grid inside the working circle drawn by `outer_circle()`, spaced far enough apart
/// (relative to the symbols' own drawn size) to stay distinct clusters — see
/// `rune_diagram::geometry::cluster_thresholds`. `place` draws one instance at each grid spot
/// and returns whether the target count has been reached.
fn walk_rune_grid(spacing: f32, mut place: impl FnMut(f32, f32) -> bool) {
    let steps = (0.80 / spacing).ceil() as i32 + 1;
    for row in -steps..=steps {
        for col in -steps..=steps {
            let x = 0.50 + col as f32 * spacing;
            let y = 0.50 + row as f32 * spacing;
            let nx = (x - 0.50) / 0.40;
            let ny = (y - 0.50) / 0.38;
            if nx * nx + ny * ny > 1.0 {
                continue;
            }
            if place(x, y) {
                return;
            }
        }
    }
}

/// Places up to `count_target` clean instances of `rune_id` on the grid.
fn small_rune_grid(
    rune_id: &str,
    count_target: usize,
    spacing: f32,
    scale: f32,
) -> Vec<DrawnStroke> {
    let mut strokes = Vec::new();
    walk_rune_grid(spacing, |x, y| {
        strokes.extend(template_at(rune_id, x, y, scale));
        strokes.len() >= count_target
    });
    strokes
}

/// Like `small_rune_grid`, but every placed instance is run through `perturb` with a jitter
/// amplitude scaled to that instance's own on-canvas `scale` (unlike `confusion_gate.rs`'s
/// `jitter_a`, which perturbs a template at its own natural, unscaled size — a fixed absolute
/// amplitude would be proportionally huge on the small instances this sweep uses at its larger
/// sizes) so a size sweep exercises jittered ink, not clean templates — Phase 6 item 1. Counts
/// *instances*, not raw strokes, so it stays accurate for multi-stroke runes too.
fn jittered_rune_grid(
    rune_id: &str,
    count_target: usize,
    spacing: f32,
    scale: f32,
    seed: u64,
) -> Vec<DrawnStroke> {
    let mut strokes = Vec::new();
    let mut placed = 0usize;
    walk_rune_grid(spacing, |x, y| {
        let instance = template_at(rune_id, x, y, scale);
        strokes.extend(perturb(
            &instance,
            1.0,
            (0.0, 0.0),
            scale * 0.03,
            seed + placed as u64,
        ));
        placed += 1;
        placed >= count_target
    });
    strokes
}

/// Like `jittered_rune_grid`, but cycles through several rune ids so adjacent
/// symbols are *different* runes — merge/eat regressions between unlike
/// neighbors don't show up on a single-rune grid.
fn mixed_jittered_rune_grid(
    rune_ids: &[&'static str],
    count_target: usize,
    spacing: f32,
    scale: f32,
    seed: u64,
) -> (Vec<DrawnStroke>, Vec<&'static str>) {
    let mut strokes = Vec::new();
    let mut expected: Vec<&'static str> = Vec::new();
    walk_rune_grid(spacing, |x, y| {
        let rune_id = rune_ids[expected.len() % rune_ids.len()];
        // Multi-stroke runes carry fine detail (2-point rays) that
        // physically cannot survive at the smallest sizes single-stroke
        // flicks can — draw those a bit larger, like a real hand would.
        let instance_scale = if rune_id == "light" {
            scale * 1.7
        } else {
            scale
        };
        let instance = template_at(rune_id, x, y, instance_scale);
        strokes.extend(perturb(
            &instance,
            1.0,
            (0.0, 0.0),
            scale * 0.03,
            seed + expected.len() as u64,
        ));
        expected.push(rune_id);
        expected.len() >= count_target
    });
    (strokes, expected)
}
