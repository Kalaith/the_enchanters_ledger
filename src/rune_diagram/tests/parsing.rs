//! What the parser promises about its *output*, as opposed to which runes it
//! finds (`recognition`) or how well it scores them (`quality`).
//!
//! Four things, all of which the rest of the game reads without re-checking:
//! the shape and ordering of a successful interpretation, what malformed ink
//! does to it, where the working circle's inside ends, and what comes back when
//! there is no readable diagram at all. Every one of these is a case a save file
//! or a shaky hand can produce, and none of them may panic.

use super::fixtures::*;
use crate::data::GameData;
use crate::rune_diagram::{
    interpret_diagram, is_inside_working_circle, StrokeBounds, MIN_DIAGRAM_RUNE_CONFIDENCE,
};
use crate::rune_drawing::{DrawnStroke, StrokePoint};

// ---------------------------------------------------------------- interpretation

/// `interpret_diagram_in_context` sorts by centre y then x before returning, and
/// `crate::reading` and the ledger row both present runes in that order. An
/// unsorted result would reorder the read-back sentence for no reason the player
/// drew.
#[test]
fn read_runes_come_back_top_to_bottom_then_left_to_right() {
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("continuous", 0.62, 0.66, 0.15));
    strokes.extend(template_at("light", 0.36, 0.34, 0.15));
    strokes.extend(template_at("sphere", 0.50, 0.50, 0.16));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));

    assert!(interpretation.runes.len() >= 2, "{interpretation:?}");
    for pair in interpretation.runes.windows(2) {
        let (first, second) = (&pair[0], &pair[1]);
        assert!(
            first.center.y < second.center.y
                || (first.center.y == second.center.y && first.center.x <= second.center.x),
            "out of order: {first:?} before {second:?}"
        );
    }
}

/// Every consumer downstream — potency tags, recipe matching, the containment
/// budget — does arithmetic on these fields without guarding them. A NaN or an
/// out-of-band value would propagate silently into a grade.
#[test]
fn every_read_rune_reports_values_inside_their_documented_bands() {
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.32, 0.36, 0.16));
    strokes.extend(template_at("sphere", 0.52, 0.52, 0.18));
    strokes.extend(template_at("safer", 0.68, 0.66, 0.10));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));

    assert!(!interpretation.runes.is_empty(), "{interpretation:?}");
    assert!(
        (0.0..=1.0).contains(&interpretation.circle_quality),
        "{interpretation:?}"
    );
    for rune in &interpretation.runes {
        assert!(
            (MIN_DIAGRAM_RUNE_CONFIDENCE..=1.0).contains(&rune.confidence),
            "confidence out of band: {rune:?}"
        );
        assert!((0.0..=1.0).contains(&rune.quality), "{rune:?}");
        // `recognition::potency_for_rune` clamps to exactly this range.
        assert!((0.35..=2.2).contains(&rune.potency), "{rune:?}");
        assert!(rune.scale > 0.0 && rune.scale.is_finite(), "{rune:?}");
        assert!(
            rune.center.x.is_finite() && rune.center.y.is_finite(),
            "{rune:?}"
        );
    }
}

// -------------------------------------------------------------- malformed glyphs

/// A tap that never became a drag, and a stroke whose points all landed inside
/// `DrawnStroke::push`'s dedup radius, are both real things the slate produces.
/// `has_ink` is meant to drop them before anything scores them, so adding them
/// to a diagram must not move a single number.
#[test]
fn inkless_strokes_change_nothing_about_the_reading() {
    let data = GameData::load().unwrap();
    let mut clean = outer_circle();
    clean.extend(template_at("light", 0.42, 0.46, 0.18));

    let mut littered = clean.clone();
    littered.push(DrawnStroke { points: Vec::new() });
    littered.push(DrawnStroke::new(StrokePoint::new(0.30, 0.30)));
    littered.push(DrawnStroke {
        points: vec![StrokePoint::new(0.61, 0.61)],
    });

    assert_eq!(
        interpret_diagram(&clean, rank_one(&data)),
        interpret_diagram(&littered, rank_one(&data)),
    );
}

/// A stroke with two or more *identical* points passes `has_ink` but has zero
/// extent, so every span, ratio and normalization in the recognizer divides by
/// something that started at zero. It must survive to a rejection, not a NaN.
#[test]
fn a_zero_extent_stroke_is_refused_rather_than_dividing_by_zero() {
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.34, 0.40, 0.16));
    strokes.push(DrawnStroke {
        points: vec![StrokePoint::new(0.62, 0.58); 6],
    });

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(
        interpretation.circle_quality.is_finite(),
        "{interpretation:?}"
    );
    assert!(ids.contains(&"light"), "{ids:?}");
    for rune in &interpretation.runes {
        assert!(rune.potency.is_finite(), "{rune:?}");
        assert!(rune.quality.is_finite(), "{rune:?}");
        assert!(rune.confidence.is_finite(), "{rune:?}");
    }
}

/// Coordinates outside the unit slate cannot be drawn, but a hand-edited or
/// version-skewed save can hold them. The parser has to keep reading the marks
/// it does understand instead of letting the outlier drag the working circle's
/// bounds off the board.
#[test]
fn ink_far_off_the_slate_does_not_swallow_the_working_circle() {
    let data = GameData::load().unwrap();
    let mut strokes = outer_circle();
    strokes.extend(template_at("light", 0.42, 0.46, 0.18));
    strokes.push(stroke_at(&[(0.0, 0.0), (1.0, 1.0)], -14.0, 22.0, 3.0));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    assert!(interpretation.accepted(), "{interpretation:?}");
    assert!(ids.contains(&"light"), "{ids:?}");
}

// ----------------------------------------------------------- enclosure detection

/// The two guards in `is_inside_working_circle` that are not the radius test:
/// a mark is inside only if it also fits comfortably *within* the circle, so a
/// mark the size of the working circle is a peer, not a contained rune.
#[test]
fn a_mark_as_wide_as_the_working_circle_is_not_inside_it() {
    let circle = StrokeBounds {
        min_x: 0.10,
        max_x: 0.90,
        min_y: 0.10,
        max_y: 0.90,
    };
    let concentric = |half: f32| DrawnStroke {
        points: vec![
            StrokePoint::new(0.50 - half, 0.50 - half),
            StrokePoint::new(0.50 + half, 0.50 + half),
        ],
    };

    assert!(is_inside_working_circle(&concentric(0.20), circle));
    // 0.92 of the circle's 0.80 width is 0.736; a mark wider than that is out.
    assert!(!is_inside_working_circle(&concentric(0.39), circle));
}

/// The radius test is deliberately generous (1.25 normalized, not 1.0) so a
/// rune drawn touching the rim still belongs to the working. Well past that,
/// though, ink outside the circle is not part of the diagram.
#[test]
fn ink_beyond_the_rim_is_not_enclosed() {
    let circle = StrokeBounds {
        min_x: 0.10,
        max_x: 0.90,
        min_y: 0.10,
        max_y: 0.90,
    };
    let mark_at = |cx: f32| DrawnStroke {
        points: vec![
            StrokePoint::new(cx - 0.02, 0.48),
            StrokePoint::new(cx + 0.02, 0.52),
        ],
    };

    assert!(is_inside_working_circle(&mark_at(0.88), circle));
    assert!(!is_inside_working_circle(&mark_at(1.10), circle));
}

/// End to end: a rune drawn well clear of the circle is not read into it, even
/// though on its own it recognizes perfectly.
#[test]
fn a_rune_drawn_outside_the_circle_is_not_read_into_the_working() {
    let data = GameData::load().unwrap();
    let mut strokes = rough_circle(0.28, 0.50, 0.20, 0.19, 32);
    strokes.extend(template_at("sphere", 0.28, 0.50, 0.14));
    strokes.extend(template_at("light", 0.88, 0.14, 0.10));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));
    let ids = interpretation
        .runes
        .iter()
        .map(|rune| rune.rune_id.as_str())
        .collect::<Vec<_>>();

    // The inner `sphere` proves the working is live and being read; the outer
    // `light` is refused for where it sits, not because nothing was recognized.
    assert!(interpretation.circle_found, "{interpretation:?}");
    assert!(ids.contains(&"sphere"), "{ids:?}");
    assert!(!ids.contains(&"light"), "{ids:?}");
}

// ------------------------------------------------------ invalid-diagram recovery

/// No ink at all is the state the slate starts in, and `Interpret` is reachable
/// from it through a save/load. The answer is the empty interpretation, in full
/// — not a partially-filled struct some later field reads as meaningful.
#[test]
fn an_empty_drawing_reads_as_the_empty_interpretation() {
    let data = GameData::load().unwrap();

    let interpretation = interpret_diagram(&[], rank_one(&data));

    assert!(!interpretation.circle_found);
    assert!(!interpretation.accepted());
    assert_eq!(interpretation.circle_quality, 0.0);
    assert!(interpretation.runes.is_empty());
    assert_eq!(interpretation.rejected_marks, 0);
    assert!(interpretation.spell.is_none());
    assert!(interpretation.scope_spell.is_none());
    assert_eq!(interpretation.average_rune_quality(), 0.0);
    assert_eq!(interpretation.average_rune_potency(), 0.0);
}

/// A circle and nothing else is the most common half-finished diagram there is.
/// It is not acceptable, but it *is* a found circle — the drafting panel tells
/// the player their working is fine and the marks are missing, which it can only
/// do if these two facts stay separate.
#[test]
fn a_bare_working_circle_is_found_but_not_accepted() {
    let data = GameData::load().unwrap();

    let interpretation = interpret_diagram(&outer_circle(), rank_one(&data));

    assert!(interpretation.circle_found, "{interpretation:?}");
    assert!(!interpretation.accepted(), "{interpretation:?}");
    assert!(interpretation.runes.is_empty(), "{interpretation:?}");
    // A circle was found, so the scope tree exists and is simply empty of runes.
    let scope = interpretation.scope_spell.as_ref().expect("scope tree");
    assert!(scope.effects.is_empty(), "{scope:?}");
    assert_eq!(scope.total_potency("light"), 0.0);
}

/// Marks with no circle around them are the other half-finished diagram. The
/// runes are legible, but without a working they mean nothing, so none of them
/// are reported — and nothing downstream gets a spell to grade.
#[test]
fn marks_without_a_working_circle_report_no_runes_and_no_spell() {
    let data = GameData::load().unwrap();
    let mut strokes = template_at("light", 0.30, 0.34, 0.16);
    strokes.extend(template_at("sphere", 0.62, 0.60, 0.16));

    let interpretation = interpret_diagram(&strokes, rank_one(&data));

    assert!(!interpretation.circle_found, "{interpretation:?}");
    assert!(interpretation.runes.is_empty(), "{interpretation:?}");
    assert!(interpretation.spell.is_none(), "{interpretation:?}");
    assert!(interpretation.scope_spell.is_none(), "{interpretation:?}");
}
