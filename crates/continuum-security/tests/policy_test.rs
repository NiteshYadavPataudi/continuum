#![allow(unused_imports)]

use continuum_core::caps::{CallModels, HostExec, NetworkEgress, ReadSecrets};
use continuum_security::{HardeningMode, SecurityPolicy, SeverityFloor};

#[test]
fn test_audit_mode_no_caps() {
    let policy = SecurityPolicy::new(HardeningMode::Audit);
    assert!(policy.mint_call_models().is_some());
    assert!(policy.mint_network_egress().is_none());
    assert!(policy.mint_host_exec().is_none());
    assert!(policy.mint_read_secrets().is_none());
}

#[test]
fn test_hardening_mode_model_and_secrets() {
    let policy = SecurityPolicy::new(HardeningMode::Hardening);
    assert!(policy.mint_call_models().is_some());
    assert!(policy.mint_network_egress().is_some());
    assert!(policy.mint_host_exec().is_none());
    assert!(policy.mint_read_secrets().is_some());
}

#[test]
fn test_enterprise_mode_all_caps() {
    let policy = SecurityPolicy::new(HardeningMode::Enterprise);
    assert!(policy.mint_call_models().is_some());
    assert!(policy.mint_network_egress().is_some());
    assert!(policy.mint_host_exec().is_some());
    assert!(policy.mint_read_secrets().is_some());
}

#[test]
fn test_severity_floor_audit() {
    use continuum_core::validator::Severity;
    let floor = SeverityFloor::Audit;
    assert!(floor.allows(&Severity::Critical));
    assert!(floor.allows(&Severity::Error));
    assert!(!floor.allows(&Severity::Warning));
    assert!(!floor.allows(&Severity::Info));
}

#[test]
fn test_severity_floor_hardening() {
    use continuum_core::validator::Severity;
    let floor = SeverityFloor::Hardening;
    assert!(floor.allows(&Severity::Critical));
    assert!(floor.allows(&Severity::Warning));
    assert!(!floor.allows(&Severity::Info));
}

#[test]
fn test_severity_floor_enterprise_allows_all() {
    use continuum_core::validator::Severity;
    let floor = SeverityFloor::Enterprise;
    assert!(floor.allows(&Severity::Info));
    assert!(floor.allows(&Severity::Critical));
}

#[test]
fn test_hardening_mode_parse() {
    assert_eq!(
        "audit".parse::<HardeningMode>().unwrap(),
        HardeningMode::Audit
    );
    assert_eq!(
        "hardening".parse::<HardeningMode>().unwrap(),
        HardeningMode::Hardening
    );
    assert_eq!(
        "enterprise".parse::<HardeningMode>().unwrap(),
        HardeningMode::Enterprise
    );
    assert!("invalid".parse::<HardeningMode>().is_err());
}
