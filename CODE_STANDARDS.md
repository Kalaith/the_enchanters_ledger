# Rust Coding Standards for Macroquad Games

**Engine**: Macroquad + macroquad-toolkit  
**Language**: Rust  
**Platform**: WebGL (WASM) + Native

This document defines the coding standards for Macroquad game projects. Copy this template when starting a new game and customize as needed.

These standards prioritize:  
- Readability over cleverness  
- Data-driven design over hardcoded values  
- Clean state management  
- Modular services for game logic  
- A clear mental model for game phases and transitions  

## 1. Core Philosophy

### 1.1 Write for Maintainability
Code should be easy to debug and extend.  
- Prefer obvious, straightforward code  
- Avoid hidden state or side effects  
- If a junior Rust developer can understand the flow, you are doing it right.

### 1.2 Consistency Beats Preference
If a pattern already exists in the codebase, follow it even if you dislike it. A consistent codebase is more valuable than a perfect one.

### 1.3 Data-Driven Design
All game constants, balance values, and static data should be defined in JSON files under `assets/`. Load this data at startup using Serde for easy balancing and iteration without recompiling code. Avoid hardcoding values in Rust code; reference loaded data structures instead.

For WebGL compatibility, static JSON should usually be embedded with `include_str!()` and parsed with Serde. Runtime-replaceable JSON must use `macroquad::prelude::load_string(...).await` on `wasm32`.

### 1.4 No Unused Code
- Remove unused variables, fields, and functions immediately
- Never suppress unused warnings with `_` prefixes on struct fields
- If a field is unused, delete it - don't mark it as unused
- Parameter prefixes with `_` are acceptable only when required by trait signatures

## 2. Project Structure Rules

### 2.1 Module Responsibilities
Each module/subdirectory owns a single conceptual domain:

**Root Level:**
- `main.rs` – Entry point, game loop, phase transitions, and high-level coordination

**Subdirectories:**
- `data/` – Data structures and JSON loading
  - Type definitions for game entities
  - Constants and configuration structures

- `engine/` – Game logic services (stateless where possible)
  - Core game calculations
  - Entity management and state machines
  - Visual effects (particles, transitions)

- `state/` – Game state management
  - Current game state
  - Persistent player progression
  - Save/load functionality

- `ui/` – User interface components
  - Base UI utilities and styling
  - Reusable UI widgets
  - Uses macroquad-toolkit for buttons and interactions

- `screens/` – Screen-specific rendering (if separated from main.rs)

**Cross-Domain Rules:**
- ❌ UI must never mutate game state directly
- ❌ Engine services should be stateless - receive state, return results
- ❌ Data module has no knowledge of engine or UI
- ✅ All domains can read from `data/` types
- ✅ State mutations happen only in main.rs via clearly defined actions

### 2.2 File Size Guideline
- Target: 200–400 lines per file
- Soft limit: 600 lines
- Hard limit: 800 lines (main.rs excepted for game loop complexity)
- If a file grows beyond this, split by responsibility.

### 2.3 Folder Structure

```
game_name/
├── Cargo.toml              # Project manifest
├── CODE_STANDARDS.md       # This file
├── src/
│   ├── main.rs             # Entry point, game loop, screen rendering
│   ├── data/               # Data types and loading
│   │   ├── mod.rs          # Re-exports all data types
│   │   ├── loader.rs       # JSON deserialization
│   │   └── constants.rs    # Game constants structures
│   ├── engine/             # Game logic services
│   │   ├── mod.rs          # Re-exports
│   │   └── game_engine.rs  # Core calculations
│   ├── state/              # State management
│   │   ├── mod.rs
│   │   ├── game_state.rs   # Current game state
│   │   └── persistence.rs  # Save/load
│   ├── ui/                 # UI components
│   │   ├── mod.rs
│   │   ├── core.rs
│   │   └── components.rs
│   └── screens/            # Screen renderers (optional)
├── assets/                 # Game data
│   ├── constants.json      # Balance values
│   └── localization/       # Text strings
└── .gitignore
```

## 3. Naming Conventions

### 3.1 General Rules
- Types: PascalCase  
- Functions & variables: snake_case  
- Constants: SCREAMING_SNAKE_CASE  
- Modules: snake_case  

Names should describe what the thing is, not how it works.

### 3.2 Boolean Naming
Booleans should read like facts:  
```rust
is_active  
can_interact  
has_unlocked  
should_update  
```  
Avoid `flag`, `value`, or `state` in names.

### 3.3 Service Naming
Engine services follow a naming pattern:
- `*Service` for stateless helpers
- `*Engine` for complex stateless processors
- `*StateMachine` for state progressions

## 4. Functions & Methods

### 4.1 Function Size
- Target: 20–50 lines  
- Absolute max: 100 lines  
- If a function needs scrolling, it probably needs refactoring.

### 4.2 Single Responsibility
Each function should answer one question or perform one action.

### 4.3 Argument Count
- Prefer ≤ 3 parameters  
- If more are needed, use a struct or reference to state  
- Services should take `&GameState` or `&Config` rather than many individual fields

### 4.4 Return Types
- Use `Option<T>` for potentially missing values  
- Use custom result structs for complex outcomes
- Avoid returning multiple values via tuple; create a named struct instead

## 5. Data & State Management

### 5.1 Game State Ownership
- `GameState` owns the current game state  
- `PlayerStats` owns persistent progression  
- Mutation happens through methods on `Game` struct in main.rs  
- Services return results; they don't mutate state directly  

### 5.2 Prefer Plain Data
Use structs with clear fields. Avoid overly clever enums with embedded logic unless they model a real state machine.  

Game data should be:  
- Serializable (Serde-friendly for save/load)  
- Easy to debug and inspect  
- Immutable after loading from JSON  

### 5.3 Data-Driven Design
- All game balance and configuration in JSON under `assets/`
- Load data at application startup; data is embedded at compile time unless it must be replaceable after build
- Use structs that mirror JSON structure for type safety
- Never hardcode magic numbers; reference loaded config data

### 5.4 Filesystem, Assets, and WebGL
- `std::fs` is native-only. Do not use `std::fs::read_to_string`, `std::fs::read_dir`, `std::fs::write`, or path existence checks from code that compiles for `wasm32-unknown-unknown`.
- For static JSON, use `include_str!()` plus Serde. This is the safest default for config, balance, text, and manifests.
- For runtime browser assets, use Macroquad async APIs: `load_string(...).await`, `load_texture(...).await`, `load_file(...).await`, or macroquad-toolkit asset loaders.
- For save files, use macroquad-toolkit persistence helpers. They use files on native builds and localStorage on WebGL builds.
- If desktop-only filesystem access is required, put both imports and call sites behind `#[cfg(not(target_arch = "wasm32"))]` and provide a browser fallback.

### 5.5 Enums for Game Phases
Use enums to model distinct game states:
```rust
pub enum GamePhase {
    Loading,
    MainMenu,
    Playing,
    Paused,
    GameOver,
    // Add game-specific phases
}
```

## 6. Error Handling

### 6.1 Prefer Option Over Panics
- `panic!` is acceptable only for truly unrecoverable states  
- Missing entities or items should return `None`, not panic  
- Use:  
  - `Option<T>` for potentially missing values  
  - `Result<T, E>` for fallible I/O operations (save/load)  
  - Graceful degradation for missing data  

### 6.2 Logging Over Silent Failures
Use `eprintln!` for error conditions that should be visible during development but shouldn't crash the game.

## 7. UI Code (Macroquad-Toolkit)

### 7.1 UI Is Dumb
UI code:  
- Reads game state  
- Returns actions/intents  
- It should never contain game logic.  

### 7.2 Action Pattern
UI components return `Option<UiAction>` to signal user intent:
```rust
pub enum UiAction {
    StartGame,
    Pause,
    Resume,
    // Add game-specific actions
}
```

### 7.3 Component Organization
- `core.rs` – Color schemes, fonts, base styling  
- `components.rs` – Reusable widgets  
- Each component is a pure function: `fn draw_thing(state: &State) -> Option<UiAction>`

### 7.4 Macroquad-Toolkit Usage

This project uses `macroquad-toolkit` for common UI patterns. Import via `use ui::*;` which re-exports all toolkit modules.

**Available Modules:**
- `ui::button()` – Standard clickable button (fires on release)
- `ui::button_on_press()` – Button that fires on mouse down
- `ui::button_styled()` – Button with custom styling
- `ui::panel()` – Draws a panel with optional title
- `ui::progress_bar()` – Progress indicator
- `ui::colors::dark::*` – Standard dark theme colors
- `ui::input::*` – Mouse/keyboard input helpers

**Button Click Semantics:**
```rust
// Standard button - fires on mouse RELEASE (safer, allows cancel)
if button(x, y, w, h, "Click Me") {
    return UiAction::DoThing;
}

// Press button - fires on mouse DOWN (instant feedback)
if button_on_press(x, y, w, h, "Emergency", &style) {
    // Immediate action
}
```

**Color Palette:**
```rust
use macroquad_toolkit::colors::dark;

clear_background(dark::BACKGROUND);  // Standard background
draw_rectangle(x, y, w, h, dark::PANEL);  // Panel color
draw_text("Hello", x, y, 20.0, dark::TEXT);  // Text color
// Also: dark::ACCENT, dark::POSITIVE, dark::WARNING, dark::NEGATIVE
```

**Input Helpers:**
```rust
use ui::input::*;

if is_hovered(x, y, w, h) { /* Mouse over area */ }
if was_clicked(x, y, w, h) { /* Left click released on area */ }
if was_pressed(x, y, w, h) { /* Left click pressed on area */ }
```

## 8. Deployment & Web Standards

### 8.1 Required Files
Every game must have these files for deployment:
- `publish.ps1` – Build and deploy script
- `index.html` – WebGL host page

### 8.2 Build Targets
The game must build for:
- **Windows**: `cargo build --release`
- **Web/WASM**: `cargo build --release --target wasm32-unknown-unknown`

### 8.3 WebGL Requirements
The `index.html` must:
- Load `mq_js_bundle.js` (Miniquad loader)
- Call `load("game_name.wasm")`
- Include canvas with `id="glcanvas"`
- Use `image-rendering: pixelated` for pixel art

The Rust code must also avoid unsupported browser APIs:
- No uncfg'd `std::fs` access in startup, asset loading, saves, or cache code
- No uncfg'd `std::time::Instant::now()`; use `macroquad::time::get_time()` or frame `dt` for WebGL timers
- All async browser asset loads must happen before the first screen depends on the data

## 9. Comments & Documentation

### 9.1 Comment Why, Not What
Code already explains what it does. Comments should explain why it exists.

### 9.2 Module-Level Docs
Each module should contain a short `//!` comment explaining its purpose:
```rust
//! Player inventory and item effects.
```

## 10. Formatting & Tooling

### 10.1 rustfmt
- Always use `cargo fmt`  
- Never fight the formatter  

### 10.2 Clippy
- Run `cargo clippy` regularly  
- Fix warnings unless intentionally ignored  
- Document any `#[allow]` with a comment

### 10.3 Variable Shadowing
- Avoid variable shadowing (hiding)
- Do not declare a new variable with the same name as an existing one in the same scope

### 10.4 Unused Code
- Remove unused variables immediately
- Remove unused struct fields immediately  
- Never use `_` prefix on struct fields to suppress warnings
- `_` prefix on function parameters is acceptable when required by API

## 11. Testing Guidelines

### 11.1 What to Test
Focus tests on:  
- Core game calculations  
- State machine transitions  
- JSON data loading  
- UI and rendering generally do not need unit tests.

### 11.2 Test Style
- Tests should read like rules  
- Avoid complex setups  
- If a test is hard to write, the code is probably too tangled.

## 12. Final Rule

If a piece of code feels fragile, confusing, or brittle, it probably is. Refactor early. Leave the code calmer than you found it.
