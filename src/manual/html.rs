//! Renders the manual to one self-contained HTML page.
//!
//! Self-contained on purpose: inline CSS, inline SVG, no external requests, so
//! the file can be opened from disk, dropped next to the published game, or
//! mailed to someone, and still looks like the game it documents.

use super::{ManualEntry, ManualRune};
use crate::data::GameData;
use std::fmt::Write;

const STYLE: &str = "\
:root{color-scheme:dark}\
*{box-sizing:border-box}\
body{margin:0;padding:32px 20px 64px;background:#100e0b;color:#ebe0c8;\
font-family:'Iowan Old Style','Palatino Linotype',Georgia,serif;line-height:1.55}\
main{max-width:1080px;margin:0 auto}\
header.page{border:1px solid #6b5426;background:#181510;padding:24px 28px;margin-bottom:28px}\
h1{margin:0 0 6px;font-size:30px;letter-spacing:.06em;color:#bd9250}\
.lede{margin:0;color:#a49b85;font-size:15px}\
.toc{display:flex;flex-wrap:wrap;gap:8px;margin-top:20px;padding:0;list-style:none}\
.toc a{display:inline-block;padding:4px 10px;border:1px solid #4a3b1e;border-radius:3px;\
color:#d3c49f;text-decoration:none;font-size:13px}\
.toc a:hover{border-color:#bd9250;color:#f3e8cd}\
h2.section{margin:34px 0 14px;font-size:15px;letter-spacing:.22em;text-transform:uppercase;\
color:#8d7b58;border-bottom:1px solid #33291a;padding-bottom:8px}\
article{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1.05fr);gap:26px;\
border:1px solid #3a2f1c;background:#161310;padding:22px 24px;margin-bottom:22px}\
@media (max-width:820px){article{grid-template-columns:minmax(0,1fr)}}\
h3{margin:0 0 2px;font-size:21px;color:#e7d6ac}\
.customer{margin:0 0 12px;color:#9a917c;font-size:14px}\
blockquote{margin:0 0 16px;padding-left:14px;border-left:2px solid #4a3b1e;color:#c3b795;\
font-style:italic}\
table{border-collapse:collapse;width:100%;font-size:14px;margin-bottom:14px}\
th,td{text-align:left;padding:5px 8px;border-bottom:1px solid #2b2317}\
th{width:88px;color:#8d7b58;font-weight:400;font-size:12px;letter-spacing:.1em;\
text-transform:uppercase}\
.badges{display:flex;flex-wrap:wrap;gap:8px;margin:0 0 12px;padding:0;list-style:none}\
.badges li{border:1px solid #4a3b1e;border-radius:3px;padding:3px 9px;font-size:12px;\
color:#cdbf9b}\
.note{margin:0;font-size:13px;color:#a49b85}\
.warn{margin:12px 0 0;padding:9px 12px;border:1px solid #6d3327;background:#241310;\
color:#e0a99b;font-size:13px}\
figure{margin:0}\
svg.diagram{display:block;width:100%;height:auto;border:1px solid #4a3b1e}\
figcaption{margin-top:8px;font-size:12px;color:#8d7b58}\
.reading{margin-top:12px;border-left:2px solid #4a3b1e;padding:2px 0 2px 14px}\
.reading h4{margin:0 0 4px;font-size:11px;letter-spacing:.18em;text-transform:uppercase;\
color:#8d7b58;font-weight:400}\
.reading ul{margin:0;padding:0;list-style:none;font-size:14px;color:#d8caa4}\
.reading li{margin:0 0 2px}\
footer{margin-top:40px;color:#6f6754;font-size:12px;text-align:center}\
";

pub fn render_manual_page(entries: &[ManualEntry], data: &GameData) -> String {
    let mut page = String::new();
    page.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">");
    page.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    let _ = write!(
        page,
        "<title>{} — Diagram Manual</title><style>{STYLE}</style></head><body><main>",
        escape(&data.config.display_name)
    );
    write_header(&mut page, entries, data);

    let mut current_section = "";
    for entry in entries {
        if entry.section != current_section {
            current_section = entry.section;
            let _ = write!(
                page,
                "<h2 class=\"section\">{}</h2>",
                escape(current_section)
            );
        }
        write_entry(&mut page, entry);
    }

    page.push_str(
        "<footer>Generated from the game's own data and diagram layout — \
         regenerate with <code>cargo run -- --manual &lt;dir&gt;</code>.</footer>",
    );
    page.push_str("</main></body></html>");
    page
}

fn write_header(page: &mut String, entries: &[ManualEntry], data: &GameData) {
    let _ = write!(
        page,
        "<header class=\"page\"><h1>{} — Diagram Manual</h1>\
         <p class=\"lede\">Every commission and day talisman, with the diagram that fills it: \
         an enclosing circle, the runes it asks for drawn at the size the reading rewards, and \
         any structural work the order demands. Draw what you see — the shapes are the ones the \
         workshop's own recognizer reads back.</p><ul class=\"toc\">",
        escape(&data.config.display_name)
    );
    for entry in entries {
        let _ = write!(
            page,
            "<li><a href=\"#{}\">{}</a></li>",
            escape(&entry.id),
            escape(&entry.title())
        );
    }
    page.push_str("</ul></header>");
}

fn write_entry(page: &mut String, entry: &ManualEntry) {
    let _ = write!(
        page,
        "<article id=\"{}\"><div><h3>{}</h3><p class=\"customer\">{}</p>\
         <blockquote>{}</blockquote>",
        escape(&entry.id),
        escape(&entry.title()),
        escape(&entry.customer),
        escape(&entry.request)
    );

    page.push_str("<table>");
    for rune in &entry.notation {
        write_notation_row(page, rune);
    }
    if !entry.structure.is_empty() {
        let _ = write!(
            page,
            "<tr><th>Structure</th><td>{}</td></tr>",
            escape(&entry.structure.join(", "))
        );
    }
    page.push_str("</table>");

    // Ladder rungs are drills, not orders: no coins, no insight, and the risk
    // badge carries the complexity word instead.
    page.push_str("<ul class=\"badges\">");
    let _ = write!(page, "<li>{}</li>", escape(&entry.risk));
    if entry.reward > 0 {
        let _ = write!(page, "<li>{} coins</li>", entry.reward);
    }
    if entry.insight > 0 {
        let _ = write!(page, "<li>+{} insight</li>", entry.insight);
    }
    page.push_str("</ul>");
    let _ = write!(page, "<p class=\"note\">{}</p>", escape(&entry.note));
    if !entry.unreadable.is_empty() {
        let _ = write!(
            page,
            "<p class=\"warn\">Not currently readable: {}. The workshop cannot yet make out \
             {} notation inside a diagram, however carefully it is drawn.</p>",
            escape(&entry.unreadable.join(", ")),
            if entry.unreadable.len() == 1 {
                "this"
            } else {
                "these"
            }
        );
    }

    let _ = write!(
        page,
        "</div><figure>{}{}<figcaption>{}</figcaption></figure></article>",
        super::diagram_svg(&entry.diagram),
        reading_block(entry),
        escape(&format!(
            "{} — {}",
            entry.kind,
            entry
                .notation
                .iter()
                .map(|rune| rune.name.as_str())
                .collect::<Vec<_>>()
                .join(" + ")
        ))
    );
}

/// The diagram, read aloud — the same sentences the slate gives the player for
/// the same ink. This is where a manual page teaches the placement grammar
/// rather than just listing runes.
fn reading_block(entry: &ManualEntry) -> String {
    if entry.reading.is_empty() {
        return String::new();
    }
    let mut block = String::from("<div class=\"reading\"><h4>Reads as</h4><ul>");
    for line in &entry.reading {
        let _ = write!(block, "<li>{}</li>", escape(line));
    }
    block.push_str("</ul></div>");
    block
}

fn write_notation_row(page: &mut String, rune: &ManualRune) {
    let _ = write!(
        page,
        "<tr><th>{}</th><td>{}</td></tr>",
        escape(rune.label),
        escape(&rune.name)
    );
}

/// Quest text is authored data, not a literal — escape it rather than trusting
/// it to be HTML-safe.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}
