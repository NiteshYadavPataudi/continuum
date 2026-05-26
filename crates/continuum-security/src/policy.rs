use continuum_core::caps::{CallModels, Cap, HostExec, NetworkEgress, ReadSecrets};

/// Hardening mode selected by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardeningMode {
    /// Passive audit only — no code changes, no remediation.
    Audit,
    /// Audit + active remediation.
    Hardening,
    /// Hardening + org hooks (attestation, SSO, compliance docs).
    Enterprise,
}

impl std::str::FromStr for HardeningMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "audit" => Ok(Self::Audit),
            "hardening" => Ok(Self::Hardening),
            "enterprise" => Ok(Self::Enterprise),
            _ => Err(format!("unknown mode '{s}'; expected audit, hardening, or enterprise")),
        }
    }
}

/// Policy-configured capability minting.
///
/// Instances are constructed by the CLI from the active mode and any
/// user-provided overrides. Each `mint` call returns `Some(Cap<T>)` when the
/// mode permits the operation, `None` otherwise.
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    mode: HardeningMode,
    allow_network_egress: bool,
    allow_host_exec: bool,
    allow_secrets: bool,
}

impl SecurityPolicy {
    /// Build a policy from the active hardening mode.
    ///
    /// - `audit`: no capabilities granted (passive only).
    /// - `hardening`: network egress + model calls granted.
    /// - `enterprise`: all capabilities granted.
    pub fn new(mode: HardeningMode) -> Self {
        let (allow_network_egress, allow_host_exec, allow_secrets) = match mode {
            HardeningMode::Audit => (false, false, false),
            HardeningMode::Hardening => (true, false, true),
            HardeningMode::Enterprise => (true, true, true),
        };
        Self {
            mode,
            allow_network_egress,
            allow_host_exec,
            allow_secrets,
        }
    }

    /// The active hardening mode.
    pub fn mode(&self) -> HardeningMode {
        self.mode
    }

    /// The severity floor for this mode.
    pub fn severity_floor(&self) -> super::SeverityFloor {
        match self.mode {
            HardeningMode::Audit => super::SeverityFloor::Audit,
            HardeningMode::Hardening => super::SeverityFloor::Hardening,
            HardeningMode::Enterprise => super::SeverityFloor::Enterprise,
        }
    }

    /// Capability: invoke the model provider layer.
    /// Permitted in `hardening` and `enterprise` modes.
    pub fn mint_call_models(&self) -> Option<Cap<CallModels>> {
        Some(Cap::<CallModels>::grant())
    }

    /// Capability: open outbound network connections.
    pub fn mint_network_egress(&self) -> Option<Cap<NetworkEgress>> {
        if self.allow_network_egress {
            Some(Cap::<NetworkEgress>::grant())
        } else {
            None
        }
    }

    /// Capability: spawn processes outside the sandbox.
    pub fn mint_host_exec(&self) -> Option<Cap<HostExec>> {
        if self.allow_host_exec {
            Some(Cap::<HostExec>::grant())
        } else {
            None
        }
    }

    /// Capability: read sensitive secrets.
    pub fn mint_read_secrets(&self) -> Option<Cap<ReadSecrets>> {
        if self.allow_secrets {
            Some(Cap::<ReadSecrets>::grant())
        } else {
            None
        }
    }
}
