use super::*;

#[test]
fn run_diagnose_cli_reads_a_bare_stroke_list_and_reports_the_recognized_rune() {
    let strokes = rune_drawing::template_strokes_for_rune("spark").unwrap();
    let json = serde_json::to_string(&strokes).unwrap();
    let path = std::env::temp_dir().join(format!(
        "enchanters_ledger_cli_test_bare_{}.json",
        std::process::id()
    ));
    std::fs::write(&path, json).unwrap();

    let log = run_diagnose_cli(&path);
    std::fs::remove_file(&path).ok();

    let log = log.unwrap();
    assert!(log.contains("spark"), "{log}");
}

#[test]
fn run_diagnose_cli_reads_a_corpus_sample_and_reports_the_recognized_rune() {
    let strokes = rune_drawing::template_strokes_for_rune("spark").unwrap();
    let sample = corpus::CorpusSample {
        label: "spark".to_owned(),
        strokes,
    };
    let json = serde_json::to_string(&sample).unwrap();
    let path = std::env::temp_dir().join(format!(
        "enchanters_ledger_cli_test_sample_{}.json",
        std::process::id()
    ));
    std::fs::write(&path, json).unwrap();

    let log = run_diagnose_cli(&path);
    std::fs::remove_file(&path).ok();

    let log = log.unwrap();
    assert!(log.contains("spark"), "{log}");
}

#[test]
fn run_diagnose_cli_reports_an_error_for_a_missing_file() {
    let path = std::env::temp_dir().join("enchanters_ledger_cli_test_missing_file.json");
    std::fs::remove_file(&path).ok();

    assert!(run_diagnose_cli(&path).is_err());
}
