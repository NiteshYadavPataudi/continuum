use continuum_config::Config;
use continuum_models_registry::{MODELS, PROVIDERS};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use super::CmdResult;

pub async fn run() -> CmdResult {
    let mut rl = DefaultEditor::new()?;

    // Build sorted provider list
    let mut providers: Vec<_> = PROVIDERS.entries().collect();
    providers.sort_by_key(|(k, _)| *k);

    // Step 1: Provider selection
    println!();
    println!("  Select a provider:");
    println!();
    println!("  {:<4} {:<24} {:<24} MODELS", "#", "ID", "NAME");
    println!("  {}", "─".repeat(65));

    for (i, (pid, meta)) in providers.iter().enumerate() {
        let model_count = MODELS
            .entries()
            .filter(|(k, _)| k.starts_with(&format!("{pid}/")))
            .count();
        println!(
            "  {:<4} {:<24} {:<24} {model_count} model(s)",
            i + 1,
            pid,
            crate::output::truncate_str(meta.name, 23),
        );
    }

    println!();

    let choice = match rl.readline("  Enter number or name: ") {
        Ok(line) => line,
        Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    let choice = choice.trim().to_lowercase();

    let selected = if let Ok(num) = choice.parse::<usize>() {
        if num > 0 && num <= providers.len() {
            providers[num - 1].0.to_string()
        } else {
            println!("  Invalid number.");
            return Ok(());
        }
    } else if PROVIDERS.get(&choice).is_some() {
        choice
    } else {
        println!("  Unknown provider: {choice}");
        return Ok(());
    };

    let meta = PROVIDERS
        .get(&selected)
        .expect("provider must exist after selection");

    println!();
    println!("  Provider: {} ({})", meta.name, selected);

    // Step 2: API key input
    let is_local = matches!(
        selected.as_str(),
        "ollama" | "lmstudio" | "privatemode-ai" | "localhost"
    );

    if meta.env_var.is_empty() || is_local {
        println!("  No API key needed for {}. Done!", meta.name);
        return Ok(());
    }

    println!();
    println!(
        "  Enter your {} API key (or press Ctrl+C to cancel):",
        meta.name
    );
    println!("  (env var: {})", meta.env_var);
    println!();

    let key = match rl.readline("  API key: ") {
        Ok(line) => line,
        Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
            println!();
            println!("  Cancelled.");
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };
    let key = key.trim().to_string();

    if key.is_empty() {
        println!("  No key provided. Cancelled.");
        return Ok(());
    }

    // Save
    let mut cfg = Config::load();
    cfg.set_api_key(&selected, &key);

    if cfg.save().is_ok() {
        let masked = crate::output::mask_key(&key);
        println!();
        println!("  ✓ Saved {selected}.api_key = {masked}");
    } else {
        println!("  ⚠ Warning: could not save configuration to disk");
    }

    Ok(())
}
