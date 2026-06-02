use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "xtask", about = "Continuum workspace automation")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Re-fetch <https://models.dev/api.json> and overwrite the vendored snapshot.
    RefreshModels,
    /// Regenerate JSON schemas for the engineering docs.
    GenSchemas,
    /// Run release-prep checks.
    Release,
    /// Run benchmarks.
    Bench,
    /// Lint for `Cap<T>::grant()` calls outside continuum-security.
    SecurityLint,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::RefreshModels => refresh_models(),
        Command::GenSchemas => gen_schemas(),
        Command::Release => release(),
        Command::Bench => bench(),
        Command::SecurityLint => security_lint(),
    }
}

fn snapshot_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap()
        .join("crates/continuum-models-registry/models-snapshot.json")
}

fn refresh_models() -> Result<(), Box<dyn std::error::Error>> {
    let path = snapshot_path();

    match try_fetch_models() {
        Ok(value) => {
            let providers = value["providers"].as_object().unwrap();
            let model_count: usize = providers
                .values()
                .filter_map(|v| v["models"].as_object())
                .map(|m| m.len())
                .sum();

            let pretty = serde_json::to_string_pretty(&value)?;
            fs::write(&path, &pretty)?;
            println!("✓ Fetched and validated models snapshot");
            println!("  Providers: {}", providers.len());
            println!("  Models:    {model_count}");
            println!("  Written:   {}", path.display());
        }
        Err(e) => {
            eprintln!("⚠ Could not fetch remote models: {e}");
            eprintln!("  Using local snapshot as-is.");
        }
    }

    Ok(())
}

fn try_fetch_models() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("continuum-xtask/0.0.0")
        .build()?;

    let resp = client.get("https://models.dev/api.json").send()?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()).into());
    }
    let raw: serde_json::Value = resp.json()?;

    // models.dev top-level is either:
    //   A) our internal format (schema_version key present) — already transformed
    //   B) models.dev native format (provider objects with `env`, `api`, `models[]`)
    if raw.get("schema_version").is_some() {
        // Already our format — just stamp fetched_at and return.
        let mut value = raw;
        let now = iso_timestamp();
        if let Some(obj) = value.as_object_mut() {
            obj.insert("fetched_at".to_string(), serde_json::Value::String(now));
        }
        validate_snapshot(&value)?;
        return Ok(value);
    }

    // Transform models.dev native format → our internal snapshot format.
    let raw_obj = raw
        .as_object()
        .ok_or("models.dev response is not a JSON object")?;

    let mut providers = serde_json::Map::new();

    for (provider_id, provider_val) in raw_obj {
        let pobj = match provider_val.as_object() {
            Some(o) => o,
            None => continue,
        };

        let name = pobj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(provider_id)
            .to_string();

        // First env var becomes the env_var field.
        let env_var = pobj
            .get("env")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let api_base_url = pobj
            .get("api")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // models.dev `models` can be an array or an object.
        let models_val = match pobj.get("models") {
            Some(v) => v,
            None => continue,
        };

        let model_iter: Box<dyn Iterator<Item = (String, &serde_json::Value)>> =
            if let Some(arr) = models_val.as_array() {
                Box::new(arr.iter().filter_map(|m| {
                    m.get("id")
                        .and_then(|v| v.as_str())
                        .map(|id| (id.to_string(), m))
                }))
            } else if let Some(obj) = models_val.as_object() {
                Box::new(obj.iter().map(|(k, v)| (k.clone(), v)))
            } else {
                continue;
            };

        let mut models_out = serde_json::Map::new();
        for (model_id, model_val) in model_iter {
            let mobj = match model_val.as_object() {
                Some(o) => o,
                None => continue,
            };

            let model_name = mobj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&model_id)
                .to_string();

            // context window: limit.context or context_window
            let context_window = mobj
                .get("limit")
                .and_then(|v| v.get("context"))
                .and_then(|v| v.as_u64())
                .or_else(|| mobj.get("context_window").and_then(|v| v.as_u64()))
                .unwrap_or(4096);

            // max output: limit.output or max_output
            let max_output = mobj
                .get("limit")
                .and_then(|v| v.get("output"))
                .and_then(|v| v.as_u64())
                .or_else(|| mobj.get("max_output").and_then(|v| v.as_u64()))
                .unwrap_or(4096);

            // tool calls: tool_call (singular from models.dev) or tool_calls
            let tool_calls = mobj
                .get("tool_call")
                .and_then(|v| v.as_bool())
                .or_else(|| mobj.get("tool_calls").and_then(|v| v.as_bool()))
                .unwrap_or(false);

            // streaming: most modern models support it; default true
            let streaming = mobj
                .get("streaming")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            // pricing: cost.input / cost.output or pricing.input_per_mtok
            let input_per_mtok = mobj
                .get("cost")
                .and_then(|v| v.get("input"))
                .and_then(|v| v.as_f64())
                .or_else(|| {
                    mobj.get("pricing")
                        .and_then(|v| v.get("input_per_mtok"))
                        .and_then(|v| v.as_f64())
                })
                .unwrap_or(0.0);

            let output_per_mtok = mobj
                .get("cost")
                .and_then(|v| v.get("output"))
                .and_then(|v| v.as_f64())
                .or_else(|| {
                    mobj.get("pricing")
                        .and_then(|v| v.get("output_per_mtok"))
                        .and_then(|v| v.as_f64())
                })
                .unwrap_or(0.0);

            models_out.insert(
                model_id,
                serde_json::json!({
                    "name": model_name,
                    "context_window": context_window,
                    "max_output": max_output,
                    "tool_calls": tool_calls,
                    "streaming": streaming,
                    "pricing": {
                        "input_per_mtok": input_per_mtok,
                        "output_per_mtok": output_per_mtok,
                    }
                }),
            );
        }

        if models_out.is_empty() {
            continue;
        }

        providers.insert(
            provider_id.clone(),
            serde_json::json!({
                "name": name,
                "env_var": env_var,
                "api_base_url": api_base_url,
                "models": models_out,
            }),
        );
    }

    if providers.is_empty() {
        return Err("models.dev returned no usable providers".into());
    }

    let value = serde_json::json!({
        "schema_version": "1.0.0",
        "fetched_at": iso_timestamp(),
        "providers": providers,
    });

    validate_snapshot(&value)?;
    Ok(value)
}

fn validate_snapshot(value: &serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
    let obj = value.as_object().ok_or("root is not an object")?;

    obj.get("schema_version")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or("missing or empty schema_version")?;

    let providers = obj
        .get("providers")
        .and_then(|v| v.as_object())
        .ok_or("missing providers object")?;

    if providers.is_empty() {
        return Err("providers object is empty".into());
    }

    for (pid, pv) in providers {
        let pobj = pv
            .as_object()
            .ok_or_else(|| format!("provider '{pid}' is not an object"))?;
        pobj.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("provider '{pid}' missing name"))?;

        let models = pobj
            .get("models")
            .and_then(|v| v.as_object())
            .ok_or_else(|| format!("provider '{pid}' missing models object"))?;

        for (mid, mv) in models {
            let mobj = mv
                .as_object()
                .ok_or_else(|| format!("model '{mid}' is not an object"))?;

            mobj.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("model '{mid}' missing name"))?;
            mobj.get("context_window")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| format!("model '{mid}' missing context_window"))?;
        }
    }

    Ok(())
}

fn iso_timestamp() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = d.as_secs();
    let days = total_secs / 86400;
    let time_secs = total_secs % 86400;

    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let dim = if is_leap(y) { 366 } else { 365 };
        if remaining < dim {
            break;
        }
        remaining -= dim;
        y += 1;
    }

    let is_ly = is_leap(y);
    let month_days: [i64; 12] = [
        31,
        if is_ly { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 1u32;
    for &md in &month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        m += 1;
    }

    let h = time_secs / 3600;
    let min = (time_secs % 3600) / 60;
    let sec = time_secs % 60;

    format!(
        "{y:04}-{m:02}-{:02}T{h:02}:{min:02}:{sec:02}Z",
        remaining + 1
    )
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn gen_schemas() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest.parent().unwrap();
    let docs_dir = workspace_root.join("docs");

    if !docs_dir.exists() {
        println!("⚠ docs/ directory not found at {}", docs_dir.display());
        return Ok(());
    }

    let mut count = 0u32;
    for entry in fs::read_dir(&docs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "md") {
            let content = fs::read_to_string(&path)?;
            if content.contains("---") && content.starts_with("---") {
                // Has YAML frontmatter — extract and output schema
                let schema_path = path.with_extension("schema.json");
                let schema = serde_json::json!({
                    "file": path.file_name().unwrap().to_str().unwrap_or(""),
                    "has_frontmatter": true,
                    "size_bytes": content.len(),
                    "frontmatter_fields": extract_frontmatter_fields(&content),
                });
                fs::write(&schema_path, serde_json::to_string_pretty(&schema)?)?;
                println!("  ✓ generated schema for {}", path.display());
                count += 1;
            }
        }
    }

    if count == 0 {
        println!("  ℹ No markdown files with YAML frontmatter found.");
    }
    println!("✓ gen-schemas: generated {count} schema(s)");

    Ok(())
}

fn extract_frontmatter_fields(content: &str) -> Vec<String> {
    let mut fields = Vec::new();
    if let Some(end) = content[3..].find("---") {
        let frontmatter = &content[3..3 + end];
        for line in frontmatter.lines() {
            if let Some(pos) = line.find(':') {
                let key = line[..pos].trim().to_string();
                if !key.is_empty() {
                    fields.push(key);
                }
            }
        }
    }
    fields
}

fn release() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest.parent().unwrap();

    println!("--- Release checks ---\n");

    // 1. Check workspace Cargo.toml version
    let cargo_toml = workspace_root.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml)?;
    let version = content
        .lines()
        .find(|l| l.starts_with("version ="))
        .map(|l| l.trim().trim_start_matches("version = ").trim_matches('"'))
        .unwrap_or("unknown");
    println!("  Workspace version: {version}");

    // 2. Check git status
    let git_status = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(workspace_root)
        .output();
    match git_status {
        Ok(out) => {
            let changes = String::from_utf8_lossy(&out.stdout);
            let change_count = changes.lines().filter(|l| !l.is_empty()).count();
            if change_count > 0 {
                println!("  ⚠ {change_count} uncommitted change(s)");
            } else {
                println!("  ✓ Clean working tree");
            }
        }
        Err(_) => println!("  ⚠ Could not check git status (not a git repo?)"),
    }

    // 3. Check changelog
    let changelog = workspace_root.join("CHANGELOG.md");
    if changelog.exists() {
        println!("  ✓ CHANGELOG.md found");
    } else {
        println!("  ⚠ CHANGELOG.md not found (create one for release)");
    }

    // 4. Quick cargo check
    let check = std::process::Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(workspace_root)
        .output();
    match check {
        Ok(out) if out.status.success() => println!("  ✓ cargo check passed"),
        _ => println!("  ⚠ cargo check failed — run `cargo check` manually"),
    }

    println!("\n✓ Release checks complete.");
    Ok(())
}

fn bench() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest.parent().unwrap();

    println!("--- Benchmarks ---\n");

    // Run cargo test --release as a basic benchmark proxy
    let start = std::time::Instant::now();
    let result = std::process::Command::new("cargo")
        .args(["test", "--quiet", "--lib"])
        .current_dir(workspace_root)
        .output();

    let elapsed = start.elapsed();
    match result {
        Ok(out) if out.status.success() => {
            println!(
                "  ✓ All library tests passed ({:.2}s)",
                elapsed.as_secs_f64()
            );
        }
        Ok(_) => {
            println!("  ⚠ Some tests failed ({:.2}s)", elapsed.as_secs_f64());
            return Ok(());
        }
        Err(e) => {
            println!("  ⚠ Could not run tests: {e}");
            return Ok(());
        }
    }

    // Workspace stats
    let crate_count = workspace_root
        .join("crates")
        .read_dir()
        .map(|d| d.count())
        .unwrap_or(0);
    let loc = count_loc(&workspace_root.join("crates"));
    println!("  Crates: {crate_count}");
    println!("  Lines of code (crates/): {loc}");
    println!("\n✓ Bench complete ({:.2}s)", elapsed.as_secs_f64());
    Ok(())
}

fn count_loc(dir: &std::path::Path) -> usize {
    let mut total = 0usize;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                total += count_loc(&path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                if let Ok(content) = fs::read_to_string(&path) {
                    total += content.lines().count();
                }
            }
        }
    }
    total
}

/// Lint for `Cap::<*>::grant()` calls outside `continuum-security`.
fn security_lint() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest.parent().unwrap();
    let security_crate = workspace_root.join("crates/continuum-security");

    let mut found_issues = false;

    // Walk all crate directories under /crates (excluding continuum-security itself)
    let crates_dir = workspace_root.join("crates");
    for entry in fs::read_dir(&crates_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.canonicalize()? == security_crate.canonicalize()? {
            continue;
        }

        // Search for Cap::<...>::grant() or Cap::grant() in .rs files
        let src_dir = path.join("src");
        if !src_dir.exists() {
            continue;
        }
        found_issues |= lint_directory(&src_dir)?;
    }

    if found_issues {
        println!("\n❌ Security lint FAILED: Cap::grant() found outside continuum-security.");
        println!("   Only continuum-security may mint capability tokens.");
        std::process::exit(1);
    } else {
        println!("✅ Security lint passed: no Cap::grant() outside continuum-security.");
    }

    Ok(())
}

fn lint_directory(dir: &PathBuf) -> Result<bool, Box<dyn std::error::Error>> {
    let mut found = false;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            found |= lint_directory(&path)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            let content = fs::read_to_string(&path)?;
            if content.contains("Cap::<") && content.contains("::grant()") {
                println!("  ⚠  {}: uses Cap::grant()", path.display());
                found = true;
            }
            if content.contains("Cap::grant()") {
                println!("  ⚠  {}: uses Cap::grant()", path.display());
                found = true;
            }
        }
    }
    Ok(found)
}
