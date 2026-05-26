#![warn(missing_docs)]

//! Umbrella crate for tool integrations.
//!
//! Provides the [`ToolRegistry`] that sibling sub-crates populate.

use std::collections::HashMap;
use std::sync::Arc;

pub use continuum_core::tool::{
    ProjectKind, ToolError, ToolFamily, ToolInvocation, ToolReport, ToolRunner,
};

/// In-memory registry of [`ToolRunner`] implementations.
#[derive(Default)]
pub struct ToolRegistry {
    runners: HashMap<String, Arc<dyn ToolRunner>>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("count", &self.runners.len())
            .finish()
    }
}

impl ToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            runners: HashMap::new(),
        }
    }

    /// Register a tool runner keyed by its [`ToolRunner::id`].
    pub fn register(&mut self, runner: Arc<dyn ToolRunner>) {
        let id = runner.id().to_string();
        self.runners.insert(id, runner);
    }

    /// Look up a runner by tool ID.
    pub fn get(&self, id: &str) -> Option<&Arc<dyn ToolRunner>> {
        self.runners.get(id)
    }

    /// Iterate over all registered runners.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn ToolRunner>> {
        self.runners.values()
    }

    /// Find runners that support a given project kind.
    pub fn for_project(&self, project: &ProjectKind) -> Vec<&Arc<dyn ToolRunner>> {
        self.runners
            .values()
            .filter(|r| r.supports(project))
            .collect()
    }

    /// Number of registered runners.
    pub fn len(&self) -> usize {
        self.runners.len()
    }

    /// Returns `true` if no runners are registered.
    pub fn is_empty(&self) -> bool {
        self.runners.is_empty()
    }
}
