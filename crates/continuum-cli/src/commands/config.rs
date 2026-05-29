use continuum_config::Config;
use continuum_models_registry::PROVIDERS;

use super::{CmdResult, ConfigArgs, ConfigSubcommand};

pub async fn run(args: ConfigArgs) -> CmdResult {
    match args.command {
        ConfigSubcommand::Set { key, value } => cmd_set(&key, &value),
        ConfigSubcommand::Get { key } => cmd_get(&key),
        ConfigSubcommand::Unset { key } => cmd_unset(&key),
        ConfigSubcommand::List => cmd_list(),
        ConfigSubcommand::Providers => cmd_providers(),
    }
}

/// `continuum config set anthropic.api_key sk-ant-...`
fn cmd_set(key: &str, value: &str) -> CmdResult {
    let (provider, field) = parse_key(key)?;
    let mut cfg = Config::load();

    match field {
        "api_key" => cfg.set_api_key(provider, value),
        "base_url" => cfg.set_base_url(provider, value),
        other => {
            return Err(
                format!("unknown config field '{other}'. Valid fields: api_key, base_url").into(),
            )
        }
    }

    cfg.save().map_err(|e| e.to_string())?;

    let display = if field == "api_key" {
        mask_key(value)
    } else {
        value.to_string()
    };
    println!("✓ Set {provider}.{field} = {display}");
    if let Some(path) = continuum_config::config_path() {
        println!("  Saved to: {}", path.display());
    }
    Ok(())
}

/// `continuum config get anthropic.api_key`
fn cmd_get(key: &str) -> CmdResult {
    let (provider, field) = parse_key(key)?;
    let cfg = Config::load();

    let value = match field {
        "api_key" => {
            // Try env vars first, then config file
            let env_hint = PROVIDERS.get(provider).map(|p| p.env_var).unwrap_or("");
            cfg.api_key(provider, env_hint)
        }
        "base_url" => cfg
            .base_url(provider)
            .or_else(|| PROVIDERS.get(provider).map(|p| p.api_base_url.to_string())),
        other => return Err(format!("unknown field '{other}'. Valid: api_key, base_url").into()),
    };

    match value {
        Some(v) => {
            let display = if field == "api_key" { mask_key(&v) } else { v };
            println!("{provider}.{field} = {display}");
        }
        None => println!("{provider}.{field} = (not set)"),
    }
    Ok(())
}

/// `continuum config unset anthropic.api_key`
fn cmd_unset(key: &str) -> CmdResult {
    let (provider, field) = parse_key(key)?;
    let mut cfg = Config::load();
    cfg.unset(provider, field);
    cfg.save().map_err(|e| e.to_string())?;
    println!("✓ Unset {provider}.{field}");
    Ok(())
}

/// `continuum config list`
fn cmd_list() -> CmdResult {
    let cfg = Config::load();

    if let Some(path) = continuum_config::config_path() {
        println!("Config file: {}", path.display());
        if !path.exists() {
            println!("  (file does not exist yet — use `continuum config set` to create it)");
        }
    }
    println!();

    if cfg.providers.is_empty() {
        println!("No providers configured.");
        println!("Run `continuum config providers` to see available providers.");
        return Ok(());
    }

    let mut sorted: Vec<_> = cfg.providers.iter().collect();
    sorted.sort_by_key(|(k, _)| k.as_str());

    for (provider, p) in sorted {
        println!("[providers.{provider}]");
        match &p.api_key {
            Some(k) => println!("  api_key  = {}", mask_key(k)),
            None => println!("  api_key  = (not set)"),
        }
        match &p.base_url {
            Some(u) => println!("  base_url = {u}"),
            None => {
                if let Some(meta) = PROVIDERS.get(provider.as_str()) {
                    println!("  base_url = {} (default)", meta.api_base_url);
                }
            }
        }
        println!();
    }
    Ok(())
}

/// `continuum config providers`
fn cmd_providers() -> CmdResult {
    let cfg = Config::load();

    println!("Available providers  ({} total)\n", PROVIDERS.len());
    println!("{:<14} {:<26} {:<34} KEY", "ID", "NAME", "BASE URL");
    println!("{}", "─".repeat(90));

    let mut sorted: Vec<_> = PROVIDERS.entries().collect();
    sorted.sort_by_key(|(k, _)| *k);

    for (pid, meta) in sorted {
        let key_status = if meta.env_var.is_empty() {
            "no key needed".to_string()
        } else {
            let env_hint = meta.env_var;
            match cfg.api_key(pid, env_hint) {
                Some(_) => "✓ configured".to_string(),
                None => format!("✗ {env_hint}"),
            }
        };

        let base_url = if meta.api_base_url.is_empty() {
            "(local)".to_string()
        } else {
            meta.api_base_url.to_string()
        };

        println!(
            "{:<14} {:<26} {:<34} {}",
            pid,
            meta.name,
            truncate(&base_url, 33),
            key_status
        );

        // Count models for this provider
        let model_count = continuum_models_registry::MODELS
            .entries()
            .filter(|(k, _)| k.starts_with(&format!("{pid}/")))
            .count();
        if model_count > 0 {
            println!("               {} model(s)", model_count);
        }
    }

    println!("\nUsage:");
    println!("  continuum config set <provider>.api_key <key>");
    println!("  continuum config set <provider>.base_url <url>   (optional override)");
    println!("  continuum config list");

    Ok(())
}

fn parse_key(key: &str) -> Result<(&str, &str), String> {
    let dot = key
        .find('.')
        .ok_or_else(|| format!("invalid key '{key}': expected <provider>.<field>"))?;
    Ok((&key[..dot], &key[dot + 1..]))
}

fn mask_key(key: &str) -> String {
    crate::output::mask_key(key)
}

fn truncate(s: &str, max: usize) -> String {
    crate::output::truncate_str(s, max)
}
