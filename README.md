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
- Right mouse: hold on the diagram slate to erase ink; right-click an armed or selected guide rune to deselect it.
- `T`: test the current diagram.
- `D`: deliver the current commission.
- `R`: research the next archive tier.
- `N`: decline the current commission.
- `S` / `L`: save / load.
- `Esc`: clear the drafting page.

## Validation

```powershell
.\publish.ps1
```
# Practical Future Improvements

- Add parser tests for rune interpretation, malformed glyphs, enclosure detection, and invalid diagram recovery.
- Move commission delivery, testing results, accident chances, and research unlocks into pure reducers with accounting tests.
- Add recipe-discovery fixtures that verify ledger entries, reputation, coins, insight, and archive tier progression together.
- Separate drawing input capture from enchantment evaluation so slate rendering cannot alter commission logic.

