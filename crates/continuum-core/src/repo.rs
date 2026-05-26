//! Repository intelligence traits.
//!
//! `continuum-repo` builds and maintains a tree-sitter-backed view of the
//! target repository. Large monorepos are handled via retrieval-driven
//! loading — bodies are fetched on-demand, never whole-repo.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

/// A symbol reference (e.g. function, struct, module).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolRef {
    /// Fully-qualified symbol name.
    pub qualified: String,
    /// File the symbol is defined in.
    pub file: PathBuf,
    /// 1-indexed line number.
    pub line: u32,
}

/// A symbol-lookup query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolQuery {
    /// Substring to match against `qualified`.
    pub needle: String,
    /// Maximum hits.
    pub limit: u32,
}

/// A proposed change considered for impact analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedChange {
    /// Files the change touches.
    pub files: Vec<PathBuf>,
    /// Optional symbols deleted or renamed.
    pub removed_symbols: Vec<SymbolRef>,
}

/// Result of impact analysis.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImpactSet {
    /// Symbols directly broken.
    pub broken: Vec<SymbolRef>,
    /// Symbols that need re-verification.
    pub affected: Vec<SymbolRef>,
}

/// A retrieval query — answered with on-demand context, never whole-repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextQuery {
    /// Natural-language query.
    pub text: String,
    /// Optional symbol anchors (start retrieval from here).
    pub anchors: Vec<SymbolRef>,
    /// Maximum tokens of context.
    pub token_budget: u32,
}

/// Retrieved context bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBundle {
    /// Snippets in retrieval order.
    pub snippets: Vec<ContextSnippet>,
    /// Total tokens (within `token_budget`).
    pub tokens: u32,
}

/// One snippet inside a [`ContextBundle`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnippet {
    /// File the snippet came from.
    pub file: PathBuf,
    /// 1-indexed starting line.
    pub line: u32,
    /// Source text.
    pub text: String,
}

/// Options for [`RepoLoader::build`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexOptions {
    /// Whether to follow gitignore.
    pub respect_gitignore: bool,
    /// Maximum files to index (0 = no limit).
    pub max_files: u32,
}

/// Errors specific to repository intelligence.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RepoError {
    /// I/O failure walking the repository.
    #[error("io: {0}")]
    Io(String),
    /// Tree-sitter could not parse a file or build a grammar.
    #[error("parse: {0}")]
    Parse(String),
    /// The requested language is not supported.
    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),
    /// Catch-all.
    #[error("repo error: {0}")]
    Other(String),
}

/// Synchronous in-memory queries over an already-built index.
pub trait RepoIndex: Send + Sync {
    /// Look up symbols matching a query.
    fn symbols(&self, query: SymbolQuery) -> Vec<SymbolRef>;

    /// Symbols that depend on `sym`.
    fn dependents_of(&self, sym: SymbolRef) -> Vec<SymbolRef>;

    /// Symbols `sym` depends on.
    fn dependencies_of(&self, sym: SymbolRef) -> Vec<SymbolRef>;

    /// Compute the impact set of a proposed change.
    fn impact_set(&self, change: &ProposedChange) -> ImpactSet;
}

/// Builds and refreshes a [`RepoIndex`].
#[async_trait]
pub trait RepoLoader: Send + Sync {
    /// Build a fresh index of the repository rooted at `root`.
    async fn build(&self, root: &Path, opts: IndexOptions)
        -> Result<Arc<dyn RepoIndex>, RepoError>;

    /// Incrementally refresh the index for the given paths.
    async fn refresh(&self, paths: &[PathBuf]) -> Result<(), RepoError>;

    /// Retrieval-driven context query (used by agents to fetch on-demand).
    async fn retrieve(&self, q: ContextQuery) -> Result<ContextBundle, RepoError>;
}
