use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use continuum_core::repo::{
    ContextBundle, ContextQuery, ContextSnippet, IndexOptions, RepoError, RepoIndex, RepoLoader,
    SymbolQuery, SymbolRef,
};
use tokio::sync::RwLock;

use crate::index::{build_graph, RepoIndexImpl};
use crate::language::Language;
use crate::symbol::{extract_imports, extract_symbols, SymbolNode};

pub struct Loader {
    index: Arc<RwLock<Option<Arc<RepoIndexImpl>>>>,
    root: PathBuf,
}

impl Loader {
    pub fn new(root: PathBuf) -> Self {
        Loader {
            index: Arc::new(RwLock::new(None)),
            root,
        }
    }
}

#[async_trait]
impl RepoLoader for Loader {
    async fn build(
        &self,
        root: &Path,
        opts: IndexOptions,
    ) -> Result<Arc<dyn RepoIndex>, RepoError> {
        let mut paths = Vec::new();
        let max = opts.max_files;

        let walker = ignore::WalkBuilder::new(root)
            .git_ignore(opts.respect_gitignore)
            .build();

        for entry in walker {
            let entry = entry.map_err(|e| RepoError::Io(e.to_string()))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if Language::from_path(path).is_none() {
                continue;
            }
            if max > 0 && paths.len() >= max as usize {
                break;
            }
            paths.push(path.to_path_buf());
        }

        let mtimes: HashMap<PathBuf, SystemTime> = paths
            .iter()
            .filter_map(|p| {
                std::fs::metadata(p)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| (p.clone(), t))
            })
            .collect();

        let tasks: Vec<_> = paths
            .into_iter()
            .map(|path| {
                tokio::task::spawn_blocking(move || -> Result<_, RepoError> {
                    let source =
                        std::fs::read_to_string(&path).map_err(|e| RepoError::Io(e.to_string()))?;
                    let lang = match Language::from_path(&path) {
                        Some(l) => l,
                        None => return Ok((path, Vec::new(), Vec::new())),
                    };
                    let symbols = extract_symbols(&path, &source, lang);
                    let imports = extract_imports(&source, lang);
                    Ok((path, symbols, imports))
                })
            })
            .collect();

        let mut all_symbols: BTreeMap<PathBuf, Vec<SymbolNode>> = BTreeMap::new();
        let mut all_imports: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();

        for task in tasks {
            match task
                .await
                .map_err(|e| RepoError::Other(format!("join error: {e}")))?
            {
                Ok((path, symbols, imports)) => {
                    all_symbols.insert(path.clone(), symbols);
                    all_imports.insert(path, imports);
                }
                Err(e) => {
                    tracing::warn!("skipping file: {e}");
                }
            }
        }

        let (graph, node_indices) = build_graph(&all_symbols, &all_imports);
        let index = Arc::new(RepoIndexImpl {
            symbols: all_symbols,
            graph,
            node_indices,
            mtimes,
        });

        *self.index.write().await = Some(index.clone());
        Ok(index)
    }

    async fn refresh(&self, paths: &[PathBuf]) -> Result<(), RepoError> {
        let guard = self.index.read().await;
        let existing = guard.as_ref().cloned();
        drop(guard);

        let Some(old_index) = existing else {
            let opts = IndexOptions::default();
            return self.build(&self.root, opts).await.map(|_| ());
        };

        let changed: Vec<PathBuf> = paths
            .iter()
            .filter(|p| {
                let current = std::fs::metadata(p).ok().and_then(|m| m.modified().ok());
                let stored = old_index.mtimes.get(p.as_path());
                current
                    .map(|c| stored.map_or(true, |s| c != *s))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        if changed.is_empty() {
            return Ok(());
        }

        let root = self.root.clone();
        let opts = IndexOptions::default();
        self.build(&root, opts).await?;
        Ok(())
    }

    async fn retrieve(&self, q: ContextQuery) -> Result<ContextBundle, RepoError> {
        let guard = self.index.read().await;
        let index = guard
            .as_ref()
            .ok_or_else(|| RepoError::Other("index not built; call build() first".into()))?;

        let mut snippets = Vec::new();
        let mut tokens = 0u32;
        let mut used_files = std::collections::HashSet::new();

        let search_hits: Vec<SymbolRef> = if q.anchors.is_empty() {
            index.symbols(SymbolQuery {
                needle: q.text,
                limit: 20,
            })
        } else {
            q.anchors.clone()
        };

        for sym in search_hits {
            if tokens >= q.token_budget {
                break;
            }
            if !used_files.insert(sym.file.clone()) {
                continue;
            }

            let source = tokio::fs::read_to_string(&sym.file)
                .await
                .map_err(|e| RepoError::Io(e.to_string()))?;
            let lines: Vec<&str> = source.lines().collect();
            let zero_idx = (sym.line as usize).saturating_sub(1);
            let start = zero_idx.saturating_sub(3);
            let end = std::cmp::min(zero_idx + 5, lines.len());

            if end > start {
                let text = lines[start..end].join("\n");
                tokens += text.len() as u32 / 4;
                snippets.push(ContextSnippet {
                    file: sym.file.clone(),
                    line: start as u32 + 1,
                    text,
                });
            }
        }

        Ok(ContextBundle { snippets, tokens })
    }
}
