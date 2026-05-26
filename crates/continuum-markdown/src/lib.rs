#![doc = "Engineering markdown document loader: parses YAML frontmatter + body from docs/."]
#![warn(missing_docs)]

mod loader;
mod parser;
mod schemas;

use thiserror::Error;

pub use loader::load;
pub use schemas::*;

/// Names of all recognized engineering doc files (without `.md` extension).
pub const DOC_NAMES: &[&str] = &[
    "VISION",
    "PRODUCT",
    "ARCHITECTURE",
    "ENGINEERING",
    "TASKS",
    "AGENTS",
    "MODEL_RULES",
    "SECURITY",
];

/// Errors that can occur during markdown document loading and parsing.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MarkdownError {
    /// An I/O error occurred while reading a file.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// The document content could not be parsed.
    #[error("parse: {0}")]
    Parse(String),
    /// A required document (e.g. ARCHITECTURE, MODEL_RULES) was not found.
    #[error("missing required doc: {0}")]
    MissingRequiredDoc(String),
    /// YAML frontmatter could not be deserialized.
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    /// JSON processing failed.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}
