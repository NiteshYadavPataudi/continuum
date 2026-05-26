//! Vendored snapshot of the [models.dev](https://models.dev) registry plus
//! build-time codegen producing typed provider / model identifiers.
//!
//! `cargo xtask refresh-models` re-fetches `https://models.dev/api.json`,
//! validates against the schema, and overwrites `models-snapshot.json`.
//! Diffs are reviewed before commit.

#![warn(missing_docs)]

#[allow(missing_docs)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}

pub use generated::*;
