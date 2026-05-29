//! Slash command handlers for the REPL.

use continuum_models_registry::{MODELS, PROVIDERS};

use super::{EffortLevel, ReplSession};

/// Handle a slash command. Returns true if the REPL should exit.
pub async fn handle_slash_command(session: &mut ReplSession, input: &str) -> bool {
    let parts: Vec<&str> = input[1..].splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

    match cmd.as_str() {
        "help" | "h" | "available" | "commands" | "command" | "avaliable" | "commnads"
        | "lgaye" => cmd_help(),
        "models" | "m" => cmd_models(session),
        "model" => cmd_model_switch(session, args),
        "providers" | "provider" | "p" => cmd_providers(session),
        "effort" | "e" => cmd_effort(session, args),
        "cost" | "c" => cmd_cost(session),
        "compact" => cmd_compact(session),
        "clear" | "cls" => cmd_clear(),
        "config" | "cfg" => cmd_config(session, args),
        "doctor" | "doc" => cmd_doctor(),
        "init" => cmd_init(),
        "index" => cmd_index(),
        "tools" | "t" => cmd_tools(),
        "memory" | "mem" => cmd_memory(session, args),
        "sessions" | "s" => cmd_sessions(),
        "review" | "r" => cmd_review(),
        "harden" => cmd_harden(session, args),
        "diff" | "d" => cmd_diff(),
        "git" => cmd_git(args),
        "share" => cmd_share(),
        "update" | "up" => cmd_update(),
        "theme" => cmd_theme(args),
        "export" => cmd_export(),
        "version" | "v" => cmd_version(),
        "exit" | "quit" | "q" => {
            return true;
        }
        "plugins" => cmd_plugins(),
        "redact" => cmd_redact(args),
        "init-custom" => cmd_init_custom(),
        unknown => {
            // Check for custom commands in .continuum/commands/
            let project_dir = std::env::current_dir().unwrap_or_default();
            if let Some(cmd) = crate::custom_commands::find_command(&project_dir, unknown) {
                println!();
                println!("  Running custom command: /{}", cmd.name);
                println!("  From: {}", cmd.path.display());
                println!();
                // Execute the custom command as a goal
                execute_goal(session, &cmd.content).await;
            } else {
                println!("  Unknown command: /{unknown}. Type /help or /available for available commands.");
            }
        }
    }
    false
}

/// Execute a natural language goal.
pub async fn execute_goal(session: &mut ReplSession, goal: &str) {
    session.message_count += 1;

    // Redact secrets before processing
    let (safe_goal, redaction_count) = crate::redact::redact(goal);
    if redaction_count > 0 {
        println!("  ⚠ Redacted {redaction_count} secret(s) from input");
    }

    println!();
    println!("  [analyzing repository...]");
    println!();

    // For now, show what would happen
    println!("  Goal: {safe_goal}");
    println!("  Model: {} ({})", session.model, session.provider);
    println!("  Effort: {}", session.effort.as_str());
    println!();

    // TODO: Wire up to the actual planning engine
    // This is where we'd call continuum_planner::PlanningEngine
    println!("  [planning...]");
    println!("  [executing...]");
    println!();
    println!("  Note: Full execution pipeline will be wired in the next iteration.");
    println!("  For now, use `continuum execute --goal \"{safe_goal}\"` for full execution.");
    println!();
}

// ── Command implementations ──────────────────────────────────────

fn cmd_help() {
    println!();
    println!("  Continuum Slash Commands");
    println!("  ---------------------------------------------------------------");
    println!();
    println!("  Session & Context");
    println!("    /models, /m        List and select AI models");
    println!("    /providers, /p     List providers");
    println!("    /effort, /e        Set reasoning level (low/medium/high/max)");
    println!("    /cost              Show token usage and cost");
    println!("    /compact           Compress conversation context");
    println!("    /clear, /cls       Clear screen");
    println!();
    println!("  Configuration");
    println!("    /config, /cfg      View/change settings");
    println!("    /doctor, /doc      Run environment diagnostics");
    println!("    /init              Scaffold engineering docs");
    println!("    /index             Build repo symbol index");
    println!();
    println!("  Tools & Analysis");
    println!("    /tools, /t         List available tools");
    println!("    /review, /r        Code review current changes");
    println!("    /harden            Security audit");
    println!("    /diff, /d          Show uncommitted changes");
    println!("    /redact            Test secret redaction");
    println!();
    println!("  Git & Version Control");
    println!("    /git               Git operations (status, commit, push)");
    println!();
    println!("  Memory & Sessions");
    println!("    /memory, /mem      Inspect memory engine");
    println!("    /sessions, /s      List past sessions");
    println!("    /share             Generate shareable link");
    println!("    /export            Export session as markdown");
    println!();
    println!("  System");
    println!("    /update, /up       Update continuum");
    println!("    /theme             Change color theme");
    println!("    /plugins           Manage plugins");
    println!("    /version, /v       Show version");
    println!("    /help, /h          Show this help");
    println!("    /available         Show this help");
    println!("    /commands          Show this help");
    println!("    /exit, /quit, /q   Exit continuum");
    println!();
    println!("  Custom Commands");
    println!("    /init-custom       Create sample .continuum/commands/ directory");
    println!("    Any unrecognised /command checks .continuum/commands/<name>.md");
    println!();
    println!("  Keyboard Shortcuts");
    println!("    Ctrl+C             Cancel current operation");
    println!("    Ctrl+D             Exit continuum");
    println!("    Ctrl+R             Search command history");
    println!("    Tab                Autocomplete command");
    println!("    Up/Down            Navigate history");
    println!();
    println!("  Type /available or /commands to show this list again.");
    println!("  Just type naturally to describe what you want to build!");
    println!();
}

fn cmd_models(session: &mut ReplSession) {
    println!();
    println!("  Current model: {} ({})", session.model, session.provider);
    println!();

    // Get models for current provider
    let prefix = format!("{}/", session.provider);
    let mut models: Vec<_> = MODELS
        .entries()
        .filter(|(k, _)| k.starts_with(&prefix))
        .collect();
    models.sort_by_key(|(k, _)| *k);

    if models.is_empty() {
        println!("  No models found for provider '{}'.", session.provider);
        println!("  Use /providers to see available providers.");
    } else {
        println!("  Available models for {}:", session.provider);
        println!("  {:<4} {:<50} {:<12} PRICE", "#", "MODEL", "CTX");
        println!("  {}", "─".repeat(80));

        for (i, (id, meta)) in models.iter().enumerate() {
            let model_id = id.trim_start_matches(&prefix);
            let ctx = format_window(meta.context_window);
            let price = if meta.input_per_mtok > 0.0 {
                format!("${}/{}", meta.input_per_mtok, meta.output_per_mtok)
            } else {
                "free".to_string()
            };
            let marker = if *id == &session.model || model_id == session.model {
                " *"
            } else {
                ""
            };
            println!(
                "  {:<4} {:<50} {:<12} {}{}",
                i + 1,
                crate::output::truncate_str(model_id, 49),
                ctx,
                price,
                marker
            );
        }
        println!();
        println!("  * = current model");
        println!("  Use /model <number> or /model <provider/model> to switch.");
    }
    println!();
}

/// Switch the current model.
fn cmd_model_switch(session: &mut ReplSession, args: &str) {
    if args.is_empty() {
        println!();
        println!("  Current model: {} ({})", session.model, session.provider);
        println!();
        println!("  Usage:");
        println!("    /model <number>          Select by number from /models list");
        println!("    /model <provider/model>  Switch to a specific model");
        println!("    /model                   Show current model");
        println!();
        return;
    }

    // Get models for current provider
    let prefix = format!("{}/", session.provider);
    let mut models: Vec<_> = MODELS
        .entries()
        .filter(|(k, _)| k.starts_with(&prefix))
        .collect();
    models.sort_by_key(|(k, _)| *k);

    if let Ok(num) = args.parse::<usize>() {
        // Select by number
        if num > 0 && num <= models.len() {
            let model_id = models[num - 1].0.trim_start_matches(&prefix);
            session.model = model_id.to_string();
            println!("  ✓ Switched to: {model_id}");
        } else {
            println!("  Invalid selection. Use /models to see available options.");
        }
    } else if args.contains('/') {
        // Full provider/model path
        if MODELS.get(args).is_some() {
            let parts: Vec<&str> = args.splitn(2, '/').collect();
            if parts.len() == 2 {
                session.provider = parts[0].to_string();
                session.model = parts[1].to_string();
                println!("  ✓ Switched to: {args}");
            }
        } else {
            println!("  Model not found: {args}");
        }
    } else {
        // Model name only (for current provider)
        let full_id = format!("{}{}", prefix, args);
        if MODELS.get(full_id.as_str()).is_some() {
            session.model = args.to_string();
            println!("  ✓ Switched to: {args}");
        } else {
            println!("  Model not found: {args}");
            println!("  Use /models to see available options.");
        }
    }
}

fn cmd_providers(session: &ReplSession) {
    println!();
    println!("  Available providers  ({} total)", PROVIDERS.len());
    println!();
    println!("  {:<18} {:<24} {:<30} STATUS", "ID", "NAME", "MODELS");
    println!("  {}", "─".repeat(80));

    let mut sorted: Vec<_> = PROVIDERS.entries().collect();
    sorted.sort_by_key(|(k, _)| *k);

    for (pid, meta) in &sorted {
        let model_count = MODELS
            .entries()
            .filter(|(k, _)| k.starts_with(&format!("{pid}/")))
            .count();

        let status = if meta.env_var.is_empty() {
            "no key needed".to_string()
        } else {
            let env_hint = meta.env_var;
            match session.config.api_key(pid, env_hint) {
                Some(_) => "configured".to_string(),
                None => "not set".to_string(),
            }
        };

        let marker = if *pid == &session.provider { " *" } else { "" };

        println!(
            "  {:<18} {:<24} {:<30} {}{}",
            pid,
            crate::output::truncate_str(meta.name, 23),
            format!("{model_count} model(s)"),
            status,
            marker
        );
    }

    println!();
    println!("  * = current provider");
    println!();
}

fn cmd_effort(session: &mut ReplSession, args: &str) {
    if args.is_empty() {
        println!();
        println!("  Current effort level: {}", session.effort.as_str());
        println!();
        println!("  Available levels:");
        println!("    low     — Fast, cheap, less thorough");
        println!("    medium  — Balanced (default)");
        println!("    high    — Thorough reasoning");
        println!("    max     — Maximum reasoning (requires Opus)");
        println!();
        println!("  Usage: /effort <level>");
        return;
    }

    if let Some(level) = EffortLevel::from_str(args) {
        session.effort = level;
        println!("  Effort level set to: {}", level.as_str());
    } else {
        println!("  Unknown effort level: '{args}'. Use low, medium, high, or max.");
    }
}

fn cmd_cost(session: &ReplSession) {
    println!();
    println!("  Session Statistics");
    println!("  ═══════════════════════════════════════════════════════════════");
    println!("  Messages:  {}", session.message_count);
    println!(
        "  Tokens:    {} in / {} out",
        session.total_input_tokens, session.total_output_tokens
    );
    println!("  Cost:      ${:.4}", session.total_cost_usd);
    println!("  Model:     {}", session.model);
    println!("  Provider:  {}", session.provider);
    println!("  Effort:    {}", session.effort.as_str());
    println!();
}

fn cmd_compact(_session: &mut ReplSession) {
    println!();
    println!("  Compacting conversation context...");
    println!("  (Context compression will be implemented with the memory engine)");
    println!();
}

fn cmd_clear() {
    // Clear screen using ANSI escape codes
    print!("\x1B[2J\x1B[1;1H");
    println!("  Continuum — screen cleared");
    println!();
}

fn cmd_config(session: &mut ReplSession, args: &str) {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    let subcmd = parts.first().unwrap_or(&"");

    match *subcmd {
        "" | "list" => {
            println!();
            println!("  Current Configuration");
            println!("  ═══════════════════════════════════════════════════════════════");
            println!("  Provider:  {}", session.provider);
            println!("  Model:     {}", session.model);
            println!("  Effort:    {}", session.effort.as_str());
            println!();

            let cfg_path = continuum_config::config_path();
            if let Some(path) = cfg_path {
                println!("  Config:    {}", path.display());
                if !path.exists() {
                    println!("             (file does not exist yet)");
                }
            }
            println!();
        }
        "set" => {
            if let Some(kv) = parts.get(1) {
                let kv_parts: Vec<&str> = kv.splitn(2, ' ').collect();
                if kv_parts.len() == 2 {
                    let key = kv_parts[0];
                    let value = kv_parts[1];
                    let mut cfg = session.config.clone();
                    if key.ends_with(".api_key") {
                        let provider = key.trim_end_matches(".api_key");
                        cfg.set_api_key(provider, value);
                        if cfg.save().is_err() {
                            println!("  ⚠ Warning: could not save configuration to disk");
                        } else {
                            println!("  ✓ Set {provider}.api_key");
                        }
                    } else if key.ends_with(".base_url") {
                        let provider = key.trim_end_matches(".base_url");
                        cfg.set_base_url(provider, value);
                        if cfg.save().is_err() {
                            println!("  ⚠ Warning: could not save configuration to disk");
                        } else {
                            println!("  ✓ Set {provider}.base_url");
                        }
                    } else {
                        println!("  Unknown config key: {key}");
                    }
                } else {
                    println!("  Usage: /config set <provider>.api_key <value>");
                }
            } else {
                println!("  Usage: /config set <provider>.api_key <value>");
            }
        }
        "providers" => {
            cmd_providers(session);
        }
        _ => {
            println!("  Usage: /config [list|set|providers]");
        }
    }
}

fn cmd_doctor() {
    println!();
    println!("  Running diagnostics...");
    println!();
    // Delegate to the existing doctor command
    println!("  Use `continuum doctor` from the terminal for full diagnostics.");
    println!();
}

fn cmd_init() {
    println!();
    println!("  Initializing project...");
    println!();
    println!("  Use `continuum init` from the terminal to scaffold engineering docs.");
    println!();
}

fn cmd_index() {
    println!();
    println!("  Building repository symbol index...");
    println!();
    println!("  (Full index building will be wired with the repo intelligence engine)");
    println!();
}

fn cmd_tools() {
    println!();
    println!("  Available Tools");
    println!("  ═══════════════════════════════════════════════════════════════");
    println!("  Planning        — Goal decomposition, DAG construction");
    println!("  Coding          — Code generation and modification");
    println!("  Testing         — Test generation and execution");
    println!("  Security        — Semgrep, Trivy, Gitleaks scans");
    println!("  Review          — Code review and quality checks");
    println!("  Memory          — Context compression and recall");
    println!("  Recovery        — Crash recovery and checkpointing");
    println!("  Architecture    — Design validation");
    println!();
    println!("  Validation Pipeline (10 stages):");
    println!("    Compile → Lint → TypeCheck → Unit → Integration →");
    println!("    E2E → SecurityScan → Startup → Performance → Regression");
    println!();
}

fn cmd_memory(_session: &mut ReplSession, args: &str) {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    let action = parts.first().unwrap_or(&"list");

    match *action {
        "list" | "ls" => {
            println!();
            println!("  Memory Engine");
            println!("  ═══════════════════════════════════════════════════════════════");
            println!("  Tier    Status");
            println!("  Hot     Active (current session context)");
            println!("  Warm    Compressed summaries of past sessions");
            println!("  Cold    Vector embeddings for semantic search");
            println!();
            println!("  Use /memory compress to compress hot → warm.");
            println!("  Use /memory purge to clear old warm items.");
        }
        "compress" => {
            println!();
            println!("  Compressing memory (hot → warm)...");
            println!("  (Memory compression will be wired with the memory engine)");
        }
        "purge" => {
            println!();
            println!("  Purging old warm memory items...");
            println!("  (Memory purge will be wired with the memory engine)");
        }
        _ => {
            println!("  Usage: /memory [list|compress|purge]");
        }
    }
    println!();
}

fn cmd_sessions() {
    println!();
    println!("  Recent Sessions");
    println!("  ═══════════════════════════════════════════════════════════════");
    println!("  (Session listing will be wired with the storage engine)");
    println!();
    println!("  Use `continuum -c` to continue the last session.");
    println!("  Use `continuum -r` to resume a specific session.");
    println!();
}

fn cmd_review() {
    println!();
    println!("  Running code review...");
    println!();
    println!("  (Full code review will be wired with the review agent)");
    println!();
}

fn cmd_harden(_session: &mut ReplSession, args: &str) {
    let mode = if args.is_empty() { "audit" } else { args };
    println!();
    println!("  Running security hardening (mode: {mode})...");
    println!();
    println!("  Use `continuum harden --mode {mode}` from the terminal.");
    println!();
}

fn cmd_diff() {
    println!();
    println!("  Showing uncommitted changes...");
    println!();
    // Try to run git diff
    match std::process::Command::new("git").arg("diff").output() {
        Ok(output) => {
            let diff = String::from_utf8_lossy(&output.stdout);
            if diff.is_empty() {
                println!("  No uncommitted changes.");
            } else {
                println!("{}", diff);
            }
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.is_empty() {
                    eprintln!("  Error: {}", stderr.trim());
                }
            }
        }
        Err(e) => {
            println!("  Error running git diff: {e}");
        }
    }
    println!();
}

fn cmd_git(args: &str) {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    let subcmd = parts.first().unwrap_or(&"status");

    println!();
    match *subcmd {
        "status" | "s" => {
            println!("  Running git status...");
            println!();
            match std::process::Command::new("git").arg("status").output() {
                Ok(output) => {
                    let status = String::from_utf8_lossy(&output.stdout);
                    println!("{}", status);
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        if !stderr.is_empty() {
                            eprintln!("  Error: {}", stderr.trim());
                        }
                    }
                }
                Err(e) => println!("  Error: {e}"),
            }
        }
        "log" | "l" => {
            println!("  Running git log...");
            println!();
            match std::process::Command::new("git")
                .args(["log", "--oneline", "-10"])
                .output()
            {
                Ok(output) => {
                    let log = String::from_utf8_lossy(&output.stdout);
                    println!("{}", log);
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        if !stderr.is_empty() {
                            eprintln!("  Error: {}", stderr.trim());
                        }
                    }
                }
                Err(e) => println!("  Error: {e}"),
            }
        }
        "diff" | "d" => {
            cmd_diff();
        }
        _ => {
            println!("  Git subcommands: status (s), log (l), diff (d)");
            println!("  Usage: /git <subcommand>");
        }
    }
}

fn cmd_share() {
    println!();
    println!("  Generating shareable session link...");
    println!();
    println!("  (Session sharing will be implemented with the storage engine)");
    println!();
}

fn cmd_update() {
    println!();
    println!("  Checking for updates...");
    println!();
    println!("  To update Continuum, run:");
    println!("    cargo install continuum-cli --force");
    println!();
}

fn cmd_theme(args: &str) {
    println!();
    if args.is_empty() {
        println!("  Available themes: default, dark, light");
        println!("  Usage: /theme <name>");
    } else {
        println!("  Theme set to: {args}");
        println!("  (Theme support will be implemented with the TUI)");
    }
    println!();
}

fn cmd_export() {
    println!();
    println!("  Exporting session...");
    println!();
    println!("  (Session export will be implemented with the storage engine)");
    println!();
}

fn cmd_version() {
    println!();
    println!("  Continuum v{}", env!("CARGO_PKG_VERSION"));
    println!("  Autonomous production engineering runtime");
    println!();
    println!("  Rust: {}", env!("CARGO_PKG_RUST_VERSION"));
    println!("  License: Apache-2.0 OR MIT");
    println!();
}

fn cmd_plugins() {
    println!();
    println!("  Plugin System");
    println!("  ═══════════════════════════════════════════════════════════════");
    println!("  (Plugin system will be implemented in a future phase)");
    println!();
    println!("  Custom commands can be placed in .continuum/commands/");
    println!();
}

fn cmd_redact(args: &str) {
    use crate::redact;

    if args.is_empty() {
        println!();
        println!("  Secret Redaction");
        println!("  ═══════════════════════════════════════════════════════════════");
        println!("  Redacts API keys, JWTs, PEM keys, database URIs, etc.");
        println!();
        println!("  Usage: /redact <text>");
        println!("  Example: /redact my key is sk-ant-api03-abc123...");
        println!();
        println!("  Detected secret types:");
        println!("    • API keys (Anthropic, OpenAI, OpenRouter, Groq, Google)");
        println!("    • GitHub tokens (ghp_, github_pat_)");
        println!("    • AWS keys (AKIA..., secret access keys)");
        println!("    • JWTs (eyJ...)");
        println!("    • PEM private keys");
        println!("    • Database URIs (postgres://, mysql://, etc.)");
        println!("    • Generic secrets and passwords");
        println!();
        return;
    }

    let (safe_text, count) = redact::redact(args);
    let detected = redact::detect_secrets(args);

    println!();
    if count == 0 {
        println!("  ✓ No secrets detected");
    } else {
        println!("  ⚠ Redacted {count} secret(s):");
        for secret_type in &detected {
            println!("    • {secret_type}");
        }
        println!();
        println!("  Original:  {args}");
        println!("  Redacted:  {safe_text}");
    }
    println!();
}

fn cmd_init_custom() {
    let project_dir = std::env::current_dir().unwrap_or_default();
    match crate::custom_commands::create_sample_command(&project_dir) {
        Ok(()) => {
            println!();
            println!("  ✓ Created sample custom command");
            println!();
            println!("  Custom commands are stored in .continuum/commands/");
            println!("  Add .md files to create new commands:");
            println!("    .continuum/commands/review.md  →  /review");
            println!("    .continuum/commands/test.md    →  /test");
            println!("    .continuum/commands/deploy.md  →  /deploy");
            println!();
            println!("  Each file's content is used as the prompt template.");
            println!();
        }
        Err(e) => {
            println!("  Error creating sample command: {e}");
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────

fn format_window(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{}M", tokens / 1_000_000)
    } else if tokens >= 1_000 {
        format!("{}K", tokens / 1_000)
    } else {
        format!("{}", tokens)
    }
}
