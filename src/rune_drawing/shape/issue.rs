//! Player-facing structural feedback: the one issue a shape report may carry,
//! and the wording it turns into on the slate.

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ShapeIssue {
    NotClosed,
    NotRoundEnough,
    TooManyStraightLines(String),
    NotEnoughSides(String, u32),
    TooManySides(String, u32),
    NotStraightEnough,
    ShouldBeOpen(String),
    MissingArrowRight(String),
    MissingArrowDown(String),
    MissingCenterBar(String),
    MissingRayStructure(String),
}

impl ShapeIssue {
    pub(crate) fn message(&self) -> String {
        match self {
            ShapeIssue::NotClosed => "The stroke needs to close cleanly.".into(),
            ShapeIssue::NotRoundEnough => "The circle drifts too far from a steady radius.".into(),
            ShapeIssue::TooManyStraightLines(name) => {
                format!("Too many straight sides are showing for {name}.")
            }
            ShapeIssue::NotEnoughSides(name, count) => {
                format!("{name} needs {} clear sides.", number_word(*count))
            }
            ShapeIssue::TooManySides(name, count) => {
                format!(
                    "{name} has too many corners; keep it to {} sides.",
                    number_word(*count)
                )
            }
            ShapeIssue::NotStraightEnough => "The straight rune lines need to be cleaner.".into(),
            ShapeIssue::ShouldBeOpen(name) => {
                format!("{name} should be an open arrow, not a closed shape.")
            }
            ShapeIssue::MissingArrowRight(name) => {
                format!("{name} needs a clear right-pointing shaft and arrow head.")
            }
            ShapeIssue::MissingArrowDown(name) => {
                format!("{name} needs a clear shaft and arrow head.")
            }
            ShapeIssue::MissingCenterBar(name) => {
                format!("{name} needs a closed outer shape with a clear center bar.")
            }
            ShapeIssue::MissingRayStructure(name) => {
                format!("{name} needs straight rays crossing through the center.")
            }
        }
    }
}

fn number_word(n: u32) -> String {
    match n {
        1 => "one".into(),
        2 => "two".into(),
        3 => "three".into(),
        4 => "four".into(),
        5 => "five".into(),
        6 => "six".into(),
        7 => "seven".into(),
        8 => "eight".into(),
        9 => "nine".into(),
        10 => "ten".into(),
        11 => "eleven".into(),
        12 => "twelve".into(),
        other => other.to_string(),
    }
}

pub(super) fn display_name(rune_id: &str) -> String {
    let mut chars = rune_id.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
