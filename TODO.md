# TODO — The Enchanter's Ledger

All six phases of the magic-symbol-system plan are closed out. The rules
themselves live in `.project/prd.md`; what follows is what that work
deliberately left open, plus outstanding test and structure work.

## Recognizer

- Collect real human corpus samples. The capture path works end to end
  (practice slate → "Capture Sample" → `tests/corpus/<rune>/*.json` → gate
  test), but `tests/corpus/` is still empty, so `src/corpus.rs` is a no-op.
- Beam-search segmentation for overlapping and contaminated clusters, replacing
  the window-based `extract_overlapped_spheres` /
  `recover_contaminated_multi_stroke_rune` recovery.
- Close out the one tracked confusion — `("safer", "sphere")` under sparse
  (14-point) resampling. Measured, not yet fixed: the cause is *not* only the
  resample-grid shift the `confusion_gate` comment describes. `corner_count` is
  not scale-free. Sweeping the input density through
  `shape::geometry::corner_count` gives, for the `safer` hexagon, 6.7 corners on
  the raw 7-point template but 4.0 at 14 points; for the `sphere` template, 3.0
  at 24+ points but **7.9 at 14 and 10.9 at 18** — a sparsely sampled circle is
  a coarse polygon, and its per-vertex turns clear the corner threshold. So the
  measure reports a hexagon as rounder than a circle at exactly the density the
  gate tests. Two consequences: `safer`'s `corners` check (target 6, tolerance
  4) collapses to ~0.48, and `sphere`'s `corner_penalty` (`above: 5.0`, so it
  needs ≥ 6.0) never fires on a hexagon — nothing in `sphere`'s spec can tell a
  hexagon from a circle, because `circularity` is radius-consistency-based and a
  regular hexagon has near-constant radius. Replacing the three-point turn
  stencil with turning accumulated over the window fixes the density dependence
  above ~18 points but not below, because at ±2 steps of 36 the uniform turning
  of *any* closed curve (0.70 rad) already exceeds the 0.60 threshold. The fix
  wants a corner measure defined as turning *concentrated* relative to the
  shape's own uniform rate, which reshuffles every rune's corner target in
  `rune_templates.json` — a piece of work in its own right, gated by
  `confusion_gate`.
- `sound` and `larger` cannot be read inside a diagram at all. Both recognize
  perfectly on their own, but their component strokes sit far enough apart
  (relative to their own size) that `rune_diagram::geometry::cluster_strokes`
  never groups them: `sound`'s three chevrons split three ways, and `larger`'s
  box and four detached arrow ticks split five ways with the box alone reading
  as `sphere`. The clustering thresholds are size-relative, so no scale a player
  can draw at helps — it needs either template geometry that clusters or a
  grouping rule that spans the gap. Tracked by
  `perfect_diagram::tests::fragmenting_runes_still_fragment`, which fails once
  either is fixed. Affects the `alarm_bell` commission (`sound`) and
  `spark_sign`'s optional `larger`.

## Performance

- Release-mode perf gate. CI asserts a 5 s debug sanity bound; the target is
  100 ms native / 250 ms wasm, which needs a `--release` benchmark job.
- Per-cluster result cache and dirty-region re-interpretation. Not needed at
  current sizes (300-symbol diagrams interpret comfortably) — revisit if
  freehand play pushes past that.

## Placement grammar

`.project/placement-rules.md` is implemented (`src/reading/`); what it
deliberately left for later:

- Direction between two effects (`Light -> Fire` vs `Fire -> Light`), reserved
  in §7. Adjacency has to be fluent first, and defining effect-with-effect now
  would have to be undone to make room for it.
- Ladder rungs that demand a specific reading rather than just a set of marks —
  the grammar is the ladder's natural next axis of difficulty.

## Balance & UI

- Tune the containment budget. Root and sub-scope capacity coefficients are
  still the documented first cut.
- ~~Per-rune potency itemization on the slate; it shows only the average
  today.~~ Done — `ui::drawing::feedback::draw_potency_tags` tags each read rune
  with its own figure, banded weak / reference / strong against the potency
  curve's 1.0 anchor. The ledger row now says "avg potency" for the summary.

## Tests & structure

- ~~Parser tests for rune interpretation, malformed glyphs, enclosure detection,
  and invalid-diagram recovery.~~ Done — `rune_diagram::tests::parsing` covers
  all four: the output contract (ordering, value bands), inkless and zero-extent
  strokes and off-slate coordinates, both non-radius guards in
  `is_inside_working_circle`, and the empty / bare-circle / circle-less
  interpretations.
- Move commission delivery, test results, accident chances, and research
  unlocks into pure reducers with accounting tests.
- ~~Recipe-discovery fixtures verifying ledger entries, reputation, coins,
  insight, and archive-tier progression together.~~ Done —
  `state::tests::discovery` walks one run from first test through delivery to
  research: the ledger row and its `uses`/`best_score` counters, the single
  journal entry, coins/reputation/insight moving by exactly what the report
  claims, a failed grade recording nothing, and accumulated insight buying the
  rank that unlocks `fire`.
- Separate drawing input capture from enchantment evaluation so slate rendering
  cannot alter commission logic.
