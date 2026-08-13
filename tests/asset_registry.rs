use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[test]
fn asset_registry_matches_external_runtime_assets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let registry: Value = serde_json::from_str(
        &fs::read_to_string(root.join("asset_registry.json"))
            .expect("asset_registry.json must be readable"),
    )
    .expect("asset_registry.json must be valid JSON");
    assert_eq!(registry["version"], 1);

    let registered: BTreeSet<&str> = registry["assets"]
        .as_array()
        .expect("asset registry needs an assets array")
        .iter()
        .map(|entry| entry.as_str().expect("asset paths must be strings"))
        .collect();
    let external_runtime_assets: BTreeSet<&str> = BTreeSet::new();
    assert_eq!(registered, external_runtime_assets);
}
