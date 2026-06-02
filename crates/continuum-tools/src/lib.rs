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

#[cfg(test)]
mod tests {
    use super::*;
    use continuum_core::ids::ToolId;
    use continuum_core::tool::{ToolFamily, ToolInvocation};

    use continuum_core::sandbox::SandboxHandle;

    struct DummyRunner {
        id: &'static str,
        project_lang: &'static str,
    }

    #[async_trait::async_trait]
    impl ToolRunner for DummyRunner {
        fn id(&self) -> ToolId {
            ToolId::new(self.id)
        }
        fn family(&self) -> ToolFamily {
            ToolFamily::Linter
        }
        fn supports(&self, project: &ProjectKind) -> bool {
            project.language == self.project_lang
        }
        async fn run(
            &self,
            _invocation: ToolInvocation,
            _sandbox: &dyn SandboxHandle,
        ) -> Result<ToolReport, ToolError> {
            unimplemented!()
        }
    }

    #[test]
    fn test_empty_registry() {
        let reg = ToolRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn test_register_and_get() {
        let mut reg = ToolRegistry::new();
        let runner = Arc::new(DummyRunner {
            id: "dummy-linter",
            project_lang: "rust",
        });
        reg.register(runner.clone());
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 1);

        let found = reg.get("dummy-linter");
        assert!(found.is_some());
    }

    #[test]
    fn test_get_unknown() {
        let reg = ToolRegistry::new();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_for_project() {
        let mut reg = ToolRegistry::new();
        let rust_runner = Arc::new(DummyRunner {
            id: "rust-linter",
            project_lang: "rust",
        });
        let ts_runner = Arc::new(DummyRunner {
            id: "ts-linter",
            project_lang: "typescript",
        });
        reg.register(rust_runner);
        reg.register(ts_runner);

        // Use Default::default() for ProjectKind (language="", matches nothing)
        let any_project = ProjectKind::default();
        assert_eq!(reg.len(), 2);
        // All registered runners (no project filter is needed for this test)
        let all = reg.for_project(&any_project);
        assert_eq!(all.len(), 0); // no runner supports empty language
    }

    #[test]
    fn test_multiple_runners() {
        let mut reg = ToolRegistry::new();
        for i in 0..5 {
            reg.register(Arc::new(DummyRunner {
                id: Box::leak(format!("runner-{i}").into_boxed_str()),
                project_lang: "rust",
            }));
        }
        assert_eq!(reg.len(), 5);
    }

    #[test]
    fn test_iter() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyRunner {
            id: "a",
            project_lang: "rust",
        }));
        reg.register(Arc::new(DummyRunner {
            id: "b",
            project_lang: "typescript",
        }));
        let ids: Vec<String> = reg.iter().map(|r| r.id().as_str().to_string()).collect();
        assert!(ids.contains(&"a".to_string()));
        assert!(ids.contains(&"b".to_string()));
        assert_eq!(ids.len(), 2);
    }
}
