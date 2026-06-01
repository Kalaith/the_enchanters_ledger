# Macroquad Toolkit

A collection of common utilities for Macroquad game development, extracted from multiple games to reduce duplication and provide consistent patterns.

## Features

- **Input utilities**: Mouse hovering, clicking, rectangle collision detection
- **UI rendering**: Rect/enabled/font-aware buttons, surfaces, panels, text fitting, tooltips, virtual UI scaling, meters, badges
- **Asset management**: Texture loading, font loading, placeholders, and JSON texture manifests
- **Camera2D**: Configurable pan, zoom, bounds, drag button, and keyboard/mouse behavior
- **Event bus**: Generic event system for decoupled game logic
- **Color palettes**: Consistent dark theme colors
- **Sprite system**: Builder pattern for texture rendering with transformations
- **Notifications**: Toast queue, styling, fading, and built-in rendering
- **Grid helpers**: `Grid`, `FlatGrid`, `TilePos`, fog of war, BFS paths, flood fill, line of sight
- **Persistence**: Native atomic JSON saves and WASM/localStorage save slots with version migration helpers
- **Data loading**: Embedded JSON, runtime JSON, registries, and fallback loaders

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
macroquad-toolkit = { path = "../macroquad-toolkit" }
```

The `template/` folder is now a complete starter crate using these modules together.
Run it with:

```powershell
cargo run --manifest-path template/Cargo.toml
```

### Quick Start

```rust
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;

#[macroquad::main("My Game")]
async fn main() {
    let mut assets = AssetManager::new();
    assets.load_texture("player", "assets/player.png").await.ok();

    loop {
        clear_background(dark::BACKGROUND);

        // Draw a button
        if button(10.0, 10.0, 100.0, 40.0, "Click Me") {
            println!("Button clicked!");
        }

        next_frame().await;
    }
}
```

## Modules

### Input (`input` module)

```rust
use macroquad_toolkit::input::*;

// Check if mouse is over a rectangle
if is_hovered(x, y, w, h) {
    // ...
}

// Check if rectangle was clicked (released)
if was_clicked(x, y, w, h) {
    // ...
}

// Check if rectangle was pressed (down)
if was_pressed(x, y, w, h) {
    // ...
}

// Capture input state
let input = InputState::capture();
if input.left_click {
    // ...
}
```

### UI (`ui` module)

```rust
use macroquad::prelude::*;
use macroquad_toolkit::ui::*;

let rect = Rect::new(24.0, 24.0, 180.0, 42.0);

// Rect/enabled button with semantic tone.
if button_rect_tone(rect, "Save", true, ButtonTone::Positive) {
    // Button was clicked
}

let surface = SurfaceStyle::new(dark::PANEL)
    .with_header(34.0, dark::PANEL_HEADER)
    .with_border(1.0, dark::ACCENT);
draw_surface_with_title(rect, Some("Panel"), &surface, TextStyle::new(18.0, dark::TEXT));

meter(Rect::new(24.0, 82.0, 220.0, 24.0), current, max, dark::POSITIVE, Some("Energy"));
```

### Virtual UI Scaling

Use `VirtualUi` when the game should render at a fixed logical resolution while
the browser/native window can resize. The current template draws its UI through this path.

```rust
use macroquad_toolkit::ui::{begin_virtual_ui_frame, end_virtual_ui_frame};

let virtual_ui = begin_virtual_ui_frame(1280.0, 720.0);
let ui_mouse = virtual_ui.mouse_position();

// Draw logical-resolution UI here. For custom virtual controls, compare
// ui_mouse against logical Rects.

end_virtual_ui_frame();
```

### Assets (`assets` module)

```rust
use macroquad_toolkit::assets::AssetManager;

let mut assets = AssetManager::new();

assets.load_texture("player", "assets/player.png").await.ok();

if let Some(tex) = assets.get_texture("player") {
    draw_texture(tex, x, y, WHITE);
}
```

Texture manifest example:

```json
[
  { "key": "player", "path": "assets/images/player.png", "filter": "nearest" }
]
```

```rust
let loaded = assets
    .load_texture_manifest_file("assets/data/texture_manifest.json")
    .await?;
```

For WebGL builds, load runtime assets through Macroquad or toolkit async
loaders. Do not scan or read `assets/` with `std::fs` unless that code is
guarded with `#[cfg(not(target_arch = "wasm32"))]` and has a browser fallback.
For static JSON data, prefer `include_str!()` with the toolkit data-loader
helpers so the data is available in both native and WebGL builds.

### Camera (`camera` module)

```rust
use macroquad_toolkit::camera::{Camera2D, Camera2DConfig, CameraBounds};

let mut camera = Camera2D::with_config(
    vec2(0.0, 0.0),
    1.0,
    Camera2DConfig {
        drag_button: Some(MouseButton::Right),
        bounds: Some(CameraBounds::new(vec2(-500.0, -300.0), vec2(500.0, 300.0))),
        min_zoom: 0.5,
        max_zoom: 2.0,
        ..Default::default()
    },
);

camera.update(get_frame_time(), false);
```

### Grid, BFS, and Flood Fill

```rust
use macroquad_toolkit::grid::{FlatGrid, TilePos};

let mut walkable = FlatGrid::new(10, 10, true);
walkable.set(TilePos::new(4, 4), false);

let path = walkable.bfs_path(
    TilePos::new(0, 0),
    TilePos::new(9, 9),
    false,
    |_, tile| *tile,
);

let reachable = walkable.flood_fill(TilePos::new(0, 0), false, |_, tile| *tile);
```

### Events (`events` module)

```rust
use macroquad_toolkit::events::EventBus;

enum GameEvent {
    PlayerDied,
    EnemySpawned,
}

let mut events = EventBus::new();
events.push(GameEvent::PlayerDied);

// Process events
for event in events.drain() {
    match event {
        GameEvent::PlayerDied => { /* ... */ }
        GameEvent::EnemySpawned => { /* ... */ }
    }
}
```

### Notifications

```rust
use macroquad_toolkit::notifications::{NotificationAnchor, NotificationManager, NotificationRenderConfig};

let mut notifications = NotificationManager::new();
notifications.success("Saved");

notifications.update(get_frame_time());
notifications.draw_with_config(&NotificationRenderConfig {
    anchor: NotificationAnchor::BottomRight,
    ..Default::default()
});
```

### Persistence: Save/Load With Migration

```rust
use macroquad_toolkit::persistence::{
    load_from_slot_with_migration, save_to_slot_with_version,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct SaveData {
    version: String,
    score: i64,
}

let save = SaveData {
    version: "1.0.0".to_string(),
    score: 100,
};

save_to_slot_with_version("my_game", "autosave", &save, "1.0.0")?;

let loaded: SaveData = load_from_slot_with_migration(
    "my_game",
    "autosave",
    "1.0.0",
    |old_version, raw_json| {
        // Convert old save shapes into SaveData here.
        let score = raw_json
            .get("data")
            .and_then(|data| data.get("score"))
            .or_else(|| raw_json.get("score"))
            .and_then(|value| value.as_i64())
            .unwrap_or_default();

        Ok(SaveData {
            version: "1.0.0".to_string(),
            score,
        })
    },
)?;
```

### Colors (`colors` module)

```rust
use macroquad_toolkit::colors::dark;

clear_background(dark::BACKGROUND);
draw_rectangle(x, y, w, h, dark::PANEL);
draw_text("Hello", x, y, 20.0, dark::TEXT);
```

Available colors:
- `BACKGROUND`, `PANEL`, `PANEL_HEADER`
- `TEXT`, `TEXT_BRIGHT`, `TEXT_DIM`
- `ACCENT`, `POSITIVE`, `WARNING`, `NEGATIVE`
- `HOVERED`

### Sprite (`sprite` module)

```rust
use macroquad_toolkit::sprite::Sprite;

let sprite = Sprite::new()
    .with_texture(texture)
    .at(100.0, 100.0)
    .scaled(2.0, 2.0)
    .rotated(0.5)
    .colored(RED);

sprite.draw();
```

## Button Click Semantics

The toolkit provides two button variants to handle different click behaviors:

- **`button()` and `button_on_release()`**: Fire when mouse button is **released** over the button. This is the safer default as it prevents accidental double-clicks and allows users to move the mouse away to cancel.

- **`button_on_press()`**: Fires when mouse button is **pressed down** over the button. Use this for instant feedback scenarios.

## License

This toolkit is extracted from game projects and shared for reuse across multiple games.
