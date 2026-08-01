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
  (14-point) resampling.
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

- Parser tests for rune interpretation, malformed glyphs, enclosure detection,
  and invalid-diagram recovery.
- Move commission delivery, test results, accident chances, and research
  unlocks into pure reducers with accounting tests.
- Recipe-discovery fixtures verifying ledger entries, reputation, coins,
  insight, and archive-tier progression together.
- Separate drawing input capture from enchantment evaluation so slate rendering
  cannot alter commission logic.
