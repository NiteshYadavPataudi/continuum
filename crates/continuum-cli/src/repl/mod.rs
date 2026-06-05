//! Interactive REPL for Continuum.
//!
//! Provides a Claude Code-like experience: type natural language goals,
//! use slash commands, and get real-time feedback.

pub mod commands;
pub mod onboarding;

use continuum_config::Config;
use continuum_models_registry::PROVIDERS;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

/// Effort level for model reasoning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffortLevel {
    Low,
    Medium,
    High,
    Max,
}

impl EffortLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Max => "max",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "low" | "l" => Some(Self::Low),
            "medium" | "med" | "m" => Some(Self::Medium),
            "high" | "h" => Some(Self::High),
            "max" => Some(Self::Max),
            _ => None,
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for EffortLevel {
    fn default() -> Self {
        Self::Medium
    }
}

/// Session state for the REPL.
#[derive(Debug, Clone)]
pub struct ReplSession {
    pub config: Config,
    pub effort: EffortLevel,
    pub model: String,
    pub provider: String,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    pub message_count: usize,
}

impl ReplSession {
    pub fn new() -> Self {
        let config = Config::load();
        let (provider, model) = detect_default_provider(&config);

        Self {
            config,
            effort: EffortLevel::default(),
            model,
            provider,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost_usd: 0.0,
            message_count: 0,
        }
    }

    pub fn is_configured(&self) -> bool {
        // Check if the current provider is configured
        let env_hint = PROVIDERS
            .get(self.provider.as_str())
            .map(|p| p.env_var)
            .unwrap_or("");
        if self.config.api_key(&self.provider, env_hint).is_some() {
            return true;
        }

        // Also check if ANY provider is configured (user may have switched)
        for (pid, meta) in PROVIDERS.entries() {
            if !meta.env_var.is_empty() && self.config.api_key(pid, meta.env_var).is_some() {
                return true;
            }
        }

        false
    }
}

/// Detect the default provider from env vars or config.
pub fn detect_default_provider(config: &Config) -> (String, String) {
    // Check common env vars in order of preference
    let env_checks = [
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

    for (provider, env_var, default_model) in &env_checks {
        if std::env::var(env_var).is_ok() {
            return (provider.to_string(), default_model.to_string());
        }
        let continuum_var = format!(
            "CONTINUUM_{}_API_KEY",
            provider.to_uppercase().replace('-', "_")
        );
        if std::env::var(&continuum_var).is_ok() {
            return (provider.to_string(), default_model.to_string());
        }
    }

    // Check config file
    for (provider, _, default_model) in &env_checks {
        let env_hint = PROVIDERS.get(*provider).map(|p| p.env_var).unwrap_or("");
        if config.api_key(provider, env_hint).is_some() {
            return (provider.to_string(), default_model.to_string());
        }
    }

    // Default to OpenRouter if configured, otherwise Anthropic
    (
        "openrouter".to_string(),
        "anthropic/claude-sonnet-4-20250514".to_string(),
    )
}

/// Print the Continuum banner.
fn print_banner() {
    println!();
    println!(
        "  Continuum v{} - Autonomous production engineering",
        env!("CARGO_PKG_VERSION")
    );
    println!("  Type /help or /available for commands, or just describe what you want to build.");
    println!();
}

/// Print session summary on exit.
fn print_session_summary(session: &ReplSession) {
    if session.message_count > 0 {
        crate::output::section("Session Summary");
        crate::output::kv("Messages", &session.message_count.to_string());
        crate::output::kv(
            "Tokens",
            &format!(
                "{} in / {} out",
                session.total_input_tokens, session.total_output_tokens
            ),
        );
        if session.total_cost_usd > 0.0 {
            crate::output::kv("Cost", &format!("${:.4}", session.total_cost_usd));
        }
        crate::output::kv("Model", &session.model);
        crate::output::kv("Provider", &session.provider);
        println!();
    }
}

impl Default for ReplSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the interactive REPL.
pub async fn run_repl(initial_prompt: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut session = ReplSession::new();

    // Check if first run
    if !session.is_configured() {
        onboarding::run_onboarding(&mut session).await?;
    }

    print_banner();
    println!(
        "  Provider: {} | Model: {} | Effort: {}",
        session.provider,
        session.model,
        session.effort.as_str()
    );
    println!();

    let mut rl = DefaultEditor::new()?;

    // Load history if it exists
    let history_path =
        directories::UserDirs::new().map(|u| u.home_dir().join(".continuum").join("history.txt"));
    if let Some(ref path) = history_path {
        let _ = rl.load_history(path);
    }

    // Execute initial prompt if provided
    if let Some(prompt) = initial_prompt {
        println!("  > {}", prompt);
        println!();
        commands::execute_goal(&mut session, &prompt).await;
    }

    // Main REPL loop
    loop {
        let prompt = "continuum > ".to_string();
        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }

                // Add to history
                let _ = rl.add_history_entry(&line);

                // Handle slash commands
                if line.starts_with('/') {
                    let should_exit = commands::handle_slash_command(&mut session, &line).await;
                    if should_exit {
                        break;
                    }
                } else {
                    // Execute as a goal
                    commands::execute_goal(&mut session, &line).await;
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("  (interrupted)");
                continue;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(e) => {
                eprintln!("  Error: {e}");
                break;
            }
        }
    }

    // Save history
    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }

    print_session_summary(&session);
    println!("  Session saved. Resume with `continuum -c`");
    Ok(())
}
