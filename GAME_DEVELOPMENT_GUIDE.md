# Rust Game Development Guide

**Engine**: Macroquad + macroquad-toolkit  
**Language**: Rust (Edition 2021)  
**Platform**: WebGL (WASM) + Native Windows

This guide covers both creating new games and migrating existing web applications to standalone Rust games.

---

## Quick Start

### New Game Setup

```bash
# Recommended: copy the toolkit-ready template crate.
cp -r template my_game
cd my_game
```

Then rename the package in `Cargo.toml`, update `assets/data/game_config.json`,
and change the wasm filename in `index.html`.

### Dependencies (`Cargo.toml`)

```toml
[package]
name = "my_game"
version = "0.1.0"
edition = "2021"

[dependencies]
macroquad = "0.4"
macroquad-toolkit = { path = "../macroquad-toolkit" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

> **Note**: Profile settings (`[profile.release]`) are defined at the workspace root.

---

## Architecture Overview

### Web to Rust Migration Map

| Feature | Web (Old) | Rust (New) |
| :--- | :--- | :--- |
| **Frontend** | React/DOM/CSS | Macroquad (Canvas, Immediate UI) |
| **Backend** | PHP/Node | Rust internal logic |
| **Database** | MySQL | JSON files or SQLite |
| **Styling** | CSS | Rust constants/functions |

### Tech Stack Philosophy

**Use Macroquad for:**
- Rendering (shapes, textures, text)
- Input handling (keyboard, mouse)
- Audio playback
- Main loop timing

**Do NOT use Macroquad for:**
- Scene management (use state machines)
- Game state authority (use your own structs)
- UI framework (use immediate-mode from macroquad-toolkit)

> Macroquad should remain a *thin* rendering/input layer.

---

## Project Structure

```
game_name/
├── Cargo.toml
├── CODE_STANDARDS.md       # Coding standards
├── publish.ps1             # Build & deploy script
├── index.html              # WebGL host page
├── src/
│   ├── main.rs             # Entry point, window config
│   ├── game.rs             # Game loop & state machine
│   ├── state/              # Game states (one per screen)
│   │   ├── mod.rs
│   │   ├── menu.rs
│   │   └── gameplay.rs
│   ├── engine/             # Game logic (stateless)
│   │   ├── mod.rs
│   │   └── game_engine.rs
│   ├── data/               # Data types & embedded toolkit loaders
│   │   ├── mod.rs
│   │   └── loader.rs
│   ├── ui/                 # UI helpers
│   │   └── mod.rs
├── assets/
│   ├── data/
│   │   ├── game_config.json
│   │   ├── actions.json
│   │   └── texture_manifest.json
│   └── images/             # Sprites
└── README.md
```

---

## Core Patterns

### Entry Point (`main.rs`)

```rust
use macroquad::prelude::*;

mod game;
mod state;
mod data;

use game::Game;

fn window_conf() -> Conf {
    Conf {
        window_title: "Game Name".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new().await;
    
    loop {
        clear_background(Color::from_rgba(20, 20, 25, 255));
        game.update();
        game.draw();
        next_frame().await;
    }
}
```

### State Machine Pattern

```rust
// state/mod.rs
pub enum GameState {
    Menu(MenuState),
    Gameplay(GameplayState),
    Results(ResultState),
}

pub enum StateTransition {
    ToMenu,
    ToGameplay(GameplayState),
    ToResults(ResultState),
}
```

**Rules:**
- Only ONE state active at a time
- Transitions are explicit (no magic callbacks)
- No shared mutable global state

### Individual State Pattern

```rust
pub struct GameplayState {
    // State-specific data
}

impl GameplayState {
    pub fn new() -> Self { ... }
    
    pub fn update(&mut self) -> Option<StateTransition> {
        // Return None to stay, Some(transition) to change
    }
    
    pub fn draw(&self, textures: &HashMap<String, Texture2D>) {
        // Render this state
    }
}
```

### Game Struct (`game.rs`)

```rust
pub struct Game {
    pub state: GameState,
    pub textures: HashMap<String, Texture2D>,
}

impl Game {
    pub async fn new() -> Self { ... }
    
    pub fn update(&mut self) {
        // Match on current state, call state.update()
        // Handle StateTransition return values
    }
    
    pub fn draw(&self) {
        // Match on current state, call state.draw()
    }
    
    pub fn transition(&mut self, transition: StateTransition) {
        // Apply explicit state change
    }
}
```

---

## UI: Immediate Mode

### Layout (Replacing CSS Flexbox)

**React (CSS):**
```css
.container { display: flex; justify-content: center; }
```

**Rust:**
```rust
let center_x = screen_width() / 2.0;
let button_w = 200.0;
let start_x = center_x - button_w / 2.0;
let mut y = 100.0;
const PADDING: f32 = 20.0;

if button(start_x, y, button_w, 50.0, "Start Game") {
    // Handle click
}
y += 50.0 + PADDING;
```

### UI Philosophy

```rust
fn draw_button(x: f32, y: f32, text: &str) -> bool {
    let rect = Rect::new(x, y, 200.0, 40.0);
    let hovered = rect.contains(mouse_position().into());
    let clicked = hovered && is_mouse_button_pressed(MouseButton::Left);
    
    let color = if hovered { LIGHTGRAY } else { GRAY };
    draw_rectangle(x, y, 200.0, 40.0, color);
    draw_text(text, x + 10.0, y + 28.0, 24.0, WHITE);
    
    clicked
}
```

**Rules:**
- UI reads state, returns intents (bools/enums)
- UI never contains game logic
- Game logic applies changes

---

## Data Loading

WebGL builds cannot read deployed `assets/` through `std::fs`. For game data,
prefer embedded JSON with `include_str!()` when the data is static, or
`macroquad::prelude::load_string(...).await` when the browser should fetch a
runtime asset.

Use `std::fs` only inside native-only code:

```rust
#[cfg(not(target_arch = "wasm32"))]
let json = std::fs::read_to_string("assets/cards.json")?;
```

Do not call `std::fs::read_to_string`, `std::fs::read_dir`,
`std::fs::write`, or `std::time::Instant::now()` from code that compiles for
`wasm32-unknown-unknown`. Use Macroquad asset/time APIs, embedded data, or
macroquad-toolkit persistence instead.

### JSON Definition (`assets/cards.json`)

```json
[
  {
    "id": "strike",
    "name": "Strike",
    "cost": 1,
    "description": "Deal 6 damage",
    "effects": [{ "Damage": 6 }]
  }
]
```

### Loader (`data/loader.rs`)

```rust
use serde::{Deserialize, Serialize};

const CARDS_JSON: &str = include_str!("../../assets/cards.json");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardData {
    pub id: String,
    pub name: String,
    pub cost: i32,
    pub description: String,
    pub effects: Vec<CardEffect>,
}

impl CardData {
    pub fn load_all() -> Result<Vec<CardData>, Box<dyn std::error::Error>> {
        let cards: Vec<CardData> = serde_json::from_str(CARDS_JSON)?;
        Ok(cards)
    }
}
```

If the data must remain replaceable after compiling, load it asynchronously:

```rust
#[cfg(target_arch = "wasm32")]
use macroquad::prelude::load_string;
use serde::{Deserialize, Serialize};

const CARDS_JSON: &str = include_str!("../../assets/cards.json");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardData {
    pub id: String,
    pub name: String,
}

pub async fn load_cards() -> Result<Vec<CardData>, Box<dyn std::error::Error>> {
    #[cfg(target_arch = "wasm32")]
    let json = load_string("assets/cards.json")
        .await
        .unwrap_or_else(|_| CARDS_JSON.to_string());

    #[cfg(not(target_arch = "wasm32"))]
    let json =
        std::fs::read_to_string("assets/cards.json").unwrap_or_else(|_| CARDS_JSON.to_string());

    Ok(serde_json::from_str(&json)?)
}
```

---

## Persistence (Save/Load)

Use `macroquad-toolkit` save slots. They write atomic JSON files on native builds
and localStorage-compatible data on WASM builds.

```rust
use macroquad_toolkit::persistence::{
    save_to_slot_with_version,
    load_from_slot_with_migration,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: String,
    pub points: i64,
}

let save = SaveData {
    version: "1.0.0".to_string(),
    points: 100,
};

save_to_slot_with_version("my_game", "autosave", &save, "1.0.0")?;

let loaded: SaveData = load_from_slot_with_migration(
    "my_game",
    "autosave",
    "1.0.0",
    |old_version, value| {
        // Convert old save JSON into the current SaveData.
        let points = value
            .get("data")
            .and_then(|data| data.get("points"))
            .or_else(|| value.get("points"))
            .and_then(|value| value.as_i64())
            .unwrap_or_default();

        Ok(SaveData {
            version: "1.0.0".to_string(),
            points,
        })
    },
)?;
```

### SQLite (For Complex Data)

Use `rusqlite` if migrating a complex MySQL database:

```rust
use rusqlite::{Connection, Result};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn init() -> Result<Self> {
        let conn = Connection::open("game_data.db")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS player_stats (
                id INTEGER PRIMARY KEY,
                gold INTEGER NOT NULL
            )",
            (),
        )?;
        Ok(Self { conn })
    }
}
```

---

## Deployment

### Required Files

Every game MUST have:
- `publish.ps1` – Build and deploy script
- `index.html` – WebGL host page

### Build Targets

```bash
# Windows release
cargo build --release

# WebGL/WASM
cargo build --release --target wasm32-unknown-unknown
```

### Web Template (`index.html`)

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>My Game | Web Hatchery</title>
    <link rel="stylesheet" href="../shared.css">
    <style>
        .game-container canvas {
            image-rendering: pixelated;
        }
    </style>
</head>
<body>
    <div class="game-page">
        <canvas id="glcanvas" tabindex="1"></canvas>
    </div>
    <script src="mq_js_bundle.js"></script>
    <script>load("my_game.wasm");</script>
</body>
</html>
```

---

## Future Image Prompts

Use a JSON catalog for managing placeholder-to-generated-image transitions.

### Catalog (`assets/image_prompts.json`)

```json
{
  "player_idle": {
    "prompt": "A futuristic space marine standing idle, pixel art style",
    "filename": "player_idle.png",
    "width": 64,
    "height": 64
  }
}
```

> **Important**: `width` and `height` must be divisible by 16.

**Workflow:**
1. **Define**: Add assets to `image_prompts.json`
2. **Develop**: Game uses placeholder if file missing
3. **Generate**: Create images from prompts
4. **Deploy**: Place images in `assets/`, game picks them up

---

## Checklists

### New Game

1. [ ] `cargo new game_name`
2. [ ] Add dependencies to `Cargo.toml`
3. [ ] Create folder structure (`src/state/`, `src/data/`, etc.)
4. [ ] Implement `GameState` and `StateTransition` enums
5. [ ] Create `Game` struct with update/draw loop
6. [ ] Set up `assets/` folder
7. [ ] Copy `publish.ps1` from template
8. [ ] Create `index.html` with correct WASM filename
9. [ ] Keep or adapt the template save/load system

### Migration (Web → Rust)

1. [ ] Define Rust structs for game entities
2. [ ] Set up `macroquad::main` entry point
3. [ ] Copy `publish.ps1` and `index.html` from template
4. [ ] Port PHP/backend logic to Rust functions
5. [ ] Rebuild React UI using immediate-mode
6. [ ] Migrate MySQL data to JSON or SQLite
7. [ ] Wire UI to modify game state

---

## Non-Goals

- ❌ No ECS overengineering
- ❌ No custom editor tooling (initially)
- ❌ No procedural generation until core stable

> **Simplicity is a feature.**
