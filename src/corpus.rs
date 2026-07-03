//! Phase 0 ground-truth capture: serializes the current practice ink to
//! `tests/corpus/<label>/<label>_<stamp>.json` so real human drawings can
//! grow a regression corpus alongside the confusion-matrix gate. See
//! `.project/magic-symbol-system-plan.md` Phase 0, item 1.

use crate::rune_drawing::DrawnStroke;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusSample {
    pub label: String,
    pub strokes: Vec<DrawnStroke>,
}

pub struct CorpusCapture {
    pub json: String,
    pub file_path: Option<String>,
}

pub fn capture_corpus_sample(label: &str, strokes: &[DrawnStroke]) -> CorpusCapture {
    let sample = CorpusSample {
        label: label.to_string(),
        strokes: strokes.to_vec(),
    };
    let json = serde_json::to_string_pretty(&sample).unwrap_or_default();
    let file_path = write_corpus_file(label, &json);
    CorpusCapture { json, file_path }
}

#[cfg(target_arch = "wasm32")]
fn write_corpus_file(_label: &str, _json: &str) -> Option<String> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn write_corpus_file(label: &str, json: &str) -> Option<String> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let safe_label: String = label
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    let dir = format!("tests/corpus/{safe_label}");
    std::fs::create_dir_all(&dir).ok()?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_millis();
    let path = format!("{dir}/{safe_label}_{stamp}.json");
    std::fs::write(&path, json).ok()?;
    Some(path)
}

/// Phase 0 exit criterion: every captured human sample must still recognize
/// as the rune it was labeled with. Trivially passes while the corpus is
/// empty; starts pulling weight as `Capture Sample` grows it.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::rune_drawing::recognize_rune;

    #[test]
    fn every_corpus_sample_recognizes_as_its_label() {
        let data = GameData::load().unwrap();
        let runes: Vec<&crate::data::RuneDef> = data.runes.iter().collect();
        let mut checked = 0;
        let mut failures = Vec::new();

        for entry in walk_json_files("tests/corpus") {
            let text = std::fs::read_to_string(&entry).unwrap_or_default();
            let Ok(sample) = serde_json::from_str::<CorpusSample>(&text) else {
                failures.push(format!("{}: not a valid CorpusSample", entry.display()));
                continue;
            };
            checked += 1;
            let outcome = recognize_rune(&sample.strokes, runes.iter().copied());
            let ok = outcome
                .as_ref()
                .is_some_and(|result| result.rune_id == sample.label && result.accepted);
            if !ok {
                failures.push(format!(
                    "{}: label={} -> {:?}",
                    entry.display(),
                    sample.label,
                    outcome
                ));
            }
        }

        if !failures.is_empty() {
            panic!(
                "{}/{} corpus samples misread:\n{}",
                failures.len(),
                checked,
                failures.join("\n")
            );
        }
    }

    fn walk_json_files(root: &str) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        let mut pending = vec![std::path::PathBuf::from(root)];
        while let Some(dir) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    pending.push(path);
                } else if path.extension().is_some_and(|ext| ext == "json") {
                    files.push(path);
                }
            }
        }
        files
    }
}
