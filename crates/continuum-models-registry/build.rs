use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(serde::Deserialize)]
struct Snapshot {
    schema_version: String,
    providers: HashMap<String, ProviderEntry>,
}

#[derive(serde::Deserialize)]
struct ProviderEntry {
    name: String,
    #[serde(default)]
    env_var: String,
    #[serde(default)]
    api_base_url: String,
    models: HashMap<String, ModelEntry>,
}

#[derive(serde::Deserialize)]
struct ModelEntry {
    name: String,
    context_window: u64,
    max_output: u64,
    tool_calls: bool,
    streaming: bool,
    pricing: Pricing,
}

#[derive(serde::Deserialize)]
struct Pricing {
    input_per_mtok: f64,
    output_per_mtok: f64,
}

fn main() {
    println!("cargo:rerun-if-changed=models-snapshot.json");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let snapshot_path = manifest.join("models-snapshot.json");

    let json = fs::read_to_string(&snapshot_path).expect("read models-snapshot.json");
    let snapshot: Snapshot = serde_json::from_str(&json).expect("parse models-snapshot.json");

    let mut code = String::new();

    code.push_str("pub const SCHEMA_VERSION: &str = ");
    code.push_str(&quote(&snapshot.schema_version));
    code.push_str(";\n\n");

    // Provider ID constants (e.g. pub const ANTHROPIC: &str = "anthropic";)
    let mut sorted_providers: Vec<_> = snapshot.providers.keys().collect();
    sorted_providers.sort();
    for pid in &sorted_providers {
        let const_name = provider_const_name(pid);
        code.push_str(&format!(
            "pub const {const_name}: &str = {q}{pid}{q};\n",
            q = "\""
        ));
    }
    code.push('\n');

    // Model ID constants
    let mut model_consts = Vec::new();
    let mut model_entries = Vec::new();
    let mut used_names = std::collections::HashSet::new();
    for pid in &sorted_providers {
        let pentry = &snapshot.providers[*pid];
        let mut sorted_models: Vec<_> = pentry.models.keys().collect();
        sorted_models.sort();
        for mid in sorted_models {
            let mentry = &pentry.models[mid];
            let full_key = format!("{pid}/{mid}");
            let const_name = model_const_name(pid, mid, &mut used_names);
            model_consts.push((const_name.clone(), full_key.clone()));

            let ipm = fmt_f64(mentry.pricing.input_per_mtok);
            let opm = fmt_f64(mentry.pricing.output_per_mtok);
            model_entries.push(format!(
                "    {q}{full_key}{q} => ModelMeta {{\
                 \n        name: {name},\
                 \n        context_window: {cw},\
                 \n        max_output: {mo},\
                 \n        tool_calls: {tc},\
                 \n        streaming: {st},\
                 \n        input_per_mtok: {ipm}_f64,\
                 \n        output_per_mtok: {opm}_f64,\
                 \n    }}",
                q = "\"",
                name = quote(&mentry.name),
                cw = mentry.context_window,
                mo = mentry.max_output,
                tc = mentry.tool_calls,
                st = mentry.streaming,
                ipm = ipm,
                opm = opm,
            ));
        }
    }

    for (const_name, full_key) in &model_consts {
        code.push_str(&format!(
            "pub const {const_name}: &str = {q}{full_key}{q};\n",
            q = "\""
        ));
    }
    code.push('\n');

    // Struct definitions
    code.push_str(
        "#[derive(Debug, Clone, Copy)]\n\
         pub struct ModelMeta {\n\
             pub name: &'static str,\n\
             pub context_window: u64,\n\
             pub max_output: u64,\n\
             pub tool_calls: bool,\n\
             pub streaming: bool,\n\
             pub input_per_mtok: f64,\n\
             pub output_per_mtok: f64,\n\
         }\n\n\
         #[derive(Debug, Clone, Copy)]\n\
         pub struct ProviderMeta {\n\
             pub name: &'static str,\n\
             pub env_var: &'static str,\n\
             pub api_base_url: &'static str,\n\
         }\n\n",
    );

    // MODELS phf map
    code.push_str("pub static MODELS: phf::Map<&'static str, ModelMeta> = phf::phf_map! {\n");
    for entry in &model_entries {
        code.push_str(entry);
        code.push_str(",\n");
    }
    code.push_str("};\n\n");

    // PROVIDERS phf map
    let mut provider_entries = Vec::new();
    for pid in &sorted_providers {
        let pentry = &snapshot.providers[*pid];
        provider_entries.push(format!(
            "    {q}{pid}{q} => ProviderMeta {{\
             \n        name: {name},\
             \n        env_var: {env_var},\
             \n        api_base_url: {api_base_url},\
             \n    }}",
            q = "\"",
            name = quote(&pentry.name),
            env_var = quote(&pentry.env_var),
            api_base_url = quote(&pentry.api_base_url),
        ));
    }
    code.push_str("pub static PROVIDERS: phf::Map<&'static str, ProviderMeta> = phf::phf_map! {\n");
    for entry in &provider_entries {
        code.push_str(entry);
        code.push_str(",\n");
    }
    code.push_str("};\n");

    let generated = out_dir.join("generated.rs");
    fs::write(&generated, code).expect("write generated.rs");
}

fn quote(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn provider_const_name(id: &str) -> String {
    let name = id.replace(['-', '.'], "_").to_uppercase();
    // Prefix with _ if starts with a digit (Rust identifiers can't start with digits)
    if name.as_bytes()[0].is_ascii_digit() {
        format!("_{name}")
    } else {
        name
    }
}

fn model_const_name(
    provider_id: &str,
    model_id: &str,
    used: &mut std::collections::HashSet<String>,
) -> String {
    // Use just the last path segment for the model part
    let core = model_id.rsplit('/').next().unwrap_or(model_id);
    let stripped = strip_date_suffix(core);
    let provider_part = provider_const_name(provider_id);
    let model_part = stripped
        .replace(|c: char| !c.is_ascii_alphanumeric() && c != '_', "_")
        .to_uppercase();
    // Collapse runs of underscores
    let mut out = String::new();
    let mut prev_us = true; // start true to trim leading _
    for ch in model_part.chars() {
        if ch == '_' {
            if !prev_us {
                out.push('_');
                prev_us = true;
            }
        } else {
            out.push(ch);
            prev_us = false;
        }
    }
    let model_part = out.trim_end_matches('_');
    let mut name = format!("{provider_part}_{model_part}");
    // Prefix with _ if starts with a digit
    if name.as_bytes()[0].is_ascii_digit() {
        name = format!("_{name}");
    }
    // Handle duplicates by appending a suffix
    if used.contains(&name) {
        let mut counter = 2u32;
        loop {
            let candidate = format!("{name}_{counter}");
            if !used.contains(&candidate) {
                name = candidate;
                break;
            }
            counter += 1;
        }
    }
    used.insert(name.clone());
    name
}

fn strip_date_suffix(s: &str) -> String {
    let mut s = s.to_string();
    // Strip 8-digit YYYYMMDD suffix (e.g. -20250514)
    loop {
        let len = s.len();
        if len > 9 && s.as_bytes()[len - 9] == b'-' {
            let tail = &s[len - 8..];
            if tail.bytes().all(|b| b.is_ascii_digit()) {
                s.truncate(len - 9);
                continue;
            }
        }
        break;
    }
    // Strip YYYY-MM-DD suffix
    loop {
        let len = s.len();
        if len > 10 && s.as_bytes()[len - 11] == b'-' {
            let tail = &s[len - 10..];
            let b = tail.as_bytes();
            if b[4] == b'-'
                && b[7] == b'-'
                && b[..4].iter().all(|x| x.is_ascii_digit())
                && b[5..7].iter().all(|x| x.is_ascii_digit())
                && b[8..10].iter().all(|x| x.is_ascii_digit())
            {
                s.truncate(len - 11);
                continue;
            }
        }
        break;
    }
    // Strip -latest suffix
    if let Some(rest) = s.strip_suffix("-latest") {
        s = rest.to_string();
    }
    s
}

fn fmt_f64(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        // Ensure the number parses as a float literal
        let s = format!("{v}");
        if s.contains('.') {
            s
        } else {
            format!("{s}.0")
        }
    }
}
