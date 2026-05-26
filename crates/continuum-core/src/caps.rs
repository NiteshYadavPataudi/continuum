//! Compile-time capability tokens that gate dangerous operations.
//!
//! A function that may make a network call accepts `&Cap<NetworkEgress>`;
//! callers must obtain a token from the runtime's policy layer to compile.
//! This makes "this code could exfiltrate data" auditable at the type level.

use core::marker::PhantomData;

/// A phantom-typed capability token. Holding one proves that the runtime
/// authorized the bearer to perform a specific class of operation.
///
/// `Cap` cannot be constructed by downstream crates — only the runtime's
/// policy layer (`continuum-runtime` or test helpers) may mint tokens.
#[derive(Debug)]
pub struct Cap<T> {
    _marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Cap<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Cap<T> {}

impl<T> Cap<T> {
    /// Mint a token. Internal to the workspace — call sites are reviewed.
    ///
    /// # Safety contract
    /// This is not `unsafe` in the Rust sense, but minting a token is the
    /// authorization step. It must only be invoked by policy code that has
    /// verified the caller's configuration permits the capability.
    pub fn grant() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

/// Capability: spawn processes outside the sandbox (host execution).
pub enum HostExec {}

/// Capability: open outbound network connections.
pub enum NetworkEgress {}

/// Capability: write to the user's filesystem outside the project directory.
pub enum HostFsWrite {}

/// Capability: read sensitive secrets (API keys, env vars).
pub enum ReadSecrets {}

/// Capability: invoke the model provider layer.
pub enum CallModels {}
