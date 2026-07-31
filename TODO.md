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

## Performance

- Release-mode perf gate. CI asserts a 5 s debug sanity bound; the target is
  100 ms native / 250 ms wasm, which needs a `--release` benchmark job.
- Per-cluster result cache and dirty-region re-interpretation. Not needed at
  current sizes (300-symbol diagrams interpret comfortably) — revisit if
  freehand play pushes past that.

## Balance & UI

- Tune the containment budget. Root and sub-scope capacity coefficients are
  still the documented first cut.
- Per-rune potency itemization on the slate; it shows only the average today.

## Tests & structure

- Parser tests for rune interpretation, malformed glyphs, enclosure detection,
  and invalid-diagram recovery.
- Move commission delivery, test results, accident chances, and research
  unlocks into pure reducers with accounting tests.
- Recipe-discovery fixtures verifying ledger entries, reputation, coins,
  insight, and archive-tier progression together.
- Separate drawing input capture from enchantment evaluation so slate rendering
  cannot alter commission logic.
