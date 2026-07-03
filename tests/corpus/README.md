# Rune corpus

Real human drawings, grown over time to catch what synthetic
jitter/scale/translate perturbations in
`src/rune_drawing/confusion_gate.rs` can't: actual hand-drawing habits,
hesitation, overshoot, and device-specific noise.

## Capturing a sample

In-game: Practice a rune (Guide -> pick a rune -> Practice), draw it, then
press **Capture Sample**. This serializes the current ink plus the rune
label to JSON.

- Native builds write the file directly to `tests/corpus/<rune_id>/`.
- If the write fails (e.g. a wasm/browser build, or a read-only working
  directory), the JSON is copied to the clipboard instead — paste it into a
  new file at `tests/corpus/<rune_id>/<name>.json` by hand.

## Format

```json
{
  "label": "sphere",
  "strokes": [
    { "points": [ { "x": 0.51, "y": 0.14 }, { "x": 0.53, "y": 0.15 }, ... ] }
  ]
}
```

`label` is the rune id the sample was drawn as (matches `assets/data/runes.json`
ids). `strokes`/`points` are `DrawnStroke`/`StrokePoint` exactly as captured
(0..1 slate space) — see `src/rune_drawing.rs`.

## Using the corpus

`src/corpus.rs`'s `every_corpus_sample_recognizes_as_its_label` test walks
this directory (recursively) and asserts every sample still recognizes as
its `label` and is `accepted` — the same bar as
`confusion_matrix_perturbations_recognize_their_own_rune` in
`src/rune_drawing/confusion_gate.rs`, but for real human ink instead of
synthetic perturbations. It runs as part of `cargo test` and is a no-op
while this directory is empty.
