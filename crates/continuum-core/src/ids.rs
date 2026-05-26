//! Strongly-typed identifiers used throughout Continuum.
//!
//! Every ID is a newtype around a `uuid::Uuid` (or a string for human-readable
//! identifiers like provider/model names). The `id_type!` macro below keeps
//! definitions terse while preserving type-safety: a [`SessionId`] cannot be
//! confused with an [`AgentId`] at compile time.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! uuid_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Generate a fresh random ID.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Borrow the underlying UUID.
            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl From<Uuid> for $name {
            fn from(u: Uuid) -> Self {
                Self(u)
            }
        }
    };
}

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Construct from anything that converts into a `String`.
            pub fn new(s: impl Into<String>) -> Self {
                Self(s.into())
            }
            /// Borrow the inner string.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_owned())
            }
        }
    };
}

uuid_id!(
    /// Identifies a top-level user-driven execution session.
    SessionId
);
uuid_id!(
    /// Identifies a single task node within an [`crate::planner::ExecutionPlan`].
    TaskId
);
uuid_id!(
    /// Identifies an [`crate::agent::Agent`] instance.
    AgentId
);
uuid_id!(
    /// Identifies a sandbox instance.
    SandboxId
);
uuid_id!(
    /// Identifies a sandbox filesystem/state snapshot.
    SnapshotId
);
uuid_id!(
    /// Identifies a checkpoint persisted by `continuum-recovery`.
    CheckpointId
);
uuid_id!(
    /// Identifies a memory item stored in the memory engine.
    MemoryId
);
uuid_id!(
    /// Identifies a single execution run (one walk of an `ExecutionPlan`).
    RunId
);

string_id!(
    /// Identifies a model provider (e.g. `"openai"`, `"anthropic"`).
    ProviderId
);
string_id!(
    /// Identifies a model within a provider's catalog (e.g. `"gpt-4o"`).
    ModelId
);
string_id!(
    /// Identifies a tool integration (e.g. `"biome"`, `"trivy"`).
    ToolId
);
