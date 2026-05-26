use continuum_core::planner::{EngineeringDocs, PlanError, RepoAnalysis};
use continuum_core::repo::RepoIndex;
use std::sync::Arc;

/// Analyze repository symbols and produce a [`RepoAnalysis`].
pub async fn analyze(
    repo: Arc<dyn RepoIndex>,
    _docs: &EngineeringDocs,
) -> Result<RepoAnalysis, PlanError> {
    let query = serde_json::from_value(serde_json::json!({
        "needle": "",
        "limit": 10000,
    }))
    .map_err(|e| PlanError::AnalysisFailed(e.to_string()))?;

    let all_symbols = repo.symbols(query);
    let symbol_count = all_symbols.len();

    let mut entry_points = Vec::new();
    for sym in &all_symbols {
        if let Some(stem) = sym.file.file_stem().and_then(|s| s.to_str()) {
            let lower = stem.to_lowercase();
            if lower == "main" || lower == "index" {
                entry_points.push(sym.file.display().to_string());
            }
        }
    }
    entry_points.sort();
    entry_points.dedup();

    let mut analysis = RepoAnalysis::default();
    analysis.summary = format!(
        "Repository contains {} symbols across the index. {} entry points detected.",
        symbol_count,
        entry_points.len(),
    );
    analysis.services = entry_points;
    Ok(analysis)
}
