use continuum_repo::{Language, extract_symbols, extract_imports};
use std::path::Path;

#[test]
fn test_extract_rust_function_symbol() {
    let source = r#"
fn hello() {}
pub fn add(a: i32, b: i32) -> i32 { a + b }
"#;
    let symbols = extract_symbols(Path::new("lib.rs"), source, Language::Rust);
    assert!(!symbols.is_empty(), "should find at least 'hello' and 'add'");
    let names: Vec<&str> = symbols.iter().map(|s| s.symbol.qualified.as_str()).collect();
    assert!(names.contains(&"hello"));
    assert!(names.contains(&"add"));
}

#[test]
fn test_extract_python_function_symbol() {
    let source = r#"
def hello():
    pass

async def fetch():
    return 1

class MyClass:
    pass
"#;
    let symbols = extract_symbols(Path::new("main.py"), source, Language::Python);
    let names: Vec<&str> = symbols.iter().map(|s| s.symbol.qualified.as_str()).collect();
    assert!(names.contains(&"hello"), "should find 'hello', got {:?}", names);
    assert!(names.contains(&"MyClass"), "should find 'MyClass', got {:?}", names);
}

#[test]
fn test_extract_go_function_symbol() {
    let source = r#"
package main

func Hello() string { return "hi" }
func (s *Server) Serve() error { return nil }

type Config struct {
    Port int
}

type Storage interface {
    Get(key string) (string, error)
}
"#;
    let symbols = extract_symbols(Path::new("main.go"), source, Language::Go);
    let names: Vec<&str> = symbols.iter().map(|s| s.symbol.qualified.as_str()).collect();
    assert!(names.contains(&"Hello"));
    assert!(names.contains(&"Config"));
    assert!(names.contains(&"Storage"));
}

#[test]
fn test_extract_imports_rust() {
    let source = "use std::collections::HashMap;\nuse serde::{Serialize, Deserialize};";
    let imports = extract_imports(source, Language::Rust);
    assert!(imports.len() >= 2);
}

#[test]
fn test_extract_imports_python() {
    let source = "import os\nfrom pathlib import Path";
    let imports = extract_imports(source, Language::Python);
    assert!(!imports.is_empty());
}
