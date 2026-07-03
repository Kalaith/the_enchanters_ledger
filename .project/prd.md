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
> through Phase 2. It documents the system **as it exists today**: Hungarian
> stroke assignment, open-stroke corner fix, eraser-fragment merge,
> strict/identity variant parity, deterministic tie-breaks, mismatch-segment
> alignment (D3), a continuous (not stepped) ambiguity penalty and corner
> confidence, capture-time stroke canonicalization (A8), rune templates
> moved to `assets/data/rune_templates.json`, a real size/completeness
> magnitude channel (`potency`, §4), and the D1/D2 scoring-stack cleanup
> (layout-position bias deleted, circle quality now additive not stacked).
> It is not a design proposal — proposals live in the plan document's
> Phase 3–5 sections (Phase 1's
> generic/data-driven *structural checks* — as opposed to template shapes,
> which are already data-driven — remain future work; see §2.3). This doc
> is also the intended source for in-game journal/tutorial copy once
> that's written.

---

## 1. Overview

### 1.1 Problem statement

The player enchants items by freehand-drawing runes inside a working circle.
The drawing has to be readable by a recognizer with no hand-authored
per-drawing hints, robust to ordinary hand-drawing noise, and — eventually —
scale from a 3-symbol fireball to a 100+-symbol grand diagram (see the plan
document's goal statement). Today's system covers single-symbol recognition
and small structured circles solidly; it does not yet scale to hundreds of
symbols or reward size/length as a magnitude channel. Both are tracked in
the plan (Phases 2–4), not here.

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
data-driven yet is the *structural check* layer below: eight of 33 runes
have a hand-written check in `shape_report_for_rune`
(`src/rune_drawing/shape.rs:52`) that multiplies into their score:

| Rune | Structural check | Corner target | Notes |
|---|---|---|---|
| `sphere` | closure × circularity, corner penalty above 6 corners | n/a (round) | `sphere_report` |
| `safer` | closure, side count, straightness | 6 (±4 tolerance) | hexagon; `safer_report` |
| `force` | closure, side count, straightness | 4 (±3 tolerance) | diamond; `diamond_report` |
| `touch` | arrow/shaft structure | — | `touch_report` |
| `beam` / `aura` / `burst` / `cone` | shape-specific structure | — | see `shape.rs` |

**Deferred:** generalizing this table into data-driven rules (per plan item
3 — "generic feature checks... computed from the template itself; delete
the per-rune match arms") did not happen this pass. `sphere`/`safer`/
`force`/`cone` reduce fairly cleanly to `{closed, corner_range}`, but
`touch`/`beam`/`aura`/`burst` use bespoke geometry (arrow-tip detection,
horizontal-bar crossing, radial-spoke angles) that doesn't fit a
5-parameter generic model without a real rule DSL — a separate design
task, not a mechanical data move like the template shapes above were.

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
Position semantics may return in Phase 4's spell grammar, where placement
(e.g. a modifier between an effect and its ring) can mean something taught
to the player — not before.

---

## 5. Circle grammar — structured diagrams

`interpret_diagram()` (`src/rune_diagram.rs`) reads a full drawing as: pick
the working circle → classify every other stroke as rune ink or a structure
mark → cluster rune ink into rune groups → recognize each cluster →
summarize into a spell via `analyze_magical_circle()`
(`src/magical_circle.rs`).

### 5.1 Working circle

- Must be drawn as a **single stroke** today (Phase 3 item 1 — chaining
  multi-arc circles — is not implemented). Candidate strokes need ≥8
  points and a span ≥0.22 with width/height ≥0.15 to even be considered
  (`circle.rs:49-75`).
- `circle_quality` is a weighted sum (`circle.rs`): closure 32%, aspect
  18%, center-proximity 4%, radius consistency 22%, angular coverage 18%,
  top-start bias 8%. `circle_found` requires `circle_quality ≥
  MIN_CIRCLE_QUALITY (0.32)`. When several closed strokes are candidates,
  the one enclosing the most other ink wins, tie-broken by meeting the
  quality floor, then span, then raw quality.
- A candidate rune only counts as "inside" the circle if (a) its center,
  normalized to the circle's elliptical radii, satisfies `nx² + ny² ≤
  1.25`, **and** (b) its width and height are each under `0.92×` the
  circle's corresponding dimension (`is_inside_working_circle`). A rune
  drawn large enough to nearly fill the circle fails (b) and is silently
  dropped rather than scored low (D5, tracked as a paper cut, not fixed
  here).

### 5.2 Structure marks vs. rune ink

Every non-rune stroke is first classified by its geometry relative to the
circle (`classify_circle_stroke`, `magical_circle.rs:134-195`), using
`orbit` (distance from center, normalized to the circle's radius) and
`scale` (stroke size relative to the circle):

| Classification | Condition |
|---|---|
| `ReinforcementRing` | closed, `orbit ≤ 0.15`, `scale` 0.28–0.92 |
| `SatelliteSeal` | closed, ≥12 points, `orbit` 0.22–0.78, `scale` 0.055–0.24 |
| `PerimeterMark` | `orbit` 0.78–1.10, `scale ≤ 0.13`, short |
| `ScriptMark` | `orbit` 0.18–0.88, `scale ≤ 0.075`, very short |
| `RadialSpoke` | directness > 0.86, spans center → ring |
| `RuneInk` | none of the above |

A mark only actually counts as circle *structure* (and gets excluded from
rune clustering) once its kind clears a population threshold
(`is_circle_structure`, `rune_diagram.rs:186-217`): satellites need ≥3 at
quality > 0.68; rings need ≥2 at quality > 0.48; scripts need ≥8 at quality
> 0.42; radials need ≥6 at quality > 0.68. Below threshold, the marks are
just left as unrecognized ink, not reclassified as runes.

`MIN_RUNE_SCALE_IN_CIRCLE = 0.12` (`rune_diagram/recognition.rs:9`) is the
separate, harder floor applied when a cluster *is* being scored as a rune:
if its scale relative to the circle is under 12%, it's rejected outright
regardless of recognition confidence. Between this floor and the
`ScriptMark` classification catching most small marks first, **dense
diagrams with many small runes are not possible today** (C1/C2) — this is
the single largest blocker to the "100+ symbols" goal and is Phase 3's job,
not Phase 0's. Rune-level clustering itself (grouping strokes that belong
to the same rune) uses fixed absolute thresholds, not scale-relative ones:
`MAX_CLUSTER_STROKE_DISTANCE = 0.045` (segment-to-segment) and
`MAX_CLUSTER_CENTER_DISTANCE = 0.09` (bounding-box centers) — this is C3,
also Phase 3 scope.

### 5.3 Spell stat blob

`analyze_magical_circle()` (`magical_circle.rs`) requires either
`rings + satellites + radials + perimeter + scripts ≥ 2` or a rune of tier
≥4 present, else there's no circle spell at all. It produces `complexity`,
`intensity`, `containment` (each a weighted sum of circle quality, average
rune quality, and the structure-count ratios below), plus the raw counts,
then looks up a spell name/tier keyed on the dominant effect rune (highest
tier × quality × scale among Effect-category runes) plus these counts.

Complexity, containment, and intensity are each measured against **fixed
targets** (`ratio_count`): rings/**3**, satellites/**5**, radials/**4**,
perimeter marks/**14**, script marks/**28**, rune count/**6** — once a
diagram exceeds a target, additional work on that axis contributes nothing
further (C6). This is why a 100-symbol diagram cannot currently out-score a
well-drawn 30-symbol one; Phase 4's diminishing-returns amplifier model
replaces this fixed-target cap.

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

### 5.5 What circle grammar does *not* yet have

- **Sub-scopes / nested circles.** A circle drawn inside a circle isn't
  interpreted as a nested composite glyph — this is the structural unlock
  Phase 3 item 2 targets, needed to express a "volcano"-scale diagram
  hierarchically instead of flatly.
- **Compositional semantics.** There's no spell grammar today — a named
  spell is a stat-blob-plus-lookup, not a data-defined predicate over
  effects/shape/trigger/structure. Phase 4 scope.
- **The *full* containment budget rule.** §5.4 now has a first-cut version
  (total potency vs. a capacity from circle quality + containment). Phase
  4's job is to replace the rough baseline/coefficients with a properly
  balanced version, likely tied into the spell-grammar's structure
  requirements rather than a flat additive formula.

---

## 6. Constants index

Every tunable constant that governs the rules above, in one place, so a
tuning change always has a specific sentence in this document to update
alongside it.

| Constant | Value | File | Governs |
|---|---|---|---|
| `MIN_RECOGNITION_CONFIDENCE` | 0.32 | `rune_drawing.rs` | §2.4 acceptance floor |
| `MIN_RECOGNITION_MARGIN` | 0.04 | `rune_drawing.rs` | §2.4 ambiguity band |
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
| `MIN_RUNE_SCALE_IN_CIRCLE` | 0.12 | `rune_diagram/recognition.rs` | §5.2 |
| `MAX_CLUSTER_STROKE_DISTANCE` | 0.045 | `rune_diagram/geometry.rs` | §5.2 clustering (absolute, not scale-relative — C3) |
| `MAX_CLUSTER_CENTER_DISTANCE` | 0.09 | `rune_diagram/geometry.rs` | §5.2 clustering |
| structure-mark population thresholds | satellites ≥3 @ q>0.68, rings ≥2 @ q>0.48, scripts ≥8 @ q>0.42, radials ≥6 @ q>0.68 | `rune_diagram.rs` | §5.2 |
| complexity/intensity/containment targets | rings 3, satellites 5, radials 4, perimeter 14, scripts 28, runes 6 | `magical_circle.rs` | §5.3 |
| tier thresholds | tier4: complexity≥0.72 + tier≥4 rune + ≥2 rings + ≥3 satellites + ≥6 perimeter; tier3: complexity≥0.61 or tier≥4 rune; tier2: complexity≥0.48 | `magical_circle.rs` | §5.3 |
| `RuneCategory::ideal_scale_in_circle` | Effect 0.18, Shape 0.15, Trigger 0.14, Modifier 0.12 | `data.rs` | §4.1 potency's size reference, also `size_harmony` |
| potency scale curve | (0.5× ratio → 0.6), (1.0× → 1.0), (2.0× → 1.6), slopes continue past anchors | `rune_diagram/recognition.rs` | §4.2 |
| potency ink_ratio factor | `ink_ratio.clamp(0.5, 1.0)` | `rune_diagram/recognition.rs` | §4.2 |
| potency final clamp | `[0.35, 2.2]` | `rune_diagram/recognition.rs` | §4.2 |
| circle-quality score contribution | `(circle_quality - 0.55) × {20 stability, 14 safety, 10 score}`, additive once per evaluation | `state/evaluate.rs` | §5.4 (D2) |
| power/mana potency scaling | `power × potency`, `mana_cost's base × potency` (on top of quality's own effect) | `state/evaluate.rs` | §5.4 |
| containment budget | `capacity = 2.0 + circle_quality×6.0 + containment×4.0`; excess potency costs `×10.0` stability | `state/evaluate.rs` | §5.4 (rough first cut) |
| `evaluate()` score cap | `68 + avg_quality × 52` | `state/evaluate.rs` | §5.4 |
| grade thresholds | Unstable: stability<42 or safety<32 or score<64; Brilliant: score≥92 and stability≥68 and safety≥48 | `state/evaluate.rs` | §5.4 |

---

## 7. Verification

- **`src/rune_drawing/confusion_gate.rs`** — every rune template plus 10
  deterministic perturbations (translate, scale, sparse/dense resample,
  seeded jitter) must recognize as itself and be accepted, unless the
  specific (truth, predicted) pair is in `KNOWN_CONFUSIONS` (§2.3). Down to
  one tracked entry as of Phase 1 (was four before the corner-confidence
  softening).
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
  guards D1.
- **`src/state/tests.rs`** — `doubling_effect_rune_size_raises_power_in_report`
  is the exit criterion at the `evaluate()` level (not just `InterpretedRune`).

Any change to a constant in §6 should keep all of the above green, or the
affected sentence in this document (and, if it's a tracked confusion, the
`KNOWN_CONFUSIONS` list) should be updated in the same change.

---

*Last updated: 2026-07-03 — Phase 0, Phase 1, and Phase 2 of `.project/magic-symbol-system-plan.md`.
Phase 1 remaining: generalize the structural-check layer (§2.3) into
data-driven rules. Phase 2 remaining: none of the plan's four items are
outstanding, but the containment budget (§5.4) is explicitly a rough first
cut for Phase 4 to properly balance, not a tuned final version.*
