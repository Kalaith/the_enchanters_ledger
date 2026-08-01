# The Enchanter's Ledger — Magic Symbol System Ruleset

> **Document location:** `.project/prd.md`
>
> This is the authoritative spec for the rune-drawing and magic-circle
> system: what a drawing has to look like to be read, what makes it read
> *well*, and what a full circle diagram means. It exists so tuning the
> recognizer is never "change a constant and see what happens" — every
> constant below should trace back to a sentence here, and every sentence
> here should trace back to a test in `src/rune_drawing/confusion_gate.rs`,
> `src/rune_drawing/property_tests.rs`, or `tests/corpus/`.
>
> Written as Phase 0 of `.project/magic-symbol-system-plan.md`, updated
> through Phase 6 (the plan's final phase). It documents the system **as it
> exists today**: Hungarian
> stroke assignment, open-stroke corner fix, eraser-fragment merge,
> strict/identity variant parity, deterministic tie-breaks, mismatch-segment
> alignment (D3), a continuous (not stepped) ambiguity penalty and corner
> confidence, capture-time stroke canonicalization (A8), rune templates
> moved to `assets/data/rune_templates.json`, a real size/completeness
> magnitude channel (`potency`, §4), the D1/D2 scoring-stack cleanup
> (layout-position bias deleted, circle quality now additive not stacked),
> multi-stroke circle assembly, a recursive containment hierarchy for nested
> scopes, scale-relative (not absolute) clustering with a spatial grid,
> geometry-driven (not size-driven) structure-vs-rune classification, pure
> spatial (not draw-order) stroke grouping (§5), a compositional spell tree
> with named spells as data-defined recipes (`assets/data/recipes.json`),
> diminishing-returns structural scoring, a recursive per-scope containment
> budget, cause-specific backfire messaging (§8), per-context acceptance
> bands (Practice/Commission/Sandbox), a rune-mastery history that fades
> guide aids over time, specific player-facing rejection hints, and
> story-pacing verification (§9), and — as of Phase 6 — a size-swept
> synthetic round-trip test and a CLI diagnostic replay tool for fast tuning
> loops (§10).
> It is not a design proposal — Phase 1's
> generic/data-driven *structural checks* — as opposed to template shapes,
> which are already data-driven — remain future work; see §2.3. This doc
> is also the intended source for in-game journal/tutorial copy once
> that's written.

---

## 1. Overview

### 1.1 Problem statement

The player enchants items by freehand-drawing runes inside a working circle.
The drawing has to be readable by a recognizer with no hand-authored
per-drawing hints, robust to ordinary hand-drawing noise, and — eventually —
scale from a 3-symbol fireball to a 100+-symbol grand diagram (see the plan
document's goal statement). Single-symbol recognition (§2–3), the magnitude
channel (§4), scaling a structured circle to hundreds of small symbols
(§5), a compositional grammar (named spells as data-defined predicates over
the diagram, §8), progression and aids for story mode (per-context
acceptance bands, a fading mastery/guide system, player-facing failure
hints, and story-pacing verification, §9), and continuous scale/tuning
verification (a jittered synthetic round-trip sweep and a CLI diagnostic
replay tool, §10) are all in place. What's left is a fully-balanced
containment budget (still a documented first cut, §8.3) and the deferred
items each phase above already calls out explicitly.

### 1.2 Vocabulary

| Term | Meaning |
|---|---|
| **Stroke** | One continuous pen-down-to-pen-up drag, stored as a `DrawnStroke` (`Vec<StrokePoint>`, 0..1 slate space). |
| **Rune** | A single named symbol (`RuneDef` in `assets/data/runes.json`, 33 today), drawn as 1+ strokes. |
| **Identity** | *Which* rune a drawing is recognized as. |
| **Quality** | *How well* a drawing matches its rune once identity is decided — order, start point, direction, shape fidelity. |
| **Working circle** | The single outer stroke that scopes a diagram; runes must be drawn inside it to count. |
| **Diagram** | An interpreted working circle plus every rune found inside it, structure marks, and the resulting spell. |

### 1.3 Rune categories

From `RuneCategory` (`src/data.rs`): `Effect`, `Shape`, `Trigger`,
`Modifier`. Every `RuneDef` belongs to exactly one. Categories currently
carry no recognition weight — they matter for `layout_quality`'s home-
position scoring (§4.3) and for spell-recipe requirements (Phase 4, not yet
built).

---

## 2. Identity — what a drawing tolerates and still reads as the same rune

Identity is decided by `recognize_rune()` (`src/rune_drawing.rs:134`). For
each candidate rune, the drawing is scored against every accepted stroke
layout for that rune (`template_variants_for_rune`) and the best score wins;
ties break on rune id for determinism (`src/rune_drawing.rs:154`).

### 2.1 Invariant under

- **Translation and uniform scale.** `NormalizedDrawing::from_strokes`
  (`src/rune_drawing/scoring.rs:11`) re-centers and rescales every stroke to
  its own bounding box before any comparison runs. A rune drawn small in a
  corner and one drawn large in the center score identically. Verified by
  `property_tests::translation_and_scale_do_not_change_confidence`.
- **Point density.** Arc-length resampling means a fast, sparse mouse drag
  and a slow, dense one should read the same (this is the target; §2.5
  documents where it currently falls short). Verified by
  `property_tests::point_density_does_not_change_identity`.
- **Stroke draw order, within a rune.** `match_strokes_order_insensitive`
  (`src/rune_drawing/scoring.rs:160`) finds the optimal (Hungarian, ≤12
  strokes; greedy with deterministic tie-breaks beyond that) pairing between
  drawn and template strokes — draw a rune's strokes in any order and
  identity is unaffected. Verified by
  `rune_drawing::tests::stroke_order_shuffle_keeps_identity`.
- **Small capture gaps.** `merge_continuation_strokes`
  (`src/rune_drawing.rs:203`) rejoins strokes that continue each other
  smoothly across a tiny gap (≤0.02 slate units, or 15% of the shorter
  stroke's length, whichever is smaller) with a turn of ≤~35° (`cos ≥ 0.82`)
  at the join. This is what makes a circle repaired in two arcs, or a mark
  fixed after erasing, still read as one stroke.
- **Eraser debris.** Fragments shorter than `MIN_ERASE_FRAGMENT_LENGTH`
  (0.008) left behind by an eraser stroke are dropped rather than kept as
  extra, confusing strokes (`src/rune_drawing.rs:71`).

### 2.2 Not invariant under (by design)

- **Stroke count.** A rune template has a fixed `min/max` stroke count
  (implicitly, its template's stroke count); missing or extra strokes cost
  `count_score` in `score_against_template`
  (`src/rune_drawing/scoring.rs:106`) — missing strokes cost more (×0.52
  per missing fraction) than extra ones (×0.10, or ×0.70 for single-stroke
  runes, since a lone-stroke rune with extra ink is usually a different
  shape entirely).
- **Direction is mostly free but not scored for it.** Segment similarity is
  computed pointwise after resampling in the drawn order; a rune traced
  backward matches about as well as forward because template variants
  already cover common reversals, not because direction is explicitly
  normalized away. This is an emergent property today, not a designed rule
  — flagged in the plan as B3/A3-adjacent and intended to become an
  explicit, documented factor in Phase 1.

### 2.3 Structural gates (per-rune formulas; shapes are data-driven, checks are not)

Template *shapes* are data-driven: all 33 runes' stroke layouts (plus the
`touch`/`continuous` variants) live in `assets/data/rune_templates.json`,
loaded once into a lazily-built table (`rune_drawing/templates.rs`) —
adding a rune's drawable shape is a JSON edit, no Rust. What is **not**
data-driven as of the Phase 1 completion pass is the *structural check*
layer below: a rune may declare a `structure` block in
`rune_templates.json`, evaluated by one generic checker
(`shape::structure_report`) with **zero per-rune-id branches in Rust** —
the plan's Phase 1 item 3 exit criterion. The schema:

```json
"structure": {
  "blend": 0.72,                      // score × (1 − blend + blend × structure)
  "circle_floor": 0.92,               // optional: score.max(circle_likeness × floor)
  "must_be_open": {"score": 0.18},    // optional: any closed stroke ⇒ this score + issue
  "min_strokes": 1, "max_strokes": 1, // outside ⇒ fallback_score + fallback_issue
  "fallback_score": 0.30, "fallback_issue": "closure|corners|center_bar",
  "suppressed_by": {                  // optional cross-rune disambiguation
    "rune": "sphere", "their_structure_min": 0.80,
    "own_structure_below": 0.62, "factor": 0.42 },
  "checks": [                         // weighted generic feature checks, in
    ...                               // issue-priority order
  ]
}
```

Check features: `closure`, `roundness`, `corners` (`target`/`tolerance`),
`corner_penalty` (`above`/`per_corner`), `straightness`, `directness`,
`arrow_right`, `arrow_down`, `center_bar`, `ray_angles`, `ray_center`,
`stroke_count` (`target`), `stroke_count_band` (`min`/`max`/
`out_of_band_score`). Each check may carry `issue_below` (and for corners
`issue_below_count`/`issue_above_count`) — the first triggered check in
declaration order supplies the player-facing feedback message, generated
generically from the feature kind, the rune's display name and the corner
target ("Safer needs six clear sides."). Current declarations mirror the
former hand-written checks exactly: `sphere`, `safer`, `force`, `touch`,
`beam`, `aura`, `burst`, `cone`. Adding rune #34 with structural character
is now a JSON edit. A rune's known-confusable partners also live in the
data (`"confusable_with": ["sphere"]` on `safer`) and feed the
confusion-matrix gate's tracked-confusion allowlist via
`templates::known_confusions()`.

The eight structural-check functions the table in earlier revisions of
this document described (`sphere_report` … `touch_report`) are deleted;
their weights and thresholds moved verbatim into the JSON blocks above, so
scoring behavior is unchanged.

Corner counting (`corner_count`, `shape.rs`) resamples a stroke to 36
arc-length points and computes a turn angle between the sample 2 steps
before and 2 steps after each point. As of Phase 1, `corner_count` returns a
**continuous** value, not an integer: each turn angle is passed through a
logistic ramp (`corner_confidence`, slope 20, centered on the threshold)
instead of a hard `angle > threshold` cutoff, and the total is the sum of
each local peak's confidence (`sum_of_corner_peaks`) — a turn a few degrees
short of the line now contributes partial weight instead of vanishing
outright. **Closed strokes use a 0.60 rad (~34°) threshold; open strokes use
0.68 rad (~39°)** — closed shapes get a lower bar because hand-authored
closed templates (e.g. the hexagon) can have corners short of a right
angle, while open-stroke corners (arrowheads, crosses) tend to be sharp by
construction. The two thresholds exist because at a single shared 0.68,
two of `safer`'s six hexagon corners fell below it and the shape read as a
4-corner `force` diamond instead — see
`shape::tests::closed_hexagon_reads_closer_to_six_corners_than_four`.
Consumers (`sphere_report`, `safer_report`, `diamond_report`, `cone_report`,
`aura_report`) compare this float directly (e.g. `corners < 4.0`) rather
than casting an integer.

**Known limitation, much reduced (tracked, not silently hidden):** before
the continuous-confidence change, `sphere`/`safer`/`force`/`summoning`
(round or near-round single-stroke shapes) could flip into each other under
moderate perturbation — 4 tracked pairs in `confusion_gate.rs`'s
`KNOWN_CONFUSIONS`. After softening the corner-count cliff, only one
remains: `("safer", "sphere")` under sparse (14-point) resampling — see
§2.5. *New* confusions still fail CI; this allowlist should keep shrinking
as remaining cliffs (the identity/quality blend's own thresholds, closure
scoring) get the same treatment, not by re-loosening the gate.

### 2.4 Acceptance

`MIN_RECOGNITION_CONFIDENCE = 0.32` — below this, nothing is accepted
regardless of margin. `MIN_RECOGNITION_MARGIN = 0.04` — if the winner beats
the runner-up by less than this, the read is `ambiguous`; an ambiguous read
is still `accepted` if confidence clears `AMBIGUOUS_ACCEPTANCE_CONFIDENCE =
0.58` (a close-but-confident read). As of Phase 1, the resulting discount to
confidence/quality is a **continuous ramp**, not a step: `margin_relief =
(score_gap / MIN_RECOGNITION_MARGIN).clamp(0,1)`, then `confidence *= 1.0 -
(1.0 - margin_relief) * 0.08` and `quality *= 1.0 - (1.0 - margin_relief) *
0.04` — full discount at `score_gap = 0`, none at `score_gap ≥
MIN_RECOGNITION_MARGIN`, smooth in between (previously a hard `×0.92/×0.96`
step at the margin line). See
`property_tests::ambiguity_penalty_has_no_visible_cliff`.
(`src/rune_drawing.rs:19-21`, `recognize_rune`.)

**Readout hysteresis (plan Phase 1 item 4).** A previously shown reading
only yields to a *decisively* better one: `recognize_rune_stable(strokes,
runes, context, previous)` keeps `previous` as the winner when the new
best beats it by less than `RECOGNITION_HYSTERESIS_MARGIN = 0.03`. The
demoted true best surfaces as the first alternative and the sticky outcome
is marked `ambiguous`, so the acceptance band still gets the final say.
Hysteresis only acts when a caller explicitly passes its previous reading —
plain recognition and diagram interpretation stay history-free, so the
"same ink → same result" determinism invariant (§2.1) is untouched. Wired
into the practice slate (`practice_report_for_rune_stable`, fed the
previous check's `read_rune_id` by `game::score_practice`). Tests:
`rune_drawing::tests::hysteresis_*`,
`stable_recognition_without_previous_matches_plain_recognition`.

### 2.5 Density dependence (A8) — mitigated at capture, still present at the recognizer level

Corner detection resamples to a **fixed 36 points** regardless of input
density, so very sparse (≤~20 points) or very dense (≥~70 points) input to
a tight-margin shape like `safer`'s hexagon can still shift where those 36
samples land relative to its corners enough to misread — `safer` remains
excluded from `property_tests::point_density_does_not_change_identity`, and
`("safer", "sphere")` remains the one entry in `confusion_gate.rs`'s
`KNOWN_CONFUSIONS`.

Phase 1 item 1 is implemented: `canonicalize_stroke`
(`src/rune_drawing.rs`) resamples every stroke to a fixed **0.01 arc-length
spacing** (`CANONICAL_STROKE_SPACING`) the moment it finishes drawing
(`state.rs::finish_drawing_stroke`, `game.rs`'s `FinishPracticeStroke`
handler) — before it is ever scored, matched, or saved. Two drawings of the
same physical motion captured at different frame rates now converge on
(near-)identical stored points (`rune_drawing::tests::canonicalize_stroke_converges_regardless_of_capture_density`).
This removes the device/framerate leak **for real gameplay capture**. It
does not by itself change `corner_count`'s own internal 36-sample grid, so
a rune fed a genuinely very short/sparse *stroke* (not just a low sample
rate) — or a synthetic/corpus sample captured before this landed — can
still land in the residual gap documented above.

---

## 3. Quality — how well a drawing matches, once identity is settled

`strict_quality_for_rune` (`src/rune_quality.rs:28`) scores the drawing
against the **best-matching accepted variant** of its recognized rune (kept
in parity with identity as of the quick-wins fix — previously it only
checked the canonical template, so a legal variant could get full identity
confidence but be punished on quality):

```
quality = shape_score × 0.46 + start_score × 0.22 + stroke_order_score × 0.32
```

- **Shape (0.46)** — `ordered_shape_score`: per-stroke shape fidelity using
  the same optimal stroke assignment as identity.
- **Start point (0.22)** — `start_point_score`: did the drawing begin where
  the template begins? This is direction- and order-sensitive by design —
  quality rewards the *canonical* way to draw a rune even though identity
  doesn't require it.
- **Stroke order (0.32)** — `stroke_order_score`: did strokes get drawn in
  template order? Combined with start point, this is where "draw order
  matters" (Goal 2) currently lives — but only for quality, not identity,
  and only ~54% combined weight, not a hard requirement.

Final recognition quality blends this strict score into the identity match:
`quality = best_score × (0.70 + strict × 0.30)` (`src/rune_drawing.rs:150`)
— shape match still dominates, but drawing it the "right" way adds up to
30% on top.

`RecognitionOutcome` also carries `ink_ratio` (Phase 2): the winning rune's
drawn arc length ÷ its best-matching template's arc length, both measured
post-normalization (scale-invariant — this isolates "was the stroke traced
to completion" from "how big was it drawn"). It doesn't affect identity or
quality; it feeds `potency` (§4) instead.

---

## 4. Magnitude — size and length as effect strength

A rune's shape can be identified and well-drawn (§2, §3) and *still* carry
a magnitude: how big it was drawn, and whether the stroke was carried all
the way through. That's `potency` — a per-rune multiplier, independent of
shape quality, computed in `push_recognized_rune`
(`rune_diagram/recognition.rs`) and stored on `InterpretedRune.potency`.

### 4.1 The two inputs

**Size**, via `scale` (already captured — `bounds.scale_relative(circle_bounds)`,
the rune's bounding box vs. the working circle's, §5.2), compared against
a **reference size per category**, `RuneCategory::ideal_scale_in_circle`
(`src/data.rs`): Effect 0.18, Shape 0.15, Trigger 0.14, Modifier 0.12 — the
same constants `magical_circle::size_harmony` already used for diagram
symmetry scoring, now a single shared source of truth.
`scale_ratio = scale / ideal`; 1.0 = drawn at reference size.

**Completeness**, via `ink_ratio` (§3) — a stroke that stops short of the
template's full length pulls potency down *before* it's short enough to
break identity outright.

### 4.2 The curve

```
potency_from_scale_ratio(ratio):
    ratio < 1.0:  1.0 + (ratio - 1.0) × 0.8   # → 0.6 at ratio 0.5
    ratio ≥ 1.0:  1.0 + (ratio - 1.0) × 0.6   # → 1.6 at ratio 2.0

potency = (potency_from_scale_ratio(scale_ratio) × ink_ratio.clamp(0.5, 1.0))
              .clamp(0.35, 2.2)
```

This is exactly the plan's example curve (0.6× at half reference size, 1.6×
at double), piecewise-linear through (0.5, 0.6) → (1.0, 1.0) → (2.0, 1.6),
each segment's slope continuing past those anchors rather than flattening,
then hard-clamped. `ink_ratio` only ever *reduces* potency (clamped to
`[0.5, 1.0]` — no bonus for excess ink beyond the template's length) and
never worse than half credit.

**Deliberately not folded into potency:** shape `quality`. The plan lists
scale, ink_ratio, and "quality as today" as potency's inputs; this
implementation keeps `quality` as its own, separately-applied factor in
`evaluate()` (§5.4) rather than multiplying it into potency too — doing
otherwise would reintroduce exactly the kind of compounded, opaque
multiplier stack D2 (§5.4) removes elsewhere in this pass. A sloppy but
huge rune shows high potency and low quality as two distinct, inspectable
numbers, not one blended score.

There is no per-rune identity/magnitude "band" table — the curve and
reference sizes are uniform across all runes via category, not hand-tuned
per rune. `RuneCategory::ideal_scale_in_circle` is the one number a new
rune's category determines; nothing else about potency needs per-rune data.

### 4.3 Where it shows up

- `evaluate()` (§5.4): `power` and the base component of `mana_cost` scale
  with potency, on top of quality's existing (separate) effect.
- A containment budget (§5.4): total potency across all placed runes must
  be covered by circle quality (and, once present, structure); exceeding
  it costs stability.
- UI: the drawing slate's post-interpretation readout shows average potency
  (`ui/drawing.rs`, "potency NN%"); the rune guide reference card
  (`ui/rune_guide.rs`) explains the mechanic in one line per rune category.

### 4.4 Layout position — resolved (D1: deleted)

The previous `layout_quality` position bias (category home positions,
0–24% quality penalty for drawing off-position) has been **deleted**, per
the plan's own recommendation: freehand scoring should not silently
penalize where a well-drawn rune was placed. See
`rune_diagram::tests::rune_quality_does_not_depend_on_board_position`.
Position semantics returned as promised, in `crate::reading` — see
`.project/placement-rules.md`. Placement now means something *relative*:
marks drawn deliberately close are read together, so a modifier pulled in
beside one effect tempers that effect alone instead of the whole working,
and a shape at the heart of the circle is the default the ring's marks make
exceptions to. Absolute direction still means nothing: rotating a diagram
produces the identical reading. None of it costs quality — the rule adds
meaning rather than a penalty, and a diagram drawn evenly spread reads
exactly as it did before the rule existed.

---

## 5. Circle grammar — structured diagrams

`interpret_diagram()` (`src/rune_diagram.rs`) picks the working circle, then
hands everything inside it to `interpret_scope()`
(`src/rune_diagram/scope.rs`, §5.6): classify every stroke as rune ink or a
structure mark → recurse into any nested sub-scope it finds → cluster the
remaining rune ink into rune groups → recognize each cluster. The result
feeds `analyze_magical_circle()` (`src/magical_circle.rs`) for the spell
summary.

### 5.1 Working circle

- May be drawn as a **single stroke, or chained from 2-4 open arcs whose
  endpoints nearly touch** (`assemble_multi_stroke_circles`, `circle.rs`,
  Phase 3 item 1 / C5) — a circle repaired mid-draw, or drawn in several
  passes, is scored exactly like a one-stroke circle once the arcs are
  merged into one polyline. No direction/turn-angle check gates the join
  (unlike `rune_drawing`'s continuation merge): arcs of a circle are
  expected to turn continuously, so requiring a nearly-straight join would
  reject exactly the shapes this exists to catch. Only strokes with a span
  ≥0.08 are considered possible arcs, keeping this cheap even with hundreds
  of small rune strokes elsewhere in the drawing.
- Single-stroke candidates still need ≥8 points and a span ≥0.22 with
  width/height ≥0.15 to be considered at all (`circle.rs`). `circle_quality`
  is a weighted sum: closure 32%, aspect 18%, center-proximity 4%, radius
  consistency 22%, angular coverage 18%, top-start bias 8%. `circle_found`
  requires `circle_quality ≥ MIN_CIRCLE_QUALITY (0.32)`. Among all
  candidates (single-stroke and chained), the one enclosing the most other
  ink wins, tie-broken by meeting the quality floor, then span, then raw
  quality, then (for a chained candidate) its highest member stroke index.
- A candidate rune only counts as "inside" the circle if (a) its center,
  normalized to the circle's elliptical radii, satisfies `nx² + ny² ≤
  1.25`, **and** (b) its width and height are each under `0.92×` the
  circle's corresponding dimension (`is_inside_working_circle`). A rune
  drawn large enough to nearly fill the circle fails (b) and is silently
  dropped rather than scored low (D5, tracked as a paper cut, not fixed
  here).

### 5.2 Structure marks vs. rune ink

Every non-rune stroke is first classified by its geometry relative to its
scope (`classify_circle_stroke`, `magical_circle.rs`), using `orbit`
(distance from center, normalized to the scope's radius) and `scale`
(stroke size relative to the scope):

| Classification | Condition |
|---|---|
| `ReinforcementRing` | closed, `orbit ≤ 0.15`, `scale` 0.28–0.92 |
| `SatelliteSeal` | closed, ≥12 points, `orbit` 0.22–0.78, `scale` 0.055–0.24 |
| `PerimeterMark` | `orbit` 0.78–1.10, `scale ≤ 0.13`, short but `length_ratio ≥ 0.02`, `directness > 0.65`, ≤3 points |
| `ScriptMark` | `orbit` 0.18–0.88, `scale ≤ 0.075`, very short but `length_ratio ≥ 0.02`, `directness > 0.65`, ≤3 points |
| `RadialSpoke` | directness > 0.86, spans center → ring |
| `RuneInk` | none of the above |

The `length_ratio ≥ 0.02` floor (relative to the scope's radius) encodes
that ticks and script dashes have real drawn *length* — a near-zero-length
dot (e.g. the detached point some rune glyphs carry, like `on_touch`'s) is
not decorative writing and stays rune ink.

**Grouped-ink rescue (geometry over size, Phase 3 item 4).** After
classification, any `PerimeterMark`/`ScriptMark` whose ink practically
touches another comparably-sized stroke (bounds gap ≤
`GROUPED_INK_GAP_FACTOR = 0.12` × its own diagonal, other stroke ≤
`GROUPED_INK_MAX_SPAN_RATIO = 4×` its size) is flipped back to `RuneInk`
(`scope::keep_grouped_ink_as_runes`): what makes a tick *decorative* is
that it stands alone, while a small multi-stroke rune's 2-point rays (e.g.
the `light` asterisk drawn at 2–3% circle scale) overlap each other. Band
ticks keep their spacing — adjacent ticks sit apart by a good fraction of
their own length — so real script/perimeter bands are not rescued. The
size-ratio cap keeps a large ring's bounding box from vacuuming every tick
it encloses.

`directness` (start-to-end distance ÷ total arc length) and the ≤3-point
cap are Phase 3 item 4 additions (C1/C2): a real perimeter tick or script
mark is a small, near-straight, minimally-sampled flick (matching the
structure-mark test fixtures — see §7), while even a small drawn *rune*
usually has at least one real corner and more sampled shape (`spark`'s
4-point zigzag, for instance). Without this, a diagram with many small
legitimate runes at the same orbit/scale/length as decorative ticks could
have them swept into these structure-mark buckets — and excluded from rune
recognition entirely — purely by being small and numerous, which is exactly
the failure mode the plan calls "structure vs. rune ink by size, not
geometry."

A mark only actually counts as circle *structure* (and gets excluded from
rune clustering) once its kind clears a population threshold
(`is_circle_structure`, `rune_diagram.rs`): satellites need ≥3 at quality
> 0.68; rings need ≥2 at quality > 0.48; scripts need ≥8 at quality > 0.42;
radials need ≥6 at quality > 0.68. Below threshold, the marks are just left
as unrecognized ink, not reclassified as runes.

**Resolved as of Phase 3 (C1/C2):** the old `MIN_RUNE_SCALE_IN_CIRCLE`
(0.12) absolute-scale floor on recognized rune clusters is gone. In its
place, `MIN_RUNE_CLUSTER_POINTS` (4 — see §6) requires only that a
cluster's strokes carry enough sampled points to be legible at all; 4 is
the lowest total point count among any real rune template (`spark`,
`cone`), so this never rejects a rune drawn cleanly at its own shape,
however small. Combined with the directness/point-count gate above and
scale-relative clustering (below), a rune can now be drawn at 3-6% circle
scale and still read — the blocker the plan called "the single largest
blocker to the 100+ symbols goal" is resolved.

**Resolved as of Phase 3 (C3, C4):** rune-level clustering
(`cluster_strokes`, `rune_diagram/geometry.rs`) no longer uses fixed
absolute distance thresholds. Each stroke pair's center- and
stroke-distance limits now scale off the pair's own bounding-box
diagonal(s) — see §5.7 for the exact rule and why it isn't a single linear
formula. A spatial hash grid (bucketing by bounds-center, searching only
same/adjacent cells) replaces the old all-pairs scan, and a cheap
bounding-box-gap pre-check skips the expensive segment-to-segment distance
scan whenever the boxes alone are already too far apart.

### 5.3 Spell stat blob

`analyze_magical_circle()` (`magical_circle.rs`) requires either
`rings + satellites + radials + perimeter + scripts ≥ 2` or a rune of tier
≥4 present, else there's no circle spell at all. It produces `complexity`,
`intensity`, `containment` (each a weighted sum of circle quality, average
rune quality, and the structure-count ratios below), plus the raw counts,
then looks up a spell name/tier keyed on the dominant effect rune (highest
tier × quality × scale among Effect-category runes) plus these counts.

**Resolved as of Phase 4 (C6).** Complexity, containment, and intensity are
each measured against targets (rings/**3**, satellites/**5**, radials/**4**,
perimeter marks/**14**, script marks/**28**, rune count/**6**) via
`diminishing_count`, not the old `ratio_count`: uncapped and monotonic
instead of saturating at 1.0 exactly at the target and contributing nothing
beyond it. A 100-symbol diagram now always out-scores a well-drawn
30-symbol one, sub-linearly. See §8.2 for the exact formula.

Tier is thresholded on complexity and structure counts (`magical_circle.rs`):
tier 4 ("grand") needs a tier-≥4 rune present, complexity ≥0.72, ≥2 rings,
≥3 satellites, ≥6 perimeter marks; tier 3 ("high") needs complexity ≥0.61
or a tier-≥4 rune; tier 2 ("woven") needs complexity ≥0.48; else tier 1
("simple"). Tier and complexity/intensity/containment feed additive bonuses
to score, power, stability, mana cost, and safety in `evaluate()`
(§5.4) — they do not multiply.

### 5.4 From per-rune quality/potency to score (D2: additive, not stacked)

**Resolved as of Phase 2.** Per placed rune, the stored `quality` is now
just `recognized.quality.clamp(0.0, 1.0)` — identity/strict quality (§3),
unmodified. Circle quality and layout position no longer multiply into it
(`push_recognized_rune`, `rune_diagram/recognition.rs`); layout was deleted
outright (§4.4) and circle quality now contributes exactly once, additively,
in `evaluate()` (`src/state/evaluate.rs`):

```
circle_quality = board.last_diagram.circle_quality   // 0.0 if no diagram yet
stability += round((circle_quality - 0.55) × 20.0)
safety    += round((circle_quality - 0.55) × 14.0)
score     += round((circle_quality - 0.55) × 10.0)
```

— a "decent circle" (0.55) is neutral; better circles add, worse ones
subtract, once per evaluation regardless of how many runes are placed
(previously its effect scaled with rune count, since it multiplied into
*every* rune's quality).

**Potency** (§4) scales `power` and the base of `mana_cost` per placed
rune, on top of quality's own (separate, unchanged-in-kind) effect:

```
power      += round(rune.power × (0.35 + quality × 0.65) × potency)
mana_cost  += round(rune.mana_cost × potency) + round((1 - quality).max(0) × 12.0)
stability  += round(rune.stability - (1 - quality).max(0) × 22.0)   // unchanged
safety     += round(rune.safety   - (1 - quality).max(0) × 18.0)    // unchanged
```

**Containment budget** (plan item 3 — a first cut of the Phase 4 risk
mechanic, not the final version):

```
total_potency = sum(potency for every placed rune)
containment_capacity = 2.0 + circle_quality × 6.0
                            + circle_spell.map_or(0.0, |s| s.containment × 4.0)
stability -= round((total_potency - containment_capacity).max(0.0) × 10.0)
```

A circle alone supports a modest total potency; ring/perimeter structure
(via `circle_spell.containment`, §5.3) raises the ceiling. Exceeding it
costs stability predictably, not randomly — this is deliberately rough
(baseline 2.0 and the 6.0/4.0/10.0 coefficients are a starting point, not
balanced), since it exists to give Phase 4's full budget rule somewhere to
build from.

After the circle spell's own bonuses (`power_bonus` etc., §5.3) are folded
in, `score` is capped at `68 + avg_quality × 52`. Grade thresholds:
**Failed** if the request wasn't matched, a core rune is missing, or
`score < 35`; **Unstable** if `stability < 42 || safety < 32 || score <
64`; **Brilliant** if `score ≥ 92 && stability ≥ 68 && safety ≥ 48`; else
**Reliable**. `accident = stability < 26 || safety < 18`.

Note for tuning: removing the old multiplicative discount means `quality`
values (and therefore the score cap) run measurably higher across the
board than before Phase 2 — `state::tests`' rough-vs-clean fixture needed
rebalancing to stay meaningfully "rough" under the new, more transparent
scoring. Expect other hand-tuned test fixtures/thresholds to need similar
recalibration as this system's constants keep moving.

### 5.5 Containment hierarchy — nested scopes (Phase 3 item 2)

**Resolved, first cut.** A closed ring found inside a scope that also
encloses ink of its own is no longer just a `ReinforcementRing` decoration
— it's recursively interpreted as its own scope
(`interpret_scope`/`nested_ring_candidates`, `src/rune_diagram/scope.rs`),
exactly like the top-level working circle. This is the structural unlock
for a "volcano"-scale diagram: a working circle can enclose several
off-center vents, each with its own effect rune, and every rune reads at
*its own* scope's scale/orbit (§4.1's `scale`), not the outer circle's —
`InterpretedRune::center` was already absolute slate coordinates and
`scale`/`orbit` already relative to whatever bounds they were scored
against, so recursion needed no new fields, just a recursive call.

A ring qualifies as a nested scope only if **both**:

- its scale relative to its parent scope is `0.28..=0.90` (the same band
  `ReinforcementRing` uses — a nested scope *is* a reinforcement ring that
  also happens to enclose ink of its own), and
- it sits clearly **off-center**: orbit `≥ 0.20` (`NESTED_RING_MIN_ORBIT`,
  just above `ReinforcementRing`'s own `orbit ≤ 0.15` band).

The orbit requirement exists specifically so the game's existing
"concentric reinforcement ring stack" idiom (several closed rings sharing
the working circle's own center — see the high-tier city-shield fixture in
§7) is **not** reinterpreted as nested scopes: those rings stay plain
decoration, exactly as before. Only a genuinely separate sub-circle, drawn
off to one side, recurses. Ring *shape* fidelity itself is scored with
`ring_shape_score` (`circle.rs`) rather than `circle_quality` — the same
closure/aspect/radius/coverage math, but without `circle_quality`'s
absolute slate-space size floor (meaningless for a ring whose absolute size
is only ever a fraction of its parent) and without the "near the middle of
the whole slate" term (meaningless for a ring that can sit anywhere inside
its parent).

Recursion is capped at `MAX_SCOPE_DEPTH = 3` to bound worst-case cost on
degenerate geometry. Nested scopes are a **first cut**: they fold their
runes into one flat list (which `analyze_magical_circle` already consumes
via each rune's own scale/orbit) rather than becoming named nodes in a
compositional grammar — see §5.7.

### 5.6 Scale-relative clustering (Phase 3 item 3) and performance (item 6)

**Resolved.** `cluster_strokes` (`rune_diagram/geometry.rs`) no longer uses
one pair of fixed absolute distance thresholds for every stroke pair in the
drawing. Two *different* factor sets are used, chosen per pair, because a
single linear "threshold scales with size" rule cannot satisfy both shapes
below at once (this took several failed single-formula attempts against the
existing test suite to establish — see the git history on this file's
Phase 3 pass for the specific counter-examples):

- **Open pair** (neither stroke closed): threshold scales off the **larger**
  of the two strokes' bounding-box diagonals. A multi-stroke rune's open
  component strokes are often quite different sizes and don't literally
  touch (an arrow's short chevron head next to its long shaft) — bridging
  that gap needs a reach generous enough for the larger part.
- **Closed-involved pair** (either stroke closed, e.g. a circle rune like
  `sphere`): threshold scales off the **smaller** diagonal, using tighter
  factors. A closed shape already reads as a complete glyph on its own, so
  its reach must stay conservative or it swallows unrelated ink drawn
  nearby — a big circle's own bounding diagonal is not a sensible "how far
  can I reach for a neighbor" radius.
- **Override back to the open/generous path**, even when a stroke is
  closed, if the pair is **nested** (one bounding box mostly contains the
  other — `aura`'s hexagon ring around its own crossbar, `fire`'s loop
  around its own inner squiggle) **or** their bounding boxes actually touch
  (gap ≤ 0.004 — `continuous`'s two diamonds, which share a drawn vertex).
  Both are still one multi-stroke rune, not a closed glyph with a
  coincidentally-adjacent neighbor.

A spatial hash grid replaces the old O(n²) all-pairs scan once a scope has
≥24 ink strokes: items are bucketed by bounds-center into cells sized to
the largest plausible per-pair threshold (`max_diagonal ×
OPEN_CENTER_DISTANCE_FACTOR`, an upper bound on both factor sets), so only
same/adjacent-cell pairs are ever compared — the standard uniform-grid
neighbor-search correctness invariant. A cheap bounding-box-gap check (a
valid lower bound on true stroke distance) skips the O(p²)
segment-to-segment scan whenever the boxes alone are already too far apart.
`recognize_rune` also skips a template variant outright if its stroke count
differs from the drawn stroke count by more than 2, before paying for
normalization and full scoring.

`hundred_plus_symbol_diagram_round_trips_within_perf_budget_and_order_independence`
(§7) is the exit-criterion test: ~120 small `spark` runes packed on a grid
inside the working circle all round-trip, well inside a generous sanity
time budget (a debug `cargo test` bound, not the plan doc's native-release
~100ms target — that would need a `--release` benchmark, not attempted
here), and reversing the draw order doesn't change how many are found.

**Explicitly deferred, not attempted this pass:** cross-edit caching keyed
on a canonical-stroke hash with dirty-region re-interpretation. That needs
a persistent cache plumbed through `state.rs`/`game.rs`'s call sites, not
just the recognizer — a separate change once this pass's algorithmic
complexity work is validated in play.

### 5.7 What circle grammar does *not* yet have

- **Compositional semantics — resolved as of Phase 4, first cut.** Named
  spells are now data-defined predicates over a `ScopeSpell` tree
  (`assets/data/recipes.json`, §8), not a stat-blob-plus-lookup — see §8 for
  the schema and matching algorithm. Still not attempted: recipe
  *discovery/editing* UI beyond the existing ledger/journal (Phase 5
  scope), and backfire rules only cover three causes (§8.5), not every way
  a diagram's grammar can be broken.
- **The *full* containment budget rule — resolved as of Phase 4, first
  cut.** §8.3 replaces the flat single-scope formula with a recursive
  per-scope walk. Still deliberately rough: coefficients are a starting
  point, not a balanced-through-playtesting version, same caveat as Phase
  2's original cut.
- **A full beam-search recovery segmentation.** `best_recovery_window`
  (`rune_diagram/recognition.rs`, a band-aid over clustering failures,
  A5) still enumerates contiguous windows rather than a true beam search
  over spatial subsets — Phase 3 item 5 made it order-independent (windows
  over a nearest-neighbor spatial visiting order, not original draw order)
  but did not replace the windowing approach itself. `remaining_stroke_groups`
  (used by the sphere-extraction band-aid) *was* fully replaced — it now
  reuses `cluster_strokes`' ordinary spatial clustering instead of an
  index-adjacency heuristic.

---

## 6. Constants index

Every tunable constant that governs the rules above, in one place, so a
tuning change always has a specific sentence in this document to update
alongside it.

| Constant | Value | File | Governs |
|---|---|---|---|
| `MIN_RECOGNITION_CONFIDENCE` | 0.32 | `rune_drawing.rs` | §2.4 acceptance floor |
| `MIN_RECOGNITION_MARGIN` | 0.04 | `rune_drawing.rs` | §2.4 ambiguity band |
| `RECOGNITION_HYSTERESIS_MARGIN` | 0.03 | `rune_drawing.rs` | §2.4 readout hysteresis (only via `recognize_rune_stable`) |
| `AMBIGUOUS_ACCEPTANCE_CONFIDENCE` | 0.58 | `rune_drawing.rs` | §2.4 confident-despite-ambiguous |
| `MIN_ERASE_FRAGMENT_LENGTH` | 0.008 | `rune_drawing.rs` | §2.1 eraser debris |
| `MERGE_MAX_GAP` | 0.02 (or 15% of shorter stroke) | `rune_drawing.rs` | §2.1 continuation merge |
| `MERGE_MAX_TURN_COS` | 0.82 (~35°) | `rune_drawing.rs` | §2.1 continuation merge |
| `MAX_OPTIMAL_ASSIGNMENT_STROKES` | 12 | `rune_drawing/scoring.rs` | §2.1 Hungarian vs. greedy cutoff |
| `CLOSED_CORNER_THRESHOLD` | 0.60 rad | `rune_drawing/shape.rs` | §2.3 corner-confidence center (closed) |
| `OPEN_CORNER_THRESHOLD` | 0.68 rad | `rune_drawing/shape.rs` | §2.3 corner-confidence center (open) |
| `CORNER_SIGMOID_SLOPE` | 20.0 | `rune_drawing/shape.rs` | §2.3 how sharply corner-ness ramps around the threshold |
| corner resample target | 36 points | `rune_drawing/shape.rs` | §2.3, §2.5 density gap |
| `CANONICAL_STROKE_SPACING` | 0.01 | `rune_drawing.rs` | §2.5 fixed arc-length spacing applied at capture |
| ambiguity penalty ramp | full at `score_gap=0` (×0.92 conf / ×0.96 quality), none at `score_gap ≥ MIN_RECOGNITION_MARGIN`, linear between | `rune_drawing.rs` | §2.4 |
| strict quality weights | shape 0.46 / start 0.22 / order 0.32 | `rune_quality.rs` | §3 |
| identity/strict blend | `best_score × (0.70 + strict × 0.30)` | `rune_drawing.rs` | §3 |
| `MIN_CIRCLE_QUALITY` | 0.32 | `rune_diagram.rs` | §5.1 `circle_found` |
| `MIN_DIAGRAM_RUNE_CONFIDENCE` | 0.32 | `rune_diagram.rs` | §5.1 per-rune floor |
| `MIN_RECOVERED_RUNE_CONFIDENCE` | 0.52 | `rune_diagram/recognition.rs` | recovered-cluster floor (contamination band-aid, A5) |
| `is_inside_working_circle` bounds | `nx²+ny² ≤ 1.25` (ellipse) and `< 0.92×` circle dims (bbox) | `rune_diagram/circle.rs` | §5.1 |
| `MAX_CIRCLE_CHAIN_STROKES` | 4 | `rune_diagram/circle.rs` | §5.1 multi-stroke circle assembly cap |
| `MIN_ARC_SPAN` | 0.08 | `rune_diagram/circle.rs` | §5.1 minimum span to be considered a circle-arc candidate |
| chain gap limit | `(chain_span × 0.12).max(0.02)` | `rune_diagram/circle.rs` | §5.1 endpoint-touch tolerance when chaining arcs |
| `MIN_RUNE_CLUSTER_POINTS` | 4 | `rune_diagram/recognition.rs` | §5.2 rune legibility floor (replaces `MIN_RUNE_SCALE_IN_CIRCLE`, C1/C2) |
| structure-mark `directness` gate | `> 0.65`, ≤3 points | `magical_circle.rs` | §5.2 `PerimeterMark`/`ScriptMark` (C1/C2) |
| structure-mark dash-length floor | `length_ratio ≥ 0.02` | `magical_circle.rs` | §5.2 dots are rune ink, not script |
| `GROUPED_INK_GAP_FACTOR` / `GROUPED_INK_MAX_SPAN_RATIO` | 0.12 × own diagonal / 4× size cap | `rune_diagram/scope.rs` | §5.2 grouped-ink rescue |
| structure-mark population thresholds | satellites ≥3 @ q>0.68, rings ≥2 @ q>0.48, scripts ≥8 @ q>0.42, radials ≥6 @ q>0.68 | `rune_diagram.rs` | §5.2 |
| `OPEN_CENTER_DISTANCE_FACTOR` / `OPEN_STROKE_DISTANCE_FACTOR` | 0.72 / 0.35 (× larger of pair's diagonals; 0.72 so an elongated glyph's shaft still reaches the small head past its tip, e.g. `on_touch`) | `rune_diagram/geometry.rs` | §5.6 clustering, open-pair reach (C3) |
| `CLOSED_CENTER_DISTANCE_FACTOR` / `CLOSED_STROKE_DISTANCE_FACTOR` | 0.42 / 0.21 (× smaller of pair's diagonals) | `rune_diagram/geometry.rs` | §5.6 clustering, closed-involved-pair reach (C3) |
| `MIN_CLUSTER_CENTER_DISTANCE` / `MIN_CLUSTER_STROKE_DISTANCE` | 0.018 / 0.009 | `rune_diagram/geometry.rs` | §5.6 clustering floors |
| nested/touching-bbox override | full containment (8% tolerance) or bbox gap ≤ 0.004 | `rune_diagram/geometry.rs` | §5.6 closed-pair override back to the open/generous factors |
| clustering grid activation / cell size | ≥24 ink strokes; cell = `max_diagonal × OPEN_CENTER_DISTANCE_FACTOR` (floor 0.03) | `rune_diagram/geometry.rs` | §5.6 spatial grid (C4) |
| template-variant stroke-count prefilter | skip variant if `\|variant_strokes − drawn_strokes\| > 2` | `rune_drawing.rs` | §5.6 perf (item 6) |
| `MAX_SCOPE_DEPTH` | 3 | `rune_diagram/scope.rs` | §5.5 nested-scope recursion cap |
| nested-ring scale band | `0.28..=0.90` relative to parent scope | `rune_diagram/scope.rs` | §5.5 |
| `NESTED_RING_MIN_ORBIT` | 0.20 | `rune_diagram/scope.rs` | §5.5 excludes concentric reinforcement-ring stacks |
| `MIN_NESTED_RING_QUALITY` | 0.40 (via `ring_shape_score`) | `rune_diagram/scope.rs` | §5.5 |
| complexity/intensity/containment targets | rings 3, satellites 5, radials 4, perimeter 14, scripts 28, runes 6 (now via `diminishing_count`, uncapped — C6) | `magical_circle.rs` | §5.3, §8.2 |
| `diminishing_count` shape constant | `count / (count + target × 0.4)` | `magical_circle.rs` | §8.2 |
| tier thresholds | tier4: complexity≥0.72 + tier≥4 rune + ≥2 rings + ≥3 satellites + ≥6 perimeter; tier3: complexity≥0.61 or tier≥4 rune; tier2: complexity≥0.48 | `magical_circle.rs` | §5.3 |
| `RuneCategory::ideal_scale_in_circle` | Effect 0.18, Shape 0.15, Trigger 0.14, Modifier 0.12 | `data.rs` | §4.1 potency's size reference, also `size_harmony` |
| potency scale curve | (0.5× ratio → 0.6), (1.0× → 1.0), (2.0× → 1.6), slopes continue past anchors | `rune_diagram/recognition.rs` | §4.2 |
| potency ink_ratio factor | `ink_ratio.clamp(0.5, 1.0)` | `rune_diagram/recognition.rs` | §4.2 |
| potency final clamp | `[0.35, 2.2]` | `rune_diagram/recognition.rs` | §4.2 |
| circle-quality score contribution | `(circle_quality - 0.55) × {20 stability, 14 safety, 10 score}`, additive once per evaluation | `state/evaluate.rs` | §5.4 (D2) |
| power/mana potency scaling | `power × potency`, `mana_cost's base × potency` (on top of quality's own effect) | `state/evaluate.rs` | §5.4 |
| containment budget v2 | root: `2.0 + circle_quality×6.0 + containment×4.0`; each sub-scope: `1.0 + containment×3.0`; excess at every scope costs `×10.0` stability, summed | `state/evaluate.rs` | §8.3 (rough first cut, replaces the Phase 2 flat version) |
| `SAFER_CONTAINMENT_BONUS` | 1.5, added to a scope's own `containment` if it has a `safer` modifier | `rune_diagram/scope.rs` | §8.3 |
| scope containment formula | `diminishing_count(rings, 2)×0.6 + diminishing_count(perimeter, 8)×0.4`, clamped 0..1, plus the safer bonus | `rune_diagram/scope.rs` | §8.1, §8.3 |
| `evaluate()` score cap | `68 + avg_quality × 52` | `state/evaluate.rs` | §5.4 |
| grade thresholds | Unstable: stability<42 or safety<32 or score<64; Brilliant: score≥92 and stability≥68 and safety≥48 | `state/evaluate.rs` | §5.4 |
| `MIN_EFFECT_PRESENCE` | 0.001 | `recipes.rs` | §8.4 recipe effect-presence floor |
| recipe tie-break | highest `tier` wins, then lowest `id` | `recipes.rs` | §8.4 |
| backfire potency-excess threshold | `total_excess > 0.5` | `state/text.rs` | §8.5 |
| acceptance bands (confidence / margin / ambiguous-confidence) | Practice 0.40/0.05/0.64; Commission 0.32/0.04/0.58 (== §2.4's unqualified constants); Sandbox 0.24/0.03/0.50 | `rune_drawing.rs` | §9.1 |
| diagram acceptance bands (circle / diagram-rune) | Practice 0.40/0.40; Commission 0.32/0.32 (== §5.1/§6's `MIN_CIRCLE_QUALITY`/`MIN_DIAGRAM_RUNE_CONFIDENCE`); Sandbox 0.24/0.24 | `rune_diagram.rs` | §9.1 |
| `RuneMastery::score` | `accepted_count × (quality_sum / accepted_count)` | `state.rs` | §9.2 |
| guide opacity fade | `GUIDE_BASE_ALPHA (0.32) / (1.0 + mastery_score / GUIDE_FADE_SCALE (6.0))` | `ui/drawing.rs` | §9.2 |
| `GUIDE_FREE_INSIGHT` | 1 | `state/work.rs` | §9.2 |
| commission `required_structure` / `required_sub_scopes` | per-commission, in `commissions.json`; checked against the root `ScopeSpell` counts in `evaluate()` | `data.rs`, `state/evaluate.rs` | §9.4 story pacing |
| confusion-gate margins | clean/affine cases `score_gap ≥ 0.04`; resample/jitter cases `≥ 0.02`; confusable pairs exempt from margin only | `rune_drawing/confusion_gate.rs` | §7 |

---

## 7. Verification

- **`src/rune_drawing/confusion_gate.rs`** — every rune template plus 12
  deterministic perturbations (translate, uniform scale 0.5–2.0× — up-scale
  clamped per template to what physically fits the slate — sparse/dense
  resample, seeded jitter) must recognize as itself, be accepted, **and**
  beat the runner-up by the case's margin (clean ≥ 0.04, perturbed ≥ 0.02).
  Tracked-confusion pairs come from the template data itself
  (`confusable_with` → `templates::known_confusions()`, §2.3); they are
  exempt from the margin gate, never from identity. Down to one tracked
  entry (`safer`→`sphere` under sparse resampling).
- **`src/rune_drawing/property_tests.rs`** — determinism (same input twice →
  identical `RecognitionOutcome`), translation/scale invariance, point-
  density invariance (with the §2.5 exception), a jitter-monotonicity smoke
  test (more jitter should not, on average, raise quality), a ~1%-jitter
  quality-stability check, and `ambiguity_penalty_has_no_visible_cliff`
  (no single small perturbation step should swing confidence far, sweeping
  across the old hard-margin boundary).
- **`src/rune_drawing/shape.rs` unit tests** — `corner_count` on hand-built
  shapes (open line/bend, closed square/hexagon/diamond).
- **`src/rune_drawing/tests.rs`** — `every_rune_has_a_data_driven_template`
  and `touch_and_continuous_expose_their_extra_variants` guard the JSON
  template table; `canonicalize_stroke_converges_regardless_of_capture_density`
  guards §2.5's capture-time fix; `reordered_strokes_do_not_produce_spurious_mismatches`
  and `canonical_template_draws_no_mismatch_segments` guard D3.
- **`tests/corpus/`** — real human drawings, captured in-game via the
  Practice slate's **Capture Sample** button. `src/corpus.rs`'s
  `every_corpus_sample_recognizes_as_its_label` test holds every captured
  sample to the same bar. Empty today; grows as people playtest.
- **`src/rune_diagram/tests.rs`** — `doubling_effect_rune_scale_raises_potency`
  and `under_drawn_stroke_reads_but_reports_lower_potency` are Phase 2's
  exit criteria as tests; `rune_quality_does_not_depend_on_board_position`
  guards D1. Phase 3: `nested_off_center_ring_reads_its_own_effect_rune_relative_to_its_own_scope`
  (§5.5), `rune_far_below_old_twelve_percent_scale_floor_still_reads` (§5.2,
  C1/C2), and `hundred_plus_symbol_diagram_round_trips_within_perf_budget_and_order_independence`
  (§5.6, the combined scale/perf/order-independence exit criterion) are the
  new Phase 3 exit criteria as tests. The pre-existing
  `damaged_sphere_fragment_stays_out_of_neighboring_light_cluster`,
  `interprets_high_tier_structured_circle_spell`, and the full
  `structural_rune_sample_set_reads_inside_full_diagrams` sweep were the
  regression wall the §5.6 clustering-factor retune was validated against.
- **`src/state/tests.rs`** — `doubling_effect_rune_size_raises_power_in_report`
  is the exit criterion at the `evaluate()` level (not just `InterpretedRune`);
  `high_tier_circle_strengthens_city_shield_commission` guards the
  concentric-ring-stack idiom against being misread as nested scopes (§5.5).
  Phase 4: the same test doubles as the `floating_city` recipe's full-pipeline
  regression guard (§8.4) — its title assertion now passes through
  `crate::recipes::match_recipe`, not a hardcoded `match` arm.
  `diminishing_returns_let_more_structure_always_score_higher` is §8.2's exit
  criterion (more structure past the old fixed targets must score higher, not
  the same); `backfire_message_names_uncontained_potency` is §8.5's.
- **`src/recipes.rs` unit tests** — the authoritative correctness suite for
  `match_recipe` itself, run against hand-built `ScopeSpell` trees rather
  than drawn diagrams (recognizer accuracy is a separate, already-covered
  concern): bare-presence vs. `min_potency` requirements, potency summing
  across sub-scopes, tier/id tie-break determinism, and a full
  structure-plus-sub-scope-count gate modeled on `volcano`.
- **`src/rune_diagram/tests.rs`** — `fireball_recipe_recognized_purely_from_data`
  and `volcano_recipe_recognized_purely_from_data` are Phase 4's own exit
  criteria as tests: both diagrams are matched against
  `assets/data/recipes.json` with no rune-id-specific Rust in the naming
  path. `volcano_recipe_recognized_purely_from_data` also exercises
  `ScopeSpell::total_potency` summing fire potency across the root scope and
  both vent sub-scopes to clear `volcano`'s `min_potency` requirement.

  `simple_named_recipe_still_recognized_through_evaluate` is the matching
  regression guard for the other four migrated recipes (`gravity_well` via
  the `floating_stage` commission, which requires `gravity` directly) —
  note it sets `workshop_rank = 4`, since `gravity` is tier 4 and
  `GameSession::can_use_rune` (`state.rs`) excludes any rune above the
  session's own workshop rank from the recognizer's candidate set entirely;
  forgetting this makes an unlocked, perfectly-drawn tier-4 rune legitimately
  read as its best-scoring *unlocked* alternative instead — worth checking
  before suspecting a recognizer or board-placement bug in a session-level
  (as opposed to `interpret_diagram()`-level) mismatch.
- **Phase 5.** `rune_drawing::tests::acceptance_bands_are_ordered_practice_strictest_sandbox_most_lenient`
  and `context_changes_acceptance_cutoffs_never_the_underlying_score` are
  §9.1's exit criteria — the three bands order correctly and the underlying
  recognition score never changes with context, only the cutoff applied to
  it. `state::tests::sandbox_mode_accepts_a_weak_circle_commission_mode_rejects`
  is the same guarantee one level up, through `GameSession::interpret_drawing`.
  `state::tests::accepted_reads_accumulate_rune_mastery` and
  `guide_free_interpretation_earns_insight_bonus_but_guided_does_not` guard
  §9.2. `rune_diagnostics::tests::player_hint_flags_missing_circle`,
  `player_hint_flags_a_weak_circle`, and `player_hint_is_none_for_a_clean_diagram`
  guard §9.3's priority order at the function level;
  `state::tests::circle_free_diagram_rejection_surfaces_the_specific_player_hint`
  guards the same wiring through `interpret_drawing`.
  `state::tests::early_commissions_still_clear_acceptance_when_drawn_with_a_degraded_hand`
  and the pre-existing `high_tier_circle_strengthens_city_shield_commission`
  are §9.4's pacing exit criteria (early commissions tolerate a jittered
  hand; a late commission drawn with real structure reaches `Brilliant`).
- **Phase 6.** `rune_diagram::tests::synthetic_diagram_round_trips_across_sizes_with_jitter`
  is §10.1's exit criterion — a jittered synthetic diagram round-trips at
  every size from 3 up through 300. `main::cli_tests` (three unit tests:
  bare stroke list, `CorpusSample`, missing file) guards §10.3's CLI replay
  tool. §10.2 has no test of its own — it's the standing CI configuration
  (`.github/workflows/rust-ci.yml`) already exercising every test above on
  every push and PR.

Any change to a constant in §6 should keep all of the above green, or the
affected sentence in this document (and, if it's a tracked confusion, the
`KNOWN_CONFUSIONS` list) should be updated in the same change.

---

## 8. Compositional spell grammar (Phase 4)

Phase 4 replaces the stat-blob-plus-lookup spell system with named spells
defined as data predicates over the diagram — the plan's own worked example
(`volcano`, a fire effect spread across several vents inside a structured
crater) is close to literally what ships in `assets/data/recipes.json`.

### 8.1 The spell tree

`ScopeSpell` (`src/rune_diagram.rs`) is the tree `interpret_scope`
(§5.5/§5.6) was already recursively walking, finally *kept* instead of
flattened away. `ScopeOutcome` (`rune_diagram/scope.rs`) now carries
`own_runes` (this level only) and `sub_scopes: Vec<ScopeOutcome>`
(previously `.extend()`-ed into the parent's flat list and lost) — two
small recursive walks turn one `ScopeOutcome` tree into the two shapes
different callers need:

- `flatten_runes` reproduces the exact Phase 0-3 flat `Vec<InterpretedRune>`
  — every existing consumer (UI, `evaluate()`'s per-rune stat math, the
  `MagicalCircleSpell` bonus struct) is unaffected by Phase 4.
- `build_scope_spell` groups each level's own runes by category
  (`effects: Vec<(rune_id, potency)>`, `shape`, `trigger`, `modifiers`) and
  recurses into `sub_scopes: Vec<ScopeSpell>` — this is what
  `crate::recipes::match_recipe` (§8.4) evaluates predicates against.

`DiagramInterpretation.scope_spell: Option<ScopeSpell>` is populated
whenever a circle is found — **deliberately not** gated behind
`analyze_magical_circle`'s own "elaborate enough" check (`structural_count
< 2 && max_tier < 4` → `None`). That gate exists for the ring/satellite
*bonus* struct's own purpose; a small, low-structure diagram (a fireball is
the plan's own "3-4 symbols" example) still needs a scope tree to be
matchable against recipes. Tying the two together was an early bug in this
pass — `fireball_recipe_recognized_purely_from_data` (§7) is the regression
guard.

`ScopeSpell::total_potency(effect_id)` sums that effect's potency across a
scope *and every descendant sub-scope* — the mechanism behind the plan's
"repeated effect runes / sub-scopes feeding a parent ... multiply potency":
a recipe's `min_potency` is checked against this sum, not any single rune's
own potency (§8.4).

### 8.2 Diminishing-returns amplifiers (C6)

`magical_circle::diminishing_count(count, target)` replaces `ratio_count`
at every structural-count call site (`ring_score`, `satellite_score`,
`radial_score`, `perimeter_score`, `script_score`, `rune_count_score`, and
`circular_symmetry`'s own anchor-count term):

```
diminishing_count(count, target) = count / (count + target × 0.4)
```

Uncapped and monotonic instead of `(count/target).clamp(0,1)`: `≈0.71` at
`count == target` (close enough to the old curve's saturated `1.0` that
existing tier thresholds — §6 — didn't need retuning), `0.83` at
`2×target`, `0.88` at `3×target`, climbing forever, sub-linearly. This is
what makes "100 symbols always beats 30 well-drawn ones" true —
`diminishing_returns_let_more_structure_always_score_higher` (§7) is the
exit-criterion test.

### 8.3 Containment budget v2

Each `ScopeSpell.containment` is computed the same way the old
whole-diagram `MagicalCircleSpell.containment` was, but from *that scope's
own* ring/perimeter counts only:

```
structural = diminishing_count(ring_count, 2) × 0.6
           + diminishing_count(perimeter_mark_count, 8) × 0.4   // clamped 0..1
containment = structural + (1.5 if "safer" ∈ modifiers else 0.0)
```

— directly implementing the plan's explicit "circle quality, rings,
perimeter script, safer runes" containment inputs, per scope. In
`state/evaluate.rs`, `total_potency_excess` walks the whole tree instead of
computing one flat number:

```
root capacity    = 2.0 + circle_quality × 6.0 + root.containment × 4.0
sub-scope capacity = 1.0 + sub.containment × 3.0        // no circle_quality term — a vent has no drawn "circle quality" of its own
excess(scope)     = max(0, scope's own effect potency − scope's capacity)
total_excess      = sum of excess(scope) over the whole tree
stability        -= round(total_excess × 10.0)
```

A vent's fire potency has to be covered by that vent's own rings and
wards, not borrowed from the crater's — this is the concrete fix for the
plan's "circle alone supports a modest total potency" language now applying
per-scope rather than diagram-wide. Root and sub-scope coefficients are
still a first cut (§5.7), same caveat as Phase 2's original version.

### 8.4 Recipes as data

`RecipeDef` (`src/data.rs`, loaded into `GameData.recipes` from
`assets/data/recipes.json` the same way every other asset loads) is a
named spell as a predicate:

```json
{
  "id": "volcano", "name": "Grand Caldera", "tier": 4,
  "requires": {
    "effect": { "fire": { "min_potency": 3.0 } },
    "shape": "cone", "trigger": "continuous",
    "structure": { "rings": 2, "satellites": 3 },
    "sub_scopes": [{ "effect": "force", "count": 2 }]
  }
}
```

`crate::recipes::match_recipe(tree, recipes)` (`src/recipes.rs`) evaluates
`requires` against a `ScopeSpell`:

- **`effect`** — `tree.total_potency(id) ≥ min_potency.max(MIN_EFFECT_PRESENCE)`.
  A bare `{}` requirement (no explicit `min_potency`, default `0.0`) means
  "present at all" — `MIN_EFFECT_PRESENCE` (0.001) exists purely so
  "never drawn" (`total_potency == 0.0`) reads as absent rather than
  trivially satisfying a `≥ 0.0` check (every real occurrence clears 0.35+
  anyway, per potency's own clamp).
- **`shape` / `trigger` / `modifier`** — checked against the **root**
  scope's own fields only; a diagram's overall shape/trigger character is
  set by its outermost circle, not by what a vent happens to carry.
- **`structure`** — checked against the **root** scope's own
  ring/satellite/radial/perimeter/script counts.
- **`sub_scopes`** — each `{effect, count}` entry needs at least `count`
  *direct* `tree.sub_scopes` entries whose own `effects` include that id
  (not counted recursively deeper — a direct child scope, not a
  grandchild).

Among every matching recipe, the highest `tier` wins; ties break on the
lowest `id` (deterministic, same discipline as every other tie-break in
this codebase). Tier ordering is what lets a more specific recipe beat a
broader one it also happens to satisfy — `floating_city` (gravity + sphere
+ continuous, tier 4) must outrank `gravity_well` (bare gravity, tier 1)
whenever both match, so **every recipe roster addition needs a
strictly-more-specific requirement set to carry a strictly-higher tier**,
or matching becomes order-dependent by accident.

The roster today (`assets/data/recipes.json`) migrates all four of the old
hardcoded `spell_name` arms to data (`gravity_well`, `floating_city`,
`wayfold_gate`, `threshold_calling`, `chronal_ledger`) and adds the plan's
own two worked examples (`fireball`, `volcano`) — `magical_circle::spell_name`
now only holds the generic "{prefix} Circle of {effect}" / "{prefix}
Unbound Circle" fallback for anything that doesn't match a named recipe.

Wiring, in `state/evaluate.rs`:

```
matched_recipe = scope_spell.and_then(|tree| match_recipe(tree, &data.recipes))
title = matched_recipe.map(name)
          .or_else(|| circle_spell.filter(tier_rank ≥ 3).map(name))   // old generic-tier fallback
          .unwrap_or(base_title)                                      // commission-based title
```

`signature()` (discovery keying) prefers `matched_recipe.id` when present —
a stable, semantic key, so every "volcano" is the same discovery regardless
of minor potency/quality drift — falling back to the pre-Phase-4 sorted
name-bag for diagrams that don't match any recipe, so arbitrary/unnamed
combos stay discoverable exactly as before.

### 8.5 Backfire messaging

`text::backfire_cause` (`src/state/text.rs`) is tried before the existing
generic `text::side_effect` message; when it returns `Some`, that replaces
the generic message entirely (higher-priority, more specific always wins).
Priority order: an accident already gets its own message from
`side_effect` (`backfire_cause` returns `None` immediately, so callers fall
through rather than double-messaging) → uncontained potency
(`total_excess > 0.5`, §8.3) → a specifically-*named* missing requirement
(`evaluate()` now identifies *which* of effect/shape/trigger is missing,
not just *how many*, reusing the same three id comparisons
`required_hits`/`matched_request` already made) → duplicate effect runes in
the same scope → the pre-existing generic weak-marks/grade message. This is
"failures teach the grammar" (plan item 4) as a first cut — three concrete
causes, not every way a diagram's rules can be broken (§5.7).

---

## 9. Progression & aids (Phase 5)

Phase 5 is the plan's last phase, and different in character from 0-4: it's
UX/progression work on top of an already-solid recognizer, not more
recognizer engineering. One recognizer, one scoring pipeline, throughout —
every item below changes *which cutoff* a result is compared against or
*how it's surfaced*, never the underlying identity/quality/circle math in
§2-§5.

### 9.1 Per-context acceptance bands

`RecognitionContext { Practice, Commission, Sandbox }` (`rune_drawing.rs`)
replaces the single hardcoded acceptance constants (§2.4, §5.1) with a
per-context table, `acceptance_band`/`diagram_acceptance_band`:

| Context | confidence | margin | ambiguous-confidence |
|---|---|---|---|
| Practice | 0.40 | 0.05 | 0.64 |
| Commission | 0.32 | 0.04 | 0.58 |
| Sandbox | 0.24 | 0.03 | 0.50 |

Commission's row is bit-identical to §2.4/§5.1's pre-Phase-5 constants —
`recognize_rune`/`interpret_diagram` keep their exact original signatures
and delegate to the new `recognize_rune_in_context`/`interpret_diagram_in_context`
under `Commission`, so no existing call site or test needed to change.
Practice (`rune_quality::practice_report_for_rune`) is *stricter* than
Commission — practice is where technique is taught, so a rough drawing that
a commission would still accept is rejected here. Sandbox is the most
lenient, and is not a new screen: it's the existing commission slate
(`GameSession.sandbox_mode: bool`, toggled by a "Sandbox" button in
`ui/drawing.rs`, deliberately **not** persisted in `SaveData` — always off
on load) with `matched_request` trivially true (no commission to fail
against, §5.4's `evaluate()`) and no delivery reward, the same "free
experimentation, no stakes" contract Practice already has. Building a
dedicated Sandbox screen with its own reference panel/layout was
deliberately not attempted — inheriting the commission slate's UI as-is was
the bounded first cut.

**Fixed along the way:** `DiagramInterpretation::accepted()` used to
re-derive `circle_quality ≥ MIN_CIRCLE_QUALITY` — the Commission-only
constant — regardless of which context actually produced the
interpretation. Under Sandbox this was silently wrong: `circle_found` was
correctly computed against Sandbox's looser 0.24 band, but `accepted()`
would still fail the hardcoded 0.32 check right after. Simplified to
`circle_found && !runes.is_empty()`, since `circle_found` already encodes
the right context-aware comparison.

### 9.2 Aids that fade with mastery

`PlayerState.rune_mastery: HashMap<String, RuneMastery>` (`state.rs`,
`#[serde(default)]` for old saves) tracks `{ accepted_count, quality_sum }`
per rune id; `RuneMastery::score()` is `accepted_count × mean_quality`.
Recorded once per accepted rune inside `apply_interpreted_runes` (covers
both the commission slate and Sandbox) and inside `game.rs::score_practice`
(covers Practice) — mastery builds from any successful read, not just
delivered commissions, since both paths read with the same recognizer
(§9.1).

`ui/drawing.rs`'s guide-template overlay opacity — previously a flat
constant — is now `GUIDE_BASE_ALPHA / (1.0 + mastery_score /
GUIDE_FADE_SCALE)`: a monotonic curve that approaches, never hard-cuts to,
zero as mastery grows, the same "diminishing, not capped" shape as §8.2's
`diminishing_count`.

A small flat **guide-free bonus** (`GUIDE_FREE_INSIGHT = 1`, `state/work.rs`)
is awarded when `interpret_drawing` succeeds with `board.guide_templates`
empty at the moment of interpretation — the payoff for actually going the
rest of the way to guide-free, surfaced through the same note/journal-entry
mechanism `DISCOVERY_INSIGHT` already uses.

### 9.3 Friendlier failure surfacing

`rune_diagnostics::player_hint(strokes, runes) -> Option<String>`
(`rune_diagnostics.rs`) reuses the same typed calls the dev-only
`diagnose_diagram` clipboard tool already makes
(`gather_circle_candidates`, `select_working_circle_for_strokes`, cluster
classification) instead of duplicating logic, and picks the single most
relevant issue in priority order: no closed shape reads as a circle → the
circle reads but is too weak → a cluster's identity is ambiguous between
two close-scoring runes → a rune-sized mark got swallowed as decoration →
(when accepted) a generic "reads fine, technique could tighten up" for low
average quality. Returns `None` when nothing stands out — a clean diagram
gets no hint.

`GameSession::ensure_interpretable` (`state.rs`) calls this for both of its
rejection branches (no circle / no clear rune), replacing the old generic
"No enclosing circle was readable." / "circle reads at N%, but no inner
rune was clear enough" text with the specific hint whenever `player_hint`
returns one, falling back to the original generic wording only when it
returns `None` — the same "specific beats generic" pattern §8.5's
`backfire_cause` already established for evaluation failures.
`interpret_drawing`'s success-path note also appends the hint (when
`Some`) after `interpretation_note`'s summary, so borderline-but-accepted
reads still get the same soft-quality nudge.

### 9.4 Story pacing

As of the completion pass, difficulty *does* gate structure. `CommissionDef`
gained two optional fields (defaults = no demand, so early commissions are
untouched):

- `required_structure` — a `StructureRequirement` (same shape recipes use:
  rings/satellites/radials/perimeter/scripts), checked against the root
  `ScopeSpell`'s structure-mark counts;
- `required_sub_scopes` — how many distinct nested sub-scope circles the
  root must carry.

Both are enforced inside `evaluate()`'s `matched_request` gate (sandbox
exempt, like the rune-request gate). Current authoring: difficulty ≤ 3
commissions demand nothing; difficulty 4–5 (`orchard_bloom`, `alarm_bell`,
`silent_gloves`) demand 2 reinforcement rings (± satellites) — 2 because a
*single* closed ring is deliberately ambiguous with a circle-shaped rune
(§5.2's ≥2-ring population threshold); difficulty 7 (`city_shield`) demands
rings 2 + satellites 3; and the new `ember_gardens` (difficulty 7) is the
endgame multi-scope commission: rings 2, satellites 3, **and 2 sub-scope
vents**.

Verification (§7):

- **Degraded-corpus checks**: every difficulty-≤2 commission
  (`early_commissions_still_clear_acceptance_when_drawn_with_a_degraded_hand`)
  *and* every difficulty-≥3 commission including the structural and
  multi-scope ones
  (`mid_and_endgame_commissions_clear_with_degraded_hand_and_structure`)
  still clears `matched_request` and never grades `Failed` when drawn with
  the seeded ~15%-scale/translate/jitter degraded hand
  (`rune_drawing::test_support::perturb`).
- **Structure actually gates**:
  `structure_requiring_commission_rejects_a_plain_rune_diagram` — the plain
  three-rune diagram that clears early commissions does *not* match
  `city_shield`.
- **Late-game reward check**: the pre-existing
  `high_tier_circle_strengthens_city_shield_commission` — `city_shield`
  drawn with `high_tier_city_circle()`'s multi-ring/satellite structure
  reaches `Brilliant`.

---

## 10. Verification at scale (Phase 6)

Phase 6 is the plan's last phase, and — per its own heading — "continuous"
rather than one-off feature work: keep the existing regression wall broad
and green, extend it to a size sweep, and keep a fast offline tuning loop
available.

### 10.1 Size-swept synthetic round-trip test

`rune_diagram::tests::synthetic_diagram_round_trips_across_sizes_with_jitter`
extends the pre-existing single-size
`hundred_plus_symbol_diagram_round_trips_within_perf_budget_and_order_independence`
(§5.6, §7) — which stays exactly as-is, since it's also the order-independence
exit criterion — with a size sweep from 3 up through 300 `spark` instances
(`[3, 10, 30, 60, 120, 200, 300]`), each placed via a new `jittered_rune_grid`
helper, and — as of the completion pass —
`heterogeneous_diagram_round_trips_and_order_shuffle_changes_nothing`, which
places ~150 *mixed* symbols (`spark`/`warmth`/`light` cycling, so adjacent
neighbors are different runes, multi-stroke `light` drawn proportionally
larger since its 2-point rays physically cannot survive at flick size),
asserts every symbol reads as the right rune, and re-interprets the same
ink with the symbol draw order fully reversed demanding the *identical*
interpretation — same rune ids at the same centers with the same scales
and potencies, not merely the same count. (`quality` is deliberately
excluded from that equality: it rewards canonical stroke order by design,
§3.) Every instance is perturbed with `rune_drawing::test_support::perturb`
(the same helper §9.4's degraded-corpus test and the confusion-matrix gate
use), with `jitter_amp` scaled to that instance's own on-canvas `scale`
rather than a fixed absolute amplitude — a fixed amplitude tuned for one
size (à la `confusion_gate.rs`'s `jitter_a`, perturbing a template at its
own natural, unscaled size) is proportionally far too large once the sweep
reaches its smaller, more tightly-packed sizes. Spacing and scale both
tighten at the larger sizes so more instances fit inside the working circle
while staying distinct clusters under §5.6's scale-relative clustering
floors. Every size in the sweep — including 300, the plan's own upper
target — round-trips: every symbol found, none merged or lost, all read as
`spark`, within the same generous debug-`cargo test` perf bound the existing
120-symbol test already uses (not a release-mode benchmark).

### 10.2 Confusion matrix + corpus + property tests stay green — already continuous

No code change needed. `.github/workflows/rust-ci.yml`'s `checks-and-webgl`
job already runs `cargo test --all-features` — which includes
`rune_drawing::confusion_gate`, `corpus::tests::every_corpus_sample_recognizes_as_its_label`,
and `rune_drawing::property_tests` — on every push and pull request. This
exit criterion has been continuously true since Phase 0 introduced the CI
workflow; Phase 6 simply records it as the phase's own verification
requirement rather than treating it as new work.

### 10.3 Fast tuning-loop replay tool

The crate is binary-only (`src/main.rs`, no `src/lib.rs`, no
`[[example]]`/`examples/` in `Cargo.toml`), so a literal `cargo run
--example` would require a lib/bin split purely to let an example crate see
`crate::rune_diagnostics` and friends — invasive for a dev-convenience tool.
The plan text itself offers an alternative ("or debug screen"), so Phase 6
instead adds a CLI branch directly to `main()` (`src/main.rs`):
`cargo run -- --diagnose <path.json>` reads a JSON file, tries parsing it as
a `corpus::CorpusSample` (`{label, strokes}`, the exact shape
`capture_corpus_sample` already writes to `tests/corpus/`) and falls back to
a bare `Vec<DrawnStroke>` if that fails, then calls
`rune_diagnostics::diagnose_diagram` and prints the log to stdout — no game
window opens for this path.

Because `main.rs` is the crate root, every item this needs
(`corpus::CorpusSample`, `data::GameData`, `rune_diagnostics::diagnose_diagram`,
`rune_drawing::DrawnStroke`) is already visible with no visibility changes.
`#[macroquad::main(window_conf)]` is intentionally not used on `main` itself
— its macro expansion creates the game window (`Window::from_config(conf,
amain())`) before any user code runs, which the CLI branch must skip
entirely. `main` is now a plain `fn` that checks for `--diagnose` first and
returns before ever touching macroquad if present, otherwise falling
through to the exact `Window::from_config(window_conf(), amain())` call the
attribute would have generated (the async body itself, unchanged, was
renamed from `main` to `amain` to make room). The parse-and-diagnose logic
lives in a separate `run_diagnose_cli(path) -> Result<String, String>`, kept
free of `env::args()`/`process::exit` so it's directly unit-testable —
`main::cli_tests` covers a bare stroke list, a `CorpusSample`, and a missing
file.

---

*Last updated: 2026-07-04 — Phase 0 through Phase 6 of
`.project/magic-symbol-system-plan.md` (the plan's final phase).
Phase 1 remaining: generalize the structural-check layer (§2.3) into
data-driven rules. Phase 2 remaining: none of the plan's four items are
outstanding, but the containment budget (§8.3) is explicitly a rough first
cut, not a tuned final version. Phase 3 remaining: `best_recovery_window`
is order-independent but still windows rather than running a true beam
search; cross-edit recognition caching was not attempted. Phase 4 remaining
(§5.7, §8.5): recipe discovery/editing UI beyond the existing ledger/journal
was not attempted; backfire messaging covers three causes, not every broken
rule; containment-budget coefficients are still a first cut. Phase 5
remaining (§9.1): Sandbox reuses the commission slate's UI rather than
getting its own screen/reference panel; no new pacing content was authored,
only verification of the existing curve. Phase 6 remaining (§10.1): the
size sweep's perf bound is the same generous debug-build sanity check every
earlier phase used, not the plan's own native-release ~100ms budget (a
`--release` benchmark, not attempted here); the tuning-loop CLI prints to
stdout only, with no in-repo corpus samples yet to replay against beyond
what a developer captures locally via the in-game "Capture Sample" button.*
