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
