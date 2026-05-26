#![warn(missing_docs)]

//! Security implementation layer: hardening modes, policy enforcement,
//! capability token minting, audit reporting, and compliance attestation.

mod audit;
mod compliance;
mod policy;

pub use audit::{AuditReport, AuditReportBuilder};
pub use compliance::ComplianceAttestation;
pub use policy::{HardeningMode, SecurityPolicy};

/// Severity threshold for filtering findings per hardening mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeverityFloor {
    /// Only `Critical` and `Error`.
    Audit,
    /// `Critical`, `Error`, and `Warning`.
    Hardening,
    /// All severities including `Info`.
    Enterprise,
}

impl SeverityFloor {
    /// Returns `true` if `s` meets or exceeds this floor.
    pub fn allows(&self, s: &continuum_core::validator::Severity) -> bool {
        use continuum_core::validator::Severity;
        match self {
            Self::Audit => matches!(s, Severity::Critical | Severity::Error),
            Self::Hardening => !matches!(s, Severity::Info),
            Self::Enterprise => true,
        }
    }
}
