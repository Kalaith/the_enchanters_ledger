//! Renders a `PerfectDiagram` to standalone SVG for the generated manual page.
//!
//! The viewBox deliberately matches the drawing slate's own proportions rather
//! than being square: slate coordinates are normalized, so a circle the
//! recognizer scores as perfectly round is drawn as a wide ellipse, and the
//! manual has to show the shape the player is actually meant to draw.

use crate::perfect_diagram::PerfectDiagram;
use crate::rune_drawing::DrawnStroke;
use std::fmt::Write;

/// Slate proportions, from `ui::draw_drafting_panel`'s layout.
const VIEW_WIDTH: f32 = 520.0;
const VIEW_HEIGHT: f32 = 410.0;
const GRID_COLUMNS: usize = 9;
const GRID_ROWS: usize = 5;

const PAGE: &str = "#c4a873";
const GRID: &str = "#61401e";
const INK: &str = "#0c0705";

pub fn diagram_svg(diagram: &PerfectDiagram) -> String {
    let mut svg = String::new();
    let _ = write!(
        svg,
        "<svg class=\"diagram\" viewBox=\"0 0 {VIEW_WIDTH:.0} {VIEW_HEIGHT:.0}\" \
         xmlns=\"http://www.w3.org/2000/svg\" role=\"img\">"
    );
    let _ = write!(
        svg,
        "<rect width=\"{VIEW_WIDTH:.0}\" height=\"{VIEW_HEIGHT:.0}\" rx=\"6\" fill=\"{PAGE}\"/>"
    );
    write_grid(&mut svg);

    let _ = write!(
        svg,
        "<g fill=\"none\" stroke=\"{INK}\" stroke-width=\"3.4\" \
         stroke-linecap=\"round\" stroke-linejoin=\"round\">"
    );
    for stroke in diagram.strokes() {
        write_stroke(&mut svg, &stroke);
    }
    svg.push_str("</g></svg>");
    svg
}

fn write_grid(svg: &mut String) {
    let _ = write!(
        svg,
        "<g stroke=\"{GRID}\" stroke-width=\"1\" opacity=\"0.22\">"
    );
    for column in 1..GRID_COLUMNS {
        let x = VIEW_WIDTH * column as f32 / GRID_COLUMNS as f32;
        let _ = write!(
            svg,
            "<line x1=\"{x:.1}\" y1=\"8\" x2=\"{x:.1}\" y2=\"{:.1}\"/>",
            VIEW_HEIGHT - 8.0
        );
    }
    for row in 1..GRID_ROWS {
        let y = VIEW_HEIGHT * row as f32 / GRID_ROWS as f32;
        let _ = write!(
            svg,
            "<line x1=\"8\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\"/>",
            VIEW_WIDTH - 8.0
        );
    }
    svg.push_str("</g>");
}

fn write_stroke(svg: &mut String, stroke: &DrawnStroke) {
    if stroke.points.len() < 2 {
        return;
    }
    svg.push_str("<polyline points=\"");
    for (index, point) in stroke.points.iter().enumerate() {
        if index > 0 {
            svg.push(' ');
        }
        let _ = write!(
            svg,
            "{:.1},{:.1}",
            point.x * VIEW_WIDTH,
            point.y * VIEW_HEIGHT
        );
    }
    svg.push_str("\"/>");
}
