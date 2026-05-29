//! First-run onboarding wizard.
//!
//! Guides new users through provider selection and API key setup.

use continuum_models_registry::{MODELS, PROVIDERS};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use super::ReplSession;

/// Run the onboarding wizard for first-time users.
pub async fn run_onboarding(session: &mut ReplSession) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("  Welcome to Continuum!");
    println!("  ═══════════════════════════════════════════════════════════════");
    println!();
    println!("  Let's set you up. This only takes a minute.");
    println!();

    let mut rl = DefaultEditor::new()?;

    // Step 1: Auto-detect from env vars
    let detected = detect_provider_from_env();
    if let Some((provider, key, default_model)) = detected {
        println!("  Detected: {} API key in environment", provider);
        println!();
        println!("  Use {provider} as your provider? [Y/n]: ");

        let answer = match rl.readline("  > ") {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                println!();
                println!("  Using {provider} as default provider.");
                "y".to_string()
            }
            Err(e) => return Err(e.into()),
        };
        let answer = answer.trim().to_lowercase();

        if answer.is_empty() || answer == "y" || answer == "yes" {
            session.provider = provider.clone();
            session.model = default_model.to_string();
            let mut cfg = session.config.clone();
            cfg.set_api_key(&provider, &key);
            if cfg.save().is_err() {
                println!("  ⚠ Warning: could not save configuration to disk");
            }
            println!();
            println!("  ✓ Configured {provider}");
            print_next_steps(session);
            return Ok(());
        }
    }

    // Step 2: Manual provider selection
    println!("  Select a provider:");
    println!();

    let mut providers: Vec<_> = PROVIDERS.entries().collect();
    providers.sort_by_key(|(k, _)| *k);

    // Show top providers first
    let top_providers = [
        "openrouter",
        "anthropic",
        "openai",
        "google",
        "groq",
        "deepseek",
        "mistral",
        "togetherai",
        "fireworks-ai",
        "ollama",
    ];

    let mut displayed = Vec::new();
    println!("  {:<4} {:<24} {:<30} {}", "#", "ID", "NAME", "MODELS");
    println!("  {}", "─".repeat(70));

    // Show top providers
    for (i, pid) in top_providers.iter().enumerate() {
        if let Some((_, meta)) = providers.iter().find(|(k, _)| *k == pid) {
            let model_count = MODELS
                .entries()
                .filter(|(k, _)| k.starts_with(&format!("{pid}/")))
                .count();
            println!(
                "  {:<4} {:<24} {:<30} {}",
                i + 1,
                pid,
                meta.name,
                format!("{model_count} model(s)")
            );
            displayed.push(*pid);
        }
    }

    // Show "more..." option
    println!("  {:<4} {:<24}", "...", "Show all 135 providers");
    println!();

    let choice = match rl.readline("  Select provider (number or name): ") {
        Ok(line) => line,
        Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
            println!();
            println!("  Using OpenRouter as default provider.");
            "openrouter".to_string()
        }
        Err(e) => return Err(e.into()),
    };
    let choice = choice.trim();

    let selected_provider = if let Ok(num) = choice.parse::<usize>() {
        if num > 0 && num <= displayed.len() {
            displayed[num - 1].to_string()
        } else if num == displayed.len() + 1 {
            // Show all providers
            show_all_providers_menu(&mut rl)?
        } else {
            println!("  Invalid selection. Using OpenRouter.");
            "openrouter".to_string()
        }
    } else if PROVIDERS.get(choice).is_some() {
        choice.to_string()
    } else {
        println!("  Unknown provider '{choice}'. Using OpenRouter.");
        "openrouter".to_string()
    };

    session.provider = selected_provider.clone();

    // Step 3: API key input (skip for providers that don't need one)
    let meta = match PROVIDERS.get(&session.provider) {
        Some(m) => m,
        None => {
            println!("  Error: provider '{}' not found", session.provider);
            return Ok(());
        }
    };
    if !meta.env_var.is_empty() && !is_local_provider(&session.provider) {
        println!();
        println!("  Enter your {} API key:", meta.name);
        println!("  (Get one from the provider's website)");
        println!();

        let key = match rl.readline("  API key: ") {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                println!();
                println!("  No API key provided. You can set it later with:");
                println!(
                    "    /config set {provider}.api_key <key>",
                    provider = session.provider
                );
                String::new()
            }
            Err(e) => return Err(e.into()),
        };
        let key = key.trim().to_string();

        if !key.is_empty() {
            let mut cfg = session.config.clone();
            cfg.set_api_key(&session.provider, &key);
            if cfg.save().is_err() {
                println!("  ⚠ Warning: could not save configuration to disk");
            } else {
                println!();
                println!("  ✓ API key saved");
            }
        } else {
            println!();
            println!("  No API key provided. You can set it later with:");
            println!(
                "    /config set {provider}.api_key <key>",
                provider = session.provider
            );
        }
    }

    // Step 4: Select default model
    let prefix = format!("{}/", session.provider);
    let mut models: Vec<_> = MODELS
        .entries()
        .filter(|(k, _)| k.starts_with(&prefix))
        .collect();
    models.sort_by_key(|(k, _)| *k);

    if !models.is_empty() {
        println!();
        println!("  Select default model:");
        println!();
        println!("  {:<4} {:<50} {:<12} {}", "#", "MODEL", "CTX", "PRICE");
        println!("  {}", "─".repeat(80));

        for (i, (id, meta)) in models.iter().enumerate().take(10) {
            let model_id = id.trim_start_matches(&prefix);
            let ctx = if meta.context_window >= 1_000_000 {
                format!("{}M", meta.context_window / 1_000_000)
            } else {
                format!("{}K", meta.context_window / 1_000)
            };
            let price = if meta.input_per_mtok > 0.0 {
                format!("${}/{}", meta.input_per_mtok, meta.output_per_mtok)
            } else {
                "free".to_string()
            };
            println!(
                "  {:<4} {:<50} {:<12} {}",
                i + 1,
                crate::output::truncate_str(model_id, 49),
                ctx,
                price
            );
        }

        if models.len() > 10 {
            println!("  ... and {} more models", models.len() - 10);
        }
        println!();

        let model_choice = match rl.readline("  Select model (number or name): ") {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                println!();
                println!("  Using first available model.");
                models[0].0.trim_start_matches(&prefix).to_string()
            }
            Err(e) => return Err(e.into()),
        };
        let model_choice = model_choice.trim();

        let selected_model = if let Ok(num) = model_choice.parse::<usize>() {
            if num > 0 && num <= models.len() {
                models[num - 1].0.trim_start_matches(&prefix).to_string()
            } else {
                println!("  Invalid selection. Using first model.");
                models[0].0.trim_start_matches(&prefix).to_string()
            }
        } else if model_choice.is_empty() {
            models[0].0.trim_start_matches(&prefix).to_string()
        } else {
            model_choice.to_string()
        };

        session.model = selected_model;
    }

    // Step 5: Save and confirm
    println!();
    println!("  ✓ Setup complete!");
    println!();
    print_next_steps(session);

    Ok(())
}

/// Show all providers in a numbered list and let user pick.
fn show_all_providers_menu(rl: &mut DefaultEditor) -> Result<String, Box<dyn std::error::Error>> {
    let mut providers: Vec<_> = PROVIDERS.entries().collect();
    providers.sort_by_key(|(k, _)| *k);

    println!();
    println!("  {:<4} {:<24} {:<24} {}", "#", "ID", "NAME", "MODELS");
    println!("  {}", "─".repeat(65));

    for (i, (pid, meta)) in providers.iter().enumerate() {
        let model_count = MODELS
            .entries()
            .filter(|(k, _)| k.starts_with(&format!("{pid}/")))
            .count();
        println!(
            "  {:<4} {:<24} {:<24} {}",
            i + 1,
            pid,
            crate::output::truncate_str(meta.name, 23),
            format!("{model_count} model(s)")
        );
    }

    println!();
    let choice = match rl.readline("  Select provider (number): ") {
        Ok(line) => line,
        Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
            println!();
            println!("  Using OpenRouter as default provider.");
            return Ok("openrouter".to_string());
        }
        Err(e) => return Err(e.into()),
    };
    let choice = choice.trim().parse::<usize>().unwrap_or(1);

    if choice > 0 && choice <= providers.len() {
        Ok(providers[choice - 1].0.to_string())
    } else {
        Ok("openrouter".to_string())
    }
}

/// Detect provider from environment variables.
fn detect_provider_from_env() -> Option<(String, String, String)> {
    let checks = [
        (
            "openrouter",
            "OPENROUTER_API_KEY",
            "anthropic/claude-sonnet-4-20250514",
        ),
        ("anthropic", "ANTHROPIC_API_KEY", "claude-sonnet-4-20250514"),
        ("openai", "OPENAI_API_KEY", "gpt-4o-2024-11-20"),
        ("google", "GEMINI_API_KEY", "gemini-2.5-pro-preview-05-06"),
        ("groq", "GROQ_API_KEY", "llama-3.3-70b-versatile"),
        ("deepseek", "DEEPSEEK_API_KEY", "deepseek-chat"),
    ];

    for (provider, env_var, default_model) in &checks {
        if let Ok(key) = std::env::var(env_var) {
            if !key.is_empty() {
                return Some((provider.to_string(), key, default_model.to_string()));
            }
        }
    }

    None
}

/// Check if a provider is local (no API key needed).
fn is_local_provider(provider: &str) -> bool {
    matches!(
        provider,
        "ollama" | "lmstudio" | "privatemode-ai" | "localhost"
    )
}

/// Print next steps after onboarding.
fn print_next_steps(session: &ReplSession) {
    println!();
    println!("  Next steps:");
    println!("    • Type naturally to describe what you want to build");
    println!("    • /help — see all available commands");
    println!("    • /models — switch models");
    println!("    • /effort — adjust reasoning level");
    println!();
    println!(
        "  Ready to go! Provider: {} | Model: {}",
        session.provider, session.model
    );
    println!();
}
