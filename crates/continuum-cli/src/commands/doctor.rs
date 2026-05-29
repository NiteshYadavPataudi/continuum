use super::{CmdResult, DoctorArgs};
use continuum_core::caps::Cap;
use std::process::Command;

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
        name: "docker",
        required: false,
        hint: "Install Docker from https://docker.com",
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

    // Phase 2 substrate checks
    println!("--- Substrate checks ---\n");

    let project = args
        .project
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Step 1: Storage + migrations
    let state_dir = project.join(".continuum");
    let db_path = state_dir.join("state.db");
    match tokio::fs::create_dir_all(&state_dir).await {
        Ok(_) => println!("  \u{2713} {:<30} created", ".continuum/"),
        Err(e) => println!("  \u{2717} {:<30} {e}", ".continuum/"),
    }

    let storage = match continuum_storage::Storage::open(&db_path).await {
        Ok(s) => {
            println!("  \u{2713} {:<30} opened + migrated", db_path.display());
            s
        }
        Err(e) => {
            println!("  \u{2717} {:<30} {e}", "storage open");
            println!("\n  Substrate checks incomplete. Fix storage errors first.");
            return Ok(());
        }
    };

    // Step 2: Anthropic model (skip if no API key)
    let anthropic_key = std::env::var("ANTHROPIC_API_KEY").ok();
    if let Some(key) = &anthropic_key {
        let provider = continuum_models::AnthropicProvider::new(
            key.clone(),
            continuum_core::caps::Cap::grant(),
        );
        println!("  \u{2713} {:<30} configured (key present)", "Anthropic");

        // Try a one-token completion
        use continuum_core::model::ModelProvider;
        let req = continuum_core::model::CompletionRequest::new(
            continuum_core::ids::ModelId::new("claude-sonnet-4-20250514"),
            vec![continuum_core::model::Message::new(
                "user",
                "Reply with: OK",
            )],
        )
        .with_temperature(0.0)
        .with_max_tokens(10);
        match provider
            .complete(&Cap::grant(), req, continuum_core::CancellationToken::new())
            .await
        {
            Ok(_) => println!("  \u{2713} {:<30} completion works", "Anthropic"),
            Err(e) => println!("  \u{2717} {:<30} {e}", "Anthropic completion"),
        }
    } else {
        println!("  - {:<30} skipped (set ANTHROPIC_API_KEY)", "Anthropic");
    }

    // Step 3: Vector round-trip
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
                println!("  \u{2713} {:<30} vector round-trip OK", "VectorIndex");
            } else {
                println!("  \u{2717} {:<30} recall mismatch", "VectorIndex");
            }
        }
        Err(e) => println!("  \u{2717} {:<30} {e}", "VectorIndex upsert"),
    }

    // Check for stuck sessions
    if args.check_stuck {
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
            "  \u{274c} Required tools are missing. Run `continuum install` for setup guidance."
        );
    } else {
        println!("  \u{2713} All required tools present.");
    }

    Ok(())
}
