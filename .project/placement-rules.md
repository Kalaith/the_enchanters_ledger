# Placement rules

Status: **implemented** (`src/reading/`), closing out `prd.md` §4.4, which
deleted the old `layout_quality` position bias and reserved position semantics
for the spell grammar, "taught to the player — not before."

Two things moved during implementation; both are marked **[shipped]** below.

The model is **relative**: neighbours carry meaning, absolute direction does
not. Rotating a whole diagram produces the identical working. Light on the left
and Light on the right are the same mark; Light drawn *beside Safer* is not the
same as Light drawn alone.

The goal is not a recipe list. It is that a player who has drawn a hundred
circles can look at one and read it aloud before pressing Interpret.

---

## 1. What position already means (today, before any change)

Worth stating, because three of these are real and one is an accident.

| Channel | Where | What it does |
|---|---|---|
| Radial distance (`orbit`) | `magical_circle::classify_circle_stroke` | Decides what a stroke **is**: reinforcement ring ≤0.15, satellite seal 0.22–0.78, script mark 0.18–0.88, perimeter tick 0.78–1.10. A `sphere` rune and a satellite seal are the same shape; only orbit separates them. |
| Radial distance | `magical_circle::size_harmony` | Soft bonus: shapes rewarded near center, modifiers ≈0.55, effects/triggers ≈0.42. 6% of complexity. Untaught. |
| Angle, in aggregate | `magical_circle::circular_symmetry` | Rewards anchors whose unit vectors cancel — even spread. No individual angle matters. |
| Nesting | `rune_diagram::scope` | A rune inside a sub-circle belongs to that scope. The only true placement-changes-meaning rule in the game. |
| **Grid distance (accidental)** | `state::apply_interpreted_runes` → `link_quality` | Runes are sorted top-to-bottom, dropped on a hidden 5×4 grid, and chained in that order. Links charge mana per unit of distance and dock stability/safety past 2 cells. **Spreading marks out already costs the player, invisibly, through a grid they never see.** |

Absolute angle means nothing anywhere.

---

## 2. The reading rule

**A circle reads clockwise. You start at the top and go round.**

The top is the teaching convention — where a player begins reading aloud, and
where the generator starts laying marks out. **No rule in this first cut depends
on it.** Every rule below is defined on adjacency and angular gaps, both of which
survive rotation. That is what makes "left" and "right" meaningless on their own
while still giving the circle a direction to be read in.

Direction is deliberately unused for now. §7 reserves it.

### 2.1 Bands

A scope's ink divides by orbit into three bands. These are not new machinery —
they are the bands `classify_circle_stroke` already uses, named and made
meaningful for runes:

| Band | Orbit | Meaning |
|---|---|---|
| **Heart** | ≤ 0.25 | The default for the sentence. A mark here applies to the whole working. |
| **Ring** | 0.25 – 0.75 | The working itself. Marks here take part in the reading. |
| **Rim** | > 0.75 | The binding to the outside. Perimeter marks live here. |

> **Heart marks define defaults. Ring marks drawn together create exceptions.**

That is the whole of it, and it is the sentence to teach. A Sphere in the heart
means *everything here is a sphere*; a Beam pulled in beside Fire means *except
the fire, which is a beam*.

Heart marks have no neighbours and take no partners — they already apply to
everything.

This converges with a constraint the engine already has. A `sphere` drawn out on
the ring gets classified as a satellite seal and swept out of rune recognition
once three seals are present — which is exactly why `perfect_diagram` already
moves round runes to the heart in structured diagrams. "Shapes belong in the
heart" is a rule the recognizer was already enforcing by accident; this makes it
the taught rule, and gives a player a reason to break it on purpose.

---

## 3. Drawing marks together — the core rule

> **Two ring marks are joined when they are drawn deliberately close: their
> angular gap is under two thirds of the mean gap between ring marks in that
> scope.**

**[shipped: two thirds, not half.]** Half turned out to be geometrically
unreachable. Two marks cannot be drawn closer than about 1.6 of their own spans
without `cluster_strokes` merging them into a single mark, which on a four-mark
ring at reference size is a floor of 0.92 rad against a mean gap of 1.57 — a
fraction of 0.58. A half-gap rule could never have been satisfied by any diagram
of four marks or more; the rule would have existed and been impossible to use.
Two thirds clears the floor for three and four marks at reference size. Five or
more have to be drawn smaller before they can say anything about grouping, which
is the same trade a crowded diagram already makes.

Everything else follows from this one sentence.

**Why the mean-gap test and not a fixed angle.** It is scale-free (works with
three marks or nine), it is what a player can *see* ("those two are touching,
those are spread"), and above all it makes the whole grammar **opt-in**:

- Evenly spaced marks are never joined. Every diagram that exists today — every
  quest reference, every ladder rung, anything a player has already drawn —
  keeps its current meaning exactly.
- Drawing marks together is a deliberate act: you pull them in to say they
  belong together.

Nothing breaks on the day this ships. That is the property worth protecting, and
§9 turns it into an enforced test rather than a claim.

Beginners will space everything evenly and get today's game. Experts cluster
around the marks that matter. The drawing itself becomes expressive.

### 3.1 Groups

A run of mutually-joined marks is a **group**. Groups, not individual marks, are
what the rest of the system reasons about (§5.1). A diagram of three tight pairs
is three groups, not six marks.

### 3.2 What a group means

| Mark | Alone (spread) | In a group |
|---|---|---|
| **Modifier** | Applies to the whole working, as today | Applies to one mark alone — see §3.3 |
| **Shape** | Shapes the whole working | Shapes only the mark it was drawn with |
| **Effect** | Independent effect | Reserved — see §7 |
| **Trigger** | Whole working, always | Position is free |

Triggers being position-free is deliberate. A grammar where every category
behaves positionally gives a player four things to track at once; one where the
trigger is exempt gives them a fixed point to reason from. A trigger says *when
the working acts*, and there is only one working.

### 3.3 Modifiers

> **A mark may carry any number of modifiers. A modifier attaches to exactly
> one mark.**

That keeps the grammar simple and the drawing legible: a cluster of small marks
crowded around one rune is unambiguous, and reads as one phrase.

    Light
    ├── Safer
    ├── Stronger
    └── Longer Duration     ->  "a brighter, safer, longer-lasting light"

Mechanically: **a modifier attaches to the nearest non-modifier mark in its
group.** Nearest by angular gap, not strictly the next one clockwise — freehand
placement is imprecise, and a modifier tucked just behind a rune should read the
same as one tucked just ahead. Exact ties break clockwise.

Two consequences worth stating:

- Modifiers stacked outward from a rune all attach to that rune, not to each
  other. The chain in the sketch above is three separate attachments, not a
  chain of three.
- A group containing no non-modifier mark (two modifiers drawn together, alone)
  attaches nothing. Both stay global. There is nothing there to modify.

---

## 4. The voice of the reading

**The player never sees the word "binding", or "group", or "adjacent".** Those
are this document's words and the code's words. What the player sees is their
own diagram, read back to them as a sentence.

    Interpretation

    The Light has been tempered.
    The Sphere surrounds the entire working.
    The enchantment activates continuously.

and, for a different diagram:

    Interpretation

    Fire is projected as a beam.
    Frost surrounds the entire enchantment.
    The working activates when struck.

A player who draws the same four marks a second time, pulled together
differently, gets a different sentence. That is how the rule is taught — not by
a tooltip, but by the game reading their handwriting back to them. Eventually
they predict the sentence before pressing Interpret, and that is where mastery
lives.

### 4.1 No live feedback

Nothing highlights while the pen is down. No preview of what will attach to
what, no lines drawn between marks, no hover state. The reveal happens once, at
Interpret, and it has to be unambiguous.

The uncertainty is the mechanic. "…wait, Safer only affected the Light?" is the
moment the game is made of, and it only exists if the player commits to a
drawing before finding out.

This also means the readout carries the entire teaching burden, so it has to be
exact: every mark accounted for, every phrase naming what it acted on.

### 4.2 Phrasing is data, not code

Each rune needs its own wording — how it reads as a whole-working effect, how it
reads attached to something, and how it reads as a modifier applied to another
mark. That belongs in `assets/data/runes.json` alongside the rune's stats, per
the project's data-driven rule. No sentence templates hardcoded in Rust, and no
`match` on rune ids.

Roughly: a rune carries the phrases it can appear in; the reader assembles one
line per group from the phrases of the marks in it. Adding a rune stays a JSON
edit, including how it reads.

---

## 5. What has to change to make room for this

These are the load-bearing interactions. Each is a real problem, not a detail.

### 5.1 `circular_symmetry` currently punishes the grammar

Symmetry rewards anchors whose unit vectors cancel — i.e. even spread. The
grammar asks the player to cluster. As written, using the grammar costs the
symmetry bonus, which is exactly backwards.

**Fix:** compute balance over **group centroids** rather than individual marks.
Three tight groups spaced evenly around the circle then score full symmetry, as
they should — the diagram *is* balanced, it just has three anchors instead of
six.

### 5.2 The hidden grid must go

`node_for_diagram_center` → chain-by-reading-order → `link_quality` distance
costs is the accidental rule from §1. It charges mana and stability for spread,
in an order the player cannot see, on a grid that exists only in the save file.
Once grouping is real, this is the same idea done properly.

**Fix:** links come from groups, not grid adjacency. `link_quality`'s existing
category bonuses — (Effect, Shape) +10, (Shape, Trigger) +10, (Effect, Modifier)
+5 safety — stay, and finally mean something: they fire when those marks were
actually drawn together, which is a thing the player did on purpose. Distance
costs disappear; spread is free again, as PRD §4.4 intended.

This is the single biggest cleanup in the proposal. It removes an untaught
penalty and replaces it with a taught reward, using scoring that already exists.

### 5.3 `size_harmony` folds into bands

Its orbit preferences (§1) become the band rule. Keep it as a soft nudge toward
the band a category belongs in, or retire it — but it should not be a second,
differently-shaped opinion about the same thing.

### 5.4 The generator has to place groups

`perfect_diagram` spreads everything evenly, so under these rules it produces
diagrams with nothing joined — correct, and identical to today. To express a
grouping it needs to cluster deliberately: `DiagramRequest` gained `groups`, and
the layout gives each group one slot on the ring and steps its marks apart
inside it by the smallest gap the recognizer still reads as two marks.

**[shipped: grouped diagrams are drawn a little smaller.]** The two bounds — close
enough for the reading, far enough for the recognizer — nearly meet at reference
size, leaving no margin for either. A diagram that groups anything is laid out at
85% and takes the widest orbit available, which buys real headroom on both sides.
A hand crowding a phrase together does the same thing.

The quest diagrams that would change: any commission with an optional modifier
becomes a design choice — temper the effect, or temper the whole working.

### 5.5 Teaching surfaces

- The slate readout (§4) is the primary one.
- The rune guide gains a line per category: what this mark does alone, and what
  it does drawn against another.
- Manual pages show the grouping in the diagram and read the sentence out
  underneath it.
- The practice ladder gains its natural next axis of difficulty: later rungs
  demand a specific reading, not just more marks.

---

## 6. Worked example

The opening commission — Light, Sphere, Continuous, with Safer as the bonus.

**Spread evenly** (what the reference lays out today):

    Four marks at 90°. No gap beats half the mean gap, so nothing joins.

    "The working is tempered.
     The Sphere surrounds the entire working.
     The enchantment activates continuously."

**Safer pulled in beside Light:**

    Light and Safer 20° apart; Sphere and Continuous left spread.

    "The Light has been tempered.
     The Sphere surrounds the entire working.
     The enchantment activates continuously."

    The orb and the trigger are untouched: a lantern that cannot flare,
    on a working that still runs at full strength.

**Safer pulled in beside Continuous:**

    "The Light burns at full strength.
     The Sphere surrounds the entire working.
     The enchantment activates continuously, and gently."

Same four marks, three different lanterns, three different sentences. That is the
decision the grammar buys, and it costs a player nothing to ignore.

---

## 7. Reserved for later: order

Once adjacency is understood, direction is the natural next layer. The reading
order (§2) already exists to support it:

    Light -> Fire      "ignite the light"
    Fire  -> Light     "produce light from fire"

Two effects drawn together would read as a directed pair rather than a blend.
**Not in the first cut** — it is a second grammar to teach on top of one players
are still learning, and adjacency has to be fluent first. This is why §3.2
leaves effect-with-effect reserved rather than defining it as a symmetric blend:
defining it now would have to be undone to make room for direction later.

---

## 8. Open questions

1. **Effect and effect, before order arrives.** Until §7 lands, two effects
   drawn together could stay independent (simplest, and preserves the meaning of
   every diagram) or compound their potency (uses the grouping, but is a rule
   that direction would later replace). Recommend independent.
2. **Retire the 5×4 board entirely?** §5.2 removes its scoring role. It is still
   the save format for `placed`, and the ledger panel draws it. Replacing it is
   a bigger change than this proposal needs.
3. **Sub-scopes**: does a mark join across a sub-scope boundary? Recommend no —
   a scope is a sentence; marks in different sentences do not join.

---

## 9. Test obligations

Every rule above needs a test, in the style the recognizer rules already use:

- Rotating a diagram by an arbitrary angle produces an identical interpretation.
- Evenly spaced marks join nothing — asserted against every existing quest
  reference and ladder rung, so the compatibility claim in §3 is enforced, not
  hoped for.
- A modifier pulled beside an effect attaches to that effect and not its
  neighbours; moving it to another mark changes what it attaches to.
- Several modifiers crowded around one mark all attach to that mark.
- A shape drawn against one effect shapes that effect; the same shape in the
  heart shapes everything.
- Symmetry of three tight groups equals symmetry of six spread marks (§5.1).
- Spread no longer costs mana (§5.2).
- Every recognized mark appears in exactly one line of the readout — nothing is
  silently dropped from the sentence (§4.1).
