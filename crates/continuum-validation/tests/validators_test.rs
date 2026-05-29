use continuum_validation::AllValidators;

#[test]
fn test_all_validators_count() {
    let registry = AllValidators::new();
    let count = registry.iter().count();
    assert_eq!(count, 10, "should have 10 stage validators");
}

#[test]
fn test_validator_stages_are_unique() {
    use std::collections::HashSet;

    let registry = AllValidators::new();
    let mut stages = HashSet::new();
    for v in registry.iter() {
        assert!(stages.insert(v.stage()), "duplicate stage: {:?}", v.stage());
    }
    assert_eq!(stages.len(), 10);
}
