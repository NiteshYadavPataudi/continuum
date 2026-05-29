/// Static slash-command catalog used by the TUI command palette.
#[derive(Debug, Clone, Copy)]
pub struct SlashCommand {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
}

pub const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "help",
        aliases: &[
            "h",
            "available",
            "commands",
            "command",
            "avaliable",
            "commnads",
            "lgaye",
        ],
        description: "Show available commands",
    },
    SlashCommand {
        name: "model",
        aliases: &["m"],
        description: "Switch AI model",
    },
    SlashCommand {
        name: "models",
        aliases: &["ms"],
        description: "List available models",
    },
    SlashCommand {
        name: "providers",
        aliases: &["provider", "p"],
        description: "List providers",
    },
    SlashCommand {
        name: "effort",
        aliases: &["e"],
        description: "Set reasoning level",
    },
    SlashCommand {
        name: "cost",
        aliases: &["c"],
        description: "Show token usage",
    },
    SlashCommand {
        name: "compact",
        aliases: &[],
        description: "Compress conversation context",
    },
    SlashCommand {
        name: "clear",
        aliases: &["cls"],
        description: "Clear screen",
    },
    SlashCommand {
        name: "status",
        aliases: &["s"],
        description: "Show agent status",
    },
    SlashCommand {
        name: "config",
        aliases: &["cfg"],
        description: "View or change settings",
    },
    SlashCommand {
        name: "doctor",
        aliases: &["doc"],
        description: "Run environment diagnostics",
    },
    SlashCommand {
        name: "init",
        aliases: &[],
        description: "Scaffold engineering docs",
    },
    SlashCommand {
        name: "index",
        aliases: &[],
        description: "Build repository index",
    },
    SlashCommand {
        name: "tools",
        aliases: &["t"],
        description: "List available tools",
    },
    SlashCommand {
        name: "memory",
        aliases: &["mem"],
        description: "Inspect or compact memory",
    },
    SlashCommand {
        name: "sessions",
        aliases: &["s"],
        description: "List recent sessions",
    },
    SlashCommand {
        name: "review",
        aliases: &["r"],
        description: "Code review current changes",
    },
    SlashCommand {
        name: "harden",
        aliases: &[],
        description: "Run security hardening",
    },
    SlashCommand {
        name: "diff",
        aliases: &["d"],
        description: "Show file changes",
    },
    SlashCommand {
        name: "git",
        aliases: &[],
        description: "Git operations",
    },
    SlashCommand {
        name: "share",
        aliases: &[],
        description: "Generate shareable link",
    },
    SlashCommand {
        name: "update",
        aliases: &["up"],
        description: "Update Continuum",
    },
    SlashCommand {
        name: "theme",
        aliases: &[],
        description: "Change color theme",
    },
    SlashCommand {
        name: "export",
        aliases: &[],
        description: "Export session as markdown",
    },
    SlashCommand {
        name: "version",
        aliases: &["v"],
        description: "Show version",
    },
    SlashCommand {
        name: "plugins",
        aliases: &[],
        description: "Manage plugins",
    },
    SlashCommand {
        name: "redact",
        aliases: &[],
        description: "Test secret redaction",
    },
    SlashCommand {
        name: "init-custom",
        aliases: &[],
        description: "Create sample custom commands",
    },
    SlashCommand {
        name: "exit",
        aliases: &["quit", "q"],
        description: "Exit Continuum",
    },
];

fn normalize(query: &str) -> String {
    query.trim().trim_start_matches('/').to_lowercase()
}

pub fn split_command_input(input: &str) -> (String, String) {
    let trimmed = input.trim_start_matches('/');
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let command = parts.next().unwrap_or("").to_string();
    let rest = parts.next().unwrap_or("").trim_start().to_string();
    (command, rest)
}

pub fn filter_slash_commands(query: &str) -> Vec<SlashCommand> {
    let query = normalize(query);
    let mut matches: Vec<SlashCommand> = SLASH_COMMANDS
        .iter()
        .copied()
        .filter(|cmd| {
            if query.is_empty() {
                return true;
            }
            cmd.name.starts_with(&query)
                || cmd
                    .aliases
                    .iter()
                    .any(|alias| alias.starts_with(&query) || query.starts_with(alias))
        })
        .collect();
    matches.sort_by(|a, b| a.name.cmp(b.name));
    matches
}

pub fn resolve_slash_command(query: &str) -> Option<SlashCommand> {
    let query = normalize(query);
    SLASH_COMMANDS
        .iter()
        .copied()
        .find(|cmd| cmd.name == query || cmd.aliases.iter().any(|alias| *alias == query))
}

pub fn command_help_lines() -> Vec<(&'static str, &'static str)> {
    SLASH_COMMANDS
        .iter()
        .map(|cmd| (cmd.name, cmd.description))
        .collect()
}
