# UI Cleanup Plan — Duplication & Overlap

Goal: remove the visible overlap in the "Drawn Diagram" control strip and eliminate
duplicated/competing result surfaces, without changing gameplay behavior.

## Panel geometry (reference)

| Panel            | Rect (x, y, w, h)        | Source |
| ---------------- | ------------------------ | ------ |
| Header           | 6, 6, 1268, 82           | `ui.rs:269` |
| Story Quest      | 10, 104, 338, 498        | `commission.rs:10` |
| Drawn Diagram    | 356, 104, 592, 498       | `ui.rs:361` |
| Rune Guide       | 956, 104, 314, 498       | `rune_guide.rs:11` |
| Ledger (bottom)  | 10, 612, 1260, 104       | `ledger.rs:11` |

Inside the drafting panel the control strip runs x=392..912 (520px wide). Five buttons
occupy x+0..x+480; the status note starts at `controls.x + 490` (=882), leaving ~30px
before the panel edge — the root of the visible "Awaiting inte…" clipping.

## Findings

1. **Control-strip note overlap (visible bug)** — `drawing.rs:267-322`. Note column ~30px wide; text spills past the panel.
2. **Two competing result surfaces** — inline Interpret note (`drawing.rs:267`) vs. ledger "Last Test" (`ledger.rs:81`). Both read as "how good is my diagram."
3. **Redundant stacked headers** — panel title "Drawn Diagram" (`ui.rs:362`) directly above "Diagram Slate" label (`drawing.rs:31`).
4. **Debug/utility controls in the gameplay row** — `Diag` + `Sandbox` share the row with `Interpret`/`Clear Ink` (`drawing.rs:240-265`); direct cause of #1.
5. **"Last Test" text overflow** — `title | Power | Stability | Mana | side_effect` truncates in a 374×42 box (`ledger.rs:96`).
6. **Product name echoed** — same name in Discoveries card and Last Test line (minor).

## Decisions

- **Result surfaces → merge into the ledger.** Remove the inline Interpret note from the drafting panel; the ledger's bottom bar becomes the single home for both interpret and test outcomes.
- **Dev controls → `Diag` is debug-build only; `Sandbox` stays as a small toggle.**

---

## Phase 1 — Free the control strip (fixes #1, #4)

File: `src/ui/drawing.rs`

1. **Delete the inline note block** `drawing.rs:267-322` (the `note_x` / `last_interpretation_note` / `last_diagram` / "Awaiting interpretation." branch). The strip no longer renders diagram feedback.
2. **Gate `Diag` behind `#[cfg(debug_assertions)]`** (`drawing.rs:240-248`). In release builds the button and its `CopyDiagnostics` push are compiled out.
3. **Re-lay the button row** now that the note is gone and Diag may be absent:
   - Primary: `Interpret` (110) · `Clear Ink` (96) · `Guides` (92).
   - `Sandbox` as a smaller toggle (≈76px) at the right end of the row.
   - `Diag` (debug only) tucked after Sandbox.
   - Keep the "Circle + inner runes" guide label on its own line above the row.
4. Confirm no button extends past `controls.right()` (912) and nothing is drawn to the right of the last button.

## Phase 2 — Ledger becomes the single result surface (fixes #2, #5, #6)

File: `src/ui/ledger.rs`

1. **Repurpose the left block** (`draw_last_result`, `ledger.rs:81`) into a two-line "Diagram" status:
   - **Line A — Interpretation:** `Reads: <runes> | circle N% | potency N%` from `session.board.last_diagram` / `last_interpretation_note`, or "Awaiting interpretation." Reuse the formatting currently in `drawing.rs:279-322` (move it here).
   - **Line B — Last Test:** grade badge + `title | Power | Stability | Mana`. Move the long `side_effect` flavor to its own wrapped line, or ellipsize, so it stops truncating mid-word.
2. **Widen the left block** from 374 → ~470 and shift `middle` (Discoveries) right to match (`ledger.rs:19-30`); verify it doesn't collide with the Save/Load/Delete cluster (`right`, width 294).
3. Header labels: relabel to "Interpretation" / "Last Test" so the two lines read as distinct, not duplicated.
4. (#6) Optional: drop the product name from Line B since it's already the Discoveries card + Interpretation line, or keep — low priority.

## Phase 3 — Header dedup + reclaimed space (fixes #3)

File: `src/ui/drawing.rs`, `src/ui.rs`

1. **Remove the inner "Diagram Slate" label** (`drawing.rs:31-36`); the panel title "Drawn Diagram" already names it.
2. Shift the slate up by the reclaimed ~24px (`drawing.rs:24`) so the parchment gains height — helps offset the space the buttons/label reorg used.

## Phase 4 — Verify

- Build: `cargo build` (from `D:/WebHatchery/.cargo-target` target dir per project note).
- Run the app; confirm at Day 2 / Rank 1 tutorial state:
  - No text overflows the drafting panel's right edge.
  - Interpret result and Test result both appear in the ledger, clearly labeled, no truncation.
  - Only gameplay buttons show in a release build (`Diag` absent); Sandbox toggle still works.
- Regression: `cargo test` (UI is immediate-mode; layout isn't unit-tested, but state paths are).

## Out of scope

- Header layout (no overlap found there).
- Rune Guide palette paging (works as-is).
- Restyling/color changes — this pass is layout-only.
