use continuum_core::caps::{CallModels, Cap, HostExec};

#[test]
fn test_cap_grant_and_use() {
    let cap: Cap<CallModels> = Cap::grant();
    let _ = cap;
}

#[test]
fn test_cap_is_copy() {
    let cap: Cap<HostExec> = Cap::grant();
    let cap2 = cap;
    let _ = (cap, cap2);
}

#[test]
fn test_cap_types_are_distinct() {
    let model_cap: Cap<CallModels> = Cap::grant();
    let host_cap: Cap<HostExec> = Cap::grant();
    let _ = (model_cap, host_cap);
}
