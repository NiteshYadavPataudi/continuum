use super::{CmdResult, DoctorArgs};
use continuum_core::caps::Cap;
use continuum_core::model::ModelProvider;
use std::process::Command;
use std::time::Duration;

struct EnvCheck {
    name: &'static str,
    required: bool,
    hint: &'static str,
}

const CHECKS: &[EnvCheck] = &[
    EnvCheck {
        name: "rustc",
        required: true,
        hint: "Install Rust from https://rustup.rs",
    },
    EnvCheck {
        name: "cargo",
        required: true,
        hint: "Install Rust from https://rustup.rs",
    },
    EnvCheck {
        name: "git",
        required: true,
        hint: "Install git from https://git-scm.com",
    },
    EnvCheck {
        name: "docker",
        required: true,
        hint: "Install Docker from https://docker.com",
    },
    EnvCheck {
        name: "semgrep",
        required: false,
        hint: "Install Semgrep from https://semgrep.dev/docs/getting-started/",
    },
    EnvCheck {
        name: "trivy",
        required: false,
        hint: "Install Trivy from https://trivy.dev/latest/getting-started/installation/",
    },
    EnvCheck {
        name: "gitleaks",
        required: false,
        hint: "Install Gitleaks from https://gitleaks.io/",
    },
    EnvCheck {
        name: "node",
        required: false,
        hint: "Install Node.js from https://nodejs.org",
    },
    EnvCheck {
        name: "npm",
        required: false,
        hint: "Install Node.js from https://nodejs.org",
    },
    EnvCheck {
        name: "python3",
        required: false,
        hint: "Install Python 3 from https://python.org",
    },
    EnvCheck {
        name: "rustfmt",
        required: false,
        hint: "Run `rustup component add rustfmt`",
    },
    EnvCheck {
        name: "clippy-driver",
        required: false,
        hint: "Run `rustup component add clippy`",
    },
];

fn check_tool(name: &str) -> (bool, String) {
    match Command::new(name).arg("--version").output() {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            (true, ver)
        }
        _ => (false, String::new()),
    }
}

fn rustup_component_installed(name: &str) -> bool {
    Command::new("rustup")
        .args(["component", "list", "--installed"])
        .output()
        .ok()
        .is_some_and(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|l| l.trim() == name)
        })
}

/// Provider config for testing in doctor.
struct ProviderTest {
    name: &'static str,
    env_var: &'static str,
    model_id: &'static str,
}

const PROVIDER_TESTS: &[ProviderTest] = &[
    ProviderTest {
        name: "Anthropic",
        env_var: "ANTHROPIC_API_KEY",
        model_id: "claude-sonnet-4-20250514",
    },
    ProviderTest {
        name: "OpenAI",
        env_var: "OPENAI_API_KEY",
        model_id: "gpt-4o",
    },
    ProviderTest {
        name: "Gemini",
        env_var: "GOOGLE_API_KEY",
        model_id: "gemini-2.0-flash",
    },
    ProviderTest {
        name: "DeepSeek",
        env_var: "DEEPSEEK_API_KEY",
        model_id: "deepseek-chat",
    },
    ProviderTest {
        name: "Groq",
        env_var: "GROQ_API_KEY",
        model_id: "llama-3.3-70b-versatile",
    },
    ProviderTest {
        name: "Mistral",
        env_var: "MISTRAL_API_KEY",
        model_id: "mistral-large-latest",
    },
    ProviderTest {
        name: "Cohere",
        env_var: "COHERE_API_KEY",
        model_id: "command-r-plus",
    },
    ProviderTest {
        name: "Together",
        env_var: "TOGETHER_API_KEY",
        model_id: "mistralai/Mixtral-8x7B-Instruct-v0.1",
    },
    ProviderTest {
        name: "Fireworks",
        env_var: "FIREWORKS_API_KEY",
        model_id: "accounts/fireworks/models/mixtral-8x7b-instruct",
    },
];

pub async fn run(args: DoctorArgs) -> CmdResult {
    println!("Continuum environment diagnostics\n");

    let mut missing_required = false;
    for check in CHECKS {
        let (found, version) = check_tool(check.name);
        let marker = if found { "  \u{2713}" } else { "  \u{2717}" };
        let label = if check.required {
            "required"
        } else {
            "optional"
        };
        if found {
            println!("  {marker} {:<20} {} ({label})", check.name, version);
        } else {
            println!("  {marker} {:<20} not found ({label})", check.name);
            println!("         hint: {}", check.hint);
            if check.required {
                missing_required = true;
            }
        }
    }
    if !check_tool("rustfmt").0 {
        let as_comp = rustup_component_installed("rustfmt");
        println!(
            "  {} {:<20} {}",
            if as_comp { "\u{2713}" } else { "\u{2717}" },
            "rustfmt (component)",
            if as_comp {
                "installed"
            } else {
                "not installed"
            }
        );
    }
    if !check_tool("clippy-driver").0 {
        let as_comp = rustup_component_installed("clippy");
        println!(
            "  {} {:<20} {}",
            if as_comp { "\u{2713}" } else { "\u{2717}" },
            "clippy (component)",
            if as_comp {
                "installed"
            } else {
                "not installed"
            }
        );
    }
    println!();

    // Print auto-fix guidance if --fix is set
    if args.fix {
        println!("--- Auto-fix mode ---\n");
        if !check_tool("docker").0 {
            println!("  Attempting to verify Docker installation...");
            println!("  Docker is required. Please install it from: https://docker.com");
            println!("  After installing, restart your terminal and run `continuum doctor` again.");
            println!();
        }
        if !check_tool("semgrep").0 {
            println!("  Tip: Install Semgrep with: pip install semgrep");
        }
        if !check_tool("trivy").0 {
            println!(
                "  Tip: Install Trivy from: https://trivy.dev/latest/getting-started/installation/"
            );
        }
        if !check_tool("gitleaks").0 {
            println!("  Tip: Install Gitleaks from: https://gitleaks.io/");
        }
        println!();
    }

    // Phase 2 substrate checks
    let project = args
        .project
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Step 1: Storage + migrations
    let state_dir = project.join(".continuum");
    let db_path = state_dir.join("state.db");

    let p = crate::output::Progress::new("Creating .continuum directory");
    match tokio::fs::create_dir_all(&state_dir).await {
        Ok(_) => p.done("created"),
        Err(e) => {
            p.failed(&format!("{e}"));
            println!("\n  Substrate checks incomplete. Fix storage errors first.");
            return Ok(());
        }
    }

    let p2 = crate::output::Progress::new("Opening storage + migrations");
    let storage = match continuum_storage::Storage::open(&db_path).await {
        Ok(s) => {
            p2.done("ok");
            s
        }
        Err(e) => {
            p2.failed(&format!("{e}"));
            println!("\n  Substrate checks incomplete. Fix storage errors first.");
            return Ok(());
        }
    };

    // Step 2: Test configured providers (not just Anthropic)
    let config = continuum_config::Config::load();
    let mut tested_any = false;
    for pt in PROVIDER_TESTS {
        let key = std::env::var(pt.env_var)
            .ok()
            .or_else(|| config.api_key(pt.name.to_lowercase().as_str(), pt.env_var));
        if let Some(key_val) = key {
            tested_any = true;
            let p3 = crate::output::Progress::new(&format!("Testing {} API", pt.name));
            let provider =
                continuum_models::load_from_config(&config, pt.name.to_lowercase().as_str());
            if let Some(prov) = provider {
                match tokio::time::timeout(
                    Duration::from_secs(30),
                    prov.complete(
                        &Cap::grant(),
                        continuum_core::model::CompletionRequest::new(
                            continuum_core::ids::ModelId::new(pt.model_id),
                            vec![continuum_core::model::Message::new(
                                "user",
                                "Reply with: OK",
                            )],
                        )
                        .with_temperature(0.0)
                        .with_max_tokens(10),
                        continuum_core::CancellationToken::new(),
                    ),
                )
                .await
                {
                    Ok(Ok(_)) => p3.done("completion works"),
                    Ok(Err(e)) => p3.failed(&format!("{e}")),
                    Err(_) => p3.failed("timed out after 30s"),
                }
            } else {
                // Fallback: try direct provider constructors
                let secrets_cap = Cap::grant();
                let models_cap = Cap::grant();
                let result = match pt.name {
                    "Anthropic" => {
                        let prov =
                            continuum_models::AnthropicProvider::new(key_val.clone(), secrets_cap);
                        tokio::time::timeout(
                            Duration::from_secs(30),
                            prov.complete(
                                &models_cap,
                                continuum_core::model::CompletionRequest::new(
                                    continuum_core::ids::ModelId::new(pt.model_id),
                                    vec![continuum_core::model::Message::new(
                                        "user",
                                        "Reply with: OK",
                                    )],
                                )
                                .with_temperature(0.0)
                                .with_max_tokens(10),
                                continuum_core::CancellationToken::new(),
                            ),
                        )
                        .await
                    }
                    "OpenAI" => {
                        let prov =
                            continuum_models::OpenAIProvider::new(key_val.clone(), secrets_cap);
                        tokio::time::timeout(
                            Duration::from_secs(30),
                            prov.complete(
                                &models_cap,
                                continuum_core::model::CompletionRequest::new(
                                    continuum_core::ids::ModelId::new(pt.model_id),
                                    vec![continuum_core::model::Message::new(
                                        "user",
                                        "Reply with: OK",
                                    )],
                                )
                                .with_temperature(0.0)
                                .with_max_tokens(10),
                                continuum_core::CancellationToken::new(),
                            ),
                        )
                        .await
                    }
                    _ => {
                        p3.failed("provider not loadable");
                        continue;
                    }
                };
                match result {
                    Ok(Ok(_)) => p3.done("completion works"),
                    Ok(Err(e)) => p3.failed(&format!("{e}")),
                    Err(_) => p3.failed("timed out after 30s"),
                }
            }
        }
    }
    if !tested_any {
        println!("  - {:<30} no API keys configured", "LLM Providers");
        println!("         hint: Set ANTHROPIC_API_KEY, OPENAI_API_KEY, GOOGLE_API_KEY, etc.");
        println!("         hint: Or use `continuum login` to set up a provider.");
    }
    println!();

    // Step 3: Vector round-trip
    let p4 = crate::output::Progress::new("Testing vector index round-trip");
    let vec_idx = storage.vector();
    let test_id = continuum_core::ids::MemoryId::new();
    let test_vec: Vec<f32> = (0..384).map(|i| i as f32 * 0.01).collect();
    match vec_idx.upsert(test_id, test_vec.clone()).await {
        Ok(_) => {
            let results = vec_idx.search(&test_vec, 5).await.unwrap_or_default();
            if results
                .first()
                .map(|(id, _)| *id == test_id)
                .unwrap_or(false)
            {
                p4.done("vector round-trip OK");
            } else {
                p4.failed("recall mismatch");
            }
        }
        Err(e) => p4.failed(&format!("upsert failed: {e}")),
    }

    // Check for stuck sessions
    if args.check_stuck {
        println!();
        println!("--- Stuck session check ---\n");

        let runs = continuum_storage::RunRepo::new(storage.pool().clone());
        let heartbeats = continuum_storage::HeartbeatRepo::new(storage.pool().clone());
        let all_runs = runs.list().await.unwrap_or_default();

        let mut found_stuck = false;
        for run in &all_runs {
            let hb = heartbeats.latest(&run.id).await.unwrap_or(None);
            if let Some(row) = hb {
                if row.retry_count > 5 {
                    println!(
                        "  \u{2717} Session {} — stuck (retry loop: {} retries)",
                        run.id, row.retry_count
                    );
                    found_stuck = true;
                }
            }
        }
        if !found_stuck {
            println!("  \u{2713} No stuck sessions detected");
        }
        println!();
    }

    println!();
    if missing_required {
        println!(
            "  \u{274c} Required tools are missing. Run `continuum doctor --fix` for setup help."
        );
    } else {
        println!("  \u{2713} All required tools present.");
    }

    Ok(())
}
