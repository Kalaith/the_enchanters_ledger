use super::*;
use crate::data::GameData;
use crate::rune_drawing::samples;
use crate::rune_drawing::template_strokes_for_rune;
use crate::rune_drawing::test_support::perturb;

fn rank_one(data: &GameData) -> Vec<&RuneDef> {
    data.runes.iter().filter(|rune| rune.tier == 1).collect()
}

fn all_runes(data: &GameData) -> Vec<&RuneDef> {
    data.runes.iter().collect()
}

#[test]
fn structural_rune_sample_set_reads_inside_full_diagrams() {
    let data = GameData::load().unwrap();

    for sample in samples::structural_rune_samples() {
        let strokes = samples::circled_sample(&sample, 0.50, 0.50, 0.17);
        let interpretation = interpret_diagram(&strokes, all_runes(&data));
        let ids = interpretation
            .runes
            .iter()
            .map(|rune| rune.rune_id.as_str())
            .collect::<Vec<_>>();

        assert!(
            interpretation.accepted(),
            "sample={} interpretation={interpretation:?}",
            sample.name
        );
        assert!(
            ids.contains(&sample.rune_id),
            "sample={} ids={ids:?} interpretation={interpretation:?}",
            sample.name
        );
    }
}

#[test]
fn rejects_diagram_without_outer_circle() {
    let data = GameData::load().unwrap();
    let strokes = template_at("light", 0.5, 0.5, 0.22);

    let interpretation = interpret_diagram(&strokes, rank_one(&data));

    assert!(!interpretation.circle_found);
    assert!(!interpretation.accepted());
}

#[test]
fn interprets_multiple_runes_inside_enclosing_circle() {
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.26, 0.50, 0.18));
    strokes.extend(template_at("sphere", 0.50, 0.50, 0.18));
    strokes.extend(template_at("continuous", 0.74, 0.50, 0.18));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"light"), "{ids:?}");
    assert!(ids.contains(&"sphere"), "{ids:?}");
    assert!(ids.contains(&"continuous"), "{ids:?}");
}

#[test]
fn overlapped_sphere_still_reads_inside_working_circle() {
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.50, 0.50, 0.18));
    strokes.push(rough_sphere(0.50, 0.50, 0.20));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"light"), "{ids:?}");
    assert!(ids.contains(&"sphere"), "{ids:?}");
}

#[test]
fn lone_large_centered_inner_circle_reads_as_sphere() {
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.26, 0.50, 0.18));
    strokes.extend(rough_circle(0.50, 0.50, 0.17, 0.16, 20));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"sphere"), "{ids:?}");
}

#[test]
fn screenshot_clear_light_and_sphere_read_together() {
    let data = GameData::load().unwrap();
    let mut strokes = rough_circle(0.30, 0.38, 0.25, 0.27, 28);
    strokes.push(flat_rough_circle(0.20, 0.28, 0.12));
    strokes.extend(template_at("light", 0.31, 0.36, 0.18));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"light"), "{ids:?}");
    assert!(ids.contains(&"sphere"), "{ids:?}");
}

#[test]
fn damaged_sphere_fragment_does_not_make_neighboring_light_fail() {
    let data = GameData::load().unwrap();
    let mut strokes = rough_circle(0.445, 0.51, 0.205, 0.44, 36);
    strokes.extend(template_at("light", 0.36, 0.36, 0.16));
    strokes.push(stroke_at(
        &[(0.16, 0.62), (0.22, 0.30), (0.50, 0.14)],
        0.46,
        0.45,
        0.19,
    ));
    strokes.extend(template_at("continuous", 0.36, 0.66, 0.16));
    strokes.push(stroke_at(
        &[(0.60, 0.12), (0.36, 0.46), (0.56, 0.46), (0.38, 0.88)],
        0.485,
        0.42,
        0.09,
    ));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(ids.contains(&"light"), "{ids:?}");
    assert!(ids.contains(&"continuous"), "{ids:?}");
    assert!(!ids.contains(&"sphere"), "{ids:?}");
    assert!(!ids.contains(&"spark"), "{ids:?}");
}

#[test]
fn damaged_sphere_fragment_stays_out_of_neighboring_light_cluster() {
    let mut indexed_strokes = template_at("light", 0.36, 0.36, 0.16)
        .into_iter()
        .enumerate()
        .map(|(index, stroke)| (index + 1, stroke))
        .collect::<Vec<_>>();
    indexed_strokes.push((
        5,
        stroke_at(
            &[(0.16, 0.62), (0.22, 0.30), (0.50, 0.14)],
            0.46,
            0.45,
            0.19,
        ),
    ));

    let clusters = cluster_strokes(&indexed_strokes);
    let light_cluster = clusters
        .iter()
        .find(|cluster| cluster.indices.contains(&1))
        .unwrap();

    assert!(light_cluster.indices.contains(&4), "{clusters:?}");
    assert!(!light_cluster.indices.contains(&5), "{clusters:?}");
    assert!(
        clusters.iter().any(|cluster| cluster.indices == vec![5]),
        "{clusters:?}"
    );
}

#[test]
fn accepts_smaller_off_center_outer_circle() {
    let data = GameData::load().unwrap();
    let mut strokes = rough_circle(0.34, 0.42, 0.23, 0.19, 24);
    strokes.extend(template_at("light", 0.34, 0.42, 0.12));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"light"), "{ids:?}");
}

#[test]
fn rejects_simple_cross_inside_circle() {
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.push(stroke_at(&[(0.50, 0.18), (0.50, 0.82)], 0.50, 0.50, 0.20));
    strokes.push(stroke_at(&[(0.26, 0.50), (0.74, 0.50)], 0.50, 0.50, 0.20));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));

    assert!(!interpretation.accepted(), "{interpretation:?}");
    assert!(interpretation.runes.is_empty(), "{interpretation:?}");
    assert_eq!(interpretation.rejected_marks, 1);
}

#[test]
fn rune_quality_does_not_depend_on_board_position() {
    // D1: layout_quality's undocumented home-position bias was deleted for
    // freehand scoring (plan Phase 2 item 4) — a rune's *shape* quality
    // should depend on how well it was drawn, not where in the circle it
    // landed. Position bonuses/penalties belong to the future spell
    // grammar (Phase 4), where placement can mean something taught to the
    // player, not a silent recognition-quality tax.
    let data = GameData::load().unwrap();
    let centered = touch_quality_at(0.50, 0.50, &data);
    let off_center = touch_quality_at(0.26, 0.50, &data);

    assert!(
        (centered - off_center).abs() < 0.01,
        "centered={centered} off_center={off_center}"
    );
}

#[test]
fn doubling_effect_rune_scale_raises_potency() {
    // Phase 2 exit criterion: drawing the same effect rune at 2x size
    // measurably raises potency (and, in evaluate(), power). "light" is an
    // Effect rune; its ideal_scale_in_circle is 0.18, so scale 0.18 is
    // ~1.0x reference size and 0.36 is ~2x.
    let data = GameData::load().unwrap();
    let normal = potency_of("light", 0.18, &data);
    let doubled = potency_of("light", 0.36, &data);

    assert!(doubled > normal * 1.3, "normal={normal} doubled={doubled}");
}

#[test]
fn under_drawn_stroke_reads_but_reports_lower_potency() {
    // Phase 2 exit criterion: a half-hearted, short-stroked rune still
    // reads as itself (identity survives) but reports reduced potency
    // (magnitude does not) — this is the ink_ratio channel.
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.5, 0.5, 0.18));

    let full = interpret_diagram(&strokes, rank_one(&data));
    let full_rune = full
        .runes
        .iter()
        .find(|rune| rune.rune_id == "light")
        .unwrap();

    let mut short_strokes = outer_circle();
    short_strokes.extend(shortened_template_at("light", 0.5, 0.5, 0.18, 0.8));
    let short = interpret_diagram(&short_strokes, rank_one(&data));
    let short_rune = short
        .runes
        .iter()
        .find(|rune| rune.rune_id == "light")
        .unwrap();

    assert!(short.accepted(), "{short:?}");
    assert!(
        short_rune.potency < full_rune.potency * 0.9,
        "full={full_rune:?} short={short_rune:?}"
    );
}

fn potency_of(rune_id: &str, scale: f32, data: &GameData) -> f32 {
    let mut strokes = outer_circle();
    strokes.extend(template_at(rune_id, 0.5, 0.5, scale));
    let interpretation = interpret_diagram(&strokes, rank_one(data));
    interpretation
        .runes
        .iter()
        .find(|rune| rune.rune_id == rune_id)
        .map(|rune| rune.potency)
        .unwrap_or(0.0)
}

/// Every stroke of the template, cut short to `fraction` of its own length
/// (same start point, pulled-in end point) — simulates a hand that lifted
/// the pen early.
fn shortened_template_at(
    rune_id: &str,
    cx: f32,
    cy: f32,
    scale: f32,
    fraction: f32,
) -> Vec<DrawnStroke> {
    template_at(rune_id, cx, cy, scale)
        .into_iter()
        .map(|stroke| {
            let start = stroke.points[0];
            let end = *stroke.points.last().unwrap();
            DrawnStroke {
                points: vec![
                    start,
                    StrokePoint::new(
                        start.x + (end.x - start.x) * fraction,
                        start.y + (end.y - start.y) * fraction,
                    ),
                ],
            }
        })
        .collect()
}

#[test]
fn interprets_high_tier_structured_circle_spell() {
    let data = GameData::load().unwrap();
    let interpretation = interpret_diagram(&high_tier_city_circle(), all_runes(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();
    let spell = interpretation.spell.as_ref().unwrap();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"gravity"), "{ids:?}");
    assert!(ids.contains(&"sphere"), "{ids:?}");
    assert!(ids.contains(&"continuous"), "{ids:?}");
    assert_eq!(spell.dominant_effect.as_deref(), Some("gravity"));
    assert_eq!(spell.tier_rank, 4, "{spell:?}");
    assert!(spell.complexity >= 0.72, "{spell:?}");
    assert!(spell.ring_count >= 4, "{spell:?}");
    assert!(spell.satellite_count >= 6, "{spell:?}");
    assert!(spell.perimeter_mark_count >= 32, "{spell:?}");
    assert!(spell.script_mark_count >= 32, "{spell:?}");
}

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

/// Places up to `count_target` instances of `rune_id` on a grid inside the working circle drawn
/// by `outer_circle()`, spaced far enough apart (relative to their own drawn size) to stay
/// distinct clusters — see `rune_diagram::geometry::cluster_thresholds`.
fn small_rune_grid(
    rune_id: &str,
    count_target: usize,
    spacing: f32,
    scale: f32,
) -> Vec<DrawnStroke> {
    let mut strokes = Vec::new();
    let steps = (0.80 / spacing).ceil() as i32 + 1;
    'grid: for row in -steps..=steps {
        for col in -steps..=steps {
            let x = 0.50 + col as f32 * spacing;
            let y = 0.50 + row as f32 * spacing;
            let nx = (x - 0.50) / 0.40;
            let ny = (y - 0.50) / 0.38;
            if nx * nx + ny * ny > 1.0 {
                continue;
            }
            strokes.extend(template_at(rune_id, x, y, scale));
            if strokes.len() >= count_target {
                break 'grid;
            }
        }
    }
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
    let steps = (0.80 / spacing).ceil() as i32 + 1;
    'grid: for row in -steps..=steps {
        for col in -steps..=steps {
            let x = 0.50 + col as f32 * spacing;
            let y = 0.50 + row as f32 * spacing;
            let nx = (x - 0.50) / 0.40;
            let ny = (y - 0.50) / 0.38;
            if nx * nx + ny * ny > 1.0 {
                continue;
            }
            let instance = template_at(rune_id, x, y, scale);
            let jitter_amp = scale * 0.03;
            strokes.extend(perturb(
                &instance,
                1.0,
                (0.0, 0.0),
                jitter_amp,
                seed + placed as u64,
            ));
            placed += 1;
            if placed >= count_target {
                break 'grid;
            }
        }
    }
    strokes
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
fn fireball_recipe_recognized_purely_from_data() {
    // Plan Phase 4 exit criterion: "fireball ... defined purely in data" — one scope, no
    // sub-scopes, matched against `assets/data/recipes.json` with no rune-id-specific Rust
    // involved in the naming decision.
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("fire", 0.50, 0.30, 0.20));
    strokes.extend(template_at("sphere", 0.28, 0.62, 0.18));
    strokes.extend(template_at("on_command", 0.72, 0.62, 0.16));

    let interpretation = interpret_diagram(&strokes, all_runes(&data));
    assert!(interpretation.accepted(), "{interpretation:?}");
    let tree = interpretation.scope_spell.as_ref().expect("scope_spell");
    let matched = crate::recipes::match_recipe(tree, &data.recipes).unwrap_or_else(|| {
        panic!(
            "no recipe matched: tree={tree:?} runes={:?}",
            interpretation.runes
        )
    });

    assert_eq!(matched.id, "fireball", "{tree:?}");
}

#[test]
fn volcano_recipe_recognized_purely_from_data() {
    // Plan Phase 4 exit criterion: "volcano ... defined purely in data" — a root scope
    // (fire + cone + continuous + rings + satellites) enclosing two off-center vent sub-scopes
    // (each its own force + fire), matched against `assets/data/recipes.json` with no
    // rune-id-specific Rust involved. Also exercises `ScopeSpell::total_potency` summing across
    // the tree, since `volcano`'s `min_potency` is only reached by root + vent fire combined.
    let data = GameData::load().unwrap();
    let strokes = volcano_diagram();

    let interpretation = interpret_diagram(&strokes, all_runes(&data));
    assert!(interpretation.accepted(), "{interpretation:?}");
    let tree = interpretation.scope_spell.as_ref().expect("scope_spell");
    assert_eq!(
        tree.sub_scopes.len(),
        2,
        "{tree:?} runes={:?}",
        interpretation.runes
    );

    let matched = crate::recipes::match_recipe(tree, &data.recipes).unwrap_or_else(|| {
        panic!(
            "no recipe matched: tree={tree:?} runes={:?}",
            interpretation.runes
        )
    });
    assert_eq!(matched.id, "volcano", "{tree:?}");
}

fn volcano_diagram() -> Vec<DrawnStroke> {
    let mut strokes = outer_circle();
    strokes.extend(template_at("fire", 0.50, 0.24, 0.28));
    strokes.extend(template_at("continuous", 0.18, 0.50, 0.16));
    strokes.extend(template_at("cone", 0.50, 0.86, 0.16));
    // Concentric reinforcement rings, near the center — proven safe from being reinterpreted
    // as their own sub-scopes (orbit ~0, below `NESTED_RING_MIN_ORBIT`).
    strokes.extend(rough_circle(0.50, 0.50, 0.14, 0.133, 32));
    strokes.extend(rough_circle(0.50, 0.50, 0.125, 0.119, 28));
    // 3 satellite seals, clustered on the right so they stay clear of the rings (orbit floor)
    // and the two vents (below).
    strokes.extend(rough_circle(0.74, 0.50, 0.045, 0.0414, 16));
    strokes.extend(rough_circle(0.7175, 0.6015, 0.045, 0.0414, 16));
    strokes.extend(rough_circle(0.7175, 0.3985, 0.045, 0.0414, 16));
    strokes.extend(vent(0.22, 0.78));
    strokes.extend(vent(0.78, 0.78));
    strokes
}

/// One volcano vent: a ring enclosing its own `force` and `fire` runes, offset from each other
/// so they stay separate clusters.
fn vent(cx: f32, cy: f32) -> Vec<DrawnStroke> {
    let mut strokes = rough_circle(cx, cy, 0.125, 0.119, 32);
    strokes.extend(template_at("force", cx - 0.07, cy, 0.17));
    strokes.extend(template_at("fire", cx + 0.07, cy, 0.17));
    strokes
}

#[test]
fn rune_far_below_old_twelve_percent_scale_floor_still_reads() {
    // Phase 3 item 4 exit criterion (C1/C2): a rune drawn at ~4% of the working circle's scale
    // — well under the old, now-removed `MIN_RUNE_SCALE_IN_CIRCLE` (0.12) floor — must still be
    // found. This is the scale a 100+-symbol grand diagram needs most of its runes drawn at.
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("spark", 0.30, 0.30, 0.035));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"spark"), "{ids:?}");
}

#[test]
fn nested_off_center_ring_reads_its_own_effect_rune_relative_to_its_own_scope() {
    // Phase 3 item 2 exit criterion: an off-center sub-circle (a "vent" in a volcano-style
    // diagram) enclosing its own ink is interpreted as its own scope, so the rune inside it is
    // scored relative to the *sub-circle*, not the outer working circle. Drawn at 0.16 slate
    // units, "light" reads at roughly reference size (scale ~0.19, ratio ~1.0) against the huge
    // outer circle but oversized (scale ~0.5+, ratio ~2.9) against the much smaller sub-circle —
    // so a large reported scale/potency here is direct evidence recursion, not the outer circle,
    // supplied the reference frame.
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(rough_circle(0.30, 0.30, 0.15, 0.14, 24));
    strokes.extend(template_at("light", 0.30, 0.30, 0.16));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let light = interpretation
        .runes
        .iter()
        .find(|rune| rune.rune_id == "light");

    assert!(interpretation.accepted(), "{interpretation:?}");
    let light = light.unwrap_or_else(|| panic!("{interpretation:?}"));
    assert!(light.scale > 0.35, "{light:?}");
    assert!(light.potency > 1.5, "{light:?}");
}

fn touch_quality_at(x: f32, y: f32, data: &GameData) -> f32 {
    let mut strokes = outer_circle();
    strokes.extend(template_at("touch", x, y, 0.18));
    let interpretation = interpret_diagram(&strokes, rank_one(data));
    interpretation
        .runes
        .iter()
        .find(|rune| rune.rune_id == "touch")
        .map(|rune| rune.quality)
        .unwrap_or(0.0)
}

fn outer_circle() -> Vec<DrawnStroke> {
    rough_circle(0.50, 0.50, 0.42, 0.40, 36)
}

fn high_tier_city_circle() -> Vec<DrawnStroke> {
    let mut strokes = outer_circle();
    strokes.extend(rough_circle(0.50, 0.50, 0.36, 0.34, 48));
    strokes.extend(rough_circle(0.50, 0.50, 0.30, 0.29, 44));
    strokes.extend(rough_circle(0.50, 0.50, 0.23, 0.22, 38));
    strokes.extend(rough_circle(0.50, 0.50, 0.16, 0.15, 32));
    strokes.extend(template_at("gravity", 0.28, 0.48, 0.22));
    strokes.extend(template_at("sphere", 0.50, 0.50, 0.18));
    strokes.extend(template_at("continuous", 0.73, 0.48, 0.17));
    strokes.extend(template_at("safer", 0.50, 0.73, 0.15));
    strokes.extend(satellite_seals(8, 0.30, 0.038));
    strokes.extend(radial_spokes(8, 0.31));
    strokes.extend(perimeter_ticks(36, 0.39, 0.016));
    strokes.extend(perimeter_ticks(24, 0.34, 0.012));
    strokes.extend(script_marks(24, 0.20, 0.010));
    strokes.extend(script_marks(32, 0.27, 0.008));
    strokes
}

fn rough_circle(cx: f32, cy: f32, rx: f32, ry: f32, steps: usize) -> Vec<DrawnStroke> {
    let mut points = Vec::new();
    for index in 0..=steps {
        let angle = std::f32::consts::TAU * index as f32 / steps as f32;
        let wobble = if index % 5 == 0 { 0.015 } else { 0.0 };
        points.push(StrokePoint::new(
            cx + (rx + wobble) * angle.cos(),
            cy + (ry - wobble * 0.5) * angle.sin(),
        ));
    }
    vec![DrawnStroke { points }]
}

fn rough_sphere(cx: f32, cy: f32, scale: f32) -> DrawnStroke {
    stroke_at(
        &[
            (0.50, 0.14),
            (0.76, 0.24),
            (0.86, 0.52),
            (0.68, 0.80),
            (0.38, 0.84),
            (0.16, 0.62),
            (0.22, 0.30),
            (0.50, 0.14),
        ],
        cx,
        cy,
        scale,
    )
}

fn flat_rough_circle(cx: f32, cy: f32, scale: f32) -> DrawnStroke {
    stroke_at(
        &[
            (0.30, 0.24),
            (0.58, 0.20),
            (0.76, 0.32),
            (0.80, 0.58),
            (0.66, 0.76),
            (0.38, 0.78),
            (0.20, 0.60),
            (0.18, 0.36),
            (0.30, 0.24),
        ],
        cx,
        cy,
        scale,
    )
}

fn satellite_seals(count: usize, orbit: f32, radius: f32) -> Vec<DrawnStroke> {
    (0..count)
        .flat_map(|index| {
            let angle =
                std::f32::consts::TAU * index as f32 / count as f32 + std::f32::consts::FRAC_PI_4;
            rough_circle(
                0.50 + orbit * angle.cos(),
                0.50 + orbit * angle.sin(),
                radius,
                radius * 0.92,
                16,
            )
        })
        .collect()
}

fn radial_spokes(count: usize, radius: f32) -> Vec<DrawnStroke> {
    (0..count)
        .map(|index| {
            let angle = std::f32::consts::TAU * index as f32 / count as f32;
            DrawnStroke {
                points: vec![
                    StrokePoint::new(0.50, 0.50),
                    StrokePoint::new(0.50 + radius * angle.cos(), 0.50 + radius * angle.sin()),
                ],
            }
        })
        .collect()
}

fn perimeter_ticks(count: usize, orbit: f32, half_len: f32) -> Vec<DrawnStroke> {
    (0..count)
        .map(|index| {
            let angle = std::f32::consts::TAU * index as f32 / count as f32;
            let center = StrokePoint::new(0.50 + orbit * angle.cos(), 0.50 + orbit * angle.sin());
            let tangent = angle + std::f32::consts::FRAC_PI_2;
            DrawnStroke {
                points: vec![
                    StrokePoint::new(
                        center.x - half_len * tangent.cos(),
                        center.y - half_len * tangent.sin(),
                    ),
                    StrokePoint::new(
                        center.x + half_len * tangent.cos(),
                        center.y + half_len * tangent.sin(),
                    ),
                ],
            }
        })
        .collect()
}

fn script_marks(count: usize, orbit: f32, half_len: f32) -> Vec<DrawnStroke> {
    (0..count)
        .map(|index| {
            let angle = std::f32::consts::TAU * (index as f32 + 0.35) / count as f32;
            let center = StrokePoint::new(0.50 + orbit * angle.cos(), 0.50 + orbit * angle.sin());
            let tangent = angle + std::f32::consts::FRAC_PI_2;
            let skew = if index % 2 == 0 { 0.55 } else { -0.55 };
            DrawnStroke {
                points: vec![
                    StrokePoint::new(
                        center.x - half_len * tangent.cos(),
                        center.y - half_len * tangent.sin(),
                    ),
                    StrokePoint::new(
                        center.x + half_len * tangent.cos(),
                        center.y + half_len * tangent.sin(),
                    ),
                    StrokePoint::new(
                        center.x
                            + half_len * (tangent + skew).cos()
                            + half_len * 0.35 * angle.cos(),
                        center.y
                            + half_len * (tangent + skew).sin()
                            + half_len * 0.35 * angle.sin(),
                    ),
                ],
            }
        })
        .collect()
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

fn stroke_at(points: &[(f32, f32)], cx: f32, cy: f32, scale: f32) -> DrawnStroke {
    DrawnStroke {
        points: points
            .iter()
            .map(|(x, y)| StrokePoint::new(cx + (x - 0.5) * scale, cy + (y - 0.5) * scale))
            .collect(),
    }
}
