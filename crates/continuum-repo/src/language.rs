use std::path::Path;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    Go,
}

impl Language {
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "rs" => Some(Language::Rust),
            "ts" | "tsx" => Some(Language::TypeScript),
            "py" => Some(Language::Python),
            "go" => Some(Language::Go),
            _ => None,
        }
    }

    pub fn query_file(&self) -> &'static str {
        match self {
            Language::Rust => "rust.scm",
            Language::TypeScript => "typescript.scm",
            Language::Python => "python.scm",
            Language::Go => "go.scm",
        }
    }

    pub fn all() -> &'static [Language] {
        &[Language::Rust, Language::TypeScript, Language::Python, Language::Go]
    }
}
