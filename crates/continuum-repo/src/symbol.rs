use std::path::Path;

use continuum_core::repo::SymbolRef;
use regex::Regex;

use crate::language::Language;

/// A symbol node with metadata, extracted from a source file.
#[derive(Debug, Clone)]
pub struct SymbolNode {
    /// The symbol reference (qualified name, file, line).
    pub symbol: SymbolRef,
    /// The kind of the symbol (e.g. "fn", "struct", "enum").
    pub kind: String,
    /// The last line occupied by the symbol definition.
    pub end_line: u32,
    /// An optional doc-comment extracted from the source.
    pub doc_comment: Option<String>,
}

/// Extract all symbols from `source` written in the given `language`.
pub fn extract_symbols(path: &Path, source: &str, language: Language) -> Vec<SymbolNode> {
    match language {
        Language::Rust => extract_rust_symbols(path, source),
        Language::TypeScript => extract_typescript_symbols(path, source),
        Language::Python => extract_python_symbols(path, source),
        Language::Go => extract_go_symbols(path, source),
    }
}

fn extract_rust_symbols(path: &Path, source: &str) -> Vec<SymbolNode> {
    let re = Regex::new(
        r#"(?m)^[[:space:]]*(?:(?:pub(?:\([^)]*\))?|async|unsafe|extern\s+"[^"]*")\s+)*(?:fn|struct|enum|trait|mod|type)\s+([a-zA-Z_]\w*)"#,
    )
    .expect("valid regex");

    let kind_re = Regex::new(
        r#"(?m)^[[:space:]]*(?:(?:pub(?:\([^)]*\))?|async|unsafe|extern\s+"[^"]*")\s+)*(fn|struct|enum|trait|mod|type)\s+"#,
    )
    .expect("valid regex");

    let mut symbols = Vec::new();
    for cap in re.captures_iter(source) {
        let matched = cap.get(0).unwrap();
        let name = cap.get(1).unwrap().as_str().to_string();
        let line = source[..matched.start()].matches('\n').count() as u32 + 1;
        let kind = kind_re
            .captures(matched.as_str())
            .and_then(|k| k.get(1))
            .map(|k| k.as_str().to_string())
            .unwrap_or_else(|| "item".to_string());

        symbols.push(SymbolNode {
            symbol: SymbolRef {
                qualified: name,
                file: path.to_path_buf(),
                line,
            },
            kind,
            end_line: line,
            doc_comment: None,
        });
    }

    decorate_rust_fns(path, source, &mut symbols);

    symbols
}

fn decorate_rust_fns(_path: &Path, source: &str, symbols: &mut [SymbolNode]) {
    let fn_re = Regex::new(
        r#"(?m)^[[:space:]]*(?:(?:pub(?:\([^)]*\))?|async|unsafe|extern\s+"[^"]*")\s+)*fn\s+([a-zA-Z_]\w*)"#,
    )
    .expect("valid regex");

    for m in fn_re.find_iter(source) {
        let line = source[..m.start()].matches('\n').count() as u32 + 1;
        let brace_start = source[m.end()..].find('{');
        let semicolon = source[m.end()..].find(';');

        let end_line = match (brace_start, semicolon) {
            (Some(b), Some(s)) if s < b => line,
            (Some(b), _) => {
                let from_brace = m.end() + b;
                let result: Result<u32, u32> =
                    source[from_brace..]
                        .char_indices()
                        .try_fold(1u32, |depth, (i, ch)| {
                            let new_depth = match ch {
                                '{' => depth + 1,
                                '}' => depth - 1,
                                _ => depth,
                            };
                            if new_depth == 0 {
                                let pos = from_brace + i;
                                Err(source[..pos].matches('\n').count() as u32 + 1)
                            } else {
                                Ok(new_depth)
                            }
                        });
                match result {
                    Err(l) => l,
                    Ok(_) => line,
                }
            }
            (None, _) => line,
        };

        for sym in symbols.iter_mut().rev().take(20) {
            if sym.symbol.line == line && sym.kind == "fn" {
                sym.end_line = end_line;
                break;
            }
        }
    }
}

fn extract_typescript_symbols(path: &Path, source: &str) -> Vec<SymbolNode> {
    let re = Regex::new(
        r"(?m)^[[:space:]]*(?:export\s+)?(?:default\s+)?(?:async\s+)?(?:function\s+\*?\s*|class|interface|enum|type|module|namespace)\s+([a-zA-Z_$]\w*)",
    )
    .expect("valid regex");

    let kind_re = Regex::new(
        r"(?m)^[[:space:]]*(?:export\s+)?(?:default\s+)?(?:async\s+)?(function\s+\*?\s*|class|interface|enum|type|module|namespace)\s+",
    )
    .expect("valid regex");

    let mut symbols = Vec::new();
    for cap in re.captures_iter(source) {
        let matched = cap.get(0).unwrap();
        let name = cap.get(1).unwrap().as_str().to_string();
        let line = source[..matched.start()].matches('\n').count() as u32 + 1;
        let kind = kind_re
            .captures(matched.as_str())
            .and_then(|k| k.get(1))
            .map(|k| {
                let s = k.as_str();
                if s.starts_with("function") {
                    "fn"
                } else {
                    s.trim()
                }
            })
            .unwrap_or("item")
            .to_string();

        symbols.push(SymbolNode {
            symbol: SymbolRef {
                qualified: name,
                file: path.to_path_buf(),
                line,
            },
            kind,
            end_line: line,
            doc_comment: None,
        });
    }

    symbols
}

fn extract_python_symbols(path: &Path, source: &str) -> Vec<SymbolNode> {
    let re =
        Regex::new(r"(?m)^[[:space:]]*(?:async[[:space:]]+)?def[[:space:]]+([a-zA-Z_]\w*)\s*\(")
            .expect("valid regex");
    let class_re =
        Regex::new(r"(?m)^[[:space:]]*class[[:space:]]+([a-zA-Z_]\w*)").expect("valid regex");
    let mut symbols = Vec::new();

    for cap in re.captures_iter(source) {
        let matched = cap.get(0).unwrap();
        let line = source[..matched.start()].matches('\n').count() as u32 + 1;
        symbols.push(SymbolNode {
            symbol: SymbolRef {
                qualified: cap.get(1).unwrap().as_str().to_string(),
                file: path.to_path_buf(),
                line,
            },
            kind: "fn".into(),
            end_line: line,
            doc_comment: None,
        });
    }
    for cap in class_re.captures_iter(source) {
        let matched = cap.get(0).unwrap();
        let line = source[..matched.start()].matches('\n').count() as u32 + 1;
        symbols.push(SymbolNode {
            symbol: SymbolRef {
                qualified: cap.get(1).unwrap().as_str().to_string(),
                file: path.to_path_buf(),
                line,
            },
            kind: "class".into(),
            end_line: line,
            doc_comment: None,
        });
    }
    symbols
}

fn extract_go_symbols(path: &Path, source: &str) -> Vec<SymbolNode> {
    let func_re =
        Regex::new(r"(?m)^[[:space:]]*func[[:space:]]+(?:\([^)]*\)[[:space:]]+)?([A-Za-z_]\w*)")
            .expect("valid regex");
    let struct_re =
        Regex::new(r"(?m)^[[:space:]]*type[[:space:]]+([A-Za-z_]\w*)[[:space:]]+struct")
            .expect("valid regex");
    let iface_re =
        Regex::new(r"(?m)^[[:space:]]*type[[:space:]]+([A-Za-z_]\w*)[[:space:]]+interface")
            .expect("valid regex");
    let mut symbols = Vec::new();

    for (re, kind) in &[
        (func_re, "fn"),
        (struct_re, "struct"),
        (iface_re, "interface"),
    ] {
        for cap in re.captures_iter(source) {
            let matched = cap.get(0).unwrap();
            let line = source[..matched.start()].matches('\n').count() as u32 + 1;
            symbols.push(SymbolNode {
                symbol: SymbolRef {
                    qualified: cap.get(1).unwrap().as_str().to_string(),
                    file: path.to_path_buf(),
                    line,
                },
                kind: kind.to_string(),
                end_line: line,
                doc_comment: None,
            });
        }
    }
    symbols
}

/// Extract imported symbol names from `source` for the given `language`.
pub fn extract_imports(source: &str, language: Language) -> Vec<String> {
    match language {
        Language::Rust => source
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.starts_with("use ") {
                    let path = line
                        .strip_prefix("use ")
                        .and_then(|s| s.strip_suffix(';'))
                        .map(|s| s.trim())
                        .unwrap_or("");
                    if let Some(open) = path.find('{') {
                        let close = path[open..]
                            .find('}')
                            .map(|c| open + c)
                            .unwrap_or(path.len());
                        let items = &path[open + 1..close];
                        Some(
                            items
                                .split(',')
                                .filter_map(|item| {
                                    let item = item.trim().trim_end_matches("::self");
                                    let name = item.split("::").last()?;
                                    if name.is_empty()
                                        || name == "self"
                                        || name == "*"
                                        || name == "_"
                                    {
                                        None
                                    } else {
                                        Some(name.to_string())
                                    }
                                })
                                .collect::<Vec<_>>(),
                        )
                    } else {
                        let name = path.split("::").last()?;
                        if name.is_empty() || name == "self" || name == "*" || name == "_" {
                            None
                        } else {
                            Some(vec![name.to_string()])
                        }
                    }
                } else {
                    None
                }
            })
            .flatten()
            .collect(),
        Language::TypeScript => source
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.starts_with("import ") {
                    if let Some(open) = line.find('{') {
                        let close = line[open..]
                            .find('}')
                            .map(|c| open + c)
                            .unwrap_or(line.len());
                        let items = &line[open + 1..close];
                        Some(
                            items
                                .split(',')
                                .filter_map(|item| {
                                    let item = item.trim().trim_start_matches("type ");
                                    let name = item.split(" as ").next()?.trim();
                                    if name.is_empty() {
                                        None
                                    } else {
                                        Some(name.to_string())
                                    }
                                })
                                .collect::<Vec<_>>(),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .flatten()
            .collect(),
        Language::Python => source
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.starts_with("import ") || line.starts_with("from ") {
                    Some(vec!["python_import".to_string()])
                } else {
                    None
                }
            })
            .flatten()
            .collect(),
        Language::Go => source
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.starts_with("import ") || line.starts_with("\t\"") {
                    Some(vec!["go_import".to_string()])
                } else {
                    None
                }
            })
            .flatten()
            .collect(),
    }
}
