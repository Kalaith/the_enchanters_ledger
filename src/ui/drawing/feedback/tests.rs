use super::PotencyBand;

/// The bands have to agree with `recognition::potency_for_rune`'s own
/// anchors, or a rune drawn at exactly reference size would be flagged.
#[test]
fn reference_size_rune_reads_as_reference() {
    assert_eq!(PotencyBand::of(1.0), PotencyBand::Reference);
    assert_eq!(PotencyBand::of(0.88), PotencyBand::Reference);
    assert_eq!(PotencyBand::of(1.12), PotencyBand::Reference);
}

/// `potency_for_rune` clamps to [0.35, 2.2]; both ends have to band.
#[test]
fn clamped_extremes_band_apart() {
    assert_eq!(PotencyBand::of(0.35), PotencyBand::Weak);
    assert_eq!(PotencyBand::of(2.2), PotencyBand::Strong);
}
