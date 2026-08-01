//! Canonical ("perfect") diagram layout.
//!
//! Everything else in the game reads freehand ink and asks "how close is this to
//! a rune?". This module answers the inverse question — "what would ink that
//! reads perfectly actually look like?" — and it is the one place that knows.
//! Given a set of runes it lays out a working circle with those runes spaced
//! inside it, positioned and sized so that `rune_diagram::interpret_diagram`
//! reads back exactly that set: circle quality ~1.0, every rune recognized, and
//! every rune's `potency` at its category's reference magnitude (1.0).
//!
//! The layout is deterministic — the same rune set always produces the same
//! diagram — so it can be used as a repeatable in-game reference (the slate's
//! "Reference" guide, see `state::GameSession::place_reference_diagram`) as well
//! as a test oracle.
//!
//! Coordinates are slate-normalized (0..1 across the drawing slate), the same
//! space `DrawnStroke` ink lives in. That space is *anisotropic* on a
//! wider-than-tall slate, so a normalized circle renders as an on-screen
//! ellipse — deliberately, because `rune_diagram::circle::circle_quality` scores
//! its aspect ratio in normalized space. The reference draws what the recognizer
//! wants to see, not what a compass would draw.

use crate::data::RuneDef;
use crate::rune_drawing::{template_strokes_for_rune, DrawnStroke, StrokePoint};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

mod marks;
#[cfg(test)]
mod tests;

pub use marks::DiagramRing;

/// Center of every generated working circle: the middle of the slate, which is
/// also where `circle_quality`'s centering term peaks.
pub const CIRCLE_CENTER: StrokePoint = StrokePoint { x: 0.5, y: 0.5 };
/// Working-circle radius. Large enough to clear `circle_quality`'s absolute size
/// floor (span >= 0.22) with room to spare, small enough to leave a margin the
/// player's own traced line can wobble inside without clipping the slate edge.
pub const CIRCLE_RADIUS: f32 = 0.40;

/// Working-circle radius for a diagram that carries structural work. Wider than
/// the plain one purely for elbow room — see `perfect_diagram_for`.
pub const STRUCTURED_CIRCLE_RADIUS: f32 = 0.46;

/// Sample count around the circle. Well past `circle_quality`'s 8-point minimum,
/// and enough that its 8-bin angular coverage check fills every bin.
const CIRCLE_STEPS: usize = 48;

/// How far from the circle's center the runes are laid out, as a fraction of its
/// radius. The floor keeps a two- or three-rune diagram comfortably spread
/// instead of huddled at the middle; the ceiling keeps every glyph well inside
/// the circle so `circle::is_inside_working_circle` never has to make a close
/// call.
const MIN_RUNE_ORBIT: f32 = 0.50;
const MAX_RUNE_ORBIT: f32 = 0.65;
/// Multiple of the largest glyph's own span to leave between neighboring glyph
/// centers, so `rune_diagram::geometry::cluster_strokes` keeps them as separate
/// clusters rather than merging two runes into one unreadable blob.
const RUNE_CLEARANCE: f32 = 1.6;
/// Extra margin over `RUNE_CLEARANCE` for marks deliberately drawn together —
/// close enough for `crate::reading` to join them, far enough that clustering
/// never merges them into one mark.
const TOGETHER_MARGIN: f32 = 1.12;
/// How much smaller a diagram is drawn when it groups marks — see
/// `perfect_diagram_for`.
const TOGETHER_RUNE_SCALE: f32 = 0.85;

/// Reinforcement-ring radii, as fractions of the working circle's radius. Both
/// sit inside `classify_circle_stroke`'s 0.28..=0.92 `ReinforcementRing` scale
/// band (scale is *diameter* over the circle's, hence the doubling) with room
/// for several rings before the outermost leaves it.
const RING_INNER_SCALE: f32 = 0.30;
const RING_SCALE_STEP: f32 = 0.04;
/// Gap between the outermost ring and the nearest rune glyph, so a rune stroke
/// never touches a ring and gets clustered into it.
const RING_CLEARANCE: f32 = 0.05;

/// Sub-scope circle size, as a fraction of the parent's radius. Above
/// `scope::NESTED_RING_MIN_SCALE` (0.28 of the parent's *bounds*, so 0.14 of its
/// radius) with margin, and small enough that two fit around one orbit.
const SUB_SCOPE_SCALE: f32 = 0.28;
/// Gap between a sub-scope circle and the working circle it sits inside.
const SUB_SCOPE_MARGIN: f32 = 0.04;

/// Satellite-seal radius as a fraction of the circle's radius — inside
/// `SatelliteSeal`'s 0.055..=0.24 scale band, small enough to tuck between two
/// runes on the same orbit.
const SATELLITE_SCALE: f32 = 0.07;
/// Radial spokes run from the center out to here.
const RADIAL_LENGTH: f32 = 0.70;
/// Perimeter ticks sit out near the rim (`PerimeterMark` wants orbit
/// 0.78..=1.10); script marks sit further in, and are smaller.
const PERIMETER_ORBIT: f32 = 0.90;
const PERIMETER_HALF_LENGTH: f32 = 0.035;
const SCRIPT_ORBIT: f32 = 0.76;
const SCRIPT_HALF_LENGTH: f32 = 0.018;

/// `classify_circle_stroke`'s own satellite-seal shape tests, applied to a rune
/// template in its 0..1 unit space — see `reads_as_satellite_seal`.
const SEAL_MIN_POINTS: usize = 12;
const SEAL_CLOSURE_TOLERANCE: f32 = 0.24;
/// Orbit for several center-drawn runes sharing the middle, as a fraction of the
/// circle's radius. Below `SatelliteSeal`'s 0.22 orbit floor with margin.
const INNER_ORBIT_SCALE: f32 = 0.12;

/// How many marks of each kind it takes before `rune_diagram::
/// is_circle_structure` reads them as structure instead of handing them to rune
/// recognition. See `StructurePlan::readable_as_structure`.
const MIN_READABLE_RINGS: usize = 2;
const MIN_READABLE_SATELLITES: usize = 3;
const MIN_READABLE_RADIALS: usize = 6;
const MIN_READABLE_SCRIPTS: usize = 8;

/// One rune placed in a diagram, in the same `(center, scale)` form
/// `state::GuideTemplate` and the slate's guide renderer already use: template
/// unit-space point `p` is drawn at `center + (p - 0.5) * scale`.
#[derive(Debug, Clone, PartialEq)]
pub struct RunePlacement {
    pub rune_id: String,
    pub center: StrokePoint,
    pub scale: f32,
}

impl RunePlacement {
    /// This rune's ink: its canonical template, placed and sized.
    ///
    /// Deliberately *not* run through `rune_drawing::canonicalize_stroke`.
    /// That resamples captured ink to a fixed arc-length spacing to strip the
    /// pointing device's frame rate out of a gesture; applied to an exact
    /// polyline it only moves samples off the template's corners, and a
    /// corner-rounded `safer` heptagon reads as a `sphere` (the tracked
    /// (safer, sphere) confusion). Generated ink has no capture noise to
    /// normalize away, so it keeps the authored geometry.
    pub fn strokes(&self) -> Vec<DrawnStroke> {
        template_strokes_for_rune(&self.rune_id)
            .unwrap_or_default()
            .into_iter()
            .map(|stroke| DrawnStroke {
                points: stroke
                    .points
                    .into_iter()
                    .map(|point| self.place(point))
                    .collect(),
            })
            .collect()
    }

    fn place(&self, point: StrokePoint) -> StrokePoint {
        StrokePoint::new(
            self.center.x + (point.x - 0.5) * self.scale,
            self.center.y + (point.y - 0.5) * self.scale,
        )
    }
}

/// One nested scope: its own ring plus the runes drawn inside it. A working
/// circle that encloses a smaller ring with ink of its own reads that ring as a
/// sub-scope (`rune_diagram::scope`), which is what commissions asking for
/// "vents" require.
#[derive(Debug, Clone, PartialEq)]
pub struct SubScopeLayout {
    pub ring: DiagramRing,
    pub runes: Vec<RunePlacement>,
}

/// A complete diagram: one working circle, the runes inside it, and whatever
/// structural work the order demands beyond them.
#[derive(Debug, Clone, PartialEq)]
pub struct PerfectDiagram {
    pub circle_center: StrokePoint,
    pub circle_radius: f32,
    pub runes: Vec<RunePlacement>,
    /// Concentric reinforcement rings, innermost first.
    pub rings: Vec<DiagramRing>,
    pub sub_scopes: Vec<SubScopeLayout>,
    /// Satellite seals, radial spokes, perimeter ticks and script marks — the
    /// decoration that has no placement identity worth naming individually.
    pub marks: Vec<DrawnStroke>,
}

impl PerfectDiagram {
    /// The whole diagram as ink, circle first — drop this straight into
    /// `DesignBoard::drawing_strokes` and `interpret_diagram` reads it back.
    pub fn strokes(&self) -> Vec<DrawnStroke> {
        let mut strokes = vec![self.circle_stroke()];
        strokes.extend(self.structure_strokes());
        for placement in self.rune_placements() {
            strokes.extend(placement.strokes());
        }
        strokes
    }

    /// Everything that is not the working circle and not a rune: reinforcement
    /// rings, sub-scope rings, and decorative marks. Rendered as guide lines
    /// rather than guide templates, since none of it is a rune to trace.
    pub fn structure_strokes(&self) -> Vec<DrawnStroke> {
        let mut strokes = self
            .rings
            .iter()
            .map(DiagramRing::stroke)
            .collect::<Vec<_>>();
        strokes.extend(self.sub_scopes.iter().map(|sub| sub.ring.stroke()));
        strokes.extend(self.marks.iter().cloned());
        strokes
    }

    /// Every rune in the diagram, root scope first, then each sub-scope's.
    pub fn rune_placements(&self) -> impl Iterator<Item = &RunePlacement> {
        self.runes
            .iter()
            .chain(self.sub_scopes.iter().flat_map(|sub| sub.runes.iter()))
    }

    pub fn circle_stroke(&self) -> DrawnStroke {
        DrawnStroke {
            points: circle_points(self.circle_center, self.circle_radius),
        }
    }
}

/// The working circle's outline: closed (last point repeats the first, for
/// `circle_quality`'s closure term) and starting at the top, which is both how
/// people actually draw a circle and what `circle_start_score` rewards.
pub fn circle_points(center: StrokePoint, radius: f32) -> Vec<StrokePoint> {
    (0..=CIRCLE_STEPS)
        .map(|index| {
            let angle = -FRAC_PI_2 + TAU * index as f32 / CIRCLE_STEPS as f32;
            StrokePoint::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            )
        })
        .collect()
}

/// Lays out `runes` inside a working circle, with no structural work. Runes are
/// placed in the order given, clockwise from the top of the ring; a lone rune
/// sits at the center. Each is sized to its category's `ideal_scale_in_circle`,
/// the reference magnitude `rune_diagram::recognition::potency_for_rune` scores
/// 1.0 potency for. Runes with no template data are skipped.
///
/// Every in-game caller goes through `manual::diagram_for_job`, which knows how
/// to turn a commission into a full `DiagramRequest`; this stays as the plain
/// "just these runes" entry point the layout tests are written against.
#[cfg_attr(not(test), allow(dead_code))]
pub fn perfect_diagram<'a>(runes: impl IntoIterator<Item = &'a RuneDef>) -> PerfectDiagram {
    perfect_diagram_for(&DiagramRequest::new(runes))
}

/// Everything a diagram has to contain. Built from a commission by
/// `crate::manual::diagram_for_job`; `DiagramRequest::new` covers the plain
/// "just these runes" case.
#[derive(Debug, Clone, Default)]
pub struct DiagramRequest<'a> {
    pub runes: Vec<&'a RuneDef>,
    pub structure: StructurePlan,
    /// Multiplier on every rune's reference size. 1.0 (the default) is the
    /// magnitude that reads at full potency; smaller is how a crowded diagram
    /// fits more marks inside one circle, at the cost of potency — the same
    /// trade a player makes by hand.
    pub rune_scale: RuneScale,
    /// Runes drawn inside each sub-scope circle. Every sub-scope gets the same
    /// contents; a sub-scope needs *some* ink of its own to read as a scope at
    /// all rather than as plain reinforcement decoration.
    pub sub_scope_runes: Vec<&'a RuneDef>,
    /// Marks to draw together, as indices into `runes` — see
    /// `crate::reading`. A group is placed in one slot on the ring, close
    /// enough that the reading joins them and no closer. Runes named in no
    /// group are drawn on their own, which is what every quest diagram does
    /// today.
    pub groups: Vec<Vec<usize>>,
}

/// See `DiagramRequest::rune_scale`. A newtype so `Default` can mean 1.0
/// rather than 0.0, which would draw nothing at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuneScale(pub f32);

impl Default for RuneScale {
    fn default() -> Self {
        Self(1.0)
    }
}

impl<'a> DiagramRequest<'a> {
    /// A request for runes and nothing else — see `perfect_diagram`.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new(runes: impl IntoIterator<Item = &'a RuneDef>) -> Self {
        Self {
            runes: runes.into_iter().collect(),
            ..Default::default()
        }
    }
}

/// How much decorative structure a diagram carries. Mirrors
/// `data::StructureRequirement` plus the sub-scope count, and is what gets
/// raised to the counts below before anything is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StructurePlan {
    pub rings: usize,
    pub satellites: usize,
    pub radials: usize,
    pub perimeter: usize,
    pub scripts: usize,
    pub sub_scopes: usize,
}

impl StructurePlan {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Raises each count to the number of marks of that kind it takes before
    /// `rune_diagram::is_circle_structure` treats them as structure at all.
    /// Below those thresholds a mark stays in the pool sent to rune
    /// recognition, where a lone satellite seal reads as a `sphere` and quietly
    /// adds a rune the order never asked for. Requirements are minimums, so
    /// drawing the extra always satisfies them.
    fn readable_as_structure(self) -> Self {
        Self {
            rings: raise_to(self.rings, MIN_READABLE_RINGS),
            satellites: raise_to(self.satellites, MIN_READABLE_SATELLITES),
            radials: raise_to(self.radials, MIN_READABLE_RADIALS),
            perimeter: self.perimeter,
            scripts: raise_to(self.scripts, MIN_READABLE_SCRIPTS),
            sub_scopes: self.sub_scopes,
        }
    }
}

fn raise_to(count: usize, floor: usize) -> usize {
    if count == 0 {
        0
    } else {
        count.max(floor)
    }
}

/// Lays out a full diagram: the working circle, its reinforcement rings, the
/// order's runes and any sub-scope circles spaced around one orbit, and the
/// small decorative marks tucked into the gaps between them.
pub fn perfect_diagram_for(request: &DiagramRequest<'_>) -> PerfectDiagram {
    let structure = request.structure.readable_as_structure();
    // Structural work needs room the plain layout does not: a wider circle keeps
    // the rings, the rune orbit and the sub-scope circles from crowding each
    // other. Everything downstream is measured relative to the circle's own
    // bounds, so a wider circle changes no score — only the elbow room.
    let radius = if structure.is_empty() {
        CIRCLE_RADIUS
    } else {
        STRUCTURED_CIRCLE_RADIUS
    };

    let rings = (0..structure.rings)
        .map(|index| DiagramRing {
            center: CIRCLE_CENTER,
            radius: radius * (RING_INNER_SCALE + index as f32 * RING_SCALE_STEP),
        })
        .collect::<Vec<_>>();

    // A round, finely-sampled rune (`sphere`) and a satellite seal are the same
    // shape; out on the rune orbit, one is indistinguishable from the other, and
    // once three seals make the reading count them as structure the rune is
    // swept out of rune recognition with them. Drawn at the heart of the circle
    // instead — inside the reinforcement rings, well below the seal band's orbit
    // floor — it stays a rune. Same trick the grand-circle fixtures use.
    // Marks drawn together need room to be both close enough for the reading to
    // join them and far enough apart for the recognizer to still see two marks.
    // At reference size on a four-mark ring those two bounds nearly meet, so a
    // diagram that says something about grouping is written a little smaller —
    // which is what a hand does when it crowds a phrase together.
    let grouped = request.groups.iter().any(|group| group.len() > 1);
    let magnitude = if grouped {
        RuneScale(request.rune_scale.0 * TOGETHER_RUNE_SCALE)
    } else {
        request.rune_scale
    };
    let (inner, sized) = size_runes(&request.runes, radius, magnitude)
        .into_iter()
        .partition::<Vec<_>, _>(|(rune, _, _)| {
            structure.satellites > 0 && reads_as_satellite_seal(&rune.id)
        });

    let widest = sized
        .iter()
        .map(|(_, extent, scale)| extent.span * scale)
        .fold(0.0f32, f32::max);
    // Grouped diagrams take the widest orbit available: more circumference is
    // more angular room to place a phrase in.
    let orbit = if grouped {
        rune_orbit(sized.len(), widest, radius, &rings).max(radius * MAX_RUNE_ORBIT)
    } else {
        rune_orbit(sized.len(), widest, radius, &rings)
    };

    // Runes and sub-scope circles share one orbit's worth of angular slots, so
    // neither can land on top of the other however many of each there are. A
    // group takes one slot between them and spreads its own marks inside it.
    let placement_groups = grouping(sized.len(), &request.groups);
    let slots = marks::even_angles(placement_groups.len() + structure.sub_scopes);
    let inner_orbit = inner_rune_orbit(inner.len(), radius, &rings);
    let inner_angles = marks::even_angles(inner.len());
    let mut placements = inner
        .iter()
        .zip(&inner_angles)
        .map(|((rune, extent, scale), angle)| {
            place_rune(rune, *extent, *scale, CIRCLE_CENTER, inner_orbit, *angle)
        })
        .collect::<Vec<_>>();
    // Marks sharing a slot are stepped just far enough apart that the recognizer
    // still reads two marks, which is as close as the grammar ever needs them.
    let together_step = together_step(widest, orbit);
    for (group, slot) in placement_groups.iter().zip(&slots) {
        for (position, member) in group.iter().enumerate() {
            let (rune, extent, scale) = &sized[*member];
            let offset = (position as f32 - (group.len() as f32 - 1.0) * 0.5) * together_step;
            placements.push(place_rune(
                rune,
                *extent,
                *scale,
                CIRCLE_CENTER,
                orbit,
                slot + offset,
            ));
        }
    }

    let sub_radius = radius * SUB_SCOPE_SCALE;
    let sub_orbit = radius - sub_radius - radius * SUB_SCOPE_MARGIN;
    let sub_scopes = slots
        .iter()
        .skip(sized.len())
        .map(|angle| {
            let center = StrokePoint::new(
                CIRCLE_CENTER.x + sub_orbit * angle.cos(),
                CIRCLE_CENTER.y + sub_orbit * angle.sin(),
            );
            SubScopeLayout {
                ring: DiagramRing {
                    center,
                    radius: sub_radius,
                },
                runes: sub_scope_contents(&request.sub_scope_runes, center, sub_radius),
            }
        })
        .collect();

    PerfectDiagram {
        circle_center: CIRCLE_CENTER,
        circle_radius: radius,
        runes: placements,
        rings,
        sub_scopes,
        marks: decorative_marks(&structure, &slots, orbit, radius),
    }
}

/// Sizes every rune to its category's reference magnitude, dropping any without
/// template data.
fn size_runes<'a>(
    runes: &[&'a RuneDef],
    radius: f32,
    magnitude: RuneScale,
) -> Vec<(&'a RuneDef, TemplateExtent, f32)> {
    let diameter = radius * 2.0;
    runes
        .iter()
        .filter_map(|rune| {
            let extent = template_extent(&rune.id)?;
            let target_span = rune.category.ideal_scale_in_circle() * diameter * magnitude.0;
            Some((*rune, extent, target_span / extent.span))
        })
        .collect()
}

fn place_rune(
    rune: &RuneDef,
    extent: TemplateExtent,
    scale: f32,
    around: StrokePoint,
    orbit: f32,
    angle: f32,
) -> RunePlacement {
    let target = StrokePoint::new(
        around.x + orbit * angle.cos(),
        around.y + orbit * angle.sin(),
    );
    // `center` anchors template unit-space 0.5, not the glyph's bounding box, so
    // a template whose ink is off-center in its own unit square has to be
    // counter-offset or the glyph lands away from `target`.
    RunePlacement {
        rune_id: rune.id.clone(),
        center: StrokePoint::new(
            target.x - (extent.center_x - 0.5) * scale,
            target.y - (extent.center_y - 0.5) * scale,
        ),
        scale,
    }
}

/// A sub-scope's own runes, laid out inside its ring exactly the way the root
/// scope lays out its own — the scope machinery is recursive, so the layout is
/// too.
fn sub_scope_contents(runes: &[&RuneDef], center: StrokePoint, radius: f32) -> Vec<RunePlacement> {
    let sized = size_runes(runes, radius, RuneScale::default());
    let widest = sized
        .iter()
        .map(|(_, extent, scale)| extent.span * scale)
        .fold(0.0f32, f32::max);
    let orbit = ring_orbit(sized.len(), widest, radius);
    let angles = marks::even_angles(sized.len());
    sized
        .iter()
        .zip(&angles)
        .map(|((rune, extent, scale), angle)| {
            place_rune(rune, *extent, *scale, center, orbit, *angle)
        })
        .collect()
}

/// Every rune's slot assignment: the requested groups first, then one slot each
/// for the runes named in none of them. Out-of-range indices are ignored, so a
/// request can never lose a rune to a typo.
fn grouping(count: usize, requested: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut groups = Vec::new();
    let mut taken = vec![false; count];
    for group in requested {
        let members = group
            .iter()
            .copied()
            .filter(|index| *index < count && !taken[*index])
            .collect::<Vec<_>>();
        for index in &members {
            taken[*index] = true;
        }
        if !members.is_empty() {
            groups.push(members);
        }
    }
    for (index, claimed) in taken.iter().enumerate() {
        if !claimed {
            groups.push(vec![index]);
        }
    }
    groups
}

/// Angular step between two marks drawn together: the closest the recognizer
/// will still read as two separate marks, with a little margin.
fn together_step(widest: f32, orbit: f32) -> f32 {
    let separation = widest * RUNE_CLEARANCE * TOGETHER_MARGIN;
    2.0 * (separation / (2.0 * orbit.max(0.001)))
        .clamp(-1.0, 1.0)
        .asin()
}

/// Whether this rune's own shape is what `magical_circle::classify_circle_stroke`
/// calls a satellite seal: a closed, finely-sampled round stroke. Mirrors that
/// function's closed/point-count tests on the template rather than guessing from
/// the rune id, so a new round rune is handled without touching this file.
fn reads_as_satellite_seal(rune_id: &str) -> bool {
    let Some(strokes) = template_strokes_for_rune(rune_id) else {
        return false;
    };
    strokes.iter().any(|stroke| {
        let (Some(first), Some(last)) = (stroke.points.first(), stroke.points.last()) else {
            return false;
        };
        let closed = (first.x - last.x).hypot(first.y - last.y) <= SEAL_CLOSURE_TOLERANCE;
        stroke.points.len() >= SEAL_MIN_POINTS && closed
    })
}

/// Orbit for runes drawn at the heart of the circle. One sits dead center;
/// several share a small ring, kept well inside both the innermost
/// reinforcement ring and the satellite band's orbit floor.
fn inner_rune_orbit(count: usize, radius: f32, rings: &[DiagramRing]) -> f32 {
    if count <= 1 {
        return 0.0;
    }
    rings
        .first()
        .map_or(radius * INNER_ORBIT_SCALE, |ring| ring.radius * 0.45)
}

/// Orbit for the root scope's runes: far enough out to clear the outermost
/// reinforcement ring, otherwise the plain spacing rule.
fn rune_orbit(count: usize, widest: f32, radius: f32, rings: &[DiagramRing]) -> f32 {
    let spaced = ring_orbit(count, widest, radius);
    let Some(outermost) = rings.last() else {
        return spaced;
    };
    spaced.max(outermost.radius + widest * 0.5 + radius * RING_CLEARANCE)
}

/// Ring radius that keeps `count` glyphs of span `widest` at least
/// `RUNE_CLEARANCE` spans apart around the ring, clamped to the orbit band. The
/// band is expressed relative to the circle's own radius, so it means the same
/// thing whatever size the working circle is.
fn ring_orbit(count: usize, widest: f32, radius: f32) -> f32 {
    if count <= 1 {
        return 0.0;
    }
    let separation = widest * RUNE_CLEARANCE;
    let needed = separation / (2.0 * (PI / count as f32).sin()).max(0.001);
    needed.clamp(radius * MIN_RUNE_ORBIT, radius * MAX_RUNE_ORBIT)
}

/// Satellite seals, radial spokes, perimeter ticks and script marks. Satellites
/// share the runes' orbit but sit in the gaps between them; the rest have their
/// own bands, angled into the gaps for the same reason.
fn decorative_marks(
    structure: &StructurePlan,
    occupied: &[f32],
    orbit: f32,
    radius: f32,
) -> Vec<DrawnStroke> {
    let mut marks = Vec::new();
    for angle in marks::gap_angles(occupied, structure.satellites) {
        marks.push(
            DiagramRing {
                center: StrokePoint::new(
                    CIRCLE_CENTER.x + orbit * angle.cos(),
                    CIRCLE_CENTER.y + orbit * angle.sin(),
                ),
                radius: radius * SATELLITE_SCALE,
            }
            .stroke(),
        );
    }
    for angle in marks::gap_angles(occupied, structure.radials) {
        marks.push(marks::radial_spoke(
            CIRCLE_CENTER,
            angle,
            radius * RADIAL_LENGTH,
        ));
    }
    for angle in marks::even_angles(structure.perimeter) {
        marks.push(marks::tangential_tick(
            CIRCLE_CENTER,
            angle,
            radius * PERIMETER_ORBIT,
            radius * PERIMETER_HALF_LENGTH,
        ));
    }
    for angle in marks::even_angles(structure.scripts) {
        marks.push(marks::tangential_tick(
            CIRCLE_CENTER,
            angle,
            radius * SCRIPT_ORBIT,
            radius * SCRIPT_HALF_LENGTH,
        ));
    }
    marks
}

/// A rune template's own footprint inside its 0..1 unit square: how big it
/// actually draws, and where its ink sits relative to the square's middle.
#[derive(Debug, Clone, Copy)]
struct TemplateExtent {
    span: f32,
    center_x: f32,
    center_y: f32,
}

fn template_extent(rune_id: &str) -> Option<TemplateExtent> {
    let strokes = template_strokes_for_rune(rune_id)?;
    let mut points = strokes.iter().flat_map(|stroke| stroke.points.iter());
    let first = points.next()?;
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (first.x, first.x, first.y, first.y);
    for point in points {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }
    let span = (max_x - min_x).max(max_y - min_y);
    (span > 0.001).then_some(TemplateExtent {
        span,
        center_x: (min_x + max_x) * 0.5,
        center_y: (min_y + max_y) * 0.5,
    })
}
