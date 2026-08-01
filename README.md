# The Enchanter's Ledger

A Rust + Macroquad crafting game about designing magic without being able to cast it.

The player runs a small enchantment workshop. Customers bring practical, strange, or suspicious requests, and the player solves them by drawing enclosed rune diagrams. The game reads the hand-drawn circle, interprets the runes inside it, and turns the result into a linked working.

## Core Loop

1. Read the current commission.
2. Check unlocked runes in the archive.
3. Draw an enclosing circle with multiple rune marks inside it, then interpret the diagram.
4. Test the diagram for power, stability, mana cost, safety, and accidents.
5. Deliver the product for coins, reputation, and insight.
6. Spend coins and insight on research to unlock more dangerous notation.
7. Discover and record new enchantment recipes in the ledger.

## Controls

- Left mouse: draw enclosed diagrams; click a rune guide entry, then click the slate to place a tracing guide.
- `Reference` (above the slate buttons): lays out the whole ideal diagram for the pinned commission — the working circle plus its required runes, positioned and sized exactly as the recognizer wants to read them — as tracing guides. Deterministic and repeatable. In Sandbox it inks that diagram outright instead.
- Right mouse: hold on the diagram slate to erase ink; right-click an armed or selected guide rune to deselect it.
- `M`: open the diagram manual — every commission and talisman with the diagram that fills it, and a button to lay any of them out on the slate.
- `T`: test the current diagram.
- `D`: deliver the current commission.
- `R`: research the next archive tier.
- `N`: decline the current commission.
- `S` / `L`: save / load.
- `Esc`: clear the drafting page.

## Diagram manual

Every quest's diagram is generated, not authored: `src/perfect_diagram` lays out
the working circle, the runes at the size the reading rewards, and any
structural work the order demands; `src/manual` pairs that with the order's
text. The same entries feed the in-game manual (`M`) and a standalone page:

```powershell
cargo run -- --manual docs/manual   # writes docs/manual/index.html
```

The page is self-contained (inline SVG, no external requests). `cargo test`
asserts every generated diagram actually passes the quest it documents.

The manual also carries a **practice ladder** (`assets/data/ladder.json`): ten
drills that trade quantity for difficulty. Level 1 is ten easy marks around one
circle; each rung drops one mark and adds difficulty elsewhere, down to a single
mark at level 10. Rungs are data — adding one is a JSON edit — and every rung's
diagram is interpreted back through the real recognizer by the tests.

## Placement

Where a mark sits inside the circle changes what it does. Marks drawn
deliberately close are read together: a modifier pulled in beside one effect
tempers that effect alone rather than the whole working, and a shape at the
heart of the circle is the default that ring marks make exceptions to. Absolute
direction means nothing — rotating a diagram produces the identical reading, and
marks spaced evenly join nothing at all, so a diagram drawn without knowing the
rule reads exactly as it always did.

Press Interpret and the game reads your handwriting back to you as sentences.
That is the only place the rule is taught. `.project/placement-rules.md` is the
design; `src/reading/` implements it.

## Design docs

- `.project/prd.md` — the authoritative ruleset for rune recognition and magic
  circles: what a drawing must look like to be read, what makes it read *well*,
  and what a full diagram means. Every recognizer constant should trace back to
  a sentence here, and every sentence back to a test.
- `.project/magic-symbol-system-plan.md` — the six-phase plan that produced the
  system above, with a closed-out implementation audit.
- `tests/corpus/README.md` — how to capture real hand-drawn rune samples.

Remaining work is tracked in `TODO.md`.

## Validation

```powershell
.\publish.ps1
```

