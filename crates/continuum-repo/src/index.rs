use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::time::SystemTime;

use continuum_core::repo::{ImpactSet, ProposedChange, RepoIndex, SymbolQuery, SymbolRef};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;

use crate::symbol::SymbolNode;

/// A concrete implementation of `RepoIndex` backed by a dependency graph and symbol maps.
pub struct RepoIndexImpl {
    /// Symbols grouped by file path.
    pub symbols: BTreeMap<PathBuf, Vec<SymbolNode>>,
    /// Directed graph of symbol dependencies.
    pub graph: DiGraph<SymbolRef, ()>,
    /// Maps each `SymbolRef` to its node index in the graph.
    pub node_indices: HashMap<SymbolRef, NodeIndex>,
    /// Maps file paths to their last-known modification timestamps.
    pub mtimes: HashMap<PathBuf, SystemTime>,
}

impl RepoIndexImpl {
    /// Create an empty index with no symbols, graph nodes, or mtimes.
    pub fn new() -> Self {
        Self {
            symbols: BTreeMap::new(),
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
            mtimes: HashMap::new(),
        }
    }

    /// Check which files have changed since the last index by comparing mtime.
    /// Returns `true` if any file was added or modified.
    pub fn has_changes_since_last_index(&self, files: &[PathBuf]) -> bool {
        for file in files {
            let current = std::fs::metadata(file).and_then(|m| m.modified()).ok();
            let stored = self.mtimes.get(file);
            match (current, stored) {
                (Some(c), Some(s)) if c == *s => continue,
                _ => return true,
            }
        }
        false
    }

    /// Update stored mtimes for the given files after re-indexing.
    pub fn update_mtimes(&mut self, files: &[PathBuf]) {
        for file in files {
            if let Ok(meta) = std::fs::metadata(file) {
                if let Ok(mtime) = meta.modified() {
                    self.mtimes.insert(file.clone(), mtime);
                }
            }
        }
    }
}

impl Default for RepoIndexImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl RepoIndex for RepoIndexImpl {
    fn symbols(&self, query: SymbolQuery) -> Vec<SymbolRef> {
        self.symbols
            .values()
            .flatten()
            .filter(|n| n.symbol.qualified.contains(&query.needle))
            .map(|n| n.symbol.clone())
            .take(query.limit as usize)
            .collect()
    }

    fn dependents_of(&self, sym: SymbolRef) -> Vec<SymbolRef> {
        self.node_indices
            .get(&sym)
            .map(|&idx| {
                self.graph
                    .neighbors_directed(idx, Direction::Incoming)
                    .map(|n| self.graph[n].clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn dependencies_of(&self, sym: SymbolRef) -> Vec<SymbolRef> {
        self.node_indices
            .get(&sym)
            .map(|&idx| {
                self.graph
                    .neighbors_directed(idx, Direction::Outgoing)
                    .map(|n| self.graph[n].clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn impact_set(&self, change: &ProposedChange) -> ImpactSet {
        let changed: HashSet<&PathBuf> = change.files.iter().collect();
        let removed: HashSet<&SymbolRef> = change.removed_symbols.iter().collect();

        let mut broken = Vec::new();
        let mut affected = Vec::new();

        for (file, syms) in &self.symbols {
            if changed.contains(file) {
                for node in syms {
                    if removed.contains(&node.symbol) {
                        broken.push(node.symbol.clone());
                    } else {
                        affected.push(node.symbol.clone());
                    }
                }
            }
        }

        let mut visited: HashSet<SymbolRef> =
            broken.iter().chain(affected.iter()).cloned().collect();
        let mut queue: VecDeque<SymbolRef> =
            broken.iter().chain(affected.iter()).cloned().collect();

        while let Some(sym) = queue.pop_front() {
            for dep in self.dependents_of(sym) {
                if visited.insert(dep.clone()) {
                    affected.push(dep.clone());
                    queue.push_back(dep);
                }
            }
        }

        ImpactSet { broken, affected }
    }
}

/// Build a dependency graph from the extracted symbols and per-file import lists.
///
/// Each edge goes from the importing symbol to the imported symbol. Returns the graph
/// and a lookup map from `SymbolRef` to `NodeIndex`.
pub fn build_graph(
    symbols: &BTreeMap<PathBuf, Vec<SymbolNode>>,
    imports: &BTreeMap<PathBuf, Vec<String>>,
) -> (DiGraph<SymbolRef, ()>, HashMap<SymbolRef, NodeIndex>) {
    let mut graph = DiGraph::new();
    let mut node_indices = HashMap::new();

    for syms in symbols.values() {
        for node in syms {
            let idx = graph.add_node(node.symbol.clone());
            node_indices.insert(node.symbol.clone(), idx);
        }
    }

    let name_to_idx: HashMap<&str, NodeIndex> = node_indices
        .iter()
        .map(|(sym, idx)| (sym.qualified.as_str(), *idx))
        .collect();

    for (path, file_imports) in imports {
        if let Some(syms) = symbols.get(path) {
            for sym in syms {
                if let Some(&from_idx) = node_indices.get(&sym.symbol) {
                    for import_name in file_imports {
                        if let Some(&to_idx) = name_to_idx.get(import_name.as_str()) {
                            graph.add_edge(from_idx, to_idx, ());
                        }
                    }
                }
            }
        }
    }

    (graph, node_indices)
}
