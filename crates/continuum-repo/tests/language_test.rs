use continuum_repo::Language;
use std::path::Path;

#[test]
fn test_language_from_path() {
    assert_eq!(
        Language::from_path(Path::new("foo.rs")),
        Some(Language::Rust)
    );
    assert_eq!(
        Language::from_path(Path::new("foo.ts")),
        Some(Language::TypeScript)
    );
    assert_eq!(
        Language::from_path(Path::new("foo.py")),
        Some(Language::Python)
    );
    assert_eq!(Language::from_path(Path::new("foo.go")), Some(Language::Go));
    assert_eq!(Language::from_path(Path::new("foo.js")), None);
}

#[test]
fn test_language_query_file() {
    assert_eq!(Language::Rust.query_file(), "rust.scm");
    assert_eq!(Language::TypeScript.query_file(), "typescript.scm");
    assert_eq!(Language::Python.query_file(), "python.scm");
    assert_eq!(Language::Go.query_file(), "go.scm");
}

#[test]
fn test_language_all() {
    let all = Language::all();
    assert!(all.contains(&Language::Rust));
    assert!(all.contains(&Language::Go));
    assert_eq!(all.len(), 4);
}
