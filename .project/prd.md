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
> Written as Phase 0 of `.project/magic-symbol-system-plan.md`. It documents
> the system **as it exists today** (post quick-wins: Hungarian stroke
> assignment, open-stroke corner fix, eraser-fragment merge, strict/identity
> variant parity). It is not a design proposal — proposals live in the plan
> document's Phase 1–5 sections. This doc is also the intended source for
> in-game journal/tutorial copy once that's written.

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

### 2.3 Structural gates (per-rune, not generic yet)

Eight of 33 runes have a hand-written structural check in
`shape_report_for_rune` (`src/rune_drawing/shape.rs:52`) that multiplies
into their score:

| Rune | Structural check | Corner target | Notes |
|---|---|---|---|
| `sphere` | closure × circularity, corner penalty above 6 corners | n/a (round) | `sphere_report` |
| `safer` | closure, side count, straightness | 6 (±4 tolerance) | hexagon; `safer_report` |
| `force` | closure, side count, straightness | 4 (±3 tolerance) | diamond; `diamond_report` |
| `touch` | arrow/shaft structure | — | `touch_report` |
| `beam` / `aura` / `burst` / `cone` | shape-specific structure | — | see `shape.rs` |

Corner counting (`corner_count`, `shape.rs:407`) resamples a stroke to 36
arc-length points and flags a turn angle over threshold between the sample
2 steps before and 2 steps after each point. **Closed strokes use a 0.60 rad
(~34°) threshold; open strokes use 0.68 rad (~39°)** — closed shapes get a
lower bar because hand-authored closed templates (e.g. the hexagon) can have
corners short of a right angle, while open-stroke corners (arrowheads,
crosses) tend to be sharp by construction. This split exists because at a
single shared 0.68 threshold, two of `safer`'s six hexagon corners fell
below it and the shape read as a 4-corner `force` diamond instead — see
`shape::tests::closed_hexagon_counts_six_corners`.

**Known limitation (tracked, not silently hidden):** `sphere`, `safer`,
`force`, and `summoning` are all close in this scoring space (round or
near-round single-stroke shapes) and can flip into each other under
moderate perturbation (14–20% point-density change, ~1.25–1.35× scale, ~1°
jitter). The specific pairs are enumerated in `confusion_gate.rs`'s
`KNOWN_CONFUSIONS` so *new* confusions fail CI while these tracked ones
don't. Softening this properly is Phase 1 item 4 (smooth corner-confidence
ramps instead of a hard threshold) — this doc will shrink the allowlist
as that work lands, not by re-loosening the gate.

### 2.4 Acceptance

`MIN_RECOGNITION_CONFIDENCE = 0.32` — below this, nothing is accepted
regardless of margin. `MIN_RECOGNITION_MARGIN = 0.04` — if the winner beats
the runner-up by less than this, the read is `ambiguous`; an ambiguous read
is still `accepted` if confidence clears `AMBIGUOUS_ACCEPTANCE_CONFIDENCE =
0.58` (a close-but-confident read), and both confidence and quality are
lightly discounted (×0.92 / ×0.96) when this happens
(`src/rune_drawing.rs:19-21, 173-183`).

### 2.5 Known gap: density dependence (A8)

Corner detection resamples to a **fixed 36 points** regardless of input
density, but very sparse (≤~20 points) or very dense (≥~70 points) input to
a tight-margin shape like `safer`'s hexagon can shift where those 36 samples
land relative to its corners enough to misread. `safer` is excluded from
`property_tests::point_density_does_not_change_identity` for this reason.
Proper fix is Phase 1 item 1 (canonicalize to a fixed arc-length density
*at capture*, not per-check) — this removes the device/framerate leak at
the source instead of chasing it in each shape check.

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

---

## 4. Magnitude — size and length as effect strength

**This is intentionally minimal today** and is Phase 2's job to build out;
documenting the current state so the gap is explicit rather than assumed.

### 4.1 What exists

- `InterpretedRune.scale` is captured (the rune's size relative to its
  circle) but only the *dominant* effect rune's scale feeds spell
  `intensity`, clamped at 0.35 — i.e. size has a small, single-channel
  effect, not a per-rune one.
- `total_length` (post-normalization) is compared as a weak *identity*
  signal (`length_score` in `score_against_template`, weight 0.07) — an
  under-drawn stroke loses a little match confidence, not a documented
  potency reduction.

### 4.2 What's missing (Phase 2 scope, not implemented)

- No per-rune `potency` computed from scale + ink ratio (drawn length ÷
  expected length at that scale).
- No documented identity-band vs magnitude-band split per rune.
- "Larger symbol → larger effect" and "shorter line → weaker effect" are
  not real mechanics yet, only capturable data.

### 4.3 Layout position (a magnitude-adjacent penalty, currently undocumented in-game)

`layout_quality` (`src/rune_diagram/recognition.rs:270-280`) hard-codes
category home positions, relative to the circle's bounding box in 0..1
space: **Effect → (0.30, 0.50)** (left), **Trigger → (0.70, 0.50)** (right),
**Modifier → (0.50, 0.72)** (bottom), **Shape → (0.50, 0.50)** (center, so
Shape runes never take this penalty). `distance_score =
(1 - distance/0.48).clamp(0,1)`, then
`layout = (0.76 + distance_score × 0.24).clamp(0,1)` — a floor of 0.76 and
a ceiling of 1.0, i.e. **exactly a 0–24% penalty band**, applied
multiplicatively (§5.4). This is real, currently-active scoring behavior
that is not taught to the player anywhere. Per the plan (D1), this needs to
be either promoted to a taught mechanic or removed for freehand scoring;
until that decision is made, treat it as a known undocumented penalty, not
a rule to design around.

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

### 5.4 From per-rune quality to score (the multiplicative stack, D2)

Per placed rune, the final stored quality is computed in
`rune_diagram/recognition.rs` (`push_recognized_rune`), **not**
`state/evaluate.rs`:

```
quality = (recognized.quality × circle_quality × layout_quality).clamp(0.20, 1.0)
```

— identity/strict quality (§3), the working circle's own quality (§5.1),
and layout position (§4.3) all multiply together with a **0.20 floor**.
This is D2: three independently-meaningful scores collapse into one opaque
number before the player ever sees it. `evaluate()`
(`src/state/evaluate.rs`) then sums `power`/`stability`/`mana_cost`/`safety`
per placed rune scaled by this quality (e.g.
`power: rune.power × (0.35 + quality × 0.65)`), adds the circle spell's
additive bonuses from §5.3, and caps `score` at `68 + avg_quality × 52`.
Grade thresholds: **Failed** if the request wasn't matched, a core rune is
missing, or `score < 35`; **Unstable** if `stability < 42 || safety < 32 ||
score < 64`; **Brilliant** if `score ≥ 92 && stability ≥ 68 && safety ≥
48`; else **Reliable**. `accident = stability < 26 || safety < 18`.

### 5.5 What circle grammar does *not* yet have

- **Sub-scopes / nested circles.** A circle drawn inside a circle isn't
  interpreted as a nested composite glyph — this is the structural unlock
  Phase 3 item 2 targets, needed to express a "volcano"-scale diagram
  hierarchically instead of flatly.
- **Compositional semantics.** There's no spell grammar today — a named
  spell is a stat-blob-plus-lookup, not a data-defined predicate over
  effects/shape/trigger/structure. Phase 4 scope.
- **A containment budget rule.** Nothing currently checks that drawn
  potency is "covered" by containment structure (circle quality, rings,
  perimeter script). Phase 4 scope — this is meant to be the core risk
  mechanic (uncontained power degrades stability predictably).

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
| corner turn threshold (closed) | 0.60 rad | `rune_drawing/shape.rs` | §2.3 corner detection |
| corner turn threshold (open) | 0.68 rad | `rune_drawing/shape.rs` | §2.3 corner detection |
| corner resample target | 36 points | `rune_drawing/shape.rs` | §2.3, §2.5 density gap |
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
| `layout_quality` home positions | Effect (0.30,0.50), Trigger (0.70,0.50), Modifier (0.50,0.72), Shape (0.50,0.50) | `rune_diagram/recognition.rs` | §4.3 |
| `layout_quality` off-home penalty | `0.76 + (1-dist/0.48).clamp(0,1)×0.24` → 0–24% band | `rune_diagram/recognition.rs` | §4.3 |
| per-rune stored quality floor | 0.20 | `rune_diagram/recognition.rs` | §5.4 (D2) |
| `evaluate()` score cap | `68 + avg_quality × 52` | `state/evaluate.rs` | §5.4 |
| grade thresholds | Unstable: stability<42 or safety<32 or score<64; Brilliant: score≥92 and stability≥68 and safety≥48 | `state/evaluate.rs` | §5.4 |

---

## 7. Verification

- **`src/rune_drawing/confusion_gate.rs`** — every rune template plus 10
  deterministic perturbations (translate, scale, sparse/dense resample,
  seeded jitter) must recognize as itself and be accepted, unless the
  specific (truth, predicted) pair is in `KNOWN_CONFUSIONS` (§2.3).
- **`src/rune_drawing/property_tests.rs`** — determinism (same input twice →
  identical `RecognitionOutcome`), translation/scale invariance, point-
  density invariance (with the §2.5 exception), and a jitter-monotonicity
  smoke test (more jitter should not, on average, raise quality).
- **`tests/corpus/`** — real human drawings, captured in-game via the
  Practice slate's **Capture Sample** button. `src/corpus.rs`'s
  `every_corpus_sample_recognizes_as_its_label` test holds every captured
  sample to the same bar. Empty today; grows as people playtest.

Any change to a constant in §6 should keep all three green, or the affected
sentence in this document (and, if it's a tracked confusion, the
`KNOWN_CONFUSIONS` list) should be updated in the same change.

---

*Last updated: 2026-07-03 — Phase 0 of `.project/magic-symbol-system-plan.md`.*
