use super::*;

#[test]
fn embedded_data_loads() {
    let data = GameData::load().unwrap();

    assert_eq!(data.config.game_name, "the_enchanters_ledger");
    assert!(data.rune("light").is_some());
    assert!(data.rune("continuous").is_some());
    assert!(!data.commissions.is_empty());
    assert!(!data.talisman_jobs.is_empty());
}
