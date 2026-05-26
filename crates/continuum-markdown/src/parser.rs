use crate::MarkdownError;
use gray_matter::{engine::YAML, Matter};
use std::path::Path;

/// Parse a markdown file with optional YAML frontmatter into a (name, value) pair.
pub fn parse_doc(path: &Path) -> Result<(String, serde_json::Value), MarkdownError> {
    let content = std::fs::read_to_string(path)?;
    let matter = Matter::<YAML>::new();
    let result = matter.parse(&content);

    let frontmatter: Option<serde_json::Value> = result.data.map(|pod| pod.into());

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| MarkdownError::Parse("invalid file name".into()))?;
    let doc_name = stem.to_uppercase();

    Ok((
        doc_name,
        serde_json::json!({
            "body": result.content,
            "frontmatter": frontmatter,
        }),
    ))
}
