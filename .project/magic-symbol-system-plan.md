# Magic Symbol System — Review & Expansion Plan

> Goal statement (from the project owner):
> 1. **Robust & predictable** — the same drawing must always read the same way; minor
>    adjustments must not flip the result.
> 2. **Magnitude channels** — symbols have a draw order and length; a larger symbol means a
>    larger effect, a shorter line a weaker one.
> 3. **Story mode** doable by the average person, with small aids to ease progression.
> 4. **Full freehand diagrams** with *hundreds* of symbols that, if the rules are followed,
>    produce an effect — fireball ≈ 3–4 symbols, volcano ≈ 100+.

---

## Part 1 — Review of the current system

### Architecture today

```
mouse → DrawnStroke (0..1 slate space, ≥0.004pt spacing)
  → recognize_rune()            per-symbol identity  (rune_drawing/)
  → strict_quality_for_rune()   draw order / start point   (rune_quality.rs)
  → interpret_diagram()         circle select → structure classify → cluster → recognize
                                (rune_diagram/, magical_circle.rs)
  → analyze_magical_circle()    stat blob: complexity/intensity/containment + name lookup
  → evaluate()                  places runes on 5×4 board; quality → power/stability/score
```

Strengths worth keeping:
- Normalization + arc-length resampling core is sound and mostly deterministic.
- Order-insensitive *identity* + order-sensitive *quality* split is the right instinct.
- The circle-structure vocabulary (rings, satellites, radial spokes, perimeter/script
  marks) is a great foundation for grand diagrams.
- `rune_diagnostics.rs` is an excellent debugging asset.
- Tests exist with rough/jittered samples, not just clean templates.

### Issues, ordered by how hard they block the goals

#### A. Predictability & robustness (Goal 1)

- **A1. Greedy stroke↔template pairing is tie-unstable.**
  `match_strokes_order_insensitive` (scoring.rs:158) sorts all pairs by similarity and
  assigns first-come. Near-equal similarities (common on symmetric runes like `light`,
  `burst`, `frost`) mean a 1-pixel wiggle can flip which drawn stroke maps to which
  template stroke, changing the total score discontinuously.
- **A2. Recognition depends on inter-symbol draw order.**
  `remaining_stroke_groups` (rune_diagram/recognition.rs:199) splits leftover strokes by
  *original stroke index adjacency*. Draw stroke 1 of symbol A, then symbol B, then finish
  symbol A → different grouping → different reading. Violates "same drawing, same result".
- **A3. Hard-threshold cliffs.** Corner detection (`turn > 0.68`), `safer` corner window
  (5..=8), the ambiguity band (`gap < 0.04 → ×0.92`), closure cutoffs (0.70/0.72) — all
  produce flip-flop behavior right at the boundary. A hexagon drawn slightly rounder
  toggles between `safer` accepted and `sphere` misread.
- **A4. `corner_count` treats open strokes as cyclic** (shape.rs:407 wraps indices), so
  open polylines get phantom corners at their endpoints.
- **A5. Special-case sprawl.** Only 8 of 33 runes have structural reports; per-rune-id
  multipliers in `adjusted_score_for_rune`; hand-added variants for `touch`/`continuous`;
  `extract_overlapped_spheres` and `recover_contaminated_multi_stroke_rune` are band-aids
  over clustering failures. Every new rune needs bespoke Rust; behavior is unpredictable
  because each rune plays by different rules.
- **A6. Identity and quality disagree.** `recognize_rune` scores against *all template
  variants*, but `strict_quality_for_rune` compares only the canonical template — a legal
  variant drawing gets full confidence but punished quality.
- **A7. Eraser splits strokes into chunks**, so a repaired symbol carries a stroke-count
  penalty forever.
- **A8. Input density is device-dependent.** Points are filtered at 0.004 spacing at
  capture; resampling mostly hides this, but corner/closure features run on raw or
  36-sample data — frame rate and mouse speed leak into scores.

#### B. Magnitude channels (Goal 2)

- **B1. Size is discarded at rune level.** Everything is normalized to a unit box (correct
  for identity), and `InterpretedRune.scale` *is* captured — but `evaluate()` consumes only
  `quality`. Only the dominant effect's scale (clamped at 0.35) nudges spell `intensity`.
  "Larger symbol → larger effect" effectively doesn't exist per rune.
- **B2. No stroke-length magnitude.** `total_length` is only compared post-normalization
  as a weak identity signal. "Shorter line → lower effect" has no implementation; an
  under-drawn stroke just loses confidence.
- **B3. Draw order affects quality only ~30%** (via the strict blend) and direction
  reversal is entirely free in identity matching. Fine as a default, but it is not a
  designed, documented rule — it's an emergent constant.

#### C. Scale to hundreds of symbols (Goal 4) — currently impossible

- **C1. `MIN_RUNE_SCALE_IN_CIRCLE = 0.12`** rejects any rune smaller than 12% of the
  circle. A 100-symbol diagram needs runes at ~3–6% scale.
- **C2. Small strokes are eaten by structure classification.** Anything with scale ≤0.075
  and modest length becomes a `ScriptMark` (magical_circle.rs:165) and is filtered out
  before rune recognition. Small runes *cannot exist*.
- **C3. Clustering thresholds are absolute** (0.045 / 0.09 in slate space), not relative
  to symbol size — dense diagrams merge neighboring symbols into one blob cluster.
- **C4. Quadratic hot paths.** `cluster_strokes` is O(n²) stroke pairs × O(p²) segment
  pairs; `best_recovery_window` is O(windows²) full recognitions; each recognition scores
  all 33 runes × variants and re-runs `strict_quality`. Hundreds of strokes will hang the
  interpret button.
- **C5. One circle, one stroke.** The working circle must be a single stroke; a circle
  drawn as two arcs fails entirely. No nested circles / sub-scopes, so hierarchical grand
  diagrams (the natural way to structure a volcano) can't be expressed.
- **C6. Complexity is capped by fixed targets** (rings/3, satellites/5, radials/4,
  perimeter/14, scripts/28, runes/6). Beyond those counts extra work contributes nothing —
  a 100+ symbol diagram cannot out-score a 30-symbol one.
- **C7. No compositional semantics.** The spell is a stat blob + a name lookup keyed on
  the dominant effect. There is no grammar in which "volcano" could be *defined*, no way
  repetition or sub-circles amplify an effect by rule.

#### D. Paper cuts / design debt

- **D1.** `layout_quality` hard-codes category home positions (effect left, trigger right,
  modifier bottom) and silently docks up to 24% — undermines freehand layouts and is
  documented nowhere.
- **D2.** Final rune quality = recognition quality × circle quality × layout, clamped to a
  0.20 floor — multiplicative stacking makes quality opaque to the player.
- **D3.** `mismatch_segments_for_rune` zips drawn strokes to template strokes *in order*
  with no alignment — mismatch highlights are wrong whenever order differs.
- **D4.** `.project/prd.md` is an empty template. The magic language has no written
  ruleset; the code *is* the spec, which makes every tuning change a gamble.
- **D5.** `is_inside_working_circle` requires rune bounds < 0.92 × circle dims — a large
  centered shape rune silently vanishes.

---

## Part 2 — The plan

Phases are ordered so that each one de-risks the next. Phase 0 must land first; after
that 1→2 and 3→4 are the two dependency chains (they can interleave).

### Phase 0 — Ground truth & test harness (foundation, do first)

The single biggest risk is tuning blind. Before changing the recognizer, build the safety
net that makes every later phase verifiable.

1. **In-game capture to corpus.** Debug key on the slate that serializes the current
   strokes (+ intended rune / diagram label typed or picked) to
   `tests/corpus/<rune>/<name>.json`. `DrawnStroke` is already serde-ready. Grow a corpus
   of *real human* drawings — templates jittered in code are not enough.
2. **Confusion-matrix gate.** Test that renders every rune template plus N deterministic
   perturbations (translate, uniform scale 0.5–2×, resample density, seeded jitter) and
   asserts: recognized as itself, `accepted`, and margin ≥ threshold vs. every other rune.
   Output the full 33×33 matrix on failure. This is the regression wall for all tuning.
3. **Property tests (determinism & invariance).**
   - Same input → bit-identical `RecognitionOutcome` (run twice, compare).
   - Translation / uniform-scale / point-density invariance of identity.
   - Monotonicity smoke test: increasing jitter never *raises* quality.
4. **Write the ruleset into `.project/prd.md`.** The magic language spec: categories,
   what identity tolerates, what quality rewards (order, start, direction), what
   magnitude means (size, length), circle grammar (scopes, structure marks, amplifiers).
   Every constant in code should trace to a sentence in this doc. This document is also
   the source for in-game journal/tutorial text later.

*Exit criteria:* corpus capture works; confusion matrix green in CI; prd.md ratified.

### Phase 1 — Deterministic, data-driven single-symbol recognizer

1. **Canonicalize at capture.** When a stroke ends, immediately resample it to fixed
   arc-length density and store *that*. Everything downstream consumes canonical strokes;
   device/framerate dependence (A8) disappears, and results become reproducible from the
   saved stroke data alone.
2. **Optimal assignment instead of greedy.** Replace `match_strokes_order_insensitive`
   with Hungarian assignment on the stroke-similarity matrix (max ~6×6 — trivial cost).
   Kills tie-flipping (A1). Add explicit deterministic tie-breaks (stable ordering by
   template index) everywhere `max_by`/sort is used.
3. **Data-driven rune definitions.** Move templates *and* per-rune metadata into
   `assets/data/rune_templates.json`:
   ```json
   {
     "id": "safer", "strokes": [...], "variants": [[...]],
     "closed": true, "corner_range": [5, 8],
     "confusable_with": {"sphere": 0.42},
     "min_strokes": 1, "max_strokes": 1
   }
   ```
   Generic feature checks (closure, corner count, straightness, directness, symmetry)
   are computed from the template itself; delete the per-rune `match` arms in `shape.rs`,
   `scoring.rs` and `templates.rs` (A5). Adding rune #34 becomes a JSON edit + corpus
   samples, no Rust.
4. **Soften cliffs.** Replace binary corner/closure thresholds with smooth ramps
   (e.g. corner confidence = sigmoid of turn angle), and remove the discontinuous
   ambiguity ×0.92 penalty in favor of a continuous margin-based confidence shaping.
   Where the UI needs a stable readout, add hysteresis: keep the previous reading unless
   the new best beats it by a fixed delta (A3).
5. **Bug fixes:** open-stroke corner wrap (A4); strict quality must score against the
   best-matching *variant*, same as identity (A6); merge eraser-split stroke fragments
   whose endpoints touch before recognition (A7); align mismatch segments via the same
   assignment used for scoring (D3).
6. **Codify order/length rules** per prd.md: identity stays order- and
   direction-insensitive; quality = shape fidelity × order fidelity × direction fidelity ×
   start fidelity with documented weights (B3).

*Exit criteria:* confusion matrix green with margins ≥ old system; corpus quality scores
stable under ±1% jitter; zero per-rune-id branches left in recognition code.

### Phase 2 — Magnitude channels: size & length → effect strength

1. **Add `potency` to `InterpretedRune`.** Computed from:
   - `scale` (already captured) relative to its circle scope, on a documented curve —
     e.g. potency 0.6× at half reference size up to 1.6× at double, soft-clamped;
   - `ink_ratio` = drawn arc length ÷ expected template length *at that scale* —
     under-drawn strokes (short lines) reduce potency before they break identity;
   - `quality` as today.
2. **Two documented bands per rune** (in prd.md + shown in the rune guide UI):
   - *identity band* — what still reads as the rune;
   - *magnitude band* — how strength varies within the identity band.
3. **Wire into `evaluate()`:** per-rune power scales with potency; mana cost scales
   with potency; stability drops when total potency exceeds what containment structure
   supports (sets up Phase 4's budget rule). Remove the double-count where quality is
   multiplied through circle quality *and* layout — replace with additive, inspectable
   terms surfaced in the test report (D2).
4. **Replace `layout_quality` position bias** with a rule that is either promoted to a
   real, taught mechanic in prd.md or deleted (D1). Recommendation: delete for freehand
   scoring; keep placement semantics for the *grammar* (Phase 4) where position can mean
   something (e.g. modifiers between effect and ring modulate it).

*Exit criteria:* drawing the same fireball at 2× size measurably raises power in the test
report; a half-hearted short-stroked rune reads but reports reduced potency; both shown
to the player.

### Phase 3 — Segmentation that scales to hundreds of strokes

1. **Multi-stroke circle assembly.** Chain arcs whose endpoints meet into ring
   candidates, so circles drawn in 2–3 strokes work (C5).
2. **Containment hierarchy.** Build a tree of closed rings by containment; each ring is a
   *scope* interpreted recursively with its own local coordinate frame. A sub-circle is a
   composite glyph from its parent's perspective. This is the structural unlock for
   100+ symbol diagrams (C5, C7) and makes rune scale *relative to its scope*, so small
   runes in satellite seals are first-class (C1).
3. **Scale-relative clustering.** Cluster thresholds proportional to local stroke size
   (median of neighbor bounds) instead of absolute slate units (C3). Use a spatial hash
   grid for neighbor queries; segment-distance only for grid-adjacent pairs → O(n·k)
   instead of O(n²) (C4).
4. **Structure vs. rune ink by geometry, not size.** Classify rings/spokes/perimeter
   marks by their relationship to the ring they sit on (on-ring, radial through center),
   never by absolute scale; drop `MIN_RUNE_SCALE_IN_CIRCLE` in favor of a minimum point
   count (C1, C2).
5. **Pure spatial grouping, no index adjacency.** Delete `remaining_stroke_groups`'s
   draw-order dependence (A2); replace the sphere/contamination band-aids with a bounded
   recognition-guided split/merge: for each spatially coherent cluster, consider the best
   segmentation into known runes (beam search over spatial subsets, not index windows).
6. **Performance budget.** Cheap pre-filters (stroke count, closure, aspect) prune
   template candidates before full scoring; precompute template feature vectors once;
   cache per-cluster results keyed on a hash of canonical strokes and re-interpret only
   dirty regions after an edit. Target: 300-stroke diagram interprets < 100 ms native,
   < 250 ms wasm.

*Exit criteria:* synthetic 150-symbol diagram (generated from templates with jitter)
round-trips: every symbol found, none merged/eaten, within the perf budget; drawing
order shuffles do not change the interpretation.

### Phase 4 — Compositional spell grammar

1. **Compile scopes to a spell tree, not a stat blob.** Each scope yields
   `{effects: [(rune, potency)], shape, trigger, modifiers, amplifiers, containment}`.
   Amplifiers (rings, satellites, radial spokes, script bands, repeated effect runes,
   sub-scopes feeding a parent) multiply potency with *diminishing returns* — no hard
   target caps, so 100 symbols always beats 30 well-drawn ones, sub-linearly (C6).
2. **Containment budget rule** (the core risk mechanic, stated in prd.md): total potency
   must be covered by containment (circle quality, rings, perimeter script, safer runes).
   Exceeding it degrades stability predictably — never randomly.
3. **Recipes as data.** Named spells defined in JSON as predicates over the spell tree:
   ```json
   { "id": "volcano", "tier": 4,
     "requires": { "effect": {"fire": {"min_potency": 8}},
                   "shape": "cone", "trigger": "continuous",
                   "structure": {"rings": 3, "satellites": 4},
                   "sub_scopes": [{"effect": "force", "count": 2}] } }
   ```
   Fireball = 3–4 runes in one scope; volcano = the full hierarchy. The discovery/ledger
   system already keyed on signatures plugs straight into this.
4. **Backfire/side-effect rules derived from which rule was broken** (uncontained power,
   missing trigger, duplicate conflicting effects) — so failures teach the grammar.

*Exit criteria:* fireball and volcano both defined purely in data; hand-drawn volcano
built by following prd.md rules is recognized and named without any code special-case.

### Phase 5 — Progression & aids (story mode, Goal 3)

1. **One recognizer, per-context acceptance bands.** Practice strict, commissions
   moderate, sandbox lenient — thresholds differ, behavior never does. No separate
   "easy recognizer" that would break the predictability promise.
2. **Aids that fade with mastery.** Guide templates and ghost tracing already exist;
   add per-rune mastery (accepted count × mean quality) that gradually reduces guide
   opacity and eventually rewards guide-free drawing with an insight bonus.
3. **Friendlier failure surfacing.** `rune_diagnostics` output is gold — convert its key
   findings into one-line player-facing hints ("The circle never closed", "This mark read
   as Frost, not Light — the diagonals crossed"), driven by the same data-driven feature
   specs from Phase 1.
4. **Story pacing check.** Commissions 1–10 use 3–5 symbol diagrams with guides armed;
   mid-game introduces structure marks; endgame commissions require multi-scope diagrams.
   Verify each story commission is passable drawing at "average person" quality — encode
   as tests using deliberately degraded corpus samples (e.g. 15% jitter must still pass
   early commissions).

### Phase 6 — Verification at scale (continuous)

- Grammar → synthetic diagram generator → jitter → interpret → assert round-trip, for
  sizes 3 → 300, in CI with the perf budget.
- Confusion matrix + corpus + property tests stay green on every change.
- Keep a `cargo run --example` (or debug screen) that replays any corpus/diagram JSON and
  prints the diagnostic log, for fast tuning loops.

---

## Suggested sequencing & effort

| Phase | Depends on | Rough size |
|---|---|---|
| 0 Harness + prd ruleset | — | 2–4 sessions |
| 1 Recognizer overhaul | 0 | 4–6 sessions |
| 2 Magnitude channels | 1 | 2–3 sessions |
| 3 Scalable segmentation | 0 (ideally 1) | 5–8 sessions |
| 4 Spell grammar | 2, 3 | 4–6 sessions |
| 5 Progression & aids | 1 (partial anytime) | 2–4 sessions |
| 6 Scale verification | 3, 4 | ongoing |

Quick wins that can ship independently before/alongside Phase 1:
- Hungarian assignment (A1), open-stroke corner fix (A4), strict-vs-variant fix (A6),
  eraser fragment merge (A7), deterministic tie-breaks — all small, all pure wins.
