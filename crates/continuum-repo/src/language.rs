use std::path::Path;

/// A programming language supported for symbol extraction.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    Go,
}

impl Language {
    /// Infer the language from a file path's extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "rs" => Some(Language::Rust),
            "ts" | "tsx" => Some(Language::TypeScript),
            "py" => Some(Language::Python),
            "go" => Some(Language::Go),
            _ => None,
        }
    }

    /// Return the filename of the tree-sitter query file for this language.
    pub fn query_file(&self) -> &'static str {
        match self {
            Language::Rust => "rust.scm",
            Language::TypeScript => "typescript.scm",
            Language::Python => "python.scm",
            Language::Go => "go.scm",
        }
    }

    /// Return a slice of all supported languages.
    pub fn all() -> &'static [Language] {
        &[
            Language::Rust,
            Language::TypeScript,
            Language::Python,
            Language::Go,
        ]
    }
}
