use continuum_core::planner::EngineeringDocs;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use walkdir::WalkDir;

use crate::parser::parse_doc;
use crate::{MarkdownError, DOC_NAMES};

/// Scan `docs/` under `root`, parse each recognized file, and return [`EngineeringDocs`].
pub fn load(root: &Path) -> Result<EngineeringDocs, MarkdownError> {
    let docs_dir = root.join("docs");
    let mut docs = BTreeMap::new();
    let mut found_required = HashSet::new();
    let required = ["ARCHITECTURE", "MODEL_RULES"];

    for entry in WalkDir::new(&docs_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                let upper = stem.to_uppercase();
                if DOC_NAMES.contains(&upper.as_str()) {
                    let (name, value) = parse_doc(entry.path())?;
                    docs.insert(name, value);
                    found_required.insert(upper);
                }
            }
        }
    }

    for r in &required {
        if !found_required.contains(*r) {
            return Err(MarkdownError::MissingRequiredDoc(r.to_string()));
        }
    }

    let mut engineering_docs = EngineeringDocs::default();
    engineering_docs.docs = docs;
    Ok(engineering_docs)
}
