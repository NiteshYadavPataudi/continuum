//! Continuum runtime — the orchestrator that wires every domain crate
//! together.
//!
//! `Session` owns one user-driven execution (one `continuum execute` invocation).
//! `Scheduler` walks the planner's DAG, dispatches to agents, drives the
//! validation pipeline, journals to memory, and checkpoints through the
//! recovery store.
//!
//! Phase 1 stub — full orchestration lands incrementally from phase 4 onward.

#![warn(missing_docs)]

/// Task scheduler: walks the execution plan DAG and dispatches to agents.
pub mod scheduler;
/// Session: owns a single user-driven execution context.
pub mod session;

pub use scheduler::Scheduler;
pub use session::Session;
