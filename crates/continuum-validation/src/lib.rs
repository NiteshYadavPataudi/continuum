mod non_rust;
mod pipeline;
mod validators;

/// Validator for Biome (JS/TS linting).
pub use non_rust::BiomeValidator;
/// Validator for pytest (Python tests).
pub use non_rust::PytestValidator;
/// Validator for Vitest (JS/TS tests).
pub use non_rust::VitestValidator;
/// The 10-stage validation pipeline.
pub use pipeline::Pipeline;
/// Registry of all built-in stage validators.
pub use validators::AllValidators;
// Re-export core validator types for convenience.
pub use continuum_core::validator::ValidationTarget;
